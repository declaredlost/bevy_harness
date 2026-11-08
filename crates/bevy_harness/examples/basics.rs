//! The basics: an app built from the plugins under test, ticked, and its
//! entities and queries asserted on.
//!
//! `cargo test --example basics` runs its tests; `cargo run --example
//! basics`, the same app as a plain value, without the attribute.

use bevy::prelude::*;
use bevy_harness::TestApp;

/// What's left of something.
#[derive(Component, Debug, PartialEq)]
struct Health(u32);

/// Losing a point of health a tick, while there's any left.
#[derive(Component)]
struct Bleeding;

/// What has no health left.
#[derive(Component)]
struct Dead;

/// Bleeding, and dying of it: the plugin under test.
struct WoundsPlugin;

impl Plugin for WoundsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (bleed, die).chain());
    }
}

fn bleed(mut bleeding: Query<&mut Health, With<Bleeding>>) {
    for mut health in &mut bleeding {
        health.0 = health.0.saturating_sub(1);
    }
}

fn die(dying: Query<(Entity, &Health), Without<Dead>>, mut commands: Commands) {
    for (entity, health) in &dying {
        if health.0 == 0 {
            commands.entity(entity).insert(Dead);
        }
    }
}

/// A `TestApp` is a plain value: here a program's, not a test's.
fn main() {
    let mut app = TestApp::builder().plugins(WoundsPlugin).build();
    let hurt = app.spawn((Name::new("hurt"), Health(3), Bleeding));
    let ticks = app
        .until(|app| app.entity(hurt).has::<Dead>())
        .within_secs(1.0);
    println!("bled out in {ticks} ticks");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[bevy_harness::test(plugins = [WoundsPlugin])]
    fn bleeding_costs_a_point_a_tick(app: &mut TestApp) {
        let hurt = app.spawn((Name::new("hurt"), Health(10), Bleeding));
        app.ticks(3);
        app.entity(hurt)
            .assert_eq(Health(7))
            .assert_has::<Bleeding>()
            .assert_lacks::<Dead>();
    }

    #[bevy_harness::test(plugins = [WoundsPlugin])]
    fn seconds_are_counted_in_ticks(app: &mut TestApp) {
        // 64 ticks a second, unless the builder says: half a second is 32.
        let hurt = app.spawn((Health(100), Bleeding));
        app.secs(0.5);
        app.entity(hurt).assert_eq(Health(68));
    }

    /// The wounds, ticking ten times a second.
    fn slow() -> TestApp {
        TestApp::builder()
            .plugins(WoundsPlugin)
            .fixed_hz(10.0)
            .build()
    }

    #[bevy_harness::test(setup = slow)]
    fn a_second_is_as_many_ticks_as_the_app_runs(app: &mut TestApp) {
        let hurt = app.spawn((Health(100), Bleeding));
        app.secs(1.0);
        app.entity(hurt).assert_eq(Health(90));
    }

    #[bevy_harness::test(plugins = [WoundsPlugin])]
    fn queries_are_checked_as_a_whole(app: &mut TestApp) {
        app.spawn((Health(1), Bleeding));
        app.spawn(Health(5));
        app.ticks(2);
        app.query::<&Health, With<Dead>>().assert_len(1);
        app.query::<&Health, ()>()
            .assert_len(2)
            .assert_all(|health| health.0 <= 5)
            .assert_any(|health| health.0 == 0)
            .assert_none(|health| health.0 > 5);
        assert_eq!(app.count::<With<Bleeding>>(), 1);
    }

    #[bevy_harness::test(plugins = [WoundsPlugin])]
    fn it_ticks_until_something_holds_and_says_how_long(app: &mut TestApp) {
        let hurt = app.spawn((Health(5), Bleeding));
        let ticks = app
            .until(|app| app.entity(hurt).has::<Dead>())
            .within_secs(1.0);
        assert_eq!(ticks, 5);
    }

    #[bevy_harness::test]
    fn a_system_runs_once_on_its_own(app: &mut TestApp) {
        app.spawn((Health(3), Bleeding));
        app.run_system(bleed);
        let total =
            app.run_system(|healths: Query<&Health>| healths.iter().map(|h| h.0).sum::<u32>());
        assert_eq!(total, 2);
    }

    #[bevy_harness::test(plugins = [WoundsPlugin])]
    #[should_panic(expected = "\"hurt\"")]
    fn a_failure_names_the_entity_and_says_what_it_has(app: &mut TestApp) {
        let hurt = app.spawn((Name::new("hurt"), Health(10)));
        app.entity(hurt).assert_has::<Dead>();
    }
}
