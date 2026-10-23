//! Client for the Android emulator's gRPC control service (research R10).
//!
//! The message structs mirror the public `emulator_controller.proto` (package
//! `android.emulation.control`, Copyright The Android Open Source Project, Apache License 2.0):
//! only the messages Sonata uses, with the same field tags. Written by hand so building Sonata
//! needs no `protoc`.

use std::sync::OnceLock;
use std::time::Duration;

use tonic::client::Grpc;
use tonic::codegen::http::uri::PathAndQuery;
use tonic::metadata::{Ascii, MetadataValue};
use tonic::transport::{Channel, Endpoint};
use tonic::{Request, Response};
use tonic_prost::ProstCodec;

/// `ImageFormat.format` values.
pub const IMG_PNG: i32 = 0;
pub const IMG_RGBA8888: i32 = 1;

#[derive(Clone, PartialEq, prost::Message)]
pub struct ImageFormat {
    #[prost(int32, tag = "1")]
    pub format: i32,
    #[prost(uint32, tag = "3")]
    pub width: u32,
    #[prost(uint32, tag = "4")]
    pub height: u32,
    #[prost(uint32, tag = "5")]
    pub display: u32,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Image {
    #[prost(message, optional, tag = "1")]
    pub format: Option<ImageFormat>,
    #[prost(bytes = "vec", tag = "4")]
    pub image: Vec<u8>,
    #[prost(uint32, tag = "5")]
    pub seq: u32,
    #[prost(uint64, tag = "6")]
    pub timestamp_us: u64,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct Touch {
    #[prost(int32, tag = "1")]
    pub x: i32,
    #[prost(int32, tag = "2")]
    pub y: i32,
    #[prost(int32, tag = "3")]
    pub identifier: i32,
    #[prost(int32, tag = "4")]
    pub pressure: i32,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct TouchEvent {
    #[prost(message, repeated, tag = "1")]
    pub touches: Vec<Touch>,
    #[prost(int32, tag = "2")]
    pub display: i32,
}

/// `KeyboardEvent.eventType`: keydown 0, keyup 1, keypress 2.
pub const KEY_PRESS: i32 = 2;

#[derive(Clone, PartialEq, prost::Message)]
pub struct KeyboardEvent {
    #[prost(int32, tag = "2")]
    pub event_type: i32,
    #[prost(string, tag = "4")]
    pub key: String,
    #[prost(string, tag = "5")]
    pub text: String,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct MouseEvent {
    #[prost(int32, tag = "1")]
    pub x: i32,
    #[prost(int32, tag = "2")]
    pub y: i32,
    #[prost(int32, tag = "3")]
    pub buttons: i32,
    #[prost(int32, tag = "4")]
    pub display: i32,
}

/// 120 units are one wheel click.
#[derive(Clone, PartialEq, prost::Message)]
pub struct WheelEvent {
    #[prost(int32, tag = "1")]
    pub dx: i32,
    #[prost(int32, tag = "2")]
    pub dy: i32,
    #[prost(int32, tag = "3")]
    pub display: i32,
}

/// `VmRunState.state`: SHUTDOWN is 5.
pub const VM_SHUTDOWN: i32 = 5;

#[derive(Clone, PartialEq, prost::Message)]
pub struct VmRunState {
    #[prost(int32, tag = "1")]
    pub state: i32,
}

/// One decoded screen frame.
#[derive(Debug, Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub seq: u32,
    pub rgba: Vec<u8>,
}

const SERVICE: &str = "/android.emulation.control.EmulatorController/";
/// Every call fails after this, so a hung emulator cannot block an agent command forever.
const CALL_TIMEOUT: Duration = Duration::from_secs(10);

/// One runtime for every emulator connection; the rest of core is thread based.
fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("sonata-grpc")
            .enable_all()
            .build()
            .expect("gRPC runtime")
    })
}

/// A connection to one emulator on localhost. Blocking methods, safe to call from any thread
/// except the runtime's own.
#[derive(Clone)]
pub struct EmulatorClient {
    channel: Channel,
    auth: MetadataValue<Ascii>,
}

/// Ends the screen stream when dropped.
pub struct ScreenStream(tokio::task::JoinHandle<()>);

impl Drop for ScreenStream {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl EmulatorClient {
    /// Connects lazily to `127.0.0.1:<port>`; the token goes in each call's metadata, never
    /// on a command line.
    pub fn connect(port: u16, token: &str) -> Self {
        let _guard = runtime().enter();
        let channel = Endpoint::from_shared(format!("http://127.0.0.1:{port}"))
            .expect("valid endpoint")
            .connect_timeout(Duration::from_secs(3))
            .connect_lazy();
        let auth = format!("Bearer {token}")
            .parse()
            .unwrap_or_else(|_| MetadataValue::from_static("Bearer invalid"));
        Self { channel, auth }
    }

