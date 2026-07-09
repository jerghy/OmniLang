// poiesis/src/services/codegen/types.ts
// Shared types for the OmniGen pipeline.

import type { ElementData, WireData } from "../../core/types";
import type { CodeTab } from "../../core/types";
import type { PidMap, PidBlockType } from "../../core/types/pid";

// ── Error types ──

export interface Diagnostic {
  severity: "error" | "warning" | "info";
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

// ── ComponentPart (discriminated union) ──

export interface SignalDef {
  type: string;
  defaultValue?: unknown;
}

export interface ComputedDef {
  type: string;
  body: string;
}

export interface ConstDef {
  name: string;
  value: unknown;
}

export interface PropDef {
  name: string;
  type: string;
  defaultValue?: unknown;
}

export interface EventDef {
  name: string;
}

export interface FnDef {
  name: string;
  params: Array<{ name: string; type: string }>;
  returnType: string;
  body: string;
}

export interface ValidComponentPart {
  status: "valid";
  name: string;
  signals: Record<string, SignalDef>;
  computeds: Record<string, ComputedDef>;
  storeRefs: string[];
  fns: FnDef[];
  style: Record<string, Record<string, string>>;
  view: ViewNode;
  consts: ConstDef[];
  props: PropDef[];
  events: EventDef[];
}

export interface PartialComponentPart {
  status: "partial";
  name: string;
  signals: Record<string, SignalDef>;
  storeRefs: string[];
  computeds?: Record<string, ComputedDef>;
  fns?: FnDef[];
  style?: Record<string, Record<string, string>>;
  view?: ViewNode | null;
  consts?: ConstDef[];
  props?: PropDef[];
  events?: EventDef[];
  errors: Diagnostic[];
}

export type ComponentPart = ValidComponentPart | PartialComponentPart;

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

// ── ViewNode (discriminated union) ──

export interface ResolvedStyle {
  htmlTag: string;
  cssClassName: string;
  contentText?: string;
  htmlAttrs: Record<string, string>;
  eventBindings: Record<string, string>;
  bindings: Record<string, string>;
  /** No longer populated — CSS resolution delegated to WASM resolve_styles() */
  cssRules: Record<string, string>;
  usedClassNames: string[];
}

interface ViewNodeBase {
  id: string;
  path: string; // "{compName}.{path}" for round-trip stability
}

export interface ElementViewNode extends ViewNodeBase {
  kind: "element";
  tag: string;
  props: Record<string, unknown>;
  children: ViewNode[];
  resolvedStyle?: ResolvedStyle;
  inline_style?: Record<string, string> | null;
  isFolded?: boolean;
  renderMode?: "dom" | "canvas";
  /** Design system class references, e.g. ["AppDesign.Card", "AppDesign.Row"] */
  classRefs?: string[];
  /** Design variant binding reference */
  variantRef?: {
    designName: string;
    variantName: string;
    params: Record<string, string>;
  } | null;
  /** Component-local style class names (defined in component style {} block) */
  componentClasses?: string[];
  /** Inline style for signal-driven dynamic values */
  inlineStyle?: Record<string, string>;
}

export interface IfViewNode extends ViewNodeBase {
  kind: "if";
  condition: string;
  branches: { condition: string; body: ViewNode[] }[];
  fallback?: ViewNode[] | null;
  children: ViewNode[];
}

export interface ForViewNode extends ViewNodeBase {
  kind: "for";
  iterable: string;
  itemBinding: string;
  keyExpr?: string;
  emptyBody?: ViewNode[] | null;
  children: ViewNode[];
}

export interface TextViewNode extends ViewNodeBase {
  kind: "text";
  text: string;
}

export interface InterpolationViewNode extends ViewNodeBase {
  kind: "interpolation";
  expression: string;
}

export interface SlotViewNode extends ViewNodeBase {
  kind: "slot";
  slotName: string;
  children: ViewNode[];
}

export type ViewNode =
  | ElementViewNode
  | IfViewNode
  | ForViewNode
  | TextViewNode
  | InterpolationViewNode
  | SlotViewNode;

export type ViewNodeKind = ViewNode["kind"];

// ── Intermediate types ──

export interface BindingResult {
  signalBindings: Map<string, Array<{ elementId: string; propName: string }>>;
  storeRefs: Map<string, string[]>;
  computedDeps: Map<string, string[]>;
  unwiredProps: Array<{ elementId: string; tag: string; propName: string }>;
}

export interface LoweredCode {
  expressions: Map<string, string>;
  functions: FnDef[];
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

  // ── Design block result (Phase H) ──
  designName?: string;
  designTokens?: Array<{ id: string; name: string; tokenType: string; value: string }>;
  designClasses?: Array<{
    id: string;
    name: string;
    properties: Array<{ name: string; value: string }>;
    pseudoStates: Array<{ pseudo: string; properties: Array<{ name: string; value: string }> }>;
  }>;
  designVariants?: Array<{
    id: string;
    name: string;
    dimensions: Array<{ name: string; values: Array<{ name: string; isDefault: boolean }> }>;
    baseProperties: Array<{ name: string; value: string }>;
    branches: Array<{ name: string; properties: Array<{ name: string; value: string }> }>;
    defaults: Record<string, string>;
  }>;
}
