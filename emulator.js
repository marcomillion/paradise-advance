(() => {
  const $ = id => document.getElementById(id);
  const title = $('emulatorStatusTitle');
  const status = $('emulatorStatusText');
  const inputStatus = $('deviceInputStatus');
  const fileInput = $('firmwareInput');
  let worker = null;
  let initPromise = null;
  let requestId = 0;
  let pending = new Map();
  let firmwareBytes = null;
  let firmwareInfo = null;
  let coreReady = false;
  let cpuLoaded = false;

  function setStatus(heading, message) {
    if (title) title.textContent = heading;
    if (status) status.textContent = message;
  }
  function failAll(message) {
    for (const [, entry] of pending) entry.reject(new Error(message));
    pending.clear();
  }
  function startWorker() {
    if (coreReady) return Promise.resolve();
    if (initPromise) return initPromise;
    initPromise = new Promise((resolve, reject) => {
      try {
        worker = new Worker('wasm-core/browser-worker.js', { type: 'module' });
      } catch (error) {
        worker = null; initPromise = null; reject(error); return;
      }
      worker.onmessage = event => {
        const data = event.data || {};
        if (data.type === 'key-search-progress') {
          const el = $('deviceKeySearchStatus');
          if (el) el.textContent = 'Checked ' + Number(data.tested).toLocaleString() + ' / 4,294,967,296 candidates (' + Number(data.percent).toFixed(4) + '%). Search is local to this browser.';
          return;
        }
        const entry = pending.get(data.id);
        if (entry) {
          pending.delete(data.id);
          if (data.ok) entry.resolve(data.result);
          else entry.reject(new Error(data.error || 'WASM worker failed.'));
        }
      };
      worker.onerror = event => {
        const message = event.message || 'Could not load the WebAssembly worker package.';
        failAll(message);
        if (!coreReady) {
          setStatus('Browser core unavailable', 'The WASM package could not be loaded. The build must finish publishing wasm-core/pkg before firmware can execute.');
          worker?.terminate(); worker = null; initPromise = null;
        }
      };
      const id = ++requestId;
      pending.set(id, { resolve, reject });
      worker.postMessage({ id, type: 'init' });
    }).then(() => { coreReady = true; return undefined; }).catch(error => { initPromise = null; throw error; });
    return initPromise;
  }
  function request(type, payload = {}) {
    return startWorker().then(() => new Promise((resolve, reject) => {
      const id = ++requestId;
      pending.set(id, { resolve, reject });
      worker.postMessage({ id, type, payload });
    }));
  }
  async function refreshFirmwareFrame() {
    if (!cpuLoaded) return;
    const frame = await request('frame');
    const canvas = $('previewCanvas');
    if (!canvas || !frame || !frame.rgba) return;
    canvas.width = frame.width;
    canvas.height = frame.height;
    const ctx = canvas.getContext('2d');
    const pixels = frame.rgba instanceof Uint8Array ? frame.rgba : new Uint8Array(frame.rgba);
    ctx.putImageData(new ImageData(new Uint8ClampedArray(pixels), frame.width, frame.height), 0, 0);
    const empty = $('previewEmpty');
    if (empty) empty.style.display = 'none';
    canvas.style.imageRendering = 'pixelated';
    const scale = Number($('previewScale')?.value || 3);
    canvas.style.width = (frame.width * scale) + 'px';
    canvas.style.height = (frame.height * scale) + 'px';
    if (inputStatus) inputStatus.textContent = 'Experimental SRAM framebuffer snapshot · display DMA and physical inputs are not yet emulated.';
  }
  window.paradiseBrowserPetDraw = () => { refreshFirmwareFrame().catch(() => {}); };
  function formatHex(value) {
    return '0x' + (Number(value) >>> 0).toString(16).toUpperCase().padStart(8, '0');
  }
  function parseKey() {
    const raw = ($('deviceKeyInput')?.value || '').trim().replace(/^0x/i, '');
    if (!/^[0-9a-fA-F]{1,8}$/.test(raw)) throw new Error('Enter the 32-bit device key as 1–8 hexadecimal digits, optionally prefixed with 0x.');
    return Number.parseInt(raw, 16) >>> 0;
  }
  async function inspectSelectedFile(file) {
    if (!file) return;
    firmwareBytes = null; firmwareInfo = null; cpuLoaded = false;
    $('emulatorStartCpuBtn').disabled = true;
    $('emulatorStepBtn').disabled = true;
    $('emulatorRunBtn').disabled = true;
    try {
      setStatus('Reading firmware locally…', 'The dump stays in this browser tab. Nothing is uploaded.');
      const bytes = new Uint8Array(await file.arrayBuffer());
      firmwareBytes = bytes;
      const info = await request('inspect', { bytes });
      firmwareInfo = info;
      const version = '0x' + (info.version >>> 0).toString(16).toUpperCase();
      setStatus(info.encrypted ? 'Sonix firmware recognised · encrypted' : 'Sonix firmware recognised', 
        info.magic + ' · table version ' + version + ' · ' + info.tableCount + ' load table(s) · user code at ' +
        formatHex(info.userAddress) + ' (' + info.userLength.toLocaleString() + ' bytes). ' +
        (info.encrypted ? 'The real device key is required before CPU execution can start.' : 'This image is marked unencrypted; a zero key can be used for loading.'));
      $('emulatorStartCpuBtn').disabled = false;
      if (inputStatus) inputStatus.textContent = 'Firmware inspected locally · CPU and device inputs are experimental';
    } catch (error) {
      firmwareBytes = null; firmwareInfo = null;
      setStatus('Firmware inspection failed', error.message || String(error));
    }
  }
  fileInput?.addEventListener('change', event => { inspectSelectedFile(event.target.files?.[0]); });
  $('emulatorStartCpuBtn')?.addEventListener('click', async () => {
    if (!firmwareBytes || !firmwareInfo) return;
    const button = $('emulatorStartCpuBtn');
    button.disabled = true;
    try {
      let key = 0;
      if (firmwareInfo.encrypted) key = parseKey();
      await request('create-cpu');
      const result = await request('load-firmware', { bytes: firmwareBytes, deviceKey: key });
      cpuLoaded = true;
      await refreshFirmwareFrame();
      $('emulatorStepBtn').disabled = false;
      $('emulatorRunBtn').disabled = false;
      setStatus('CPU loaded · not yet a complete emulator',
        'The firmware user-code region was decrypted and copied into PRAM. Initial SP ' + formatHex(result.initialSp) +
        ', reset PC ' + formatHex(result.initialPc) + '. The current Thumb interpreter is partial and hardware MMIO/display DMA are not wired yet; step/run may stop on an unsupported instruction.');
    } catch (error) {
      cpuLoaded = false;
      setStatus('Firmware did not start', error.message || String(error));
    } finally {
      button.disabled = !firmwareBytes;
    }
  });
  $('emulatorStepBtn')?.addEventListener('click', async () => {
    if (!cpuLoaded) return;
    try {
      const r = await request('step');
      await refreshFirmwareFrame();
      setStatus('CPU instruction executed', 'Instructions: ' + r.instructions + ' · PC ' + formatHex(r.pc) + ' · halted: ' + r.halted + (r.halted ? ' · fault: ' + r.fault : '') + '. The shown image is only an SRAM framebuffer snapshot; DMA/display hardware is not emulated yet.');
      if (r.halted) { $('emulatorStepBtn').disabled = true; $('emulatorRunBtn').disabled = true; }
    } catch (error) {
      setStatus('CPU stopped', error.message || String(error));
      $('emulatorStepBtn').disabled = true; $('emulatorRunBtn').disabled = true;
    }
  });
  $('emulatorRunBtn')?.addEventListener('click', async () => {
    if (!cpuLoaded) return;
    try {
      const r = await request('run', { count: 1000 });
      await refreshFirmwareFrame();
      setStatus('CPU run batch finished', 'Executed ' + r.executed + ' instruction(s) · total ' + r.instructions + ' · PC ' + formatHex(r.pc) + ' · halted: ' + r.halted + (r.halted ? ' · fault: ' + r.fault : '') + '. The shown image is an SRAM snapshot, not verified LCD output.');
      if (r.halted) { $('emulatorStepBtn').disabled = true; $('emulatorRunBtn').disabled = true; }
    } catch (error) {
      setStatus('CPU stopped', error.message || String(error));
      $('emulatorStepBtn').disabled = true; $('emulatorRunBtn').disabled = true;
    }
  });
  $('searchDeviceKeyBtn')?.addEventListener('click', async () => {
    if (!firmwareBytes || !firmwareInfo) {
      $('deviceKeySearchStatus').textContent = 'Choose a valid 16 MiB firmware dump first.';
      return;
    }
    if (!firmwareInfo.encrypted) {
      $('deviceKeySearchStatus').textContent = 'This firmware table is marked unencrypted; a device-key search is unnecessary.';
      return;
    }
    const searchBtn=$('searchDeviceKeyBtn');
    const cancelBtn=$('cancelDeviceKeySearchBtn');
    searchBtn.disabled=true; cancelBtn.disabled=false;
    $('deviceKeySearchStatus').textContent='Starting local key search. This can take a very long time; keep this tab open.';
    try {
      const result=await request('search-key',{bytes:firmwareBytes,chunkSize:20000});
      if (result.cancelled) {
        $('deviceKeySearchStatus').textContent='Key search cancelled. No key was sent anywhere.';
      } else if (result.found) {
        $('deviceKeyInput').value=formatHex(result.key);
        $('deviceKeySearchStatus').textContent='Candidate key found after checking ' + Number(result.tested).toLocaleString() + ' candidates. Try loading firmware to validate it.';
      } else {
        $('deviceKeySearchStatus').textContent='No candidate matched the full 32-bit key space.';
      }
    } catch (error) {
      $('deviceKeySearchStatus').textContent='Key search failed: ' + (error.message || String(error));
    } finally {
      searchBtn.disabled=false; cancelBtn.disabled=true;
    }
  });
  $('cancelDeviceKeySearchBtn')?.addEventListener('click', async () => {
    $('cancelDeviceKeySearchBtn').disabled=true;
    try { await request('cancel-key-search'); } catch (_) {}
  });
  document.querySelectorAll('[data-device-button]').forEach((button, index) => {
    const send = pressed => {
      if (!cpuLoaded) {
        if (inputStatus) inputStatus.textContent = 'Load firmware before sending emulated button input.';
        return;
      }
      request('input', { button: index, pressed }).catch(error => {
        if (inputStatus) inputStatus.textContent = 'Input error: ' + (error.message || String(error));
      });
    };
    button.addEventListener('pointerdown', event => { event.preventDefault(); send(true); });
    button.addEventListener('pointerup', () => send(false));
    button.addEventListener('pointerleave', () => send(false));
    button.addEventListener('pointercancel', () => send(false));
  });
  const dial = $('zoomDial');
  dial?.addEventListener('wheel', event => {
    event.preventDefault();
    if (!cpuLoaded) {
      if (inputStatus) inputStatus.textContent = 'Load firmware before rotating the emulated dial.';
      return;
    }
    request('input', { dialDelta: event.deltaY < 0 ? 1 : -1 }).catch(error => {
      if (inputStatus) inputStatus.textContent = 'Dial input error: ' + (error.message || String(error));
    });
  }, { passive: false });
  dial?.addEventListener('click', () => {
    if (inputStatus) inputStatus.textContent = cpuLoaded
      ? 'Dial rotation is connected to the initial GPIO model; dial-press hardware is not mapped yet.'
      : 'Load firmware before sending emulated input.';
  });
  startWorker().then(() => {
    setStatus('WASM core ready · awaiting firmware', 'The browser package loaded successfully. Choose a local 16 MiB dump to inspect it. Firmware execution is experimental; no simulated pet is shown.');
  }).catch(error => {
    setStatus('WASM package not available yet', 'The worker could not load wasm-core/pkg. This can happen until the CI build publishes the generated package. Details: ' + (error.message || String(error)));
  });
})();
