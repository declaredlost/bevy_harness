# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `bevy_harness`: testing Bevy apps headless, stepped tick by tick, and
  asserted on; and `bevy_harness_macros`, its `#[test]` attribute. Moved
  from the game repository.
- Observer events, recorded as messages are: `watch_triggered`,
  `read_triggered`, `assert_triggered` and `assert_triggered_none`; and
  `trigger`, which runs the commands its observers queue before returning.
- `assert_wrote_none`, failing on a message written that shouldn't be.
- `single_mut`, the one component a filter matches, to change.
- `named` and `try_named`, the one entity with a `Name`.
- `resource`, `resource_mut`, `insert_resource` and `init_resource`,
  failing at the test's line on a missing resource.
- `EntityCheck::has`, `try_get` and `is_spawned`: questions that never
  fail, for conditions.

### Changed

- `until`'s condition is handed the app (or a `TestWorld`'s apps) by
  `&mut`, not a `&World`, so it can query, count and find by name; the
  `Ticked` trait lost its `View`.
- `EntityCheck`'s assertions take `&self`.
- Query failures name the whole query: `Query<&Health, With<Dead>>`.
