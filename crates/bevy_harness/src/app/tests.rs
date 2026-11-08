//! Tests for [`TestApp`](super::TestApp) and its builder.

use core::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

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
        .until(|app| app.entity(hurt).try_get::<Health>() == Some(&Health(0)))
        .within_secs(1.0);
    assert_eq!(ticks, 5);
}

#[test]
#[should_panic(expected = "it didn't hold within 3 ticks")]
fn ticking_until_fails_when_it_never_holds() {
    let mut app = game();
    let hurt = app.spawn(Health(50));
    app.until(|app| app.entity(hurt).try_get::<Health>() == Some(&Health(0)))
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
    let mut app = TestApp::builder().build();
    app.query::<&Never, ()>().assert_len(0);
    assert!(app.query::<Entity, With<Never>>().is_empty());
}

#[derive(Resource, Default)]
struct Runs(u32);

fn run(mut runs: ResMut<Runs>) {
    runs.0 += 1;
}

/// An app counting its fixed steps, built by `build`.
fn counting(build: impl FnOnce(crate::TestAppBuilder) -> crate::TestAppBuilder) -> TestApp {
    let builder = TestApp::builder()
        .with(|app| {
            app.init_resource::<Runs>();
        })
        .systems(FixedUpdate, run);
    build(builder).build()
}

fn runs(app: &TestApp) -> u32 {
    app.resource::<Runs>().0
}

#[test]
fn a_slow_tick_is_still_one_fixed_step() {
    let mut app = counting(|builder| builder.fixed_hz(2.0));
    app.ticks(4);
    assert_eq!(runs(&app), 4);
    app.secs(2.0);
    assert_eq!(runs(&app), 8);
}

#[test]
fn a_rate_the_app_sets_is_kept() {
    let mut app = counting(|builder| {
        builder.with(|app| {
            app.insert_resource(Time::<Fixed>::from_hz(10.0));
        })
    });
    app.secs(1.0);
    assert_eq!(runs(&app), 10);
}

#[test]
fn a_rate_the_builder_sets_wins() {
    let mut app = counting(|builder| {
        builder
            .with(|app| {
                app.insert_resource(Time::<Fixed>::from_hz(10.0));
            })
            .fixed_hz(20.0)
    });
    app.secs(1.0);
    assert_eq!(runs(&app), 20);
}

#[test]
#[should_panic(expected = "isn't a tick rate")]
fn a_rate_of_nothing_is_a_mistake() {
    let _ = TestApp::builder().fixed_hz(0.0);
}

#[test]
#[should_panic(expected = "isn't a tick rate")]
fn a_rate_that_isnt_a_number_is_a_mistake() {
    let _ = TestApp::builder().fixed_hz(f64::NAN);
}

#[test]
fn a_bare_app_keeps_its_own_time() {
    let manual = Duration::from_millis(10);
    let app = TestApp::bare_builder()
        .plugins(bevy::time::TimePlugin)
        .with(|app| {
            app.insert_resource(TimeUpdateStrategy::ManualDuration(manual));
        })
        .build();
    assert!(matches!(
        app.resource::<TimeUpdateStrategy>(),
        TimeUpdateStrategy::ManualDuration(length) if *length == manual
    ));
}

#[test]
fn a_query_without_what_was_never_registered_matches_everything() {
    #[derive(Component)]
    struct Never;
    let mut app = TestApp::builder().build();
    app.spawn(Health(1));
    app.spawn(Health(2));
    assert_eq!(app.count::<(With<Health>, Without<Never>)>(), 2);
    app.query::<(&Health, Has<Never>), Without<Never>>()
        .assert_len(2)
        .assert_all(|(_, never)| !never);
    app.query::<Option<&Never>, With<Health>>().assert_len(2);
}

#[derive(Message, Debug, Clone, PartialEq)]
struct Said(u32);

fn talking() -> TestApp {
    TestApp::builder()
        .with(|app| {
            app.add_message::<Said>();
        })
        .build()
}

#[test]
fn a_late_watch_records_only_what_comes_after() {
    let mut app = talking();
    app.write(Said(1));
    app.tick();
    app.watch::<Said>();
    app.write(Said(2));
    app.tick();
    assert_eq!(app.read::<Said>(), [Said(2)]);
}

