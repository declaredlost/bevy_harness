//! What `#[bevy_harness::test]` rejects, and how it says so.

#[test]
fn the_attribute_rejects_what_it_cant_run() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
