//! The app under test, and how it's built.

use core::fmt::Debug;
use core::time::Duration;

use bevy::MinimalPlugins;
use bevy::app::App;
use bevy::ecs::bundle::Bundle;
use bevy::ecs::entity::Entity;
use bevy::ecs::message::Message;
use bevy::ecs::query::{QueryFilter, ReadOnlyQueryData};
use bevy::ecs::system::{IntoSystem, RunSystemOnce};
use bevy::ecs::world::{EntityWorldMut, World};

pub use self::builder::TestAppBuilder;
use self::messages::{Recorded, watch};
use crate::check::{EntityCheck, QueryCheck};
use crate::describe;
use crate::until::Until;

mod builder;
mod messages;
#[cfg(test)]
mod tests;

/// A Bevy app under test: built from the plugins under test, headless, on
/// time that moves only a fixed tick at a time, when the test says.
///
/// Built by [`TestApp::builder`]; it has run its first update (`Startup`;
/// no tick yet, time starting on the next) by the time it's handed over.
pub struct TestApp {
    pub(super) app: App,
    /// How long a tick is: `Time<Fixed>`'s step as it was built.
    pub(super) step: Duration,
}

impl TestApp {
    /// A builder on Bevy's [`MinimalPlugins`]: time and tasks, no window
    /// or rendering.
    #[must_use]
    pub fn builder() -> TestAppBuilder {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        TestAppBuilder {
            app,
            step: None,
            steps_time: true,
        }
    }

    /// A builder on an empty app, for plugins that bring their own time.
    ///
    /// What a tick is is up to them: `build` leaves their time alone, but
    /// for a rate [`fixed_hz`](TestAppBuilder::fixed_hz) sets. Seconds
    /// count in their `Time<Fixed>`'s step (64 a second without one).
    #[must_use]
    pub fn bare_builder() -> TestAppBuilder {
        TestAppBuilder {
            app: App::new(),
            step: None,
            steps_time: false,
        }
    }

    /// The app, for anything this doesn't wrap.
    #[must_use]
    pub fn app(&self) -> &App {
        &self.app
    }

    /// The app, to change: add observers, systems, resources.
    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    /// Its world.
    #[must_use]
    pub fn world(&self) -> &World {
        self.app.world()
    }

    /// Its world, to change.
    pub fn world_mut(&mut self) -> &mut World {
        self.app.world_mut()
    }

    /// Runs one update: exactly one fixed tick.
    ///
    /// # Panics
    ///
    /// If the test has run past its timeout (see [`test`](crate::test)),
    /// so it stops instead of ticking on.
    pub fn tick(&mut self) {
        crate::__private::stop_if_timed_out();
        self.app.update();
    }

    /// Runs `n` updates, each exactly one fixed tick.
    pub fn ticks(&mut self, n: u32) {
        for _ in 0..n {
            self.tick();
        }
    }

    /// Runs as many ticks as `length` of game time takes, rounded to the
    /// nearest.
    pub fn advance(&mut self, length: Duration) {
        self.ticks(self.ticks_in(length));
    }

    /// Runs as many ticks as `seconds` of game time takes, rounded to the
    /// nearest: [`advance`](Self::advance) in seconds.
    ///
    /// # Panics
    ///
    /// If `seconds` is negative or not finite.
    #[track_caller]
    pub fn secs(&mut self, seconds: f32) {
        self.advance(crate::time::seconds(seconds));
    }

    /// How many ticks `length` of game time is, rounded to the nearest.
    pub(crate) fn ticks_in(&self, length: Duration) -> u32 {
        crate::time::nearest(length, self.step)
    }

    /// How long a tick is: `Time<Fixed>`'s step as it was built.
    pub(crate) fn step(&self) -> Duration {
        self.step
    }

    /// Spawns `bundle`.
    pub fn spawn(&mut self, bundle: impl Bundle) -> Entity {
        self.world_mut().spawn(bundle).id()
    }

    /// `entity`, to change: insert, remove, despawn.
    #[track_caller]
    pub fn entity_mut(&mut self, entity: Entity) -> EntityWorldMut<'_> {
        self.world_mut().entity_mut(entity)
    }

    /// `entity`, to assert on.
    pub fn entity(&self, entity: Entity) -> EntityCheck<'_> {
        EntityCheck::new(self.world(), entity)
    }

    /// What `D` matches (filtered by `F`), to assert on.
    ///
    /// It registers the components it names, so one never added to an
    /// entity is on none: `With` it matches nothing, `Without` everything.
    pub fn query<D, F>(&mut self) -> QueryCheck<'_, D, F>
    where
        D: ReadOnlyQueryData + 'static,
        F: QueryFilter + 'static,
    {
        let state = self.world_mut().query_filtered::<D, F>();
        QueryCheck::new(self.world(), state)
    }

    /// How many entities `F` matches: `count::<With<Player>>()`.
    ///
    /// It registers the components it names, as [`query`](Self::query)
    /// does, and changes nothing else.
    pub fn count<F: QueryFilter>(&mut self) -> usize {
        let world = self.world_mut();
        world.query_filtered::<(), F>().iter(world).count()
    }

    /// Writes `message`, for the next update to read.
    pub fn write<M: Message>(&mut self, message: M) {
        self.world_mut().write_message(message);
    }

    /// Records every `M` the app writes from now on.
    ///
    /// Nothing written before; otherwise as [`TestAppBuilder::watch`].
    pub fn watch<M: Message + Clone>(&mut self) {
        watch::<M>(&mut self.app);
    }

    /// Every `M` written since they were last read, oldest first; they're
    /// read after.
    ///
    /// # Panics
    ///
    /// If `M` isn't watched ([`watch`](Self::watch)).
    #[track_caller]
    pub fn read<M: Message>(&mut self) -> Vec<M> {
        let Some(mut recorded) = self.world_mut().get_resource_mut::<Recorded<M>>() else {
            panic!(
                "{} isn't watched: `watch::<M>()` it first",
                describe::short::<M>()
            );
        };
        core::mem::take(&mut recorded.written)
    }

    /// Asserts one `M` written since they were last read is as `matches`
    /// says; they stay unread.
    ///
    /// # Panics
    ///
    /// If `M` isn't watched ([`watch`](Self::watch)), or none written is as
    /// `matches` says, printing those written.
    #[track_caller]
    pub fn assert_wrote<M: Message + Debug>(&self, matches: impl Fn(&M) -> bool) {
        let Some(recorded) = self.world().get_resource::<Recorded<M>>() else {
            panic!(
                "{} isn't watched: `watch::<M>()` it first",
                describe::short::<M>()
            );
        };
        assert!(
            recorded.written.iter().any(matches),
            "no {} written as expected; written: {:#?}",
            describe::short::<M>(),
            recorded.written
        );
    }

    /// Runs `system` once, now, outside any schedule, and returns what it
    /// returns: a system tested on its own. Not the app's `run`, which runs
    /// its loop.
    ///
    /// # Panics
    ///
    /// If the system can't run: its parameters aren't valid in the world.
    #[track_caller]
    pub fn run_system<O: 'static, M>(&mut self, system: impl IntoSystem<(), O, M>) -> O {
        self.world_mut()
            .run_system_once(system)
            .unwrap_or_else(|err| panic!("the system couldn't run: {err}"))
    }

    /// Ticks until `holds` holds of the world: bounded by
    /// [`Until::within_ticks`] or [`Until::within`].
    pub fn until<P: FnMut(&World) -> bool>(&mut self, holds: P) -> Until<'_, TestApp, P> {
        Until::new(self, holds)
    }
}
