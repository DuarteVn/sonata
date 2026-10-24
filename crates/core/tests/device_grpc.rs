//! The emulator gRPC client against a fake `EmulatorController` server (research R2, R3, R10).

use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use sonata_core::device::grpc::{
    EmulatorClient, Image, ImageFormat, KeyboardEvent, TouchEvent, VmRunState, WheelEvent,
};
use tonic::codegen::{BoxFuture, Service, StdError, http};
use tonic::server::{
    ClientStreamingService, Grpc, NamedService, ServerStreamingService, UnaryService,
};
use tonic::{Request, Response, Status, Streaming};
use tonic_prost::ProstCodec;

const TOKEN: &str = "secret-token";

#[derive(Default)]
struct Seen {
    calls: Vec<String>,
    stream_formats: Vec<ImageFormat>,
}

#[derive(Clone, Default)]
struct Fake(Arc<Mutex<Seen>>);

impl Fake {
    fn log(&self, s: String) {
        self.0.lock().unwrap().calls.push(s);
    }
}

fn authorized<T>(req: &Request<T>) -> Result<(), Status> {
    let ok = req
        .metadata()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == format!("Bearer {TOKEN}"));
    if ok {
        Ok(())
    } else {
        Err(Status::unauthenticated("bad token"))
    }
}

macro_rules! unary {
    ($name:ident, $req:ty, $res:ty, |$fake:ident, $msg:ident| $body:expr) => {
        struct $name(Fake);
        impl UnaryService<$req> for $name {
            type Response = $res;
            type Future = BoxFuture<Response<$res>, Status>;
            fn call(&mut self, req: Request<$req>) -> Self::Future {
                let $fake = self.0.clone();
                Box::pin(async move {
                    authorized(&req)?;
                    let $msg = req.into_inner();
                    Ok(Response::new($body))
                })
            }
        }
    };
}

unary!(SendTouch, TouchEvent, (), |f, m| {
    let t = &m.touches[0];
    f.log(format!("touch {} {} {}", t.x, t.y, t.pressure));
});
unary!(SendKey, KeyboardEvent, (), |f, m| {
    f.log(format!("key {:?} {:?} {}", m.key, m.text, m.event_type));
});
unary!(GetScreenshot, ImageFormat, Image, |f, m| {
    f.log(format!("screenshot {}", m.format));
    Image {
        format: Some(ImageFormat {
            format: m.format,
            width: 2,
            height: 2,
            ..Default::default()
        }),
        image: b"\x89PNG".to_vec(),
        seq: 1,
        ..Default::default()
    }
});
unary!(SetVmState, VmRunState, (), |f, m| {
    f.log(format!("vm {}", m.state));
});

struct StreamScreenshot(Fake);
impl ServerStreamingService<ImageFormat> for StreamScreenshot {
    type Response = Image;
    type ResponseStream = tokio_stream::Iter<std::vec::IntoIter<Result<Image, Status>>>;
    type Future = BoxFuture<Response<Self::ResponseStream>, Status>;
    fn call(&mut self, req: Request<ImageFormat>) -> Self::Future {
        let fake = self.0.clone();
        Box::pin(async move {
            authorized(&req)?;
            let fmt = req.into_inner();
            fake.0.lock().unwrap().stream_formats.push(fmt.clone());
            let frames = (1..=3)
                .map(|seq| {
                    Ok(Image {
                        format: Some(ImageFormat {
                            width: fmt.width,
                            height: fmt.height,
                            ..fmt.clone()
                        }),
                        image: vec![seq as u8; (fmt.width * fmt.height * 4) as usize],
                        seq,
                        ..Default::default()
                    })
                })
                .collect::<Vec<_>>();
            Ok(Response::new(tokio_stream::iter(frames)))
        })
    }
}

struct InjectWheel(Fake);
impl ClientStreamingService<WheelEvent> for InjectWheel {
    type Response = ();
    type Future = BoxFuture<Response<()>, Status>;
    fn call(&mut self, req: Request<Streaming<WheelEvent>>) -> Self::Future {
        let fake = self.0.clone();
        Box::pin(async move {
            authorized(&req)?;
            let mut s = req.into_inner();
            while let Some(ev) = s.message().await? {
                fake.log(format!("wheel {} {}", ev.dx, ev.dy));
            }
            Ok(Response::new(()))
        })
    }
}

