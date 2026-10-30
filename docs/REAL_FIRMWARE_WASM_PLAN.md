# Real Paradise browser emulator: engineering plan

## Non-negotiable requirement

The browser must execute the user's own 16 MiB SONIXDEV firmware dump on an emulated Sonix SNC73410 (ARMv7-M / Cortex-M3). Do not draw a substitute pet or simulate Tamagotchi mechanics in JavaScript.

## Confirmed from the supplied dump

- Length: 16,777,216 bytes (16 MiB)
- First eight bytes: `SONIXDEV`
- SHA-256: `e1c22b3e682ac83a9189486b69970057b9d757131e95e4aa6c4b9a59356646ee`

The load table at offset `0x000000` reports version `0x5A5A0033` (V3 format), configuration `0x13`, and the encrypted flag set. Its user-code region starts at `0x60001000` with a declared length of `0x10000`; the table lists no non-empty SRAM or DPD code region. The dump stays in the user's browser. Never commit it, upload it, or include a device key in this repository.

## Core research

The open-source Capybara project contains a real SNC73410 emulator, including the ARMv7-M instruction interpreter, memory map, peripherals, Sonix flash/load-table parsing, AES routines, and display controller. Its desktop app depends on native UI/audio/file-dialog/serial libraries, so the desktop crate cannot simply be compiled unchanged to WebAssembly.

Relevant source modules in Capybara (GPL-3.0-only):
- `src/emulator/cpu/`: registers, NVIC, Thumb-16 and Thumb-32 instructions
- `src/emulator/mmu/`: PRAM, boot ROM, SRAM, SPI flash and memory bus
- `src/emulator/peripherals/`: display, GPIO, timers, flash controller, SPI, DMA, ADC, PMU, UART and system registers
- `src/emulator/sonix.rs` and `src/emulator/aes.rs`: load-table parsing and flash decryption
- `src/emulator/loader.rs`: firmware loading and memory setup
- `src/emulator/mod.rs`: machine stepping, reset and save state

A port derived from this code must preserve GPL-3.0 notices and comply with the license. The browser project must not ship game firmware, a device key, or extracted proprietary game assets.

## Intended browser architecture

1. Create a separate Rust crate for the emulation core, with a small `wasm-bindgen` API. Keep native UI, audio output, file dialogs, tray, and serial-port code out of the WASM crate.
2. Expose operations such as `load_dump(bytes)`, `step(frame_budget)`, `set_button(id, pressed)`, `rotate_dial(delta)`, `press_dial()`, `framebuffer()`, `save_state()`, and `load_state(bytes)`.
3. Run the core in a Web Worker so CPU execution and key recovery do not freeze the page. Transfer framebuffer data back to the UI at frame intervals.
4. Keep the existing Paradise shell as the input/output frontend only. The 128x128 screen must display pixels produced by the emulated display controller.
5. Store saves locally in IndexedDB; firmware and saves remain on the user's device.
6. Add repeatable tests for load-table parsing, decryption, CPU reset vectors, display output, input, and save-state round trips before claiming it boots.

## Current status

The web UI is now explicitly marked as unavailable until the real WASM core is integrated. The hand-written virtual-pet simulation has been removed. The current JavaScript can recognise a 16 MiB SONIXDEV dump locally, but **it does not execute firmware**.

## Main technical blocker

The CPU/peripheral core must be separated from native-only dependencies and compiled/tested for `wasm32-unknown-unknown`. Firmware decryption/key recovery also needs a browser-appropriate implementation; relying on native multithreading without a deliberate WASM threading design will not work on ordinary static hosting. A placeholder module or an empty WASM binary must never be presented as an emulator.


## Portability audit (2026-10-30)

The first source-level audit confirms the core can be separated, but it is not a one-line WASM build:

- `src/emulator/loader.rs` currently reads the dump with `std::fs::read(path)` and resolves a device key from a path. The browser-facing loader must accept `&[u8]` plus an explicit optional key, and must never perform filesystem access.
- `src/emulator/mod.rs` includes `std::path::Path` / `PathBuf` in the machine API and implements file-backed save states. The portable machine API should be separated from native save-file management; browser saves should serialize state to bytes and persist through IndexedDB.
- `src/emulator/peripherals/display.rs` imports `egui::Color32` solely to convert the display buffer. The hardware display controller should expose RGB565 pixels or RGBA bytes without depending on egui; native UI conversion can remain in the desktop frontend.
- `src/emulator/peripherals/mod.rs` has the hardware peripheral set needed for a real machine, including display, GPIO, DMA, timers, XIP and flash control. These should be ported as a group with the CPU and memory bus, not replaced by JavaScript pet logic.
- The dump's load table marks the user-code region encrypted. The existing loader explicitly does not boot an encrypted image without a valid device key. The browser UI must surface this requirement rather than claiming that file recognition means the firmware is bootable.
- Capybara's `recherche_cle` implementation uses native threads to search the full 32-bit device-key space. That brute-force path is not suitable to run synchronously on the page. A browser strategy needs worker-based execution and a clear progress/cancellation API, or a user-provided key. We must not hide an expensive search behind a frozen UI.

## Implementation order (no fake-success checkpoints)

1. Extract a portable machine/loader API that accepts firmware bytes and has no `std::fs`, `PathBuf`, `egui`, `eframe`, audio-device, serial-port, tray or native-dialog dependency.
2. Vendor/adapt the CPU, memory bus and all firmware-relevant peripheral modules under the applicable GPL-3.0-only terms, retaining notices and documenting modifications.
3. Add a `wasm-bindgen` wrapper with load/reset/step/input/framebuffer/save-state operations.
4. Compile for `wasm32-unknown-unknown` in CI and fix compile errors against the actual target. Do not add an empty placeholder WASM artifact.
5. Test a known-good unencrypted test image first, then the user's encrypted dump once key handling is available. Verify that the screen pixels originate from `DisplayController`.
6. Connect the Worker to `emulator.js`; only then change the status from “core not integrated” to “core running”.

**Status of this audit:** source inspected; no WASM core has been ported or compiled yet. The browser still does not execute firmware.


## Portable-core implementation started (2026-10-31)

A separate `wasm-core/` crate has now been added. Its first component is a portable RGB565 display controller and a `wasm-bindgen` wrapper that exposes an RGBA framebuffer to browser code. It includes unit tests for framebuffer size, RGB565 colour conversion, out-of-bounds writes, and frame transfer. The crate includes the GPL-3.0 license because this display component is derived from Capybara.

This is only the first hardware boundary; it is **not a bootable emulator**. The current environment has no Rust/Cargo toolchain, so the crate has not been compiled or tested here. The existing site still does not execute firmware. Next: establish a Rust build/CI path, then port and test the CPU + memory bus + Sonix loader/decryption as an integrated machine before connecting UI controls.
