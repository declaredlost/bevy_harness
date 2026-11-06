//! What [`Until`](super::Until) ticks: an app, or several together.

use core::time::Duration;

use bevy::ecs::world::World;

use crate::app::TestApp;
use crate::world::TestWorld;

/// What [`Until`](super::Until) ticks: an app, or several together; and what it
/// asks whether something holds of. Sealed: [`TestApp`] and [`TestWorld`]
/// are all there is.
pub trait Ticked: sealed::Sealed {
    /// What a condition is asked of: an app's world, or the apps.
    type View;

    /// It, as a condition sees it.
    fn view(&self) -> &Self::View;

    /// Runs one tick.
    fn tick(&mut self);

    /// How many ticks `length` of game time is, rounded to the nearest.
    fn ticks_in(&self, length: Duration) -> u32;
}

mod sealed {
    /// Keeps [`Ticked`](super::Ticked) to this crate's types.
    pub trait Sealed {}

    impl Sealed for crate::TestApp {}
    impl Sealed for crate::TestWorld {}
}

impl Ticked for TestApp {
    type View = World;

    fn view(&self) -> &World {
        self.world()
    }

    fn tick(&mut self) {
        self.tick();
    }

    fn ticks_in(&self, length: Duration) -> u32 {
        self.ticks_in(length)
    }
}

impl Ticked for TestWorld {
    type View = TestWorld;

    fn view(&self) -> &TestWorld {
        self
    }

    fn tick(&mut self) {
        self.tick();
    }

    fn ticks_in(&self, length: Duration) -> u32 {
        self.ticks_in(length)
    }
}