#[derive(Clone)]
struct Svc(Fake);

impl NamedService for Svc {
    const NAME: &'static str = "android.emulation.control.EmulatorController";
}

impl<B> Service<http::Request<B>> for Svc
where
    B: tonic::codegen::Body + Send + 'static,
    B::Error: Into<StdError> + Send + 'static,
{
    type Response = http::Response<tonic::body::Body>;
    type Error = Infallible;
    type Future = BoxFuture<Self::Response, Infallible>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<B>) -> Self::Future {
        let f = self.0.clone();
        let path = req
            .uri()
            .path()
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_string();
        Box::pin(async move {
            let res = match path.as_str() {
                "sendTouch" => grpc().unary(SendTouch(f), req).await,
                "sendKey" => grpc().unary(SendKey(f), req).await,
                "getScreenshot" => grpc().unary(GetScreenshot(f), req).await,
                "setVmState" => grpc().unary(SetVmState(f), req).await,
                "streamScreenshot" => grpc().server_streaming(StreamScreenshot(f), req).await,
                "injectWheel" => grpc().client_streaming(InjectWheel(f), req).await,
                _ => Status::unimplemented(path).into_http(),
            };
            Ok(res)
        })
    }
}

fn grpc<
    T: prost::Message + Default + Send + 'static,
    U: prost::Message + Default + Send + 'static,
>() -> Grpc<ProstCodec<T, U>> {
    Grpc::new(ProstCodec::default())
}

/// Starts the fake server on a free localhost port on its own runtime thread.
fn serve() -> (u16, Fake) {
    let fake = Fake::default();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let svc = Svc(fake.clone());
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener).unwrap();
            tonic::transport::Server::builder()
                .add_service(svc)
                .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
                .await
                .unwrap();
        });
    });
    (port, fake)
}

#[test]
fn input_calls_carry_the_token() {
    let (port, fake) = serve();
    let c = EmulatorClient::connect(port, TOKEN);
    c.touch(10, 20, true).unwrap();
    c.touch(10, 20, false).unwrap();
    c.text("hi").unwrap();
    c.key("GoBack").unwrap();
    c.wheel(0, -240).unwrap();
    c.shutdown().unwrap();
    let png = c.screenshot_png().unwrap();
    assert_eq!(png, b"\x89PNG");
    let calls = fake.0.lock().unwrap().calls.clone();
    assert_eq!(
        calls,
        vec![
            "touch 10 20 1",
            "touch 10 20 0",
            "key \"\" \"hi\" 2",
            "key \"GoBack\" \"\" 2",
            "wheel 0 -240",
            "vm 5",
            "screenshot 0",
        ]
    );
}

#[test]
fn wrong_token_is_rejected() {
    let (port, _) = serve();
    let c = EmulatorClient::connect(port, "nope");
    let err = c.touch(1, 1, true).unwrap_err();
    assert!(err.contains("bad token"), "{err}");
}

#[test]
fn screen_stream_asks_for_scaled_rgba_and_delivers_frames() {
    let (port, fake) = serve();
    let c = EmulatorClient::connect(port, TOKEN);
    let got = Arc::new(Mutex::new(Vec::new()));
    let sink = got.clone();
    let _stream = c.stream(36, 80, move |frame| {
        sink.lock()
            .unwrap()
            .push((frame.width, frame.height, frame.seq, frame.rgba.len()));
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while got.lock().unwrap().len() < 3 && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        *got.lock().unwrap(),
        vec![
            (36, 80, 1, 36 * 80 * 4),
            (36, 80, 2, 11520),
            (36, 80, 3, 11520)
        ]
    );
    let fmt = fake.0.lock().unwrap().stream_formats[0].clone();
    assert_eq!((fmt.format, fmt.width, fmt.height), (1, 36, 80));
}

#[test]
fn frames_bigger_than_four_megabytes_arrive() {
    // 1080×1200 RGBA is 5.2 MB, above tonic's default 4 MB message limit (found on a real AVD).
    let (port, _) = serve();
    let c = EmulatorClient::connect(port, TOKEN);
    let got = Arc::new(Mutex::new(0usize));
    let sink = got.clone();
    let _stream = c.stream(1080, 1200, move |f| *sink.lock().unwrap() = f.rgba.len());
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while *got.lock().unwrap() == 0 && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(*got.lock().unwrap(), 1080 * 1200 * 4);
}
