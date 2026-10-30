# Paradise Mod Studio

A local-first browser workbench for inspecting Tamagotchi Paradise firmware and editing sprite artwork.

## What works

- **Sprite Studio:** import PNGs or Studio JSON projects, create sprites up to 256 × 256, draw/erase/fill/pick colours, flip, resize, undo/redo, and export PNG or project JSON.
- **Local workspace recovery:** sprites, gallery drafts, editor colour, grid, zoom, and the active sprite are saved in this browser's local storage. Export/import a workspace backup to preserve multiple sprites, drafts, preferences, and animation frames. Firmware bytes and device keys are not saved there.
- **Animation timeline:** assemble sprite frames, preview playback, and export/import animation JSON.
- **Firmware Lab:** inspect a local dump's header and SHA-256, view a coarse byte-region map, scan experimental bitmap candidates, and scan documented sprite-package layouts for decodable frames/subimages.
- **Preview Lab:** loads the compiled WebAssembly core, inspects supported 16 MiB Sonix dumps, and exposes experimental CPU step/run controls and input buttons.
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

The Playwright suite checks sprite creation/renaming and local persistence, pixel painting and JSON export, local gallery draft management, and browser WebAssembly startup. GitHub Actions runs these checks alongside the Rust unit tests and WebAssembly build.

The WebAssembly worker and generated package are stored in `wasm-core/browser-worker.js` and `wasm-core/pkg/`. To rebuild the package after changing Rust code:

```bash
cargo test --manifest-path wasm-core/Cargo.toml
rustup target add wasm32-unknown-unknown
cargo build --manifest-path wasm-core/Cargo.toml --target wasm32-unknown-unknown --release
wasm-pack build wasm-core --target web --out-dir pkg --release
```

## Emulator limitations

The WASM core is an **experimental emulator port**, not a verified full-device emulator. It supports Sonix load-table inspection/decryption, a growing subset of Thumb instructions, a partial memory/peripheral model, and an experimental framebuffer path. Most SNC73410 peripherals, complete instruction coverage, accurate boot-ROM behavior, and a verified retail-firmware boot are still missing. Unsupported instructions intentionally report faults instead of pretending to execute.

The firmware inspector and sprite extractor also use best-effort format recognition. A candidate bitmap is not necessarily a real sprite, and unrecognised/compressed resource formats may be missed. Firmware patching/rebuilding and a public community backend are not implemented.

## Privacy and backups

Firmware files and entered device keys are processed in the browser; this app does not upload them. Keep original dumps unchanged and do not commit firmware dumps or device keys. Export important sprites/projects as JSON backups, especially before clearing browser storage.

## Reference implementation and licensing

The emulator port references code from [Capybara](https://github.com/infinition/capybara), which is licensed under GPL-3.0-only. See the notices and source files under `wasm-core/` for implementation details.
