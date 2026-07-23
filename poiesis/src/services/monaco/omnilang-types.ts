// OmniLang runtime type definitions for Monaco IntelliSense.
// Provides static .d.ts for built-in types/stdlib + dynamic store type generation.

import type { StoreDefinition } from "../../core/types/store";

// ── Guards ──────────────────────────────────────────────────────────
let injected = false;

// ── Static OmniLang runtime type definitions ────────────────────────

const OMNILANG_RUNTIME_DTS = /* ts */ `
/**
 * OmniLang Runtime — built-in types and standard library.
 * Auto-generated for Monaco IntelliSense.
 */

// ── Primitive types ─────────────────────────────────────────────────
declare type i8 = number;
declare type i16 = number;
declare type i32 = number;
declare type i64 = number;
declare type isize = number;
declare type u8 = number;
declare type u16 = number;
declare type u32 = number;
declare type u64 = number;
declare type usize = number;
declare type f32 = number;
declare type f64 = number;
declare type bool = boolean;
declare type String = string;
declare type Void = void;

// ── Compound types ──────────────────────────────────────────────────
interface Array<T> {
  length: number;
  len(): i32;
  push(item: T): void;
  pop(): T | null;
  get(index: i32): T | null;
  is_empty(): bool;
  set(index: i32, value: T): void;
  concat(other: Array<T>): Array<T>;
  map<U>(fn: (item: T, index: i32) => U): Array<U>;
  filter(fn: (item: T, index: i32) => bool): Array<T>;
}

interface Map<K, V> {
  len(): i32;
  get(key: K): V | null;
  set(key: K, value: V): void;
  contains(key: K): bool;
  remove(key: K): void;
}

// ── Built-in functions ──────────────────────────────────────────────
declare function print(...args: any[]): void;
declare function println(...args: any[]): void;

// ── Instance methods on built-ins ───────────────────────────────────
interface String {
  len(): i32;
  to_string(): string;
  to_uppercase(): string;
  to_lowercase(): string;
  get(index: i32): string;
}

// ── Signal reference ────────────────────────────────────────────────
interface Signal<T> {
  value: T;
}

// ── Standard library: std/console ───────────────────────────────────
declare module "std/console" {
  export function console_log(...args: any[]): void;
  export function console_warn(...args: any[]): void;
  export function console_error(...args: any[]): void;
}

// ── Standard library: std/http ──────────────────────────────────────
declare module "std/http" {
  export function http_get(url: string): Promise<string>;
  export function http_post(url: string, body: string): Promise<string>;
}

// ── Standard library: std/json ──────────────────────────────────────
declare module "std/json" {
  export function json_parse(json: string): any;
  export function json_stringify(value: any): string;
}

// ── Standard library: std/array ─────────────────────────────────────
declare module "std/array" {
  export function array_length<T>(arr: Array<T>): i32;
  export function array_push<T>(arr: Array<T>, item: T): void;
  export function array_remove<T>(arr: Array<T>, index: i32): void;
  export function array_map<T, U>(arr: Array<T>, fn: (item: T) => U): Array<U>;
  export function array_filter<T>(arr: Array<T>, fn: (item: T) => bool): Array<T>;
  export function array_find<T>(arr: Array<T>, fn: (item: T) => bool): T | null;
  export function array_contains<T>(arr: Array<T>, item: T): bool;
  export function array_sort<T>(arr: Array<T>): Array<T>;
}

// ── Standard library: std/string ────────────────────────────────────
declare module "std/string" {
  export function string_length(s: string): i32;
  export function string_concat(a: string, b: string): string;
  export function string_split(s: string, delimiter: string): Array<string>;
  export function string_join(arr: Array<string>, delimiter: string): string;
  export function string_replace(s: string, from: string, to: string): string;
  export function string_substring(s: string, start: i32, end?: i32): string;
  export function string_to_upper(s: string): string;
  export function string_to_lower(s: string): string;
  export function string_starts_with(s: string, prefix: string): bool;
  export function string_ends_with(s: string, suffix: string): bool;
  export function string_contains(s: string, substr: string): bool;
  export function string_trim(s: string): string;
}

// ── Standard library: std/math ──────────────────────────────────────
declare module "std/math" {
  export function math_min(...values: number[]): number;
  export function math_max(...values: number[]): number;
  export function math_abs(x: number): number;
  export function math_ceil(x: number): i32;
  export function math_floor(x: number): i32;
  export function math_round(x: number): i32;
  export function math_sqrt(x: number): number;
  export function math_pow(base: number, exp: number): number;
  export function math_sin(x: number): number;
  export function math_cos(x: number): number;
  export function math_tan(x: number): number;
  export function math_log(x: number): number;
  export function math_log2(x: number): number;
  export function math_log10(x: number): number;
  export function math_clamp(value: number, min: number, max: number): number;
  export function math_random(): f64;
  export function math_random_int(min: i32, max: i32): i32;
  export function math_pi(): f64;
  export function math_e(): f64;
}

// ── Standard library: std/async ─────────────────────────────────────
declare module "std/async" {
  export function async_spawn(fn: () => void): void;
  export function async_sleep(ms: i32): Promise<void>;
  export function async_all<T>(promises: Array<Promise<T>>): Promise<Array<T>>;
  export function async_race<T>(promises: Array<Promise<T>>): Promise<T>;
  export function async_timeout<T>(promise: Promise<T>, ms: i32): Promise<T>;
}

// ── Standard library: std/datetime ──────────────────────────────────
declare module "std/datetime" {
  export function datetime_now(): string;
  export function datetime_from_timestamp(ts: i64): string;
  export function datetime_format(dt: string, fmt: string): string;
  export function datetime_year(dt: string): i32;
  export function datetime_month(dt: string): i32;
  export function datetime_day(dt: string): i32;
  export function datetime_hour(dt: string): i32;
  export function datetime_minute(dt: string): i32;
  export function datetime_second(dt: string): i32;
}

// ── Standard library: std/encode ────────────────────────────────────
declare module "std/encode" {
  export function encode_base64_encode(data: string): string;
  export function encode_base64_decode(encoded: string): string;
  export function encode_hex_encode(data: string): string;
  export function encode_hex_decode(encoded: string): string;
  export function encode_url_encode(data: string): string;
  export function encode_url_decode(encoded: string): string;
}

// ── Standard library: std/storage ───────────────────────────────────
declare module "std/storage" {
  export function storage_get(key: string): string | null;
  export function storage_set(key: string, value: string): void;
}

// ── Standard library: std/regex ─────────────────────────────────────
declare module "std/regex" {
  export function regex_match(pattern: string, text: string): bool;
  export function regex_search(pattern: string, text: string): Array<string>;
  export function regex_replace(pattern: string, replacement: string, text: string): string;
}
`;

