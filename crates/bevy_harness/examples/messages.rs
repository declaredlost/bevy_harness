//! Messages: written into an app, and recorded coming out of it.
//!
//! `cargo test --example messages` runs its tests.

use bevy::prelude::*;
use bevy_harness::TestApp;

/// What's left of something.
#[derive(Component, Debug, PartialEq)]
struct Health(u32);

/// A blow: who it lands on, and how hard.
#[derive(Message, Clone, Debug)]
struct Hit {
    target: Entity,
    damage: u32,
}

/// Someone killed by a blow.
#[derive(Message, Clone, Debug)]
struct Died(Entity);

/// Blows landing, and the deaths they're the last of: the plugin under
/// test.
struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Hit>()
            .add_message::<Died>()
            .add_systems(FixedUpdate, land);
    }
}

fn land(
    mut hits: MessageReader<Hit>,
    mut healths: Query<&mut Health>,
    mut died: MessageWriter<Died>,
) {
    for hit in hits.read() {
        let Ok(mut health) = healths.get_mut(hit.target) else {
            continue;
        };
        let alive = health.0 > 0;
        health.0 = health.0.saturating_sub(hit.damage);
        if alive && health.0 == 0 {
            died.write(Died(hit.target));
        }
    }
}

/// A blow landing, as a program sees it.
fn main() {
    let mut app = TestApp::builder()
        .plugins(CombatPlugin)
        .watch::<Died>()
        .build();
    let target = app.spawn(Health(10));
    app.write(Hit { target, damage: 10 });
    app.tick();
    for died in app.read::<Died>() {
        println!("{:?} died", died.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The combat, its deaths recorded.
    fn combat() -> TestApp {
        TestApp::builder()
            .plugins(CombatPlugin)
            .watch::<Died>()
            .build()
    }

    #[bevy_harness::test(setup = combat)]
    fn a_blow_goes_in_and_a_death_comes_out(app: &mut TestApp) {
        let target = app.spawn(Health(10));
        app.write(Hit { target, damage: 10 });
        app.tick();
        app.assert_wrote(|died: &Died| died.0 == target);
    }

    #[bevy_harness::test(setup = combat)]
    fn what_was_recorded_is_read_once(app: &mut TestApp) {
        let target = app.spawn(Health(1));
        app.write(Hit { target, damage: 5 });
        app.tick();
        assert_eq!(app.read::<Died>().len(), 1);
        assert!(app.read::<Died>().is_empty());
    }

    #[bevy_harness::test(setup = combat)]
    fn a_blow_that_isnt_the_last_kills_no_one(app: &mut TestApp) {
        let target = app.spawn(Health(10));
        app.write(Hit { target, damage: 3 });
        app.tick();
        app.entity(target).assert_eq(Health(7));
        assert!(app.read::<Died>().is_empty());
    }

    #[bevy_harness::test(setup = combat)]
    fn messages_can_be_watched_once_built(app: &mut TestApp) {
        // What goes in is recorded too, once watched.
        app.watch::<Hit>();
        let target = app.spawn(Health(10));
        app.write(Hit { target, damage: 1 });
        app.tick();
        app.assert_wrote(|hit: &Hit| hit.damage == 1);
    }

    #[bevy_harness::test(setup = combat)]
    #[should_panic(expected = "Died")]
    fn a_failure_says_what_was_written_instead(app: &mut TestApp) {
        let target = app.spawn(Health(10));
        app.write(Hit { target, damage: 3 });
        app.tick();
        app.assert_wrote(|died: &Died| died.0 == target);
    }
}
