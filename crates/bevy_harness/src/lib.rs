//! Testing Bevy apps: the real app, headless, stepped tick by tick, and
//! asserted on.
//!
//! A [`TestApp`] wraps an [`App`](bevy::app::App) built from the plugins
//! under test, on time that only moves when the test moves it: each
//! [tick](TestApp::ticks) runs exactly one fixed step. Messages go in
//! ([`write`](TestApp::write)) and are recorded coming out
//! ([`watch`](TestAppBuilder::watch), [`read`](TestApp::read)); entities
//! and queries are asserted on ([`entity`](TestApp::entity),
//! [`query`](TestApp::query)), each failure saying what was there instead,
//! at the test's own line.
//!
//! ```
//! use bevy::prelude::*;
//! use bevy_harness::TestApp;
//!
//! #[derive(Component, Debug, PartialEq)]
//! struct Health(u32);
//!
//! fn bleed(mut healths: Query<&mut Health>) {
//!     for mut health in &mut healths {
//!         health.0 = health.0.saturating_sub(1);
//!     }
//! }
//!
//! let mut app = TestApp::builder().systems(FixedUpdate, bleed).build();
//! let hurt = app.spawn((Name::new("hurt"), Health(10)));
//! app.ticks(3);
//! app.entity(hurt).assert(|health: &Health| health.0 == 7);
//! ```
//!
//! Or as a test handed its app (see [`test`]): built by a setup function
//! shared by a crate's tests, and failed if it runs too long.
//!
//! ```ignore
//! #[bevy_harness::test(setup = testing::server, timeout = 5)]
//! fn a_grip_lets_go(app: &mut TestApp) { .. }
//! ```

// The attribute names this crate by its path, here too: in its own tests.
extern crate self as bevy_harness;

#[doc(hidden)]
pub mod __private;
mod app;
mod check;
mod describe;
mod time;
mod until;
mod world;

pub use bevy_harness_macros::test;

pub use self::app::{TestApp, TestAppBuilder};
pub use self::check::{EntityCheck, QueryCheck};
pub use self::until::{Ticked, Until};
pub use self::world::TestWorld;