// ── Dynamic store type generation ───────────────────────────────────

/**
 * Generate a TypeScript declaration module for each store,
 * so that code blocks referencing the store get IntelliSense.
 */
export function generateStoreTypes(stores: StoreDefinition[]): string {
  if (!stores || stores.length === 0) return "";

  const declarations: string[] = [];

  for (const store of stores) {
    const members: string[] = [];

    // Signals → typed properties
    for (const sig of store.signals) {
      const tsType = mapOmniTypeToTs(sig.type);
      members.push(`  /** Signal: ${sig.name} */
    ${sig.name}: Signal<${tsType}>;`);
    }

    // Getters → read-only typed properties
    for (const getter of store.getters) {
      const tsType = mapOmniTypeToTs(getter.returnType);
      members.push(`  /** Getter: ${getter.name} */
    readonly ${getter.name}: ${tsType};`);
    }

    // Actions → methods
    for (const action of store.actions) {
      const params = action.params
        .map((p) => `${p.name}: ${mapOmniTypeToTs(p.type)}`)
        .join(", ");
      const retType = action.returnType
        ? mapOmniTypeToTs(action.returnType)
        : "void";
      const asyncModifier = action.isAsync ? "Promise<" : "";
      const asyncClose = action.isAsync ? ">" : "";
      members.push(`  /** Action: ${action.name} */
    ${action.name}(${params}): ${asyncModifier}${retType}${asyncClose};`);
    }

    declarations.push(
      `declare module "store/${store.name}" {
  interface ${store.name}Store {
${members.join("\n")}
  }
  const store: ${store.name}Store;
  export default store;
}`,
    );
  }

  return declarations.join("\n\n");
}

