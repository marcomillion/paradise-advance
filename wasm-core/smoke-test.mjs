import { readFileSync } from "node:fs";
import { BrowserCpu, BrowserDisplay, BrowserFirmwareInfo, initSync } from "./pkg/paradise_wasm_core.js";

initSync(readFileSync(new URL("./pkg/paradise_wasm_core_bg.wasm", import.meta.url)));

const display = new BrowserDisplay();
if (display.width() !== 128 || display.height() !== 128) throw new Error("Unexpected LCD dimensions");
if (display.rgba_framebuffer().length !== 128 * 128 * 4) throw new Error("Unexpected RGBA framebuffer length");

const cpu = new BrowserCpu();
cpu.step(); // zero-filled PRAM encodes a valid Thumb LSL r0, r0, #0
if (cpu.instructions() !== 1n) throw new Error("WASM CPU binding did not execute one instruction");

let rejectedInvalidDump = false;
try { new BrowserFirmwareInfo(new Uint8Array(32)); } catch { rejectedInvalidDump = true; }
if (!rejectedInvalidDump) throw new Error("Firmware inspector accepted an invalid-sized dump");

console.log("WASM smoke test passed: display, CPU stepping, and firmware validation bindings.");
