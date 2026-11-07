//! A test isn't generic.

#[bevy_harness::test]
fn generic<T>(app: &mut bevy_harness::TestApp) {}

fn main() {}
