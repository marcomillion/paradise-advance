PARADISE WASM CORE — REAL FIRMWARE EMULATOR PORT IN PROGRESS

This is an incremental Rust port toward executing the actual Tamagotchi Paradise firmware. It is not yet a complete or reliably bootable SNC73410 emulator.

Implemented and covered by CI:
- 128x128 RGB565 display component and wasm-bindgen RGBA framebuffer.
- Sonix load-table parser for supported SNC73xx table versions.
- Browser-facing firmware metadata inspection.
- AES-128/AES-256 primitives ported from infinition/capybara (GPL-3.0-only).
- Sonix CBC/OFB encrypted-region decryption when a user supplies the 32-bit device key; decryption operates on an in-memory copy.
- SNC73410 PRAM, boot-ROM, SRAM, mailbox and flash storage regions.
- Incremental Thumb interpreter for common 16-bit instructions plus Thumb-2 MOVW/MOVT and BL; includes immediate operations, shifts, ALU operations, loads/stores, branches, stack and block transfers.
- Browser bindings to load a Sonix image with a supplied key, copy its declared user-code region into PRAM, validate the reset vector, and step/run instructions.
- Experimental framebuffer bridge: after each CPU step/batch, the display snapshot is refreshed from the configured SRAM framebuffer base and exposed through the CPU WASM API.
- Unit tests for display conversion, memory regions, Sonix table parsing, AES primitives, and selected CPU instructions.

Not yet implemented / known limitations:
- Full ARMv7-M/Thumb instruction coverage, exceptions, NVIC and accurate CPU timing.
- Real boot ROM behavior and the chip's full memory aliases / MMIO bus dispatch.
- Hardware peripherals wired to the CPU, including DMA/display scanout, timers, GPIO/button controls, SPI and flash controller. The current framebuffer bridge only snapshots SRAM; it does not emulate DMA or prove the retail display driver works.
- Complete initialization behavior required by the retail firmware.
- Verified boot of the user's encrypted dump. The browser UI now exposes the WASM CPU controls and an experimental SRAM framebuffer snapshot, but retail firmware has not been proven to boot.
- Key recovery in-browser; the current loading API requires the correct device key from the user. A wrong key should fail reset-vector validation, but this is not a substitute for complete firmware validation.

The current CPU is deliberately strict: unsupported instructions halt with an explicit fault instead of silently pretending to execute them. A valid load/decryption result does not mean the firmware has booted.

Build and test:
  cargo test --manifest-path wasm-core/Cargo.toml
  rustup target add wasm32-unknown-unknown
  cargo build --manifest-path wasm-core/Cargo.toml --target wasm32-unknown-unknown --release
  wasm-pack build wasm-core --target web --out-dir pkg --release

Firmware dumps and device keys must remain local and must not be committed to this repository.
