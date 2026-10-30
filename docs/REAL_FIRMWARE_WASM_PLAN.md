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