#[test]
fn watching_adds_the_message() {
    let mut app = TestApp::builder().watch::<Said>().build();
    app.write(Said(1));
    app.tick();
    assert_eq!(app.read::<Said>(), [Said(1)]);
}

/// Says which update it is, last thing in each.
fn say_last(mut updates: Local<u32>, mut said: MessageWriter<Said>) {
    *updates += 1;
    said.write(Said(*updates));
}

#[test]
fn a_message_written_last_is_recorded_the_same_tick() {
    let mut app = TestApp::builder()
        .systems(Last, say_last)
        .watch::<Said>()
        .build();
    app.read::<Said>();
    app.tick();
    assert_eq!(app.read::<Said>(), [Said(2)]);
}

#[test]
fn a_short_limit_still_ticks_once() {
    let mut app = counting(|builder| builder);
    let ticks = app
        .until(|app| app.resource::<Runs>().0 == 1)
        .within(Duration::from_millis(7));
    assert_eq!(ticks, 1);
}

#[test]
#[should_panic(
    expected = "it didn't hold within 3 ticks (46.875ms of game time, at 64 Hz): the foe is dead"
)]
fn a_failure_to_hold_says_how_long_and_what_for() {
    let mut app = game();
    app.until(|_| false)
        .describe("the foe is dead")
        .within_ticks(3);
}

/// An app built from what isn't `Send`.
fn unsendable(shared: std::rc::Rc<u32>) -> TestApp {
    let mut app = TestApp::builder().build();
    app.spawn(Health(*shared));
    app
}

#[crate::test(setup = unsendable(std::rc::Rc::new(3)))]
fn a_setup_needs_no_send_without_a_timeout(app: &mut TestApp) {
    app.query::<&Health, ()>()
        .assert_all(|health| health.0 == 3);
}

#[crate::test]
fn a_test_can_return_a_result(app: &mut TestApp) -> Result<(), String> {
    let hurt = app.spawn(Health(3));
    let health = app.entity(hurt).try_get::<Health>().ok_or("no health")?;
    assert_eq!(health.0, 3);
    Ok(())
}

#[crate::test]
#[should_panic(expected = "the test returned an error: \"no foe\"")]
fn a_test_returning_an_error_fails(_app: &mut TestApp) -> Result<(), String> {
    Err("no foe".to_owned())
}

/// Says when it's dropped: when the app holding it is.
#[derive(Resource)]
struct Dropped(std::sync::mpsc::Sender<()>);

