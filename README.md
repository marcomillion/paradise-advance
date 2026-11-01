# Paradise Mod Studio

A local-first browser workbench for inspecting Tamagotchi Paradise firmware and editing sprite artwork.

## What works

- **Sprite Studio:** import PNGs or Studio JSON projects, create sprites up to 256 × 256, draw/erase/fill/pick colours, flip, resize, undo/redo, and export PNG or project JSON.
- **Local workspace recovery:** sprites, gallery drafts, animation frames and playback speed, editor colour, grid, zoom, and the active sprite are saved in this browser's local storage. Export/import a workspace backup to preserve multiple sprites, drafts, preferences, and animation frames. Firmware bytes are not saved there. Device keys are not saved unless the user explicitly enables the separate “Remember this key on this device only” option; that option stores the key in this browser’s local storage and does not share it with other users.
- **Animation timeline:** assemble sprite frames, preview playback, and export/import animation JSON.
- **Firmware Lab:** inspect a local dump's header and SHA-256, view a coarse byte-region map, scan experimental bitmap candidates, and scan documented sprite-package layouts for decodable frames/subimages.
- **Preview Lab:** loads the compiled WebAssembly core, inspects supported 16 MiB Sonix dumps, and exposes experimental CPU step/run controls and input buttons. The optional device-key search splits candidates across up to four local workers and reports tested candidates, progress, average and peak observed throughput, elapsed time, remaining ETA, and estimated full-scan duration. These rates are measured on the current device; it remains a brute-force search across up to 4,294,967,296 candidates.
- **Local Gallery:** collect drafts, reopen them in Sprite Studio, export PNG/project JSON, or remove drafts. This is a local gallery, not a public sharing service.

## Run locally

Serve the repository root over HTTP (recommended; browser modules and WebAssembly may not work when opening `index.html` directly as a `file://` URL).

For example, with Python installed:

```bash
python -m http.server 8000
```

Then open `http://localhost:8000` in a modern browser.

To build a portable static website folder for hosting or sharing, run:

```bash
npm run build
```

The complete site is staged in `dist/`, including the WebAssembly worker and generated package. Serve that folder over HTTP(S); opening the HTML directly as a `file://` URL is not supported for the emulator.

## Run the browser smoke tests

Node.js 22 or newer and Python 3 are recommended.

```bash
npm install
npx playwright install chromium
npm run test:e2e
```

The Playwright suite checks sprite creation, renaming and persistence; pixel painting, fill, undo/redo and resizing; PNG/JSON imports and exports; local gallery drafts; workspace backup recovery and selected animation-frame persistence; animation import and preview; device controls; browser WebAssembly startup, and synthetic Sonix firmware booting through DMA to verified RGB565 pixels on the browser canvas. GitHub Actions runs these checks alongside the Rust unit tests and WebAssembly build.

The WebAssembly worker and generated package are stored in `wasm-core/browser-worker.js` and `wasm-core/pkg/`. To rebuild the package after changing Rust code:

```bash
cargo test --manifest-path wasm-core/Cargo.toml
rustup target add wasm32-unknown-unknown
cargo build --manifest-path wasm-core/Cargo.toml --target wasm32-unknown-unknown --release
wasm-pack build wasm-core --target web --out-dir pkg --release
```

## Emulator limitations

The WASM core is an **experimental emulator port**, not a verified full-device emulator. It supports Sonix load-table inspection/decryption, a growing subset of Thumb instructions (including word-exclusive operations, CLREX, SVC exception entry/return, WFE event handling, and WFI sleep/wakeup), a partial memory/peripheral model, and an experimental DMA-to-SPI LCD framebuffer path. A synthetic Thumb firmware test now programs the DMA registers through CPU instructions and verifies RGB565 pixels reach the browser framebuffer; this validates the internal pipeline, not a retail-firmware boot. Most SNC73410 peripherals, complete instruction coverage, accurate boot-ROM behavior, and a verified retail-firmware boot are still missing. Unsupported instructions intentionally report faults instead of pretending to execute.

The device-key search is still brute force: parallel workers can improve throughput on multi-core devices, but cannot guarantee a quick result and do not make a full 32-bit search small. A candidate must be validated by loading the firmware. The firmware inspector and sprite extractor also use best-effort format recognition. A candidate bitmap is not necessarily a real sprite, and unrecognised/compressed resource formats may be missed. Firmware patching/rebuilding and a public community backend are not implemented.

## Privacy and backups

Firmware files and entered device keys are processed in the browser; this app does not upload them. Keep original dumps unchanged and do not commit firmware dumps or device keys. Export important sprites/projects as JSON backups, especially before clearing browser storage.

## Reference implementation and licensing

The browser emulator adapts selected core components from [Capybara](https://github.com/infinition/capybara), an independent Sonix SNC73410 emulator licensed under GPL-3.0-only. Paradise Advance is also distributed under GPL-3.0-only. The browser implementation is a separate WebAssembly port; it is not yet feature-equivalent to Capybara's complete desktop emulator, and retail-firmware boot is not verified. See `THIRD_PARTY_NOTICES.md` for attribution and `LICENSE` for the full licence.
