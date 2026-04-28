/* tslint:disable */
/* eslint-disable */

export function complementaryColor(r: number, g: number, b: number): Uint8Array;

export function generateSwatches(rgba_data: Uint8Array, count: number, color_space: string, init_method: string, seed: bigint): string;

export function generateSwatchesMedianCut(rgba_data: Uint8Array, count: number): string;

export function generateSwatchesOctree(rgba_data: Uint8Array, count: number, color_space: string, max_depth: number): string;

export function seedsPreprocess(rgba_data: Uint8Array, width: number, height: number, num_superpixels: number, num_levels: number, histogram_bins: number): Uint8Array;

export function slicPreprocess(rgba_data: Uint8Array, width: number, height: number, num_superpixels: number, compactness: number): Uint8Array;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly complementaryColor: (a: number, b: number, c: number, d: number) => void;
    readonly generateSwatches: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: bigint) => void;
    readonly generateSwatchesMedianCut: (a: number, b: number, c: number, d: number) => void;
    readonly generateSwatchesOctree: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly seedsPreprocess: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => void;
    readonly slicPreprocess: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export2: (a: number, b: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number, d: number) => number;
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
