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
their own time, which `build` leaves alone. The builder adds what's under
test and `build`s it:

- `plugins(..)`, `systems(schedule, ..)`, and `with(|app| ..)` for
  anything else (messages, resources, observers).
- `watch::<M>()`: records every `M` the app writes (see Messages), and
  `watch_triggered::<E>()` every `E` triggered (see Events).
- `fixed_hz(hz)`: its tick rate, more than 0. Unless it says, the rate the
  app sets itself (a plugin's `Time<Fixed>`), or 64.

`build` runs the first update, so `Startup` has run by the time the app
is handed over; the first tick is the test's. `app()` and `app_mut()`,
`world()` and `world_mut()` reach under it.

`resource::<R>()`, `resource_mut::<R>()`, `insert_resource(r)` and
`init_resource::<R>()` work as Bevy's do, but a missing resource fails at
the test's line, saying which.

## Time

Each `tick()` is one update running exactly one fixed step
(`TimeUpdateStrategy::FixedTimesteps`): `FixedUpdate` once, `Update`
seeing the same step as its delta, however slow the rate;
`ticks(n)` is `n` of them. Pausing `Time<Virtual>` or changing its speed
still holds the ticks back.
`advance(duration)` is as many ticks as that much game time at the app's
rate, and `secs(s)` the same in seconds. A negative or non-finite time is
a mistake in the test, and fails it. Nothing reads the wall clock, so a
test runs the same on any machine, however loaded.

## Entities and queries

- `spawn(bundle)`; `entity_mut(e)` to change one.
- `named("hero")`: the one entity with that `Name`; failing, if there's
  none, with the names there are. `try_named("hero")` is `None` instead,
  for a condition waiting on it. Either fails if several have it.
- `entity(e)`: `assert_has::<C>()`, `assert_lacks::<C>()`,
  `assert(|c: &C| ..)`, `assert_eq(c)`, `assert_despawned()`, each chained,
  and `get::<C>()`, failing without one. `has::<C>()`, `try_get::<C>()`
  and `is_spawned()` only ask, for a condition or an assertion of the
  test's own.
- `query::<D, F>()`: `assert_len(n)`, `assert_all(..)`, `assert_any(..)`,
  `assert_none(..)`, `single()`, `len()`, `is_empty()`, each failure
  naming the query (`Query<&Health, With<Dead>>`). It registers the
  components it names, so one on no entity yet is handled as it should be:
  `With` it matches nothing, `Without` everything.
- `single_mut::<C, F>()`: the one `C` on an entity `F` matches, to change;
  failing, if there isn't exactly one, with how many there are.
- `count::<F>()`: how many match a filter, registering them the same way.

## Messages

`write(m)` writes one in, read on the next tick. `watch::<M>()` (on the
builder, or the app once built) records every `M` written from then on,
whatever writes it, `Last` too, and adds the message if nothing has:
`read::<M>()`
takes what was recorded since the last read; `assert_wrote(|m: &M| ..)`
fails unless one matches, and `assert_wrote_none(|m: &M| ..)` if one does
(`|_| true`: if any was written at all). Asserting leaves them unread.

## Events

`trigger(e)` triggers an event now: its observers run, and then the
commands they queued, so what they spawn and trigger in turn is there
when it returns. `watch_triggered::<E>()` (on the builder, from
`Startup` on, or the app once built) records every `E` triggered from
then on, on an entity or none, by a global observer: `read_triggered::<E>()`,
`assert_triggered(|e: &E| ..)` and `assert_triggered_none(|e: &E| ..)` work
as their messages' counterparts do.

## Waiting

`until(|app| ..)` ticks until something holds, bounded in game time:
`within_ticks(n)`, `within(duration)` or `within_secs(s)` (rounded up to
whole ticks), failing if it never does and returning how many ticks it
took. The failure says how much game time that was, at what rate, and
what it waited for, if `describe("the foe is dead")` said. The
condition is handed the app itself, by `&mut`, so it asks what a test
does: `|app| app.count::<With<Dead>>() == 1`. A `TestWorld`'s is handed
the apps: `|world| world["client"].entity(e).has::<Seen>()`.

## One system

`run_system(system)` runs a system once, now, outside any schedule, and returns
its output: for a system's logic on its own, or reading the world through
a query.

## Several apps

`TestWorld::new().with("server", ..).with("client", ..)` ticks its apps
together, each tick every app's, in the order they were added;
`world["client"]` is one of them, and `until` waits on all of them. They
share a tick rate: one added at another fails the test. For a
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
  time; this bounds a hang (a deadlock, a blocking call). What setup
  builds has to be `Send` then. A test that times out is failed, and its
  app stops at its next tick; one blocked outside a tick is left running:
  Rust can't stop a thread from outside.

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

A test takes what its setup builds by `&mut`, or nothing, and returns
nothing or a `Result<(), E>`, failing on an error. It's a plain `fn`, not
`async` or generic, and has no `#[test]` of its own; its other attributes
(`should_panic`, `ignore`) are kept. Anything else is a compile error
saying so.

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
