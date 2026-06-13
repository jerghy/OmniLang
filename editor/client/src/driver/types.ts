/** OmniDriver type definitions.
 *  Playwright-like API for programmatic editor control.
 *  Exposed as `window.__OMNI__` for AI-driven automation. */

import type { ElementData, WireData } from "../store/editor";
import type { EventMeta } from "../panels/ComponentPalette";

// ── Re-export store types ────────────────────────────

export type { ElementData, WireData, EventMeta };

// ── Port descriptor ──────────────────────────────────

export interface PortDescriptor {
  name: string;
  direction: "in" | "out";
  type: string;
  label?: string;
}

// ── Element query ────────────────────────────────────

export interface ElementQuery {
  tag?: string;
  nodeType?: "view" | "codeblock" | "group";
  props?: Record<string, unknown>;
  text?: string;
}

// ── Code block options ───────────────────────────────

export interface CodeBlockOptions {
  parentStore?: string;
  source?: string;
  ports?: PortDescriptor[];
  typeAnnotation?: string;
  isFolded?: boolean;
}

// ── Compile result ───────────────────────────────────

export interface CompileResult {
  success: boolean;
  errors: Array<{ code: string; message: string; line: number; severity: string }>;
  output?: string;
}

// ── Editor snapshot (full state dump) ────────────────

export interface EditorSnapshot {
  mode: "design" | "code";
  elements: ElementData[];
  wires: WireData[];
  selectedElementId: string | null;
  projectDir: string | null;
  activeFilePath: string | null;
  compileErrors: Array<{ code: string; message: string; line: number }>;
  previewUrl: string | null;
}

// ── Component protocol entry (for AI discovery) ─────

export interface ComponentInfo {
  tag: string;
  category: string;
  description: string;
  props: Record<string, { type: string; default?: unknown; values?: string[]; editor: string }>;
  events: Record<string, EventMeta>;
  styleProperties: string[];
  hasChildren: boolean;
}

// ── Event handler creation ───────────────────────────

export interface EventHandlerOptions {
  /** Human-readable handler name (auto-generated if omitted) */
  handlerName?: string;
  /** Handler source code (auto-generated if omitted) */
  source?: string;
  /** Mark handler as folded by default */
  folded?: boolean;
}
