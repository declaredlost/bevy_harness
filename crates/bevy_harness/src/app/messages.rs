//! Messages recorded as the app writes them, for a test to read back.

use bevy::app::{App, Last, MainScheduleOrder};
use bevy::ecs::message::{Message, MessageCursor, Messages};
use bevy::ecs::resource::Resource;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::{Res, ResMut};

/// Every `M` the app wrote since the test last read them.
#[derive(Resource)]
pub(crate) struct Recorded<M: Message> {
    /// What was written, oldest first.
    pub written: Vec<M>,
    /// Where recording is up to in the app's `Messages<M>`.
    cursor: MessageCursor<M>,
}

/// The schedule recording runs in: after `Last`, at each update's end.
#[derive(ScheduleLabel, Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Record;

/// Records what was written of `M` this update: whatever wrote it, in
/// whichever schedule, `Last` too.
fn record<M: Message + Clone>(messages: Res<Messages<M>>, mut recorded: ResMut<Recorded<M>>) {
    let Recorded { written, cursor } = &mut *recorded;
    written.extend(cursor.read(&messages).cloned());
}

/// Records every `M` `app` writes from now on, once however often asked;
/// adds the message if nothing has.
pub(crate) fn watch<M: Message + Clone>(app: &mut App) {
    if app.world().contains_resource::<Recorded<M>>() {
        return;
    }
    app.add_message::<M>();
    let cursor = app.world().resource::<Messages<M>>().get_cursor_current();
    app.insert_resource(Recorded::<M> {
        written: Vec::new(),
        cursor,
    });
    let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
    if !order.labels.contains(&Record.intern()) {
        order.insert_after(Last, Record);
    }
    app.add_systems(Record, record::<M>);
}
