//! Events: triggered in an app, observed, and recorded as they are.
//!
//! `cargo test --example events` runs its tests.

use bevy::prelude::*;
use bevy_harness::TestApp;

/// What's left of something.
#[derive(Component, Debug, PartialEq)]
struct Health(u32);

/// A blow landing on an entity, and how hard.
#[derive(EntityEvent, Clone, Debug)]
struct Struck {
    entity: Entity,
    damage: u32,
}

/// Someone a blow killed: told to whoever listens, on no entity.
#[derive(Event, Clone, Debug)]
struct Slain(Entity);

/// Blows landing as they're struck, and the deaths they're the last of:
/// the plugin under test.
struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(land);
    }
}

fn land(struck: On<Struck>, mut healths: Query<&mut Health>, mut commands: Commands) {
    let Ok(mut health) = healths.get_mut(struck.entity) else {
        return;
    };
    let alive = health.0 > 0;
    health.0 = health.0.saturating_sub(struck.damage);
    if alive && health.0 == 0 {
        commands.trigger(Slain(struck.entity));
    }
}

/// A blow landing, as a program sees it.
fn main() {
    let mut app = TestApp::builder()
        .plugins(CombatPlugin)
        .watch_triggered::<Slain>()
        .build();
    let target = app.spawn(Health(10));
    app.trigger(Struck {
        entity: target,
        damage: 10,
    });
    for slain in app.read_triggered::<Slain>() {
        println!("{:?} was slain", slain.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The combat, its deaths recorded.
    fn combat() -> TestApp {
        TestApp::builder()
            .plugins(CombatPlugin)
            .watch_triggered::<Slain>()
            .build()
    }

    #[bevy_harness::test(setup = combat)]
    fn a_mortal_blow_is_told(app: &mut TestApp) {
        let target = app.spawn(Health(10));
        app.trigger(Struck {
            entity: target,
            damage: 10,
        });
        // Observers run as the event is triggered: no tick in between.
        app.assert_triggered(|slain: &Slain| slain.0 == target);
    }

    #[bevy_harness::test(setup = combat)]
    fn a_blow_that_isnt_the_last_kills_no_one(app: &mut TestApp) {
        let target = app.spawn(Health(10));
        app.trigger(Struck {
            entity: target,
            damage: 3,
        });
        app.entity(target).assert_eq(Health(7));
        app.assert_triggered_none(|_: &Slain| true);
    }

    #[bevy_harness::test(setup = combat)]
    fn what_was_recorded_is_read_once(app: &mut TestApp) {
        let target = app.spawn(Health(1));
        app.trigger(Struck {
            entity: target,
            damage: 5,
        });
        assert_eq!(app.read_triggered::<Slain>().len(), 1);
        assert!(app.read_triggered::<Slain>().is_empty());
    }

    #[bevy_harness::test(setup = combat)]
    fn events_on_entities_are_recorded_too(app: &mut TestApp) {
        // What goes in is recorded too, once watched.
        app.watch_triggered::<Struck>();
        let target = app.spawn(Health(10));
        app.trigger(Struck {
            entity: target,
            damage: 1,
        });
        app.assert_triggered(|struck: &Struck| struck.entity == target && struck.damage == 1);
    }
}
