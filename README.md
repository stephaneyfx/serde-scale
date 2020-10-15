<!-- cargo-sync-readme start -->

# Overview
- [📦 crates.io](https://crates.io/crates/serde-scale)
- [📖 Documentation](https://docs.rs/serde-scale)
- [⚖ zlib license](https://opensource.org/licenses/Zlib)

Serializer and deserializer for the [SCALE encoding](https://substrate.dev/docs/en/knowledgebase/advanced/codec)
based on [`serde`](https://docs.rs/serde).

# Example
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Point {
    x: i8,
    y: i8,
}

let point = Point { x: 3, y: 4 };
let deserialized = serde_scale::from_slice(&serde_scale::to_vec(&point).unwrap()).unwrap();
assert_eq!(point, deserialized);
```

# Conformance
`Option<bool>` is serialized as a single byte according to the SCALE encoding.

# Features
`no_std` is supported by disabling default features.

- `std`: Support for `std`. It is enabled by default.
- `alloc`: Support for the `alloc` crate.

🔖 Features enabled in build dependencies and proc-macros are also enabled for normal
dependencies, which may cause `serde` to have its `std` feature on when it is not desired.
Nightly cargo prevents this from happening with
[`-Z features=host_dep`](https://github.com/rust-lang/cargo/issues/7915#issuecomment-683294870)
or the following in `.cargo/config`:

```toml
[unstable]
features = ["host_dep"]
```

For example, this issue arises when depending on `parity-scale-codec-derive`.

# Test
Most tests live in the `serde-scale-tests` crate (part of the workspace) in order to avoid
dependencies enabling `serde` features.

```sh
cargo test --workspace
```

# Contribute
All contributions shall be licensed under the [zlib license](https://opensource.org/licenses/Zlib).

# Related projects
[parity-scale-codec](https://crates.io/crates/parity-scale-codec): Reference Rust implementation

<!-- cargo-sync-readme end -->