// ── Type mapping helper ─────────────────────────────────────────────

function mapOmniTypeToTs(omniType: string): string {
  // Strip whitespace
  const t = omniType.trim();

  // Map common OmniLang types to TS equivalents
  const primitiveMap: Record<string, string> = {
    String: "string",
    bool: "boolean",
    i8: "number",
    i16: "number",
    i32: "number",
    i64: "number",
    isize: "number",
    u8: "number",
    u16: "number",
    u32: "number",
    u64: "number",
    usize: "number",
    f32: "number",
    f64: "number",
    Void: "void",
  };

  if (primitiveMap[t]) return primitiveMap[t];

  // Array<T> → Array<T>
  if (t.startsWith("Array<") && t.endsWith(">")) {
    const inner = t.slice(6, -1);
    return `Array<${mapOmniTypeToTs(inner)}>`;
  }

  // Map<K, V> → Map<K, V>
  if (t.startsWith("Map<") && t.endsWith(">")) {
    const inner = t.slice(4, -1);
    const parts = inner.split(",").map((p) => mapOmniTypeToTs(p.trim()));
    return `Map<${parts.join(", ")}>`;
  }

  // T? → T | null
  if (t.endsWith("?")) {
    return `${mapOmniTypeToTs(t.slice(0, -1))} | null`;
  }

  // T[] → Array<T>
  if (t.endsWith("[]")) {
    return `Array<${mapOmniTypeToTs(t.slice(0, -2))}>`;
  }

  // Default: use as-is (custom struct names, etc.)
  return t;
}

// ── Injection ───────────────────────────────────────────────────────

/**
 * Get TypeScript language service defaults from Monaco at runtime.
 * In Monaco 0.55+, languages.typescript is deprecated; we access it
 * through the module-level contribution system.
 */
async function getTsDefaults(): Promise<{
  addExtraLib: (content: string, filePath?: string) => void;
  setCompilerOptions: (opts: Record<string, unknown>) => void;
} | null> {
  // Access via the module-level typescript contribution
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const monacoGlobal = (globalThis as Record<string, unknown>).monaco as Record<string, unknown> | undefined;
  if (!monacoGlobal) return null;

  try {
    const { languages } = await import("monaco-editor");
    // In Monaco 0.55: monaco.languages.typescript still works at runtime
    // even though TS types mark it as deprecated
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const ts = (languages as Record<string, unknown>).typescript as Record<string, unknown> | undefined;
    if (ts && typeof ts === "object" && "typescriptDefaults" in ts) {
      return ts.typescriptDefaults as {
        addExtraLib: (content: string, filePath?: string) => void;
        setCompilerOptions: (opts: Record<string, unknown>) => void;
      };
    }
  } catch {
    // Ignore — TS language features not available
  }
  return null;
}

/**
 * Inject OmniLang runtime type definitions into Monaco's TypeScript
 * defaults so they're available for IntelliSense in the editor.
 *
 * Also registers dynamically-generated store types if provided.
 */
export async function injectOmniLangTypes(storeDefs?: StoreDefinition[]): Promise<void> {
  const tsDefaults = await getTsDefaults();
  if (!tsDefaults) return;

  if (!injected) {
    injected = true;
    tsDefaults.addExtraLib(OMNILANG_RUNTIME_DTS, "ts:omnilang-runtime.d.ts");
  }

  // Dynamic store types — overwrite previous lib on each call
  if (storeDefs && storeDefs.length > 0) {
    const storeDts = generateStoreTypes(storeDefs);
    if (storeDts) {
      tsDefaults.addExtraLib(storeDts, "ts:omnilang-stores.d.ts");
    }
  }

  // Set reasonable compiler options for the code-block context
  tsDefaults.setCompilerOptions({
    target: 7, // ScriptTarget.ES2020
    strict: true,
    moduleResolution: 2, // ModuleResolutionKind.NodeJs
    allowNonTsExtensions: true,
    noLib: false,
  } as Record<string, unknown>);
}

export { OMNILANG_RUNTIME_DTS };
