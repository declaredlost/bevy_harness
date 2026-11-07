//! A test returns nothing or a `Result`.

#[bevy_harness::test]
fn counted(app: &mut bevy_harness::TestApp) -> u32 {
    3
}

fn main() {}
