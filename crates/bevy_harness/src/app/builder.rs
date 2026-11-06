//! Building a [`TestApp`].

use core::time::Duration;

use bevy::app::{App, Plugins};
use bevy::ecs::message::Message;
use bevy::ecs::schedule::{IntoScheduleConfigs, ScheduleLabel};
use bevy::ecs::system::ScheduleSystem;
use bevy::time::{Fixed, Time, TimeUpdateStrategy};

use super::TestApp;
use super::messages::watch;

/// What a [`TestApp`] is built from: Bevy's minimal plugins (time, tasks,
/// no window), the plugins and systems under test, the messages to
/// record, and its tick rate.
pub struct TestAppBuilder {
    pub(super) app: App,
    pub(super) hz: f64,
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

    /// Records every `M` the app writes, whatever writes it, for
    /// [`TestApp::read`] and [`TestApp::assert_wrote`].
    #[must_use]
    pub fn watch<M: Message + Clone>(mut self) -> Self {
        watch::<M>(&mut self.app);
        self
    }

    /// Fixed ticks a second (64 unless set): what [`TestApp::advance`]
    /// counts in, and `Time<Fixed>`'s step.
    #[must_use]
    pub fn fixed_hz(mut self, hz: f64) -> Self {
        self.hz = hz;
        self
    }

    /// Does anything else to the app before it's finished: a resource, a
    /// state.
    #[must_use]
    pub fn with(mut self, build: impl FnOnce(&mut App)) -> Self {
        build(&mut self.app);
        self
    }

    /// Builds the app, on time a fixed tick per update: finishes its
    /// plugins, and runs its first update, `Startup` (time starts on the
    /// next, so no tick yet).
    #[must_use]
    pub fn build(mut self) -> TestApp {
        self.app
            .insert_resource(Time::<Fixed>::from_duration(Duration::from_secs_f64(
                1.0 / self.hz,
            )))
            .insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        self.app.finish();
        self.app.cleanup();
        self.app.update();
        TestApp {
            app: self.app,
            hz: self.hz,
        }
    }
}
