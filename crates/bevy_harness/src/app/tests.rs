//! Tests for [`TestApp`](super::TestApp) and its builder.

use core::time::Duration;

use bevy::prelude::*;

use crate::TestApp;

#[derive(Component, Debug, PartialEq)]
struct Health(u32);

#[derive(Message, Debug, Clone, PartialEq)]
struct Hurt(u32);

#[derive(Message, Debug, Clone, PartialEq)]
struct Died(Entity);

fn bleed(mut healths: Query<&mut Health>) {
    for mut health in &mut healths {
        health.0 = health.0.saturating_sub(1);
    }
}

fn hurt(mut hurts: MessageReader<Hurt>, mut healths: Query<&mut Health>) {
    for hurt in hurts.read() {
        for mut health in &mut healths {
            health.0 = health.0.saturating_sub(hurt.0);
        }
    }
}

fn die(healths: Query<(Entity, &Health)>, mut deaths: MessageWriter<Died>) {
    for (entity, health) in &healths {
        if health.0 == 0 {
            deaths.write(Died(entity));
        }
    }
}

fn game() -> TestApp {
    TestApp::builder()
        .systems(FixedUpdate, (hurt, bleed, die).chain())
        .with(|app| {
            app.add_message::<Hurt>().add_message::<Died>();
        })
        .watch::<Died>()
        .build()
}

#[test]
fn each_tick_is_one_fixed_step_and_seconds_count_in_them() {
    let mut app = game();
    let hurt = app.spawn(Health(100));
    app.ticks(3);
    app.entity(hurt).assert_eq(Health(97));
    app.secs(0.5);
    app.entity(hurt).assert_eq(Health(65));
}

#[test]
fn messages_go_in_and_are_recorded_coming_out() {
    let mut app = game();
    let hurt = app.spawn(Health(10));
    app.write(Hurt(8));
    app.tick();
    app.entity(hurt).assert_eq(Health(1));
    app.tick();
    app.assert_wrote(|died: &Died| died.0 == hurt);
    assert_eq!(app.read::<Died>().len(), 1);
    assert!(app.read::<Died>().is_empty(), "read once");
}

#[test]
fn it_ticks_until_something_holds_and_says_how_long() {
    let mut app = game();
    let hurt = app.spawn(Health(5));
    let ticks = app
        .until(|world| world.get::<Health>(hurt) == Some(&Health(0)))
        .within_secs(1.0);
    assert_eq!(ticks, 5);
}

#[test]
#[should_panic(expected = "it didn't hold within 3 ticks")]
fn ticking_until_fails_when_it_never_holds() {
    let mut app = game();
    let hurt = app.spawn(Health(50));
    app.until(|world| world.get::<Health>(hurt) == Some(&Health(0)))
        .within_ticks(3);
}

#[test]
#[should_panic(expected = "\"player\"")]
fn a_failure_names_the_entity_and_says_what_it_has() {
    let mut app = game();
    let player = app.spawn(Name::new("player"));
    app.entity(player).assert_has::<Health>();
}

#[test]
fn queries_are_asserted_on_as_a_whole() {
    let mut app = game();
    app.spawn(Health(3));
    app.spawn(Health(30));
    app.query::<&Health, ()>()
        .assert_len(2)
        .assert_all(|health| health.0 > 0)
        .assert_any(|health| health.0 > 20)
        .assert_none(|health| health.0 > 100);
}

#[test]
fn a_system_runs_once_on_its_own() {
    let mut app = game();
    app.spawn(Health(2));
    app.run_system(bleed);
    let total = app.run_system(|healths: Query<&Health>| healths.iter().map(|h| h.0).sum::<u32>());
    assert_eq!(total, 1);
}

#[crate::test(setup = game)]
fn the_attribute_hands_a_test_its_app(app: &mut TestApp) {
    let hurt = app.spawn(Health(4));
    app.ticks(2);
    app.entity(hurt).assert_eq(Health(2));
}

/// The game's systems, as a plugin.
struct Bleeding;

impl Plugin for Bleeding {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, bleed);
    }
}

#[crate::test(plugins = [Bleeding])]
fn the_attribute_builds_the_app_with_its_plugins(app: &mut TestApp) {
    let hurt = app.spawn(Health(4));
    app.ticks(2);
    app.entity(hurt).assert_eq(Health(2));
}

#[crate::test(timeout = 5)]
fn a_test_within_its_timeout_passes(app: &mut TestApp) {
    app.ticks(10);
}

#[crate::test(timeout = 0.1)]
#[should_panic(expected = "didn't finish within")]
fn a_test_past_its_timeout_fails(app: &mut TestApp) {
    app.until(|_| false).within_ticks(u32::MAX);
}

#[crate::test(timeout = 5)]
#[should_panic(expected = "has no Health")]
fn a_failure_on_its_own_thread_is_still_the_failure(app: &mut TestApp) {
    let nobody = app.spawn(());
    app.entity(nobody).assert_has::<Health>();
}

#[test]
fn it_counts_what_a_filter_matches_without_changing_anything() {
    let mut app = game();
    app.spawn(Health(1));
    app.spawn(Health(2));
    app.spawn(Name::new("bystander"));
    assert_eq!(app.count::<With<Health>>(), 2);
    assert_eq!(app.count::<(With<Health>, With<Name>)>(), 0);
}

/// The game, with someone at `health`.
fn with_health(health: u32) -> TestApp {
    let mut app = game();
    app.spawn(Health(health));
    app
}

#[crate::test(setup = with_health(9))]
fn the_attribute_takes_an_app_built_in_place(app: &mut TestApp) {
    app.query::<&Health, ()>()
        .assert_len(1)
        .assert_all(|health| health.0 == 9);
}

/// The game, and someone wounded in it.
struct Wounded {
    app: TestApp,
    who: Entity,
}

fn wounded() -> Wounded {
    let mut app = game();
    let who = app.spawn(Health(5));
    Wounded { app, who }
}

#[crate::test(setup = wounded)]
fn the_attribute_hands_a_test_a_fixture(wounded: &mut Wounded) {
    wounded.app.ticks(2);
    wounded.app.entity(wounded.who).assert_eq(Health(3));
}

#[test]
fn time_rounds_to_the_nearest_tick() {
    let app = TestApp::builder().fixed_hz(64.0).build();
    let ticks = |seconds: f64| app.ticks_in(Duration::from_secs_f64(seconds));
    assert_eq!(ticks(1.0), 64);
    assert_eq!(ticks(0.5), 32);
    assert_eq!(ticks(1.0 / 128.0), 1);
    assert_eq!(ticks(1.0 / 256.0), 0);
}

#[test]
#[should_panic(expected = "isn't a length of time")]
fn negative_seconds_are_a_mistake() {
    TestApp::builder().build().secs(-1.0);
}

#[test]
fn a_query_on_what_was_never_registered_matches_nothing() {
    #[derive(Component)]
    struct Never;
    let app = TestApp::builder().build();
    app.query::<&Never, ()>().assert_len(0);
    assert!(app.query::<Entity, With<Never>>().is_empty());
}
