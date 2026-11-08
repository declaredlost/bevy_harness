//! Fixtures: a setup that builds more than the app, handed to each test
//! by `&mut`; and a timeout, for a test that could hang.
//!
//! `cargo test --example fixtures` runs its tests.

use bevy::prelude::*;
use bevy_harness::TestApp;

/// What's left of something.
#[derive(Component, Debug, PartialEq)]
struct Health(u32);

/// Striking someone, a point a tick.
#[derive(Component)]
struct Strikes(Entity);

/// Fighting: the plugin under test.
struct DuelPlugin;

impl Plugin for DuelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, strike);
    }
}

fn strike(strikers: Query<&Strikes>, mut healths: Query<&mut Health>) {
    for strikes in &strikers {
        if let Ok(mut health) = healths.get_mut(strikes.0) {
            health.0 = health.0.saturating_sub(1);
        }
    }
}

/// A duel: the app, and who's in it, by name.
struct Duel {
    app: TestApp,
    hero: Entity,
    foe: Entity,
}

/// A duel of two at `health` each, the hero striking the foe and the foe
/// back.
fn duel(health: u32) -> Duel {
    let mut app = TestApp::builder().plugins(DuelPlugin).build();
    let hero = app.spawn((Name::new("hero"), Health(health)));
    let foe = app.spawn((Name::new("foe"), Health(health), Strikes(hero)));
    app.entity_mut(hero).insert(Strikes(foe));
    Duel { app, hero, foe }
}

/// A duel, as a program sees it.
fn main() {
    let mut duel = duel(3);
    duel.app.ticks(3);
    let hero = duel.app.entity(duel.hero).get::<Health>().0;
    let foe = duel.app.entity(duel.foe).get::<Health>().0;
    println!("hero {hero}, foe {foe}");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A duel of two at ten each: a setup that's a function, called.
    fn even() -> Duel {
        duel(10)
    }

    #[bevy_harness::test(setup = even)]
    fn both_sides_of_a_duel_strike(duel: &mut Duel) {
        duel.app.ticks(4);
        duel.app.entity(duel.hero).assert_eq(Health(6));
        duel.app.entity(duel.foe).assert_eq(Health(6));
    }

    // A setup that's any other expression: built in the test.
    #[bevy_harness::test(setup = duel(1))]
    fn a_duel_at_one_ends_in_a_tick(duel: &mut Duel) {
        duel.app.tick();
        duel.app
            .query::<&Health, ()>()
            .assert_len(2)
            .assert_all(|health| health.0 == 0);
    }

    #[bevy_harness::test(setup = even)]
    fn a_fighter_who_stops_is_still_struck(duel: &mut Duel) {
        duel.app.entity_mut(duel.hero).remove::<Strikes>();
        duel.app.ticks(4);
        duel.app.entity(duel.hero).assert_eq(Health(6));
        duel.app.entity(duel.foe).assert_eq(Health(10));
    }

    // Past its timeout, a test fails instead of hanging; what it waits on
    // in game time is bounded by `within_secs` either way.
    #[bevy_harness::test(setup = duel(5), timeout = 5)]
    fn a_duel_is_over_within_its_timeout(duel: &mut Duel) {
        let foe = duel.foe;
        duel.app
            .until(|app| app.entity(foe).try_get::<Health>() == Some(&Health(0)))
            .within_secs(1.0);
    }
}
