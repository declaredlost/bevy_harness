//! Several apps under test at once, ticked together: a server and its
//! clients.

use core::ops::{Index, IndexMut};
use core::time::Duration;

use crate::app::TestApp;
use crate::until::Until;

#[cfg(test)]
mod tests;

/// Several [`TestApp`]s, each by name, ticked together in the order they
/// were added: a server, then its clients, each tick. They should share a
/// tick rate; seconds count in the first's.
///
/// ```
/// use bevy_harness::{TestApp, TestWorld};
///
/// let mut world = TestWorld::new()
///     .with("server", TestApp::builder().build())
///     .with("client", TestApp::builder().build());
/// world.ticks(3);
/// world["client"].tick();
/// ```
#[derive(Default)]
pub struct TestWorld {
    apps: Vec<(&'static str, TestApp)>,
}

impl TestWorld {
    /// No apps yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `app` as `name`, ticked after those before it.
    ///
    /// # Panics
    ///
    /// If there's already an app called `name`.
    #[track_caller]
    #[must_use]
    pub fn with(mut self, name: &'static str, app: TestApp) -> Self {
        assert!(
            self.apps.iter().all(|(other, _)| *other != name),
            "there's already an app called {name:?}"
        );
        self.apps.push((name, app));
        self
    }

    /// The app called `name`.
    ///
    /// # Panics
    ///
    /// If there's none, naming those there are.
    #[track_caller]
    #[must_use]
    pub fn app(&self, name: &str) -> &TestApp {
        match self.apps.iter().find(|(other, _)| *other == name) {
            Some((_, app)) => app,
            None => panic!("no app called {name:?}; there are {:?}", self.names()),
        }
    }

    /// The app called `name`, to drive or change.
    ///
    /// # Panics
    ///
    /// If there's none, naming those there are.
    #[track_caller]
    pub fn app_mut(&mut self, name: &str) -> &mut TestApp {
        let names = self.names();
        match self.apps.iter_mut().find(|(other, _)| *other == name) {
            Some((_, app)) => app,
            None => panic!("no app called {name:?}; there are {names:?}"),
        }
    }

    /// Its apps' names, in the order they tick.
    #[must_use]
    pub fn names(&self) -> Vec<&'static str> {
        self.apps.iter().map(|(name, _)| *name).collect()
    }

    /// Runs one tick: every app's, in order.
    pub fn tick(&mut self) {
        for (_, app) in &mut self.apps {
            app.tick();
        }
    }

    /// Runs `n` ticks: each, every app's, in order.
    pub fn ticks(&mut self, n: u32) {
        for _ in 0..n {
            self.tick();
        }
    }

    /// Runs as many ticks as `length` of game time takes, at the first
    /// app's rate.
    pub fn advance(&mut self, length: Duration) {
        self.ticks(self.ticks_in(length));
    }

    /// Runs as many ticks as `seconds` of game time takes:
    /// [`advance`](Self::advance) in seconds.
    ///
    /// # Panics
    ///
    /// If `seconds` is negative or not finite.
    #[track_caller]
    pub fn secs(&mut self, seconds: f32) {
        self.advance(crate::time::seconds(seconds));
    }

    /// How many ticks `length` is, at the first app's rate.
    pub(crate) fn ticks_in(&self, length: Duration) -> u32 {
        self.apps.first().map_or(0, |(_, app)| app.ticks_in(length))
    }

    /// Ticks them all until `holds` holds of them: bounded by
    /// [`Until::within_ticks`] or [`Until::within`].
    pub fn until<P: FnMut(&TestWorld) -> bool>(&mut self, holds: P) -> Until<'_, TestWorld, P> {
        Until::new(self, holds)
    }
}

impl Index<&str> for TestWorld {
    type Output = TestApp;

    #[track_caller]
    fn index(&self, name: &str) -> &TestApp {
        self.app(name)
    }
}

impl IndexMut<&str> for TestWorld {
    #[track_caller]
    fn index_mut(&mut self, name: &str) -> &mut TestApp {
        self.app_mut(name)
    }
}
