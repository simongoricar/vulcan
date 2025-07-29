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

After you've installed the Rust programming language, clone this repository and move into it. Then, simply run:
```bash
# Compiles and runs the graphical pixel sorting app.
cargo run --release

# Alternatively: build, then run the binary. The command above does both of these at once.
cargo build --release
./target/release/vulcan-gui
```
