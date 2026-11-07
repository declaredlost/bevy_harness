//! `plugins` is a list.

struct Physics;

#[bevy_harness::test(plugins = Physics)]
fn listless(app: &mut bevy_harness::TestApp) {}

fn main() {}
