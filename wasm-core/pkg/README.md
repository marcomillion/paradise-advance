PARADISE WASM CORE — REAL FIRMWARE EMULATOR PORT IN PROGRESS

This is an incremental Rust port toward executing the actual Tamagotchi Paradise firmware. It is not yet a complete or reliably bootable SNC73410 emulator.

Implemented and covered by CI:
- 128x128 RGB565 display component and wasm-bindgen RGBA framebuffer.
- Sonix load-table parser for supported SNC73xx table versions.
- Browser-facing firmware metadata inspection.
- AES-128/AES-256 primitives ported from infinition/capybara (GPL-3.0-only).
- Sonix CBC/OFB encrypted-region decryption when a user supplies the 32-bit device key; decryption operates on an in-memory copy.
- Chunked browser-side device-key candidate search based on the first encrypted vector block, with progress and cancellation; it stays local and can take a very long time across the full 32-bit space.
- SNC73410 PRAM, boot-ROM, SRAM, mailbox and flash storage regions.
- Partial SN_SYS0 register page at `0x45000000`, including PLL lock/clock-source status, retained control registers, and read-only FEUSE device-key words.
- Incremental Thumb interpreter for common 16-bit instructions plus selected Thumb-2 MOVW/MOVT, BL/B.W, modified-immediate and shifted-register ALU operations, signed loads, LDRD/STRD, LDM/STM, LDREX/STREX, bit-field operations, CLZ, multiply/divide, IT blocks, MRS/MSR, and barriers. Unsupported encodings still fault explicitly.
- Browser bindings to load a Sonix image with a supplied key, copy its declared user-code region into PRAM, validate the reset vector, and step/run instructions.
- Experimental framebuffer bridge: after each CPU step/batch, a recognized DMA transfer to the LCD panel data register updates the display framebuffer; otherwise the snapshot is refreshed from the configured SRAM framebuffer base.
- Unit tests for display conversion, memory regions, Sonix table parsing, AES primitives, and selected CPU instructions.

Not yet implemented / known limitations:
- Full ARMv7-M/Thumb instruction coverage, nested/general exception handling, accurate CPU cycle timing, and complete special-register behavior. A narrow DMA-completion IRQ 58 path, SysTick exception, PRIMASK, ITSTATE and corresponding NVIC enable/clear-enable bits are implemented, but this is not a complete Cortex-M exception/NVIC model.
- Real boot ROM behavior and the chip's full memory aliases / MMIO bus dispatch; the SN_SYS0 page and several observed GPIO/SPI/DMA paths are only partial models.
- Most hardware peripherals are not yet wired to the CPU: timers, flash controller, interrupt priorities/pending state and much of the MMIO map remain missing. The current SPI, GPIO, SN_SYS0 and DMA paths are partial models; the DMA path supports the observed channel register layout and transfers into the LCD panel data register, but does not prove the retail display driver works.
- Complete initialization behavior required by the retail firmware.
- Verified boot of the user's encrypted dump. The browser UI now exposes the WASM CPU controls and an experimental SRAM framebuffer snapshot, but retail firmware has not been proven to boot.
- Device-key search is a heuristic candidate search over a 32-bit space, not yet validated against the user's retail dump. A candidate must still be tested by loading firmware and checking the reset vector / execution; a successful key search does not mean the emulator has booted.

The current CPU is deliberately strict: unsupported instructions halt with an explicit fault instead of silently pretending to execute them. A valid load/decryption result does not mean the firmware has booted.

Build and test:
  cargo test --manifest-path wasm-core/Cargo.toml
  rustup target add wasm32-unknown-unknown
  cargo build --manifest-path wasm-core/Cargo.toml --target wasm32-unknown-unknown --release
  wasm-pack build wasm-core --target web --out-dir pkg --release

Firmware dumps and device keys must remain local and must not be committed to this repository.
