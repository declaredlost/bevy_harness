//! A test isn't marked `#[test]` twice.

#[bevy_harness::test]
#[test]
fn twice(app: &mut bevy_harness::TestApp) {}

fn main() {}