    fn request<T>(&self, msg: T) -> Request<T> {
        let mut req = Request::new(msg);
        req.metadata_mut()
            .insert("authorization", self.auth.clone());
        req.set_timeout(CALL_TIMEOUT);
        req
    }

    fn unary<Req, Res>(&self, method: &str, msg: Req) -> Result<Res, String>
    where
        Req: prost::Message + Send + Sync + 'static,
        Res: prost::Message + Default + Send + Sync + 'static,
    {
        let path =
            PathAndQuery::try_from(format!("{SERVICE}{method}")).map_err(|e| e.to_string())?;
        let req = self.request(msg);
        let mut grpc = Grpc::new(self.channel.clone());
        runtime().block_on(async move {
            grpc.ready()
                .await
                .map_err(|e| format!("emulator not reachable: {e}"))?;
            grpc.unary(req, path, ProstCodec::default())
                .await
                .map(Response::into_inner)
                .map_err(|s| s.message().to_string())
        })
    }

    /// One finger (identifier 0) down or released at device pixels.
    pub fn touch(&self, x: i32, y: i32, down: bool) -> Result<(), String> {
        let touch = Touch {
            x,
            y,
            identifier: 0,
            pressure: i32::from(down),
        };
        self.unary(
            "sendTouch",
            TouchEvent {
                touches: vec![touch],
                display: 0,
            },
        )
    }

    /// Printable ASCII text; other characters are not translated by the emulator.
    pub fn text(&self, text: &str) -> Result<(), String> {
        self.unary(
            "sendKey",
            KeyboardEvent {
                event_type: KEY_PRESS,
                key: String::new(),
                text: text.to_string(),
            },
        )
    }

    /// A W3C key name such as `Enter`, `Backspace`, `GoBack`, `GoHome`, `AppSwitch`.
    pub fn key(&self, key: &str) -> Result<(), String> {
        self.unary(
            "sendKey",
            KeyboardEvent {
                event_type: KEY_PRESS,
                key: key.to_string(),
                text: String::new(),
            },
        )
    }

    /// Moves the mouse pointer without pressing a button (wheel events go where it is).
    pub fn mouse_move(&self, x: i32, y: i32) -> Result<(), String> {
        self.unary(
            "sendMouse",
            MouseEvent {
                x,
                y,
                buttons: 0,
                display: 0,
            },
        )
    }

    /// One wheel movement; 120 units are one click.
    pub fn wheel(&self, dx: i32, dy: i32) -> Result<(), String> {
        let path =
            PathAndQuery::from_static("/android.emulation.control.EmulatorController/injectWheel");
        let ev = WheelEvent { dx, dy, display: 0 };
        let req = self.request(tokio_stream::iter([ev]));
        let mut grpc = Grpc::new(self.channel.clone());
        runtime().block_on(async move {
            grpc.ready()
                .await
                .map_err(|e| format!("emulator not reachable: {e}"))?;
            grpc.client_streaming::<_, _, (), _>(req, path, ProstCodec::default())
                .await
                .map(|_| ())
                .map_err(|s| s.message().to_string())
        })
    }

    /// Asks the emulator to shut down (used when `adb emu kill` is not available).
    pub fn shutdown(&self) -> Result<(), String> {
        self.unary("setVmState", VmRunState { state: VM_SHUTDOWN })
    }

    /// The screen at full device size, as PNG bytes.
    pub fn screenshot_png(&self) -> Result<Vec<u8>, String> {
        let img: Image = self.unary(
            "getScreenshot",
            ImageFormat {
                format: IMG_PNG,
                ..Default::default()
            },
        )?;
        Ok(img.image)
    }

    /// Streams RGBA frames scaled by the emulator to `width`×`height` (0 keeps the device size).
    /// Frames arrive only when the screen changes. Empty images (inactive display) are skipped.
    pub fn stream(
        &self,
        width: u32,
        height: u32,
        mut on_frame: impl FnMut(Frame) + Send + 'static,
    ) -> ScreenStream {
        let path = PathAndQuery::from_static(
            "/android.emulation.control.EmulatorController/streamScreenshot",
        );
        let mut req = Request::new(ImageFormat {
            format: IMG_RGBA8888,
            width,
            height,
            display: 0,
        });
        req.metadata_mut()
            .insert("authorization", self.auth.clone());
        let mut grpc = Grpc::new(self.channel.clone());
        ScreenStream(runtime().spawn(async move {
            if grpc.ready().await.is_err() {
                return;
            }
            let Ok(res) = grpc
                .server_streaming::<_, Image, _>(req, path, ProstCodec::default())
                .await
            else {
                return;
            };
            let mut stream = res.into_inner();
            while let Ok(Some(img)) = stream.message().await {
                let Some(fmt) = img.format else { continue };
                if img.image.is_empty() || fmt.width == 0 || fmt.height == 0 {
                    continue;
                }
                on_frame(Frame {
                    width: fmt.width,
                    height: fmt.height,
                    seq: img.seq,
                    rgba: img.image,
                });
            }
        }))
    }
}
