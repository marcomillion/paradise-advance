/* @ts-self-types="./paradise_wasm_core.d.ts" */

/**
 * Browser API for incremental Thumb execution against the portable memory model.
 * Unsupported opcodes halt and report an error; this is not yet a full SNC73410 machine.
 */
export class BrowserCpu {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        BrowserCpuFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_browsercpu_free(ptr, 0);
    }
    /**
     * @returns {boolean}
     */
    display_enabled() {
        const ret = wasm.browsercpu_display_enabled(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {number}
     */
    display_frames_received() {
        const ret = wasm.browsercpu_display_frames_received(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    display_height() {
        const ret = wasm.browsercpu_display_height(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    display_nonzero_pixels() {
        const ret = wasm.browsercpu_display_nonzero_pixels(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    display_pixel_checksum() {
        const ret = wasm.browsercpu_display_pixel_checksum(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {Uint8Array}
     */
    display_rgba_framebuffer() {
        const ret = wasm.browsercpu_display_rgba_framebuffer(this.__wbg_ptr);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * @returns {number}
     */
    display_width() {
        const ret = wasm.browsercpu_display_width(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {Uint32Array}
     */
    dma_diagnostics() {
        const ret = wasm.browsercpu_dma_diagnostics(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @returns {boolean}
     */
    halted() {
        const ret = wasm.browsercpu_halted(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {bigint}
     */
    instructions() {
        const ret = wasm.browsercpu_instructions(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
     * @returns {string}
     */
    last_fault() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.browsercpu_last_fault(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {Uint8Array | number[]} bytes
     */
    load_flash(bytes) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.browsercpu_load_flash(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Load a Sonix image using the caller-supplied device key, then prepare its user-code
     * vector table in PRAM. This does not yet emulate the full boot ROM or MMIO devices.
     * @param {Uint8Array | number[]} bytes
     * @param {number} device_key
     */
    load_sonix_firmware(bytes, device_key) {
        const val2 = +device_key;
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.browsercpu_load_sonix_firmware(this.__wbg_ptr, ptr0, len0, val2);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    constructor() {
        const ret = wasm.browsercpu_new();
        this.__wbg_ptr = ret;
        BrowserCpuFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @returns {Uint32Array}
     */
    peripheral_diagnostics() {
        const ret = wasm.browsercpu_peripheral_diagnostics(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * @param {number} index
     * @returns {number}
     */
    register(index) {
        const ret = wasm.browsercpu_register(this.__wbg_ptr, index);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] >>> 0;
    }
    /**
     * @param {number} vector_base
     */
    reset_from_vectors(vector_base) {
        wasm.browsercpu_reset_from_vectors(this.__wbg_ptr, vector_base);
    }
    /**
     * @param {number} delta
     */
    rotate_dial(delta) {
        wasm.browsercpu_rotate_dial(this.__wbg_ptr, delta);
    }
    /**
     * @param {number} count
     * @returns {number}
     */
    run_steps(count) {
        const ret = wasm.browsercpu_run_steps(this.__wbg_ptr, count);
        return ret >>> 0;
    }
    /**
     * @param {number} index
     * @param {boolean} pressed
     */
    set_button(index, pressed) {
        const ret = wasm.browsercpu_set_button(this.__wbg_ptr, index, pressed);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {number} index
     * @param {number} value
     */
    set_register(index, value) {
        const ret = wasm.browsercpu_set_register(this.__wbg_ptr, index, value);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @returns {boolean}
     */
    sleeping() {
        const ret = wasm.browsercpu_sleeping(this.__wbg_ptr);
        return ret !== 0;
    }
    step() {
        const ret = wasm.browsercpu_step(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
}
if (Symbol.dispose) BrowserCpu.prototype[Symbol.dispose] = BrowserCpu.prototype.free;

export class BrowserDisplay {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        BrowserDisplayFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_browserdisplay_free(ptr, 0);
    }
    clear_dirty() {
        wasm.browserdisplay_clear_dirty(this.__wbg_ptr);
    }
    /**
     * @returns {number}
     */
    height() {
        const ret = wasm.browserdisplay_height(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {boolean}
     */
    is_dirty() {
        const ret = wasm.browserdisplay_is_dirty(this.__wbg_ptr);
        return ret !== 0;
    }
    constructor() {
        const ret = wasm.browserdisplay_new();
        this.__wbg_ptr = ret;
        BrowserDisplayFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @returns {Uint8Array}
     */
    rgba_framebuffer() {
        const ret = wasm.browserdisplay_rgba_framebuffer(this.__wbg_ptr);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * @returns {number}
     */
    width() {
        const ret = wasm.browserdisplay_width(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @param {number} x
     * @param {number} y
     * @param {number} rgb565
     */
    write_pixel(x, y, rgb565) {
        wasm.browserdisplay_write_pixel(this.__wbg_ptr, x, y, rgb565);
    }
}
if (Symbol.dispose) BrowserDisplay.prototype[Symbol.dispose] = BrowserDisplay.prototype.free;

/**
 * Structured inspection of the actual uploaded flash image; deliberately does not boot it.
 */
export class BrowserFirmwareInfo {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        BrowserFirmwareInfoFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_browserfirmwareinfo_free(ptr, 0);
    }
    /**
     * @returns {boolean}
     */
    encrypted() {
        const ret = wasm.browserfirmwareinfo_encrypted(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @param {Uint8Array | number[]} bytes
     */
    constructor(bytes) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.browserfirmwareinfo_inspect(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0];
        BrowserFirmwareInfoFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @returns {number}
     */
    load_config() {
        const ret = wasm.browserfirmwareinfo_load_config(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {string}
     */
    magic() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.browserfirmwareinfo_magic(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {number}
     */
    table_count() {
        const ret = wasm.browserfirmwareinfo_table_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    user_address() {
        const ret = wasm.browserfirmwareinfo_user_address(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    user_length() {
        const ret = wasm.browserfirmwareinfo_user_length(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    version() {
        const ret = wasm.browserfirmwareinfo_version(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) BrowserFirmwareInfo.prototype[Symbol.dispose] = BrowserFirmwareInfo.prototype.free;

/**
 * Exposes the real chip memory regions for incremental CPU/bus integration.
 * It does not execute instructions or dispatch MMIO yet.
 */
export class BrowserMemory {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        BrowserMemoryFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_browsermemory_free(ptr, 0);
    }
    /**
     * @param {Uint8Array | number[]} bytes
     */
    load_flash(bytes) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.browsermemory_load_flash(this.__wbg_ptr, ptr0, len0);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    constructor() {
        const ret = wasm.browsermemory_new();
        this.__wbg_ptr = ret;
        BrowserMemoryFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @param {number} address
     * @returns {number}
     */
    read_u32(address) {
        const ret = wasm.browsermemory_read_u32(this.__wbg_ptr, address);
        return ret >>> 0;
    }
    /**
     * @param {number} address
     * @returns {number}
     */
    read_u8(address) {
        const ret = wasm.browsermemory_read_u8(this.__wbg_ptr, address);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    reset_vector_pc() {
        const ret = wasm.browsermemory_reset_vector_pc(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    reset_vector_sp() {
        const ret = wasm.browsermemory_reset_vector_sp(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @param {number} address
     * @param {number} value
     * @returns {boolean}
     */
    write_u8(address, value) {
        const ret = wasm.browsermemory_write_u8(this.__wbg_ptr, address, value);
        return ret !== 0;
    }
}
if (Symbol.dispose) BrowserMemory.prototype[Symbol.dispose] = BrowserMemory.prototype.free;

/**
 * Decrypts the firmware's encrypted regions using a user-provided 32-bit device key.
 * The returned bytes are held in memory only; this function does not execute firmware.
 * @param {Uint8Array | number[]} bytes
 * @param {number} device_key
 * @returns {Uint8Array}
 */
export function decrypt_firmware_with_key(bytes, device_key) {
    const val2 = +device_key;
    const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.decrypt_firmware_with_key(ptr0, len0, val2);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v3 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v3;
}

/**
 * Search a bounded block of the 32-bit device-key space. Keep chunks small in browsers.
 * @param {Uint8Array | number[]} bytes
 * @param {number} start
 * @param {number} count
 * @returns {number | undefined}
 */
export function search_device_key_candidates(bytes, start, count) {
    const val2 = +start;
    const val3 = +count;
    const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.search_device_key_candidates(ptr0, len0, val2, val3);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0] === Number.MAX_SAFE_INTEGER ? undefined : ret[0];
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_throw_4c88d8bf264fd984: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbindgen_generic_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            table.set(0, undefined);
            table.set(1024, undefined);
            table.set(1025, null);
            table.set(1026, true);
            table.set(1027, false);
        },
    };
    return {
        __proto__: null,
        "./paradise_wasm_core_bg.js": import0,
    };
}

const BrowserCpuFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_browsercpu_free(ptr, 1));
const BrowserDisplayFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_browserdisplay_free(ptr, 1));
const BrowserFirmwareInfoFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_browserfirmwareinfo_free(ptr, 1));
const BrowserMemoryFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_browsermemory_free(ptr, 1));

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(bytes) {
    numBytesDecoded += bytes.length;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true });
        cachedTextDecoder.decode();
        numBytesDecoded = bytes.length;
    }
    return cachedTextDecoder.decode(bytes);
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

let __wbg_init_promise;
async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;
    if (__wbg_init_promise === undefined) {
        __wbg_init_promise = __wbg_init_once(module_or_path);
        __wbg_init_promise.catch(() => { __wbg_init_promise = undefined; });
    }
    return __wbg_init_promise;
}

async function __wbg_init_once(module_or_path) {

    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('paradise_wasm_core_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
