//! A test isn't `async`.

#[bevy_harness::test]
async fn awaited(app: &mut bevy_harness::TestApp) {}

fn main() {}
