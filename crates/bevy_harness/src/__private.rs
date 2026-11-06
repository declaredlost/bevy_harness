//! What `#[bevy_harness::test]` expands to calls: not for writing by hand.

use std::sync::mpsc;
use std::time::Duration;
use std::{panic, thread};

/// Builds what the test runs on with `setup` (its app, or a fixture
/// holding it) and runs `body` on it: right here, or, with a `timeout`, on
/// a thread of its own, failing if it isn't done by then and passing on
/// its panic if it panics.
pub fn run<T>(
    setup: impl FnOnce() -> T + Send + 'static,
    body: impl FnOnce(&mut T) + Send + 'static,
    timeout: Option<Duration>,
) {
    let test = move || body(&mut setup());
    let Some(timeout) = timeout else {
        test();
        return;
    };
    let (done, finished) = mpsc::channel();
    let running = thread::spawn(move || {
        test();
        let _ = done.send(());
    });
    match finished.recv_timeout(timeout) {
        Ok(()) => {}
        Err(mpsc::RecvTimeoutError::Timeout) => {
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
