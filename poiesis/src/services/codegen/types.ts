// poiesis/src/services/codegen/types.ts
// Shared types for the OmniGen pipeline.

import type { ElementData, WireData } from "../../core/types";
import type { CodeTab } from "../../core/types";
import type { PidMap, PidBlockType } from "../../core/types/pid";

// ── Error types ──

export interface Diagnostic {
  severity: string;
  code: string;
  message: string;
  line: number;
  col: number;
  end_line: number;
  end_col: number;
}

export interface PipelineError {
  phase: "A" | "B" | "C";
  module: string;
  severity: "fatal" | "warning";
  elementId?: string;
  code: string;
  message: string;
}

export interface StageResult<T> {
  value: T | null;
  errors: PipelineError[];
}

export interface ValidationResult {
  valid: boolean;
  errors: Diagnostic[];
}

// ── EmitInput (SourceEmitter input) ──

export interface StoreDef {
  name: string;
  signals: Record<string, { type: string; defaultValue?: unknown }>;
  getters: Record<string, { returnType: string; body: string }>;
  actions: Array<{ name: string; params: Array<{ name: string; type: string }>; body: string }>;
}

export interface ComponentPart {
  name: string;
  signals: Record<string, { type: string; defaultValue?: unknown }>;
  computeds: Record<string, { type: string; body: string }>;
  storeRefs: string[];
  /** Compile-time constant declarations (e.g. `const MAX_COUNT = 100;`) */
  consts?: Array<{ name: string; value: unknown }>;
  fns: Array<{ name: string; params: Array<{ name: string; type: string }>; returnType: string; body: string }>;
  /** Component prop declarations (e.g. `prop title: String = ""`) */
  props?: Array<{ name: string; type: string; defaultValue?: unknown }>;
  /** Component event declarations (e.g. `event onAction`) */
  events?: Array<{ name: string }>;
  view: ViewNode | null;
  style: Record<string, Record<string, string>>;
}

export interface ThemeDef {
  name: string;
  categories: Record<string, Record<string, string>>;
}

export interface EmitInput {
  codeTabType: "logic" | "event";
  imports: string[];
  stores: StoreDef[];
  component: ComponentPart;
  themes?: ThemeDef[];
  customComponents?: Array<{ name: string; source: string }>;
  /** Editor metadata embedded as `// @component`, `// @prop`, `// @event`, `// @style` comments */
  componentMeta?: ComponentMeta;
  /** Existing PID map to preserve PIDs across regenerations (PID stability). */
  existingPidMap?: PidMap;
}

/**
 * Editor-specific component metadata stored as comments in .omni source.
 * Replaces the old *.meta.json sidecar files.
 */
export interface ComponentMeta {
  id?: string;
  category?: string;
  description?: string;
  children?: boolean;
  styleProperties?: string[];
  /** Per-prop editor schema (keyed by prop name) — extends prop DSL declaration */
  propSchema?: Record<string, {
    editor?: string;   // "text" | "select" | "checkbox" | "color"
    values?: string[];  // for enum type
  }>;
  /** Per-event editor metadata (keyed by event name) — extends event DSL declaration */
  eventSchema?: Record<string, {
    payload?: string;
    description?: string;
  }>;
}

// ── Annotation (v2 PID-based) ──

export interface PidAnnotation {
  pid: string;
  blockType: PidBlockType;
  blockName: string;
}

export interface AnnotationData {
  codeTabDirective: string;
  pidAnnotations: Map<string, PidAnnotation>;
}

export interface AnnotationStrategy {
  beforeViewNode(node: ViewNode, depth: number): PidAnnotation | null;
  beforeFile(): string;
}

// ── ViewNode ──

export type ViewNodeKind = "element" | "text" | "interpolation" | "if" | "for" | "slot";

export interface ResolvedStyle {
  htmlTag: string;
  cssClassName: string;
  contentText?: string;
  htmlAttrs: Record<string, string>;
  eventBindings: Record<string, string>;
  bindings: Record<string, string>;
  cssRules: Record<string, string>;
  usedClassNames: string[];
}

export interface ViewNode {
  id: string;
  path: string;         // "{compName}.{path}" for round-trip stability
  tag: string;
  props: Record<string, unknown>;
  children: ViewNode[];
  kind: ViewNodeKind;
  resolvedStyle?: ResolvedStyle;
  inline_style?: Record<string, string> | null;
  isFolded?: boolean;
  renderMode?: "dom" | "canvas";
  condition?: string;
  branches?: { condition: string; body: ViewNode[] }[];
  fallback?: ViewNode[] | null;
  iterable?: string;
  itemBinding?: string;
  keyExpr?: string;
  emptyBody?: ViewNode[] | null;
  text?: string;
  expression?: string;
  slotName?: string;
}

// ── Intermediate types ──

export interface BindingResult {
  signalBindings: Map<string, Array<{ elementId: string; propName: string }>>;
  storeRefs: Map<string, string[]>;
  computedDeps: Map<string, string[]>;
  unwiredProps: Array<{ elementId: string; tag: string; propName: string }>;
}

export interface LoweredCode {
  expressions: Map<string, string>;
  functions: Array<{ name: string; params: Array<{ name: string; type: string }>; returnType: string; body: string }>;
  actions: Array<{ storeName: string; name: string; params: Array<{ name: string; type: string }>; body: string }>;
  getters: Array<{ name: string; returnType: string; body: string; deps: string[] }>;
}

export interface ChainBlock {
  functionName: string;
  nodeLines: string[];
  wireLines: string[];
}

// ── Parse output ──

export interface ParseResult {
  designElements: ElementData[] | null;
  designWires: WireData[];
  codeTabs: CodeTab[];
  imports: string[];
  errors: Diagnostic[];
  /** Component metadata parsed from `// @component`, `// @prop`, `// @event`, `// @style` comments. */
  componentMeta?: ComponentMeta | null;
  /** Parsed prop declarations from component body (`prop <name>: <type>;`). */
  props?: Array<{ name: string; type: string; defaultValue?: unknown }>;
  /** Parsed event declarations from component body (`event <name>;`). */
  events?: Array<{ name: string }>;
  /** PID map parsed from `// @pid:xxx` comments in the source. */
  pidMap?: PidMap;
}
