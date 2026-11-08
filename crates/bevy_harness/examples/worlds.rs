//! Several apps ticked together: a server and its client, linked in the
//! same process, as a `TestWorld`.
//!
//! `cargo test --example worlds` runs its tests.

use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy_harness::{TestApp, TestWorld};

/// A link between two apps: what one sends, the other takes.
#[derive(Resource, Clone, Default)]
struct Wire(Arc<Mutex<Vec<u32>>>);

/// The server's score, a point a tick.
#[derive(Resource, Default)]
struct Score(u32);

/// The score the client last heard.
#[derive(Resource, Default)]
struct Shown(u32);

/// Scoring a point a tick, and sending it.
struct ServerPlugin(Wire);

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.0.clone())
            .init_resource::<Score>()
            .add_systems(FixedUpdate, (score, send).chain());
    }
}

/// Showing the score it hears.
struct ClientPlugin(Wire);

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.0.clone())
            .init_resource::<Shown>()
            .add_systems(FixedUpdate, hear);
    }
}

fn score(mut score: ResMut<Score>) {
    score.0 += 1;
}

fn send(score: Res<Score>, wire: Res<Wire>) {
    wire.0.lock().unwrap().push(score.0);
}

fn hear(wire: Res<Wire>, mut shown: ResMut<Shown>) {
    if let Some(last) = wire.0.lock().unwrap().drain(..).next_back() {
        shown.0 = last;
    }
}

/// A server, and a client linked to it: ticked in that order.
fn linked() -> TestWorld {
    let wire = Wire::default();
    TestWorld::new()
        .with(
            "server",
            TestApp::builder()
                .plugins(ServerPlugin(wire.clone()))
                .build(),
        )
        .with(
            "client",
            TestApp::builder().plugins(ClientPlugin(wire)).build(),
        )
}

/// A server and its client, as a program sees them.
fn main() {
    let mut world = linked();
    world.secs(1.0);
    println!("the client shows {}", world["client"].resource::<Shown>().0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[bevy_harness::test(setup = linked)]
    fn a_tick_is_every_apps_in_order(world: &mut TestWorld) {
        world.ticks(3);
        assert_eq!(world["server"].resource::<Score>().0, 3);
        // The server ticks first: what it sent, the client hears that tick.
        assert_eq!(world["client"].resource::<Shown>().0, 3);
    }

    #[bevy_harness::test(setup = linked, timeout = 5)]
    fn it_waits_on_all_of_them(world: &mut TestWorld) {
        let ticks = world
            .until(|world| world["client"].resource::<Shown>().0 == 10)
            .within_secs(1.0);
        assert_eq!(ticks, 10);
    }

    #[bevy_harness::test(setup = linked)]
    fn one_app_is_changed_on_its_own(world: &mut TestWorld) {
        world["server"].resource_mut::<Score>().0 = 100;
        world.tick();
        assert_eq!(world["client"].resource::<Shown>().0, 101);
    }
}
