//! Pending human confirmations for land and remove requests made from the CLI (research R9).

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use uuid::Uuid;

/// The human's answer in the confirmation dialog.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Answer {
    pub accepted: bool,
    pub discard_changes: bool,
    pub delete_branch: bool,
}

#[derive(Default)]
pub struct Confirmations {
    /// request id → (lane's terminal, where the answer goes)
    pending: Mutex<HashMap<Uuid, (Uuid, Sender<Answer>)>>,
}

impl Confirmations {
    /// Opens a request for the lane of `node`; `None` when one is already waiting.
    pub fn open(&self, node: Uuid) -> Option<(Uuid, Receiver<Answer>)> {
        let mut pending = self.lock();
        if pending.values().any(|(n, _)| *n == node) {
            return None;
        }
        let (tx, rx) = channel();
        let id = Uuid::new_v4();
        pending.insert(id, (node, tx));
        Some((id, rx))
    }

    /// Delivers the answer; false when the request is no longer waiting.
    pub fn answer(&self, request: Uuid, answer: Answer) -> bool {
        match self.lock().remove(&request) {
            Some((_, tx)) => tx.send(answer).is_ok(),
            None => false,
        }
    }

    pub fn close(&self, request: Uuid) {
        self.lock().remove(&request);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<Uuid, (Uuid, Sender<Answer>)>> {
        self.pending.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Waits for an answer, calling `tick` every `every` while waiting. `None` on timeout.
pub fn wait(
    rx: &Receiver<Answer>,
    timeout: Duration,
    every: Duration,
    tick: &mut dyn FnMut(),
) -> Option<Answer> {
    let deadline = Instant::now() + timeout;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return None;
        }
        match rx.recv_timeout(left.min(every)) {
            Ok(answer) => return Some(answer),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => tick(),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHORT: Duration = Duration::from_millis(50);

    #[test]
    fn answer_reaches_the_waiter() {
        let c = Confirmations::default();
        let node = Uuid::new_v4();
        let (id, rx) = c.open(node).unwrap();
        let yes = Answer {
            accepted: true,
            delete_branch: true,
            ..Answer::default()
        };
        assert!(c.answer(id, yes));
        assert_eq!(wait(&rx, SHORT, SHORT, &mut || {}), Some(yes));
        assert!(!c.answer(id, yes), "answered requests are gone");
        assert!(c.open(node).is_some(), "the lane can be asked again");
    }

    #[test]
    fn declined_is_an_answer() {
        let c = Confirmations::default();
        let (id, rx) = c.open(Uuid::new_v4()).unwrap();
        c.answer(id, Answer::default());
        assert_eq!(
            wait(&rx, SHORT, SHORT, &mut || {}).map(|a| a.accepted),
            Some(false)
        );
    }

    #[test]
    fn timeout_returns_none_and_ticks() {
        let c = Confirmations::default();
        let (id, rx) = c.open(Uuid::new_v4()).unwrap();
        let mut ticks = 0;
        let got = wait(
            &rx,
            Duration::from_millis(120),
            Duration::from_millis(30),
            &mut || ticks += 1,
        );
        assert_eq!(got, None);
        assert!(ticks >= 2, "{ticks}");
        c.close(id);
        assert!(!c.answer(id, Answer::default()));
    }

    #[test]
    fn one_request_per_lane() {
        let c = Confirmations::default();
        let node = Uuid::new_v4();
        let (id, _rx) = c.open(node).unwrap();
        assert!(c.open(node).is_none());
        assert!(c.open(Uuid::new_v4()).is_some());
        c.close(id);
        assert!(c.open(node).is_some());
    }
}
