//! What `#[bevy_harness::test]` expands to calls: not for writing by hand.

use core::fmt::Debug;
use std::cell::OnceCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use std::{panic, thread};

thread_local! {
    /// Set on a timed test's thread once it runs past its timeout.
    static TIMED_OUT: OnceCell<Arc<AtomicBool>> = const { OnceCell::new() };
}

/// Builds what the test runs on with `setup` and runs `body` on it.
///
/// What it builds is its app, or a fixture holding it; it runs right
/// here.
pub fn run<T>(setup: impl FnOnce() -> T, body: impl FnOnce(&mut T)) {
    body(&mut setup());
}

/// [`run`], on a thread of its own, failing if it isn't done by `timeout`.
///
/// A panic in it is passed on. Past its timeout, its app stops at its next
/// tick, so it doesn't tick on unwatched.
pub fn run_with_timeout<T>(
    setup: impl FnOnce() -> T + Send + 'static,
    body: impl FnOnce(&mut T) + Send + 'static,
    timeout: Duration,
) {
    let timed_out = Arc::new(AtomicBool::new(false));
    let (done, finished) = mpsc::channel();
    let running = thread::spawn({
        let timed_out = Arc::clone(&timed_out);
        move || {
            TIMED_OUT.with(|cell| {
                cell.get_or_init(|| timed_out);
            });
            run(setup, body);
            let _ = done.send(());
        }
    });
    match finished.recv_timeout(timeout) {
        Ok(()) => {}
        Err(mpsc::RecvTimeoutError::Timeout) => {
            timed_out.store(true, Ordering::Relaxed);
            panic!("the test didn't finish within {timeout:?}")
        }
        // It panicked before it was done: its panic is the failure.
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            if let Err(panic) = running.join() {
                panic::resume_unwind(panic);
            }
        }
    }
}

/// Unwinds the test's thread if its test has run past its timeout.
///
/// Quietly: the test has failed already, saying so.
pub(crate) fn stop_if_timed_out() {
    let timed_out = TIMED_OUT.with(|cell| {
        cell.get()
            .is_some_and(|timed_out| timed_out.load(Ordering::Relaxed))
    });
    if timed_out {
        panic::resume_unwind(Box::new("the test ran past its timeout"));
    }
}

/// Fails the test if its body returned an error, printing it.
///
/// # Panics
///
/// If `result` is an error.
#[track_caller]
pub fn ok<E: Debug>(result: Result<(), E>) {
    if let Err(error) = result {
        panic!("the test returned an error: {error:?}");
    }
}
