# clew-derive

Procedural macros for the [clew](https://github.com/sysint64/clew) UI framework.

## Macros

### `#[derive(WidgetState)]`

Implements `WidgetState` trait for custom widget state structs.

```rust
#[derive(Default, WidgetState)]
pub struct MyWidget {
    counter: i32,
}
```

### `#[derive(Identifiable)]`

Implements `Identifiable` trait. Looks for a field marked with `#[id]`, or falls back to a field named `id`.

```rust
#[derive(Identifiable)]
pub struct Item {
    #[id]
    item_id: u64,
    name: String,
}

// Or simply:
#[derive(Identifiable)]
pub struct Item {
    id: u64,
    name: String,
}
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
