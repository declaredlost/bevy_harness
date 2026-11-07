//! `plugins` isn't for a test with a `setup`.

fn server() -> bevy_harness::TestApp {
    bevy_harness::TestApp::builder().build()
}

struct Physics;

impl bevy::app::Plugin for Physics {
    fn build(&self, _: &mut bevy::app::App) {}
}

#[bevy_harness::test(setup = server, plugins = [Physics])]
fn both(app: &mut bevy_harness::TestApp) {}

fn main() {}
