//! A timeout is more than no time.

#[bevy_harness::test(timeout = 0)]
fn instant(app: &mut bevy_harness::TestApp) {}

fn main() {}
