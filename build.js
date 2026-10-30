#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');

const root = __dirname;
const out = path.join(root, 'dist');
const required = [
  'index.html',
  'styles.css',
  'app.js',
  'emulator.js',
  'wasm-core/browser-worker.js',
  'wasm-core/key-search-worker.js',
  'wasm-core/pkg/paradise_wasm_core.js',
  'wasm-core/pkg/paradise_wasm_core_bg.wasm'
];

for (const relative of required) {
  const absolute = path.join(root, relative);
  if (!fs.existsSync(absolute) || !fs.statSync(absolute).isFile()) {
    throw new Error('Required runtime file is missing: ' + relative);
  }
}

fs.rmSync(out, { recursive: true, force: true });
fs.mkdirSync(out, { recursive: true });

for (const relative of ['index.html', 'styles.css', 'app.js', 'emulator.js', 'README.md']) {
  fs.copyFileSync(path.join(root, relative), path.join(out, relative));
}

const workerOut = path.join(out, 'wasm-core');
const pkgOut = path.join(workerOut, 'pkg');
fs.mkdirSync(pkgOut, { recursive: true });
for (const worker of ['browser-worker.js', 'key-search-worker.js']) {
  fs.copyFileSync(path.join(root, 'wasm-core', worker), path.join(workerOut, worker));
}
fs.cpSync(path.join(root, 'wasm-core/pkg'), pkgOut, { recursive: true });

for (const relative of required) {
  const staged = path.join(out, relative);
  if (!fs.existsSync(staged) || !fs.statSync(staged).isFile()) {
    throw new Error('Build output is missing: ' + relative);
  }
}

console.log('Built Paradise Mod Studio into dist/.');
console.log('Serve dist/ over HTTP(S); the WebAssembly worker and package are included.');
