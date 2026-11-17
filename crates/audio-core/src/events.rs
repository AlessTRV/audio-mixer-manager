//! Single event channel from the audio layer to the application.
//!
//! Events are deliberately coarse: they say *what kind* of thing changed and
//! the application takes a fresh snapshot. Windows delivers the underlying
//! callbacks on its own threads, where blocking or calling back into the same
//! object is not allowed, so the callbacks only push a value here.

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

/// A change in the audio state of the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioEvent {
    /// An output device was added, removed, enabled, disabled or renamed.
    DevicesChanged,
    /// The system default output device changed.
    DefaultOutputChanged,
    /// A session was created, expired, disconnected or changed state.
    SessionsChanged,
    /// Volume or mute of a session changed.
    SessionVolumeChanged,
}

type Waker = Arc<dyn Fn() + Send + Sync>;

/// Sending half of the event channel. Cheap to clone, usable from any thread.
#[derive(Clone)]
pub struct EventSender {
    tx: Sender<AudioEvent>,
    waker: Option<Waker>,
}

impl EventSender {
    /// Queues an event. Never blocks; a closed channel is ignored.
    pub fn send(&self, event: AudioEvent) {
        if self.tx.send(event).is_ok()
            && let Some(waker) = &self.waker
        {
            waker();
        }
    }

    /// Returns a sender that also calls `waker` after each event, for example
    /// to wake an event loop. The waker runs on Windows' notification threads
    /// and must return quickly.
    #[must_use]
    pub fn with_waker(mut self, waker: impl Fn() + Send + Sync + 'static) -> Self {
        self.waker = Some(Arc::new(waker));
        self
    }
}

impl std::fmt::Debug for EventSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventSender")
            .field("waker", &self.waker.is_some())
            .finish_non_exhaustive()
    }
}

/// Creates the event channel.
#[must_use]
pub fn channel() -> (EventSender, Receiver<AudioEvent>) {
    let (tx, rx) = mpsc::channel();
    (EventSender { tx, waker: None }, rx)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn events_arrive_in_order() {
        let (tx, rx) = channel();
        tx.send(AudioEvent::DevicesChanged);
        tx.clone().send(AudioEvent::SessionsChanged);
        assert_eq!(rx.try_recv(), Ok(AudioEvent::DevicesChanged));
        assert_eq!(rx.try_recv(), Ok(AudioEvent::SessionsChanged));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn waker_runs_once_per_event() {
        let count = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&count);
        let (tx, _rx) = channel();
        let tx = tx.with_waker(move || {
            seen.fetch_add(1, Ordering::SeqCst);
        });
        tx.send(AudioEvent::DefaultOutputChanged);
        tx.send(AudioEvent::SessionVolumeChanged);
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn send_after_receiver_dropped_is_silent() {
        let count = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&count);
        let (tx, rx) = channel();
        let tx = tx.with_waker(move || {
            seen.fetch_add(1, Ordering::SeqCst);
        });
        drop(rx);
        tx.send(AudioEvent::DevicesChanged);
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }
}
