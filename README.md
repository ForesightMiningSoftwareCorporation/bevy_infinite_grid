<div align="center">

# Bevy Infinite Grid

**Simple 3D infinite grid for bevy**

[![crates.io](https://img.shields.io/crates/v/bevy_infinite_grid)](https://crates.io/crates/bevy_infinite_grid)
[![docs.rs](https://docs.rs/bevy_infinite_grid/badge.svg)](https://docs.rs/bevy_infinite_grid)
[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)

</div>

# Demo

Run a simple implementation of this grid by cloning this repository and running:

```shell
cargo run --example simple
```

# Features

* Easily spawn an infinite grid aligned to the world origin and axes
* Spawn an unlimited number of axes aligned to arbitrary coordinate spaces

# Usage

Add the plugin to the `[dependencies]` in `Cargo.toml`

```toml
bevy_infinite_grid = { git = "https://github.com/ForesightMiningSoftwareCorporation/bevy_infinite_grid", branch = "main" }
```

Insert the infinite grid plugin after the default plugins.

```rust
.add_plugin(InfiniteGridPlugin)
```

And spawn the grid to see the results.

```rust
commands.spawn_bundle(InfiniteGridBundle::new(
    materials.add(InfiniteGridMaterial::default()),
));
```

See the [simple](examples/simple.rs) demo for an example of a minimal implementation.

# License

bevy_infinite_grid is free and open source! All code in this repository is dual-licensed under either:

* MIT License (LICENSE-MIT or <http://opensource.org/licenses/MIT>)
* Apache License, Version 2.0 (LICENSE-APACHE or <http://www.apache.org/licenses/LICENSE-2.0>)

at your option. This means you can select the license you prefer! This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are very good reasons to include both.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Sponsors

The creation and maintenance of Bevy Transform Gizmo is sponsored by Foresight Mining Software Corporation.

<img src="https://user-images.githubusercontent.com/2632925/151242316-db3455d1-4934-4374-8369-1818daf512dd.png" alt="Foresight Mining Software Corporation" width="480">
