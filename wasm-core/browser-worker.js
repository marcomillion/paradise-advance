// Browser worker adapter for the generated wasm-pack package.
// Copy the CI artifact's pkg/ directory beside this file before enabling it in the site.
// No firmware bytes or device keys are logged or sent over the network.
import init, { BrowserCpu, BrowserFirmwareInfo, search_device_key_candidates } from "./pkg/paradise_wasm_core.js";

let ready = false;
let cpu = null;

function postError(id, error) {
  self.postMessage({ id, ok: false, error: error instanceof Error ? error.message : String(error) });
}
self.onmessage = async (event) => {
  const { id, type, payload = {} } = event.data || {};
  try {
    if (type === "init") {
      await init();
      ready = true;
      self.postMessage({ id, ok: true, type: "ready" });
      return;
    }
    if (!ready) throw new Error("WASM core is not initialized.");
    if (type === "cancel-key-search") {
      keySearchToken++;
      self.postMessage({ id, ok: true, result: { cancelled: true }});
      return;
    }
    if (type === "search-key") {
      const token = ++keySearchToken;
      const bytes = new Uint8Array(payload.bytes);
      const chunkSize = Number(payload.chunkSize) || 20000;
      let start = Number(payload.start) || 0;
      const keySpace = 4294967296;
      while (start < keySpace && token === keySearchToken) {
        const count = Math.min(chunkSize, keySpace - start);
        const found = search_device_key_candidates(bytes, start >>> 0, count);
        if (found !== undefined && found !== null) {
          self.postMessage({ id, ok: true, result: { found: true, key: Number(found) >>> 0, tested: start + count }});
          return;
        }
        start += count;
        self.postMessage({ type: "key-search-progress", tested: start, total: keySpace, percent: start * 100 / keySpace });
        await new Promise(resolve => setTimeout(resolve, 0));
      }
      if (token === keySearchToken) self.postMessage({ id, ok: true, result: { found: false, exhausted: true, tested: start }});
      return;
    }
    if (type === "inspect") {
      const info = new BrowserFirmwareInfo(new Uint8Array(payload.bytes));
      self.postMessage({ id, ok: true, result: {
        magic: info.magic(), version: info.version(), loadConfig: info.load_config(),
        encrypted: info.encrypted(), tableCount: info.table_count(),
        userAddress: info.user_address(), userLength: info.user_length()
      }});
      return;
    }
    if (type === "create-cpu") {
      cpu = new BrowserCpu();
      self.postMessage({ id, ok: true, result: { created: true, displayWidth: cpu.display_width(), displayHeight: cpu.display_height() }});
      return;
    }
    if (type === "load-firmware") {
      if (!cpu) throw new Error("Create the CPU before loading firmware.");
      cpu.load_sonix_firmware(new Uint8Array(payload.bytes), Number(payload.deviceKey) >>> 0);
      self.postMessage({ id, ok: true, result: {
        loaded: true, initialSp: cpu.register(13), initialPc: cpu.register(15),
        instructions: cpu.instructions()
      }});
      return;
    }
    if (type === "step") {
      if (!cpu) throw new Error("CPU has not been created.");
      cpu.step();
      self.postMessage({ id, ok: true, result: { instructions: cpu.instructions(), pc: cpu.register(15), halted: cpu.halted(), fault: cpu.last_fault() }});
      return;
    }
    if (type === "run") {
      if (!cpu) throw new Error("CPU has not been created.");
      const executed = cpu.run_steps(Number(payload.count) || 1);
      self.postMessage({ id, ok: true, result: { executed, instructions: cpu.instructions(), pc: cpu.register(15), halted: cpu.halted(), fault: cpu.last_fault() }});
      return;
    }
    if (type === "input") {
      if (!cpu) throw new Error("CPU has not been created.");
      if (payload.button !== undefined) cpu.set_button(Number(payload.button), Boolean(payload.pressed));
      if (payload.dialDelta !== undefined) cpu.rotate_dial(Number(payload.dialDelta) | 0);
      self.postMessage({ id, ok: true, result: { accepted: true }});
      return;
    }
    if (type === "register") {
      if (!cpu) throw new Error("CPU has not been created.");
      self.postMessage({ id, ok: true, result: { value: cpu.register(Number(payload.index)) }});
      return;
    }
    if (type === "frame") {
      if (!cpu) throw new Error("CPU has not been created.");
      self.postMessage({ id, ok: true, result: { width: cpu.display_width(), height: cpu.display_height(), enabled: cpu.display_enabled(), rgba: cpu.display_rgba_framebuffer() }});
      return;
    }
    throw new Error("Unknown worker message type: " + String(type));
  } catch (error) {
    postError(id, error);
  }
};
