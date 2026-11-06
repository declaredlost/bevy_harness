# bevy_harness

Testing Bevy apps: the real app, built from the plugins under test,
headless, on time that moves a fixed tick at a time, and asserted on.

```rust
#[bevy_harness::test(plugins = [CombatPlugin])]
fn a_mortal_blow_kills(app: &mut TestApp) {
    app.watch::<Died>();
    let hurt = app.spawn((Name::new("hurt"), Health(10)));
    app.write(Hit { target: hurt, damage: 10 });
    app.tick();
    app.entity(hurt).assert_eq(Health(0));
    app.assert_wrote(|died: &Died| died.0 == hurt);
}
```

## The app

`TestApp::builder()` starts on Bevy's `MinimalPlugins` (time and tasks, no
window); `TestApp::bare_builder()` on an empty app, for plugins that bring
their own. The builder adds what's under test and `build`s it:

- `plugins(..)`, `systems(schedule, ..)`, and `with(|app| ..)` for
  anything else (messages, resources, observers).
- `watch::<M>()`: records every `M` the app writes (see Messages).
- `fixed_hz(hz)`: its tick rate; 64 unless it says.

`build` runs the first update, so `Startup` has run by the time the app
is handed over; the first tick is the test's. `app()` and `app_mut()`,
`world()` and `world_mut()` reach under it.

## Time

Each `tick()` is one update running exactly one fixed step
(`TimeUpdateStrategy::FixedTimesteps`): `FixedUpdate` once, `Update`
seeing the same step as its delta; `ticks(n)` is `n` of them.
`advance(duration)` is as many ticks as that much game time at the app's
rate, and `secs(s)` the same in seconds. A negative or non-finite time is
a mistake in the test, and fails it. Nothing reads the wall clock, so a
test runs the same on any machine, however loaded.

## Entities and queries

- `spawn(bundle)`; `entity_mut(e)` to change one.
- `entity(e)`: `assert_has::<C>()`, `assert_lacks::<C>()`,
  `assert(|c: &C| ..)`, `assert_eq(c)`, `assert_despawned()`, `get::<C>()`,
  each chained.
- `query::<D, F>()`: `assert_len(n)`, `assert_all(..)`, `assert_any(..)`,
  `assert_none(..)`, `single()`, `len()`, `is_empty()`. Read-only: one
  naming what was never registered matches nothing.
- `count::<F>()`: how many match a filter, without changing anything.

## Messages

`write(m)` writes one in, read on the next tick. `watch::<M>()` (on the
builder, or the app once built) records every `M` written: `read::<M>()`
takes what was recorded since the last read, and `assert_wrote(|m: &M| ..)`
fails unless one matches.

## Waiting

`until(|world| ..)` ticks until something holds, bounded in game time:
`within_ticks(n)`, `within(duration)` or `within_secs(s)`, failing if it
never does and returning how many ticks it took.

## One system

`run_system(system)` runs a system once, now, outside any schedule, and returns
its output: for a system's logic on its own, or reading the world through
a query.

## Several apps

`TestWorld::new().with("server", ..).with("client", ..)` ticks its apps
together, each tick every app's, in the order they were added;
`world["client"]` is one of them, and `until` waits on all of them. For a
server and its clients in one process, linked by a transport there.

## As a test

`#[bevy_harness::test]` hands a test what its setup builds, by `&mut`,
and fails it past a timeout instead of letting it hang.

```rust
#[bevy_harness::test(setup = testing::server, timeout = 5)]
fn a_grip_lets_go(app: &mut TestApp) { .. }
```

- `setup = ..`: a function's path (called), or any other expression,
  built in the test (`setup = duel(10)`). Without it, the minimal app.
- `plugins = [..]`: the minimal app with these plugins, for a test that
  needs nothing else. Not with `setup`, which adds its own.
- `timeout = 5`: wall-clock time, in seconds (`0.5` too, checked as it
  compiles), or any `Duration` expression. The test runs on a thread of its
  own then; a panic in it is still the test's failure. `within` bounds game
  time; this bounds a hang (a deadlock, a blocking call). A test that times
  out is failed, but its thread is left running: Rust can't stop a thread
  from outside.

What a setup builds needn't be an app: a fixture holding one and
whatever else the test needs, by name, is handed over the same way.

```rust
struct Duel { app: TestApp, hero: Entity, foe: Entity }

#[bevy_harness::test(setup = duel(10))]
fn both_sides_of_a_duel_strike(duel: &mut Duel) {
    duel.app.ticks(4);
    duel.app.entity(duel.foe).assert_eq(Health(6));
}
```

A test takes what its setup builds or nothing; its other attributes
(`should_panic`, `ignore`) are kept.

## Failures

Each points at the test's own line (`#[track_caller]`) and says what was
there instead: an entity by its `Name`, with its components; the value it
had; the messages written.

## Examples

Each is a small game and its tests; `cargo test` runs them, `cargo run
--example <name>` the same app as a plain value.

- `basics`: building, ticking, seconds, entities, queries, waiting, one
  system, a failure.
- `messages`: writing in, watching, reading, `assert_wrote`.
- `fixtures`: a setup building more than the app, by function and by
  expression; a timeout.
- `worlds`: a server and a client in a `TestWorld`, linked in process.
