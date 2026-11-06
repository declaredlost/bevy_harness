<div align="center">

# bevy_harness

**Testing Bevy apps: the real app, headless, stepped tick by tick, and asserted on.**

[**github.com/declaredlost**](https://github.com/declaredlost)

</div>

```rust,ignore
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

> [!WARNING]
> **Early development.** The API may change at any time.

## Crates

| Crate | Description |
|---|---|
| [`bevy_harness`](crates/bevy_harness) | Testing Bevy apps headless, stepped tick by tick, and asserted on |
| [`bevy_harness_macros`](crates/bevy_harness_macros) | `bevy_harness`'s `#[test]` attribute |

## Requirements

- **Rust + Cargo**: 1.99+, Edition 2024
- **Bevy**: 0.20

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines, and
[CHANGELOG.md](CHANGELOG.md) for release notes.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

