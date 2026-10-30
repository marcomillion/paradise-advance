/* tslint:disable */
/* eslint-disable */

/**
 * Browser API for incremental Thumb execution against the portable memory model.
 * Unsupported opcodes halt and report an error; this is not yet a full SNC73410 machine.
 */
export class BrowserCpu {
    free(): void;
    [Symbol.dispose](): void;
    display_enabled(): boolean;
    display_height(): number;
    display_rgba_framebuffer(): Uint8Array;
    display_width(): number;
    halted(): boolean;
    instructions(): bigint;
    last_fault(): string;
    load_flash(bytes: Uint8Array | number[]): void;
    /**
     * Load a Sonix image using the caller-supplied device key, then prepare its user-code
     * vector table in PRAM. This does not yet emulate the full boot ROM or MMIO devices.
     */
    load_sonix_firmware(bytes: Uint8Array | number[], device_key: number): void;
    constructor();
    register(index: number): number;
    reset_from_vectors(vector_base: number): void;
    run_steps(count: number): number;
    set_register(index: number, value: number): void;
    step(): void;
}

export class BrowserDisplay {
    free(): void;
    [Symbol.dispose](): void;
    clear_dirty(): void;
    height(): number;
    is_dirty(): boolean;
    constructor();
    rgba_framebuffer(): Uint8Array;
    width(): number;
    write_pixel(x: number, y: number, rgb565: number): void;
}

/**
 * Structured inspection of the actual uploaded flash image; deliberately does not boot it.
 */
export class BrowserFirmwareInfo {
    free(): void;
    [Symbol.dispose](): void;
    encrypted(): boolean;
    constructor(bytes: Uint8Array | number[]);
    load_config(): number;
    magic(): string;
    table_count(): number;
    user_address(): number;
    user_length(): number;
    version(): number;
}

/**
 * Exposes the real chip memory regions for incremental CPU/bus integration.
 * It does not execute instructions or dispatch MMIO yet.
 */
export class BrowserMemory {
    free(): void;
    [Symbol.dispose](): void;
    load_flash(bytes: Uint8Array | number[]): void;
    constructor();
    read_u32(address: number): number;
    read_u8(address: number): number;
    reset_vector_pc(): number;
    reset_vector_sp(): number;
    write_u8(address: number, value: number): boolean;
}

/**
 * Decrypts the firmware's encrypted regions using a user-provided 32-bit device key.
 * The returned bytes are held in memory only; this function does not execute firmware.
 */
export function decrypt_firmware_with_key(bytes: Uint8Array | number[], device_key: number): Uint8Array;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_browsercpu_free: (a: number, b: number) => void;
    readonly __wbg_browserdisplay_free: (a: number, b: number) => void;
    readonly __wbg_browserfirmwareinfo_free: (a: number, b: number) => void;
    readonly __wbg_browsermemory_free: (a: number, b: number) => void;
    readonly browsercpu_display_enabled: (a: number) => number;
    readonly browsercpu_display_height: (a: number) => number;
    readonly browsercpu_display_rgba_framebuffer: (a: number) => [number, number];
    readonly browsercpu_display_width: (a: number) => number;
    readonly browsercpu_halted: (a: number) => number;
    readonly browsercpu_instructions: (a: number) => bigint;
    readonly browsercpu_last_fault: (a: number) => [number, number];
    readonly browsercpu_load_flash: (a: number, b: number, c: number) => [number, number];
    readonly browsercpu_load_sonix_firmware: (a: number, b: number, c: number, d: number) => [number, number];
    readonly browsercpu_new: () => number;
    readonly browsercpu_register: (a: number, b: number) => [number, number, number];
    readonly browsercpu_reset_from_vectors: (a: number, b: number) => void;
    readonly browsercpu_run_steps: (a: number, b: number) => number;
    readonly browsercpu_set_register: (a: number, b: number, c: number) => [number, number];
    readonly browsercpu_step: (a: number) => [number, number];
    readonly browserdisplay_clear_dirty: (a: number) => void;
    readonly browserdisplay_height: (a: number) => number;
    readonly browserdisplay_is_dirty: (a: number) => number;
    readonly browserdisplay_new: () => number;
    readonly browserdisplay_rgba_framebuffer: (a: number) => [number, number];
    readonly browserdisplay_width: (a: number) => number;
    readonly browserdisplay_write_pixel: (a: number, b: number, c: number, d: number) => void;
    readonly browserfirmwareinfo_encrypted: (a: number) => number;
    readonly browserfirmwareinfo_inspect: (a: number, b: number) => [number, number, number];
    readonly browserfirmwareinfo_load_config: (a: number) => number;
    readonly browserfirmwareinfo_magic: (a: number) => [number, number];
    readonly browserfirmwareinfo_table_count: (a: number) => number;
    readonly browserfirmwareinfo_user_address: (a: number) => number;
    readonly browserfirmwareinfo_user_length: (a: number) => number;
    readonly browserfirmwareinfo_version: (a: number) => number;
    readonly browsermemory_load_flash: (a: number, b: number, c: number) => [number, number];
    readonly browsermemory_new: () => number;
    readonly browsermemory_read_u32: (a: number, b: number) => number;
    readonly browsermemory_read_u8: (a: number, b: number) => number;
    readonly browsermemory_reset_vector_pc: (a: number) => number;
    readonly browsermemory_reset_vector_sp: (a: number) => number;
    readonly browsermemory_write_u8: (a: number, b: number, c: number) => number;
    readonly decrypt_firmware_with_key: (a: number, b: number, c: number) => [number, number, number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
