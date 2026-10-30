PARADISE WASM CORE — PORT IN PROGRESS

This is a portable Rust foundation for the real firmware emulator. It is NOT yet a bootable emulator.

Implemented and covered by CI:
- RGB565 display controller model and wasm-bindgen RGBA framebuffer.
- Sonix load-table parser for supported SNC73xx table versions.
- 16 MiB firmware metadata inspection exposed to browser bindings.
- AES-128/AES-256 primitive port from infinition/capybara (GPL-3.0-only).
- Sonix encrypted-region decryption when the user supplies a 32-bit device key; operates on an in-memory copy and never writes firmware to the repository.
- SNC73410 memory region definitions and byte-addressable PRAM, boot ROM, SRAM, mailbox and flash storage.
- wasm-bindgen access to the memory model.
- Unit tests for display conversion, memory mapping, reset-vector storage, parser validation and safe unencrypted-image handling.

Still not implemented:
- ARMv7-M / Thumb CPU and exception controller.
- MMIO bus dispatch and CPU/peripheral cycle integration.
- Real hardware boot ROM behavior and firmware initialization.
- DMA and the complete hardware peripheral set wired to the CPU.
- Browser input wiring, machine stepping, save states and site integration.
- Verified boot of an encrypted Paradise dump. Decryption requires the correct device key, and successful decryption alone does not prove that the firmware boots.

Build/test:
  cargo test --manifest-path wasm-core/Cargo.toml
  rustup target add wasm32-unknown-unknown
  cargo build --manifest-path wasm-core/Cargo.toml --target wasm32-unknown-unknown --release
  wasm-pack build wasm-core --target web --out-dir pkg --release

CI builds the native tests and WebAssembly package. The browser-facing crate is still a collection of hardware boundaries, not a substitute for the actual CPU. Firmware dumps and device keys must remain user-local.
