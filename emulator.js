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
          setStatus('Browser core unavailable', 'The WASM package could not be loaded. Check that wasm-core/pkg is present and the site is served over HTTP(S).');
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
    if (inputStatus) inputStatus.textContent = 'LCD diagnostics · DMA frames: ' + Number(frame.framesReceived || 0).toLocaleString() + ' · non-black pixels: ' + Number(frame.nonzeroPixels || 0).toLocaleString() + ' / ' + (frame.width * frame.height).toLocaleString() + ' · pixel checksum: 0x' + (Number(frame.pixelChecksum || 0) >>> 0).toString(16).toUpperCase().padStart(8, '0') + ' · DMA writes: ' + Number(frame.dmaDiagnostics?.[0] || 0).toLocaleString() + ' · last DMA MMIO: ' + formatHex(frame.dmaDiagnostics?.[1] || 0) + ' = ' + formatHex(frame.dmaDiagnostics?.[2] || 0) + ' · transfer attempts: ' + Number(frame.dmaDiagnostics?.[3] || 0).toLocaleString() + ' · last transfer: src ' + formatHex(frame.dmaDiagnostics?.[4] || 0) + ' → dst ' + formatHex(frame.dmaDiagnostics?.[5] || 0) + ' · units ' + Number(frame.dmaDiagnostics?.[6] || 0).toLocaleString() + ';
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
  const savedKeyStorageName = 'paradise-mod-studio-device-key-v1';
  function syncRememberedKey() {
    const checkbox = $('rememberDeviceKey');
    const input = $('deviceKeyInput');
    if (!checkbox || !input) return;
    try {
      if (!checkbox.checked) {
        localStorage.removeItem(savedKeyStorageName);
        return;
      }
      const raw = input.value.trim().replace(/^0x/i, '');
      if (/^[0-9a-fA-F]{1,8}$/.test(raw)) {
        localStorage.setItem(savedKeyStorageName, formatHex(Number.parseInt(raw, 16) >>> 0));
      } else {
        localStorage.removeItem(savedKeyStorageName);
      }
    } catch (_) {
      const status = $('deviceKeySearchStatus');
      if (status) status.textContent = 'This browser blocked local storage; the key will not be remembered.';
    }
  }
  try {
    const savedKey = localStorage.getItem(savedKeyStorageName);
    if (savedKey && /^0x[0-9a-fA-F]{8}$/.test(savedKey)) {
      $('deviceKeyInput').value = savedKey.slice(0, 2).toLowerCase() + savedKey.slice(2).toUpperCase();
      $('rememberDeviceKey').checked = true;
    }
  } catch (_) {}
  $('rememberDeviceKey')?.addEventListener('change', syncRememberedKey);
  $('deviceKeyInput')?.addEventListener('input', () => {
    if ($('rememberDeviceKey')?.checked) syncRememberedKey();
  });
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
  $('emulatorLoadFirmwareBtn')?.addEventListener('click', () => fileInput?.click());
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
      setStatus(r.sleeping ? 'CPU waiting for interrupt/event' : 'CPU instruction executed', 'Instructions: ' + r.instructions + ' · PC ' + formatHex(r.pc) + ' · halted: ' + r.halted + ' · sleeping: ' + Boolean(r.sleeping) + (r.halted ? ' · fault: ' + r.fault : '') + '. Display output is experimental; DMA and LCD behavior are only partially modeled.');
      if (r.halted) { $('emulatorStepBtn').disabled = true; $('emulatorRunBtn').disabled = true; }
    } catch (error) {
      setStatus('CPU stopped', error.message || String(error));
      $('emulatorStepBtn').disabled = true; $('emulatorRunBtn').disabled = true;
    }
  });
  $('emulatorRunBtn')?.addEventListener('click', async () => {
    if (!cpuLoaded) return;
    try {
      const r = await request('run', { count: Number($('emulatorRunCount')?.value || 1000) });
      await refreshFirmwareFrame();
      setStatus('CPU run batch finished', 'Executed ' + r.executed + ' instruction(s) · total ' + r.instructions + ' · PC ' + formatHex(r.pc) + ' · halted: ' + r.halted + ' · sleeping: ' + Boolean(r.sleeping) + (r.halted ? ' · fault: ' + r.fault : '') + '. The shown image is an SRAM snapshot, not verified LCD output.');
      if (r.halted) { $('emulatorStepBtn').disabled = true; $('emulatorRunBtn').disabled = true; }
    } catch (error) {
      setStatus('CPU stopped', error.message || String(error));
      $('emulatorStepBtn').disabled = true; $('emulatorRunBtn').disabled = true;
    }
  });
  let activeKeyWorkers = [];
  let keySearchCancelled = false;
  let cancelActiveKeySearch = null;
  $('searchDeviceKeyBtn')?.addEventListener('click', async () => {
    const searchStatus = $('deviceKeySearchStatus');
    if (!firmwareBytes || !firmwareInfo) {
      searchStatus.textContent = 'Choose a valid 16 MiB firmware dump first.';
      return;
    }
    if (!firmwareInfo.encrypted) {
      searchStatus.textContent = 'This firmware table is marked unencrypted; a device-key search is unnecessary.';
      return;
    }
    const searchBtn = $('searchDeviceKeyBtn');
    const cancelBtn = $('cancelDeviceKeySearchBtn');
    const keySpace = 4294967296;
    const lanes = Math.max(1, Math.min(4, (Number(navigator.hardwareConcurrency) || 2) - 1));
    const chunkSize = 250000;
    let finished = 0;
    let tested = 0;
    let foundKey = null;
    let failure = null;
    let peakRate = 0;
    let lastSampleAt = performance.now();
    let lastSampleTested = 0;
    const searchStartedAt = lastSampleAt;
    const formatEta = seconds => {
      if (!Number.isFinite(seconds) || seconds < 0) return 'calculating…';
      if (seconds < 60) return Math.ceil(seconds) + 's';
      if (seconds < 3600) return Math.ceil(seconds / 60) + 'm';
      if (seconds < 86400) return Math.floor(seconds / 3600) + 'h ' + Math.ceil((seconds % 3600) / 60) + 'm';
      return Math.floor(seconds / 86400) + 'd ' + Math.ceil((seconds % 86400) / 3600) + 'h';
    };
    const formatDuration = seconds => {
      if (!Number.isFinite(seconds) || seconds < 0) return '—';
      if (seconds < 60) return Math.floor(seconds) + 's';
      if (seconds < 3600) return Math.floor(seconds / 60) + 'm ' + Math.floor(seconds % 60) + 's';
      if (seconds < 86400) return Math.floor(seconds / 3600) + 'h ' + Math.floor((seconds % 3600) / 60) + 'm';
      return Math.floor(seconds / 86400) + 'd ' + Math.floor((seconds % 86400) / 3600) + 'h';
    };
    const setMetric = (id, value) => { const el = $(id); if (el) el.textContent = value; };
    const renderMetrics = (elapsedSeconds, rate, eta) => {
      const percent = Math.min(100, tested * 100 / keySpace);
      setMetric('keySearchTested', tested.toLocaleString());
      setMetric('keySearchPercent', percent.toFixed(6) + '%');
      setMetric('keySearchRate', Math.round(rate).toLocaleString() + ' keys/s');
      setMetric('keySearchPeakRate', Math.round(peakRate).toLocaleString() + ' keys/s');
      setMetric('keySearchElapsed', formatDuration(elapsedSeconds));
      setMetric('keySearchEta', tested >= keySpace ? '0s' : formatEta(eta));
      setMetric('keySearchFullEta', rate > 0 ? formatEta(keySpace / rate) : 'calculating…');
      setMetric('keySearchWorkers', String(lanes));
      const fill = $('keySearchProgressFill');
      if (fill) fill.style.width = percent + '%';
      const track = $('keySearchProgressTrack');
      if (track) track.setAttribute('aria-valuenow', percent.toFixed(6));
    };
    tested = 0;
    peakRate = 0;
    renderMetrics(0, 0, Infinity);
    keySearchCancelled = false;
    activeKeyWorkers = [];
    searchBtn.disabled = true;
    cancelBtn.disabled = false;
    searchStatus.textContent = 'Starting parallel local search across ' + lanes + ' worker(s)…';
    const terminateWorkers = () => {
      for (const worker of activeKeyWorkers) worker.terminate();
      activeKeyWorkers = [];
    };
    try {
      await new Promise((resolve, reject) => {
        cancelActiveKeySearch = () => { keySearchCancelled = true; terminateWorkers(); resolve(); };
        const bytes = firmwareBytes;
        const workers = [];
        for (let lane = 0; lane < lanes; lane++) {
          const worker = new Worker('wasm-core/key-search-worker.js', { type: 'module' });
          workers.push(worker);
          activeKeyWorkers.push(worker);
          worker.onmessage = event => {
            const data = event.data || {};
            if (data.type === 'progress') {
              tested = Math.min(keySpace, tested + Math.max(0, Number(data.delta || 0)));
              const now = performance.now();
              const elapsedSeconds = Math.max(0.001, (now - searchStartedAt) / 1000);
              const rate = tested / elapsedSeconds;
              if (now - lastSampleAt >= 1000) {
                const sampleSeconds = (now - lastSampleAt) / 1000;
                const sampleRate = Math.max(0, tested - lastSampleTested) / sampleSeconds;
                peakRate = Math.max(peakRate, sampleRate);
                lastSampleAt = now;
                lastSampleTested = tested;
              }
              peakRate = Math.max(peakRate, rate);
              const eta = rate > 0 ? (keySpace - tested) / rate : Infinity;
              renderMetrics(elapsedSeconds, rate, eta);
              searchStatus.textContent = 'Searching locally · ' + lanes + ' worker(s) · average ' +
                Math.round(rate).toLocaleString() + ' keys/s · ETA ' + formatEta(eta) + '.';
              return;
            }
            if (data.type === 'found') {
              foundKey = Number(data.key) >>> 0;
              terminateWorkers();
              resolve();
              return;
            }
            if (data.type === 'done') {
              finished++;
              if (finished === lanes) resolve();
              return;
            }
            if (data.type === 'error') {
              failure = new Error(data.error || 'A key-search worker failed.');
              terminateWorkers();
              reject(failure);
            }
          };
          worker.onerror = event => {
            failure = new Error(event.message || 'A key-search worker failed.');
            terminateWorkers();
            reject(failure);
          };
          worker.postMessage({ type: 'search', bytes, lane, lanes, chunkSize });
        }
      });
      if (keySearchCancelled) {
        searchStatus.textContent = 'Key search cancelled. No key was sent anywhere.';
      } else if (foundKey !== null) {
        $('deviceKeyInput').value = formatHex(foundKey);
        if ($('rememberDeviceKey')?.checked) syncRememberedKey();
        searchStatus.textContent = 'Candidate key ' + formatHex(foundKey) + ' found. Load firmware to validate it; the key was found locally.';
      } else if (failure) {
        searchStatus.textContent = 'Key search failed: ' + failure.message;
      } else {
        searchStatus.textContent = 'No candidate matched the full 32-bit key space.';
      }
    } catch (error) {
      searchStatus.textContent = keySearchCancelled
        ? 'Key search cancelled. No key was sent anywhere.'
        : 'Key search failed: ' + (error.message || String(error));
    } finally {
      const now = performance.now();
      const elapsedSeconds = Math.max(0, (now - searchStartedAt) / 1000);
      const rate = elapsedSeconds > 0 ? tested / elapsedSeconds : 0;
      if (now > lastSampleAt && tested > lastSampleTested) {
        peakRate = Math.max(peakRate, (tested - lastSampleTested) / ((now - lastSampleAt) / 1000));
      }
      peakRate = Math.max(peakRate, rate);
      const eta = rate > 0 ? Math.max(0, keySpace - tested) / rate : Infinity;
      renderMetrics(elapsedSeconds, rate, eta);
      // The ETA describes the active search, not a hypothetical full scan
      // after the search has already found a key, been cancelled, or failed.
      if (foundKey !== null) setMetric('keySearchEta', 'Key found');
      else if (keySearchCancelled) setMetric('keySearchEta', 'Cancelled');
      else if (failure) setMetric('keySearchEta', 'Failed');
      terminateWorkers();
      cancelActiveKeySearch = null;
      searchBtn.disabled = false;
      cancelBtn.disabled = true;
    }
  });
  $('cancelDeviceKeySearchBtn')?.addEventListener('click', async () => {
    $('cancelDeviceKeySearchBtn').disabled = true;
    if (cancelActiveKeySearch) cancelActiveKeySearch();
    else { try { await request('cancel-key-search'); } catch (_) {} }
  });
  // Keyboard and pointer-drag dial events are emitted by the shared device UI.
  // Pointer buttons and the mouse wheel are handled below to avoid duplicate edges.
  document.addEventListener('paradise:device-input', event => {
    const detail = event.detail || {};
    if (!cpuLoaded) return;
    if (detail.source === 'keyboard' && detail.type === 'button') {
      const indexes = { A: 0, B: 1, C: 2, DIAL: 3 };
      const index = indexes[detail.button];
      if (index !== undefined) request('input', { button: index, pressed: Boolean(detail.pressed) }).catch(error => {
        if (inputStatus) inputStatus.textContent = 'Keyboard input error: ' + (error.message || String(error));
      });
    } else if (detail.type === 'dial' && detail.source === 'pointer' && Number(detail.steps)) {
      request('input', { dialDelta: Number(detail.steps) }).catch(error => {
        if (inputStatus) inputStatus.textContent = 'Dial input error: ' + (error.message || String(error));
      });
    }
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
  const sendDialPress = pressed => {
    if (!cpuLoaded) {
      if (inputStatus) inputStatus.textContent = 'Load firmware before sending emulated input.';
      return;
    }
    request('input', { button: 3, pressed }).catch(error => {
      if (inputStatus) inputStatus.textContent = 'Dial input error: ' + (error.message || String(error));
    });
  };
  dial?.addEventListener('pointerdown', event => { event.preventDefault(); sendDialPress(true); });
  dial?.addEventListener('pointerup', () => sendDialPress(false));
  dial?.addEventListener('pointerleave', () => sendDialPress(false));
  dial?.addEventListener('pointercancel', () => sendDialPress(false));
  startWorker().then(() => {
    setStatus('WASM core ready · awaiting firmware', 'The browser package loaded successfully. Choose a local 16 MiB dump to inspect it. Firmware execution is experimental; no simulated pet is shown.');
  }).catch(error => {
    setStatus('WASM package not available yet', 'The worker could not load wasm-core/pkg. Serve the site over HTTP(S), check the browser console, and confirm both generated package files are present. Details: ' + (error.message || String(error)));
  });
})();
