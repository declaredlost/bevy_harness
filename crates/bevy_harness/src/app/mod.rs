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

/// How many fixed ticks a second, unless the builder says.
const HZ: f64 = 64.0;

/// A Bevy app under test: built from the plugins under test, headless, on
/// time that moves only a fixed tick at a time, when the test says.
///
/// Built by [`TestApp::builder`]; it has run its first update (`Startup`;
/// no tick yet, time starting on the next) by the time it's handed over.
pub struct TestApp {
    pub(super) app: App,
    pub(super) hz: f64,
}

impl TestApp {
    /// A builder on Bevy's [`MinimalPlugins`]: time and tasks, no window
    /// or rendering.
    #[must_use]
    pub fn builder() -> TestAppBuilder {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        TestAppBuilder { app, hz: HZ }
    }

    /// A builder on an empty app, for plugins that bring their own time:
    /// what ticks it is up to them.
    #[must_use]
    pub fn bare_builder() -> TestAppBuilder {
        TestAppBuilder {
            app: App::new(),
            hz: HZ,
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
    pub fn tick(&mut self) {
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
        let tick = Duration::from_secs_f64(1.0 / self.hz).as_nanos().max(1);
        let ticks = (length.as_nanos() + tick / 2) / tick;
        u32::try_from(ticks).unwrap_or(u32::MAX)
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

    /// What `D` matches (filtered by `F`), to assert on. Read-only: it
    /// matches nothing if what it names was never registered.
    pub fn query<D, F>(&self) -> QueryCheck<'_, D, F>
    where
        D: ReadOnlyQueryData + 'static,
        F: QueryFilter + 'static,
    {
        QueryCheck::new(self.world())
    }

    /// How many entities `F` matches: `count::<(With<Player>,
    /// With<Controlled>)>()`. Read-only, so a condition
    /// ([`until`](Self::until)) can ask it; none, if what it names was
    /// never registered.
    pub fn count<F: QueryFilter>(&self) -> usize {
        self.world()
            .try_query_filtered::<(), F>()
            .map_or(0, |mut state| state.iter(self.world()).count())
    }

    /// Writes `message`, for the next update to read.
    pub fn write<M: Message>(&mut self, message: M) {
        self.world_mut().write_message(message);
    }

    /// Records every `M` the app writes from now on (see
    /// [`TestAppBuilder::watch`]).
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
        core::mem::take(&mut recorded.0)
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
            recorded.0.iter().any(matches),
            "no {} written as expected; written: {:#?}",
            describe::short::<M>(),
            recorded.0
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
