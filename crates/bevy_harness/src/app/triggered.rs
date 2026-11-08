//! Events recorded as they're triggered, for a test to read back.

use bevy::app::App;
use bevy::ecs::event::Event;
use bevy::ecs::observer::On;
use bevy::ecs::resource::Resource;
use bevy::ecs::system::ResMut;

/// Every `E` triggered since the test last read them.
#[derive(Resource)]
pub(crate) struct Triggered<E: Event> {
    /// What was triggered, oldest first.
    pub triggered: Vec<E>,
}

/// Records an `E` as it's triggered.
fn record<E: Event + Clone>(event: On<E>, mut recorded: ResMut<Triggered<E>>) {
    recorded.triggered.push(event.event().clone());
}

/// Records every `E` triggered in `app` from now on, once however often
/// asked: by a global observer, so on any entity or none.
pub(crate) fn watch_triggered<E: Event + Clone>(app: &mut App) {
    if app.world().contains_resource::<Triggered<E>>() {
        return;
    }
    app.insert_resource(Triggered::<E> {
        triggered: Vec::new(),
    });
    app.add_observer(record::<E>);
}
