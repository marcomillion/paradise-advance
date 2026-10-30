// Parallel 32-bit Sonix device-key search worker.
// Firmware bytes and candidate keys remain in this browser; nothing is uploaded.
import init, { search_device_key_candidates } from "./pkg/paradise_wasm_core.js";

let cancelled = false;
self.onmessage = async (event) => {
  const data = event.data || {};
  if (data.type === "cancel") {
    cancelled = true;
    return;
  }
  if (data.type !== "search") return;
  cancelled = false;
  try {
    await init();
    const bytes = new Uint8Array(data.bytes);
    const lane = Math.max(0, Number(data.lane) | 0);
    const lanes = Math.max(1, Number(data.lanes) | 0);
    const chunkSize = Math.max(1000, Math.min(1000000, Number(data.chunkSize) | 0));
    const keySpace = 4294967296;
    const stride = lanes * chunkSize;
    let start = lane * chunkSize;
    let pendingProgress = 0;
    let lastReport = performance.now();
    while (start < keySpace && !cancelled) {
      const count = Math.min(chunkSize, keySpace - start);
      const found = search_device_key_candidates(bytes, start >>> 0, count);
      if (found !== undefined && found !== null) {
        // The Rust search walks this chunk in ascending order, so count only
        // candidates through the match rather than the entire allocated chunk.
        const foundKey = Number(found) >>> 0;
        const checked = Math.max(1, Math.min(count, foundKey - start + 1));
        // Include earlier completed chunks that have not reached the UI yet.
        self.postMessage({ type: "progress", delta: pendingProgress + checked });
        self.postMessage({ type: "found", key: foundKey });
        return;
      }
      pendingProgress += count;
      start += stride;
      const now = performance.now();
      if (now - lastReport >= 500 || start >= keySpace) {
        self.postMessage({ type: "progress", delta: pendingProgress });
        pendingProgress = 0;
        lastReport = now;
        // Let cancellation and other worker messages be processed between chunks.
        await new Promise(resolve => setTimeout(resolve, 0));
      }
    }
    if (pendingProgress) self.postMessage({ type: "progress", delta: pendingProgress });
    self.postMessage({ type: "done", cancelled });
  } catch (error) {
    self.postMessage({ type: "error", error: error instanceof Error ? error.message : String(error) });
  }
};
