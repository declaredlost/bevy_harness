//! Tests for [`TestWorld`](super::TestWorld).

use bevy::prelude::*;

use crate::{TestApp, TestWorld};

#[derive(Resource, Default)]
struct Count(u32);

fn count(mut count: ResMut<Count>) {
    count.0 += 1;
}

fn counting() -> TestApp {
    TestApp::builder()
        .with(|app| {
            app.init_resource::<Count>();
        })
        .systems(FixedUpdate, count)
        .build()
}

fn stacked(world: &TestWorld, name: &str) -> u32 {
    world[name].world().resource::<Count>().0
}

#[test]
fn its_apps_tick_together_and_each_alone() {
    let mut world = TestWorld::new()
        .with("server", counting())
        .with("client", counting());
    world.ticks(3);
    world["client"].ticks(2);
    assert_eq!(stacked(&world, "server"), 3);
    assert_eq!(stacked(&world, "client"), 5);
    assert_eq!(world.names(), ["server", "client"]);
}

#[test]
fn it_ticks_them_all_until_something_holds_of_them() {
    let mut world = TestWorld::new()
        .with("server", counting())
        .with("client", counting());
    let ticks = world
        .until(|world| stacked(world, "client") >= 10)
        .within_secs(1.0);
    assert_eq!(ticks, 10);
}

#[test]
#[should_panic(expected = "no app called \"host\"; there are [\"server\"]")]
fn a_missing_app_says_which_there_are() {
    let world = TestWorld::new().with("server", counting());
    let _ = &world["host"];
}

#[test]
#[should_panic(expected = "\"client\" ticks at 30 Hz, \"server\" at 64 Hz")]
fn apps_at_other_rates_are_a_mistake() {
    let _ = TestWorld::new()
        .with("server", counting())
        .with("client", TestApp::builder().fixed_hz(30.0).build());
}
