//! Messages recorded as the app writes them, for a test to read back.

use bevy::app::{App, Last};
use bevy::ecs::message::{Message, MessageReader};
use bevy::ecs::resource::Resource;
use bevy::ecs::system::ResMut;

/// Every `M` the app wrote since the test last read them.
#[derive(Resource)]
pub(crate) struct Recorded<M: Message>(pub Vec<M>);

impl<M: Message> Default for Recorded<M> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

/// Records what was written of `M` this update, at its very end: whatever
/// wrote it, in whichever schedule.
pub(crate) fn record<M: Message + Clone>(
    mut written: MessageReader<M>,
    mut recorded: ResMut<Recorded<M>>,
) {
    recorded.0.extend(written.read().cloned());
}

/// Records every `M` `app` writes, once however often it's asked.
pub(crate) fn watch<M: Message + Clone>(app: &mut App) {
    if app.world().contains_resource::<Recorded<M>>() {
        return;
    }
    app.init_resource::<Recorded<M>>()
        .add_systems(Last, record::<M>);
}
