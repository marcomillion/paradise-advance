PARADISE WASM CORE — PORT IN PROGRESS

This is the first portable component extracted for the real firmware emulator. It is NOT yet a firmware emulator and cannot boot the supplied dump.

Current component:
- Rust RGB565 display controller model based on the corresponding hardware component in infinition/capybara (GPL-3.0-only).
- wasm-bindgen wrapper exposing a 128x128 RGBA framebuffer to browser JavaScript.
- Unit tests for framebuffer size, RGB565 conversion, bounds checking, and frame transfer.

Not implemented yet:
- ARMv7-M / Thumb CPU and exception controller
- Memory bus, PRAM, SRAM, SPI flash, XIP and boot setup
- Sonix load-table parser and AES decryption/device-key handling
- DMA and the full hardware peripheral set
- Input wiring, machine stepping, save states, and integration into the site
- A compiled or tested .wasm artifact

Build when Rust is available:
  cargo test --manifest-path wasm-core/Cargo.toml
  rustup target add wasm32-unknown-unknown
  cargo build --manifest-path wasm-core/Cargo.toml --target wasm32-unknown-unknown --release

The environment used to prepare this commit did not have Rust/Cargo installed, so these tests/build commands have not yet been run. The next substantial port must bring over the CPU, memory bus, loader/decryption, and required peripherals as a coherent system. Firmware dumps and device keys must remain user-local.
