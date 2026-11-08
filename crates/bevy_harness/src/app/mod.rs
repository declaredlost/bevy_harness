//! The app under test, and how it's built.

use core::fmt::Debug;
use core::time::Duration;

use bevy::MinimalPlugins;
use bevy::app::App;
use bevy::ecs::bundle::Bundle;
use bevy::ecs::component::{Component, Mutable};
use bevy::ecs::entity::Entity;
use bevy::ecs::event::Event;
use bevy::ecs::message::Message;
use bevy::ecs::name::Name;
use bevy::ecs::query::{QueryFilter, ReadOnlyQueryData, With};
use bevy::ecs::resource::Resource;
use bevy::ecs::system::{IntoSystem, RunSystemOnce};
use bevy::ecs::world::{EntityWorldMut, FromWorld, Mut, World};

pub use self::builder::TestAppBuilder;
use self::messages::{Recorded, watch};
use self::triggered::{Triggered, watch_triggered};
use crate::check::{EntityCheck, QueryCheck};
use crate::describe;
use crate::until::Until;

mod builder;
mod messages;
#[cfg(test)]
mod tests;
mod triggered;

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

    /// The one `C` on an entity `F` matches, to change.
    ///
    /// It registers the components it names, as [`query`](Self::query)
    /// does. To read one, [`query`](Self::query)'s
    /// [`single`](QueryCheck::single).
    ///
    /// # Panics
    ///
    /// If `F` matches no entity with a `C`, or more than one, saying how
    /// many.
    #[track_caller]
    pub fn single_mut<C, F>(&mut self) -> Mut<'_, C>
    where
        C: Component<Mutability = Mutable>,
        F: QueryFilter + 'static,
    {
        let world = self.world_mut();
        let mut matched = world.query_filtered::<Entity, (With<C>, F)>();
        let Ok(entity) = matched.single(world) else {
            let found = matched.iter(world).count();
            panic!(
                "{} matches {found}, not one",
                describe::query::<&mut C, F>()
            );
        };
        world
            .get_mut::<C>(entity)
            .expect("an entity a query matched with its `C`")
    }

    /// The one entity called `name` (its [`Name`]).
    ///
    /// # Panics
    ///
    /// If none is, naming those there are; or more than one is.
    #[track_caller]
    #[must_use]
    pub fn named(&self, name: &str) -> Entity {
        self.try_named(name).unwrap_or_else(|| {
            let mut listed: Vec<&str> = self.names().map(|(_, other)| other).collect();
            listed.sort_unstable();
            panic!("nothing is called {name:?}; named are {listed:?}")
        })
    }

    /// The one entity called `name` (its [`Name`]), if one is: for a
    /// condition waiting on it.
    ///
    /// # Panics
    ///
    /// If more than one is: which one a test means is a mistake to guess.
    #[track_caller]
    #[must_use]
    pub fn try_named(&self, name: &str) -> Option<Entity> {
        let called: Vec<Entity> = self
            .names()
            .filter(|(_, other)| *other == name)
            .map(|(entity, _)| entity)
            .collect();
        assert!(
            called.len() <= 1,
            "{} entities are called {name:?}: {called:?}",
            called.len()
        );
        called.first().copied()
    }

    /// Every named entity, and its name.
    fn names(&self) -> impl Iterator<Item = (Entity, &str)> {
        let world = self.world();
        world
            .try_query::<(Entity, &Name)>()
            .map(|mut query| {
                query
                    .iter(world)
                    .map(|(entity, name)| (entity, name.as_str()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
            .into_iter()
    }

    /// Its `R`.
    ///
    /// # Panics
    ///
    /// If it has no `R`.
    #[track_caller]
    #[must_use]
    pub fn resource<R: Resource>(&self) -> &R {
        self.world()
            .get_resource::<R>()
            .unwrap_or_else(|| panic!("there's no {} resource", describe::short::<R>()))
    }

    /// Its `R`, to change.
    ///
    /// # Panics
    ///
    /// If it has no `R`.
    #[track_caller]
    pub fn resource_mut<R: Resource<Mutability = Mutable>>(&mut self) -> Mut<'_, R> {
        self.world_mut()
            .get_resource_mut::<R>()
            .unwrap_or_else(|| panic!("there's no {} resource", describe::short::<R>()))
    }

    /// Inserts `resource`, replacing any `R` there was.
    pub fn insert_resource<R: Resource>(&mut self, resource: R) {
        self.world_mut().insert_resource(resource);
    }

    /// Inserts an `R` made from the world (its `Default`, often), unless
    /// there's one already.
    pub fn init_resource<R: Resource + FromWorld>(&mut self) {
        self.world_mut().init_resource::<R>();
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
            panic!("{}", unwatched::<M>("watch"));
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
    pub fn assert_wrote<M: Message + Debug>(&self, matches: impl FnMut(&M) -> bool) {
        let written = self.written::<M>();
        assert!(
            written.iter().any(matches),
            "no {} written as expected; written: {written:#?}",
            describe::short::<M>()
        );
    }

    /// Asserts no `M` written since they were last read is as `matches`
    /// says; they stay unread. `|_| true` asserts none was written at all.
    ///
    /// # Panics
    ///
    /// If `M` isn't watched ([`watch`](Self::watch)), or one written is as
    /// `matches` says, printing those that are.
    #[track_caller]
    pub fn assert_wrote_none<M: Message + Debug>(&self, mut matches: impl FnMut(&M) -> bool) {
        let unexpected: Vec<&M> = self
            .written::<M>()
            .iter()
            .filter(|message| matches(message))
            .collect();
        assert!(
            unexpected.is_empty(),
            "{} {} written that shouldn't be: {unexpected:#?}",
            unexpected.len(),
            describe::short::<M>()
        );
    }

    /// Every `M` written since they were last read, unread.
    #[track_caller]
    fn written<M: Message>(&self) -> &[M] {
        match self.world().get_resource::<Recorded<M>>() {
            Some(recorded) => &recorded.written,
            None => panic!("{}", unwatched::<M>("watch")),
        }
    }

    /// Triggers `event` now, running its observers, and then the commands
    /// they queued: what they spawn, insert and trigger in turn is there
    /// when it returns, as it would be after a system.
    pub fn trigger<'a, E: Event<Trigger<'a>: Default>>(&mut self, event: E) {
        let world = self.world_mut();
        world.trigger(event);
        world.flush();
    }

    /// Records every `E` triggered from now on, whatever triggers it: on
    /// an entity or none.
    ///
    /// Nothing triggered before; otherwise as
    /// [`TestAppBuilder::watch_triggered`].
    pub fn watch_triggered<E: Event + Clone>(&mut self) {
        watch_triggered::<E>(&mut self.app);
    }

    /// Every `E` triggered since they were last read, in the order they
    /// were; they're read after.
    ///
    /// # Panics
    ///
    /// If `E` isn't watched ([`watch_triggered`](Self::watch_triggered)).
    #[track_caller]
    pub fn read_triggered<E: Event>(&mut self) -> Vec<E> {
        let Some(mut recorded) = self.world_mut().get_resource_mut::<Triggered<E>>() else {
            panic!("{}", unwatched::<E>("watch_triggered"));
        };
        core::mem::take(&mut recorded.triggered)
    }

    /// Asserts one `E` triggered since they were last read is as `matches`
    /// says; they stay unread.
    ///
    /// # Panics
    ///
    /// If `E` isn't watched ([`watch_triggered`](Self::watch_triggered)), or
    /// none triggered is as `matches` says, printing those triggered.
    #[track_caller]
    pub fn assert_triggered<E: Event + Debug>(&self, matches: impl FnMut(&E) -> bool) {
        let triggered = self.triggered::<E>();
        assert!(
            triggered.iter().any(matches),
            "no {} triggered as expected; triggered: {triggered:#?}",
            describe::short::<E>()
        );
    }

    /// Asserts no `E` triggered since they were last read is as `matches`
    /// says; they stay unread. `|_| true` asserts none was triggered at all.
    ///
    /// # Panics
    ///
    /// If `E` isn't watched ([`watch_triggered`](Self::watch_triggered)), or
    /// one triggered is as `matches` says, printing those that are.
    #[track_caller]
    pub fn assert_triggered_none<E: Event + Debug>(&self, mut matches: impl FnMut(&E) -> bool) {
        let unexpected: Vec<&E> = self
            .triggered::<E>()
            .iter()
            .filter(|event| matches(event))
            .collect();
        assert!(
            unexpected.is_empty(),
            "{} {} triggered that shouldn't be: {unexpected:#?}",
            unexpected.len(),
            describe::short::<E>()
        );
    }

    /// Every `E` triggered since they were last read, unread.
    #[track_caller]
    fn triggered<E: Event>(&self) -> &[E] {
        match self.world().get_resource::<Triggered<E>>() {
            Some(recorded) => &recorded.triggered,
            None => panic!("{}", unwatched::<E>("watch_triggered")),
        }
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

    /// Ticks until `holds` holds of the app: bounded by
    /// [`Until::within_ticks`] or [`Until::within`].
    ///
    /// `holds` is handed the app, to ask what it would anywhere else:
    /// `|app| app.count::<With<Dead>>() == 1`.
    pub fn until<P: FnMut(&mut TestApp) -> bool>(&mut self, holds: P) -> Until<'_, TestApp, P> {
        Until::new(self, holds)
    }
}

/// The failure for reading what isn't recorded: `T` isn't watched, and
/// `watch` would.
fn unwatched<T>(watch: &str) -> String {
    format!(
        "{} isn't watched: `{watch}::<{}>()` it first",
        describe::short::<T>(),
        describe::short::<T>()
    )
}
