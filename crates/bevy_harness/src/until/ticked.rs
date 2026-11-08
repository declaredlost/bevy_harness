//! What [`Until`](super::Until) ticks: an app, or several together.

use core::time::Duration;

use crate::app::TestApp;
use crate::world::TestWorld;

/// What [`Until`](super::Until) ticks, and asks its condition of: an app,
/// or several together. Sealed: [`TestApp`] and [`TestWorld`] are all
/// there is.
pub trait Ticked: sealed::Sealed {
    /// Runs one tick.
    fn tick(&mut self);

    /// How long a tick is: its (first) app's fixed step.
    fn step(&self) -> Duration;
}

mod sealed {
    /// Keeps [`Ticked`](super::Ticked) to this crate's types.
    pub trait Sealed {}

    impl Sealed for crate::TestApp {}
    impl Sealed for crate::TestWorld {}
}

impl Ticked for TestApp {
    fn tick(&mut self) {
        self.tick();
    }

    fn step(&self) -> Duration {
        self.step()
    }
}

impl Ticked for TestWorld {
    fn tick(&mut self) {
        self.tick();
    }

    fn step(&self) -> Duration {
        self.step()
    }
}
