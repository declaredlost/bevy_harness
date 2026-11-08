//! Building a [`TestApp`].

use core::time::Duration;

use bevy::app::{App, Plugins};
use bevy::ecs::event::Event;
use bevy::ecs::message::Message;
use bevy::ecs::schedule::{IntoScheduleConfigs, ScheduleLabel};
use bevy::ecs::system::ScheduleSystem;
use bevy::time::{Fixed, Time, TimeUpdateStrategy, Virtual};

use super::TestApp;
use super::messages::watch;
use super::triggered::watch_triggered;
use crate::time;

/// What a [`TestApp`] is built from: Bevy's minimal plugins (time, tasks,
/// no window), the plugins and systems under test, the messages and
/// events to record, and its tick rate.
pub struct TestAppBuilder {
    pub(super) app: App,
    /// The tick's length, if the builder sets it.
    pub(super) step: Option<Duration>,
    /// Whether it steps time a fixed tick per update: not on a bare app.
    pub(super) steps_time: bool,
}

impl TestAppBuilder {
    /// Adds the plugins under test.
    #[must_use]
    pub fn plugins<M>(mut self, plugins: impl Plugins<M>) -> Self {
        self.app.add_plugins(plugins);
        self
    }

    /// Adds systems to `schedule`.
    #[must_use]
    pub fn systems<M>(
        mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> Self {
        self.app.add_systems(schedule, systems);
        self
    }

    /// Records every `M` the app writes, whatever writes it.
    ///
    /// For [`TestApp::read`] and [`TestApp::assert_wrote`]; it adds the
    /// message (`add_message::<M>()`) if nothing has yet.
    #[must_use]
    pub fn watch<M: Message + Clone>(mut self) -> Self {
        watch::<M>(&mut self.app);
        self
    }

    /// Records every `E` triggered in the app, whatever triggers it: on an
    /// entity or none, from its plugins' `Startup` on.
    ///
    /// For [`TestApp::read_triggered`] and [`TestApp::assert_triggered`];
    /// a global observer records them, as they're triggered.
    #[must_use]
    pub fn watch_triggered<E: Event + Clone>(mut self) -> Self {
        watch_triggered::<E>(&mut self.app);
        self
    }

    /// Fixed ticks a second: `Time<Fixed>`'s step, and each tick's.
    ///
    /// What [`TestApp::advance`] counts in. Unless set, the rate the app
    /// sets itself (a plugin's, or `with`'s `Time<Fixed>`), or 64.
    ///
    /// # Panics
    ///
    /// If `hz` isn't more than 0 and finite.
    #[track_caller]
    #[must_use]
    pub fn fixed_hz(mut self, hz: f64) -> Self {
        self.step = Some(time::step(hz));
        self
    }

    /// Does anything else to the app before it's finished: a resource, a
    /// state.
    #[must_use]
    pub fn with(mut self, build: impl FnOnce(&mut App)) -> Self {
        build(&mut self.app);
        self
    }

    /// Builds the app, on time a fixed tick per update, and starts it.
    ///
    /// It finishes its plugins, and runs its first update, `Startup` (time
    /// starts on the next, so no tick yet). Each update then moves real
    /// time one fixed step (`TimeUpdateStrategy::FixedTimesteps`), and
    /// `Time<Virtual>` is let take a whole step at once; pausing virtual
    /// time or changing its speed still holds the ticks back. A bare app's
    /// time is left as its plugins set it.
    #[must_use]
    pub fn build(mut self) -> TestApp {
        if let Some(step) = self.step {
            self.app.insert_resource(Time::<Fixed>::from_duration(step));
        }
        if self.steps_time {
            self.app
                .insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        }
        self.app.finish();
        self.app.cleanup();
        let world = self.app.world_mut();
        if self.steps_time && !world.contains_resource::<Time<Fixed>>() {
            world.insert_resource(Time::<Fixed>::from_duration(time::STEP));
        }
        let step = world
            .get_resource::<Time<Fixed>>()
            .map_or(time::STEP, Time::timestep);
        if self.steps_time
            && let Some(mut virtual_time) = world.get_resource_mut::<Time<Virtual>>()
            && virtual_time.max_delta() < step
        {
            virtual_time.set_max_delta(step);
        }
        self.app.update();
        TestApp {
            app: self.app,
            step,
        }
    }
}
