<div align="center">
  <h1 align="center">vulcan</h1>
  <h6 align="center">pixel sorting software</h6>
</div>

---

Vulcan is a small suite of tools for pixel sorting, an image glitching technique.


# 1. Project structure
This Rust project is split into two crates:
- `vulcan-core`, which is a "headless" pixel sorting crate, to allow you to integrate pixel sorting without depending on any of the GUI code, and
- `vulcan-gui`, which is a graphical application built on top of `vulcan-core` and `egui`, providing a nicer user experience for pixel sorting experiments.


# 2. Building and running
To compile either just the core crate or the GUI, you'll need only one thing: the [Rust toolchain](https://rustup.rs/) installed (latest stable; tested on 1.88.0).
The rustup installer will guide you through the entire process, including installing any kind of dependencies (e.g. on Windows, where you'll have to also install the Visual Studio Build Tools - the installer will propmt you).

On Linux, you may also need these dependencies, or equivalent for your distro:
```bash
# For printing.
sudo apt install libcups2-dev
```

After you've installed the Rust programming language, clone this repository and move into it. Then, simply run:
```bash
# Compiles and runs the graphical pixel sorting app.
cargo run --release

# Alternatively: build, then run the binary. The command above does both of these at once.
cargo build --release
./target/release/vulcan-gui
```

The time it takes to build depends highly on your machine, but around 4 minutes is expected.

## 2.1 Optimizations
If you wish to apply even more optimizations than the standard release mode when compiling, you can do the following at the expense of the compilation time:

### 2.1.1 `codegen-units` and `lto`
You may uncomment `codegen-units` and `lto` in `Cargo.toml`. This will slow down the build speed considerably, but may provide some additional speed.
```md
[profile.release]
# codegen-units = 1
# lto = "fat"
```

### 2.1.2 Profile-guided optimization (PGO)
This is a pretty time-consuming step. You will need [`cargo-pgo`](https://github.com/Kobzol/cargo-pgo), so install it before continuing (`rustup component add llvm-tools-preview`, then `cargo install cargo-pgo`).

Before continuing, modify your `.cargo/config.toml` to manually set `target-cpu=native` (see [`cargo-pgo` caveats](https://github.com/Kobzol/cargo-pgo?tab=readme-ov-file#caveats)).

For example, for the `x86_64-unknown-linux-gnu` target, add the following section to `.cargo/config.toml`:
```toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "target-cpu=native"]
```

(you can see your target by running `rustc -vV` and looking under the `host:` line)


To optimize with PGO, follow these rough steps (you may want to read more about what this does in the [`rustc` book chapter on PGO](https://doc.rust-lang.org/rustc/profile-guided-optimization.html)):
```bash
cargo pgo bench
cargo pgo optimize
```


---


## 2.2 Other notes
If you want to build more generic platform binaries, you may want to *comment out* the `rustflags` in `.cargo/config.toml`.
This is likely to produce slightly slower, but will not be bound to specific features available on the CPU of the build machine.

```md
[build]
rustflags = ["-C", "target-cpu=native"]
```