impl Drop for Dropped {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

#[test]
fn a_test_past_its_timeout_stops_ticking() {
    let (dropped, gone) = std::sync::mpsc::channel();
    let setup = move || {
        let mut app = TestApp::builder().build();
        app.insert_resource(Dropped(dropped));
        app
    };
    let timed_out = std::panic::catch_unwind(|| {
        crate::__private::run_with_timeout(
            setup,
            |app: &mut TestApp| {
                app.until(|_| false).within_ticks(u32::MAX);
            },
            Duration::from_millis(50),
        );
    });
    assert!(timed_out.is_err(), "it timed out");
    gone.recv_timeout(Duration::from_secs(5))
        .expect("its app stopped and was dropped");
}

#[derive(Event, Debug, Clone, PartialEq)]
struct Rang(u32);

#[derive(EntityEvent, Debug, Clone, PartialEq)]
struct Struck {
    entity: Entity,
    damage: u32,
}

#[test]
fn triggered_events_are_recorded_on_an_entity_or_none() {
    let mut app = TestApp::builder()
        .watch_triggered::<Rang>()
        .watch_triggered::<Struck>()
        .build();
    let struck = app.spawn(Health(3));
    app.trigger(Rang(1));
    app.trigger(Struck {
        entity: struck,
        damage: 2,
    });
    app.trigger(Rang(2));
    app.assert_triggered(|rang: &Rang| rang.0 == 2);
    app.assert_triggered(|hit: &Struck| hit.entity == struck && hit.damage == 2);
    assert_eq!(app.read_triggered::<Rang>(), [Rang(1), Rang(2)]);
    assert!(app.read_triggered::<Rang>().is_empty(), "read once");
}

/// Rings once, as it starts.
fn ring_at_startup(mut commands: Commands) {
    commands.trigger(Rang(0));
}

#[test]
fn an_event_triggered_at_startup_is_recorded_when_watched_on_the_builder() {
    let mut app = TestApp::builder()
        .systems(Startup, ring_at_startup)
        .watch_triggered::<Rang>()
        .build();
    assert_eq!(app.read_triggered::<Rang>(), [Rang(0)]);
}

#[test]
fn a_late_triggered_watch_records_only_what_comes_after() {
    let mut app = TestApp::builder().build();
    app.trigger(Rang(1));
    app.watch_triggered::<Rang>();
    app.watch_triggered::<Rang>();
    app.trigger(Rang(2));
    assert_eq!(app.read_triggered::<Rang>(), [Rang(2)], "recorded once");
}

#[test]
#[should_panic(expected = "no Rang triggered as expected; triggered: [\n    Rang(\n        1,")]
fn a_failure_says_what_was_triggered_instead() {
    let mut app = TestApp::builder().watch_triggered::<Rang>().build();
    app.trigger(Rang(1));
    app.assert_triggered(|rang: &Rang| rang.0 == 2);
}

#[test]
fn nothing_triggered_as_it_shouldnt_be_passes() {
    let mut app = TestApp::builder().watch_triggered::<Rang>().build();
    app.assert_triggered_none(|_: &Rang| true);
    app.trigger(Rang(1));
    app.assert_triggered_none(|rang: &Rang| rang.0 == 2);
}

#[test]
#[should_panic(expected = "1 Rang triggered that shouldn't be")]
fn something_triggered_as_it_shouldnt_be_fails() {
    let mut app = TestApp::builder().watch_triggered::<Rang>().build();
    app.trigger(Rang(1));
    app.trigger(Rang(2));
    app.assert_triggered_none(|rang: &Rang| rang.0 == 2);
}

#[test]
#[should_panic(expected = "Rang isn't watched: `watch_triggered::<Rang>()` it first")]
fn reading_what_isnt_triggered_and_watched_is_a_mistake() {
    TestApp::builder().build().read_triggered::<Rang>();
}

#[test]
#[should_panic(expected = "Said isn't watched: `watch::<Said>()` it first")]
fn reading_what_isnt_written_and_watched_is_a_mistake() {
    talking().read::<Said>();
}

#[test]
fn nothing_written_as_it_shouldnt_be_passes() {
    let mut app = talking();
    app.watch::<Said>();
    app.assert_wrote_none(|_: &Said| true);
    app.write(Said(1));
    app.tick();
    app.assert_wrote_none(|said: &Said| said.0 == 2);
}

#[test]
#[should_panic(expected = "1 Said written that shouldn't be: [\n    Said(\n        2,")]
fn something_written_as_it_shouldnt_be_fails() {
    let mut app = talking();
    app.watch::<Said>();
    app.write(Said(1));
    app.write(Said(2));
    app.tick();
    app.assert_wrote_none(|said: &Said| said.0 == 2);
}

/// Singled out.
#[derive(Component)]
struct Marked;

#[test]
fn the_one_component_a_filter_matches_is_changed() {
    let mut app = game();
    let bleeding = app.spawn((Health(5), Marked));
    app.spawn(Health(9));
    app.single_mut::<Health, With<Marked>>().0 = 1;
    app.entity(bleeding).assert_eq(Health(1));
}

#[test]
#[should_panic(expected = "Query<&mut Health, With<Marked>> matches 2, not one")]
fn changing_the_one_of_several_says_how_many() {
    let mut app = game();
    app.spawn((Health(5), Marked));
    app.spawn((Health(9), Marked));
    let _ = app.single_mut::<Health, With<Marked>>();
}

#[test]
#[should_panic(expected = "Query<&mut Health> matches 0, not one")]
fn changing_the_one_of_none_says_so() {
    let _ = game().single_mut::<Health, ()>();
}

#[test]
#[should_panic(expected = "Query<&Health, With<Marked>> matches 0, not one")]
fn a_query_failure_says_the_query() {
    game().query::<&Health, With<Marked>>().single();
}

#[test]
fn an_entity_is_found_by_name() {
    let mut app = game();
    app.spawn(Name::new("foe"));
    let hero = app.spawn(Name::new("hero"));
    assert_eq!(app.named("hero"), hero);
}

#[test]
fn a_name_is_looked_for_without_failing() {
    let mut app = game();
    let hero = app.spawn(Name::new("hero"));
    assert_eq!(app.try_named("hero"), Some(hero));
    assert_eq!(app.try_named("villain"), None);
    assert_eq!(TestApp::builder().build().try_named("hero"), None);
}

#[test]
#[should_panic(expected = "2 entities are called \"twin\"")]
fn a_name_several_have_is_a_mistake_looked_for_too() {
    let mut app = game();
    app.spawn(Name::new("twin"));
    app.spawn(Name::new("twin"));
    let _ = app.try_named("twin");
}

#[test]
#[should_panic(expected = "nothing is called \"villain\"; named are [\"foe\", \"hero\"]")]
fn a_name_nothing_has_says_which_there_are() {
    let mut app = game();
    app.spawn(Name::new("hero"));
    app.spawn(Name::new("foe"));
    let _ = app.named("villain");
}

#[test]
#[should_panic(expected = "nothing is called \"hero\"; named are []")]
fn a_name_in_an_app_naming_nothing_is_found_nowhere() {
    let _ = TestApp::builder().build().named("hero");
}

#[test]
#[should_panic(expected = "2 entities are called \"twin\"")]
fn a_name_several_have_is_a_mistake() {
    let mut app = game();
    app.spawn(Name::new("twin"));
    app.spawn(Name::new("twin"));
    let _ = app.named("twin");
}

#[test]
fn resources_are_read_changed_inserted_and_initialized() {
    let mut app = TestApp::builder().build();
    app.init_resource::<Runs>();
    assert_eq!(app.resource::<Runs>().0, 0);
    app.resource_mut::<Runs>().0 = 4;
    app.init_resource::<Runs>();
    assert_eq!(app.resource::<Runs>().0, 4, "initialized once");
    app.insert_resource(Runs(7));
    assert_eq!(app.resource::<Runs>().0, 7);
}

#[test]
#[should_panic(expected = "there's no Runs resource")]
fn a_missing_resource_says_which() {
    let _ = TestApp::builder().build().resource::<Runs>();
}

#[test]
#[should_panic(expected = "there's no Runs resource")]
fn a_missing_resource_to_change_says_which() {
    let _ = TestApp::builder().build().resource_mut::<Runs>();
}

#[test]
fn an_entity_is_asked_about_without_failing() {
    let mut app = game();
    let hurt = app.spawn(Health(2));
    let check = app.entity(hurt);
    assert!(check.is_spawned());
    assert!(check.has::<Health>());
    assert!(!check.has::<Name>());
    assert_eq!(check.try_get::<Health>(), Some(&Health(2)));
    assert_eq!(check.try_get::<Name>(), None);
    app.entity_mut(hurt).despawn();
    let gone = app.entity(hurt);
    assert!(!gone.is_spawned());
    assert!(!gone.has::<Health>());
    gone.assert_despawned();
}

#[test]
fn a_condition_queries_as_a_test_does() {
    let mut app = game();
    app.spawn(Health(2));
    let ticks = app
        .until(|app| app.query::<&Health, ()>().single().0 == 0)
        .within_secs(1.0);
    assert_eq!(ticks, 2);
}

#[test]
fn a_condition_finds_by_name_as_a_test_does() {
    let mut app = game();
    app.spawn((Name::new("hurt"), Health(3)));
    let ticks = app
        .until(|app| app.entity(app.named("hurt")).try_get::<Health>() == Some(&Health(0)))
        .within_secs(1.0);
    assert_eq!(ticks, 3);
}

/// Rings back, through commands, when rung.
fn ring_back(rang: On<Rang>, mut commands: Commands) {
    if rang.0 == 1 {
        commands.trigger(Rang(2));
        commands.spawn(Health(rang.0));
    }
}

#[test]
fn what_an_observer_queues_is_done_when_a_trigger_returns() {
    let mut app = TestApp::builder().watch_triggered::<Rang>().build();
    app.app_mut().add_observer(ring_back);
    app.trigger(Rang(1));
    assert_eq!(app.read_triggered::<Rang>(), [Rang(1), Rang(2)]);
    app.query::<&Health, ()>().assert_len(1);
}
