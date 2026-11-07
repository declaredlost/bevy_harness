//! A test takes one argument, not two.

#[bevy_harness::test]
fn two(app: &mut bevy_harness::TestApp, other: &mut bevy_harness::TestApp) {}

fn main() {}
