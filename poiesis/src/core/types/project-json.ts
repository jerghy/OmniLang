// Project JSON persistence types — git merge-friendly design

/**
 * ProjectJson — single file at project root storing editor metadata
 * that CANNOT be parsed from .omni source files.
 *
 * Merge-friendly design:
 * - nodes/wires use per-ID flat Record (not arrays) → git auto-merges
 * - All keys sorted alphabetically on write
 * - Each entry on one line where practical
 */

export interface ProjectJson {
  version: 3;

  /** Node metadata keyed by PID. Alice adds c5, Bob adds c6 → auto-merge. */
  nodes: Record<string, NodeMeta>;

  /** Wire metadata keyed by wire ID. Same auto-merge property. */
  wires: Record<string, WireMeta>;

  architecture: ArchitectureMeta;

  /** Canvas viewport per .omni file, keyed by relative path */
  canvasStates: Record<string, CanvasViewport>;

  panels: PanelLayout;

  /** Code flow graphs keyed by fn/action PID. */
  codeFlowGraphs: Record<string, CodeFlowGraph>;
}

export interface NodeMeta {
  pos: { x: number; y: number };
  folded?: boolean;
  /** migrated from old @edit render=canvas annotation */
  renderMode?: "dom" | "canvas";
  /** Node type for code flow graph nodes: "if" | "elif" | "else" | "return" | "code-block" | "for" | "while" | "fn" | "action" */
  nodeType?: string;
  /** Owning fn/action name, used for PID-based matching during parse */
  blockName?: string;
}

export interface WireMeta {
  from: string;        // source PID
  fromPort: string;
  to: string;          // target PID
  toPort: string;
  /** Skeleton node branch routing path */
  path?: Array<{ x: number; y: number }>;
}

export interface ArchitectureMeta {
  mode: "overall" | "local";
  expandedBubbles: string[];
  focusComponent: string | null;
  camera: { x: number; y: number; zoom: number };
}

export interface CanvasViewport {
  zoom: number;
  panX: number;
  panY: number;
}

export interface PanelLayout {
  leftWidth: number;
  rightWidth: number;
}

// ── New: Code Flow Graph persistence ──────────────────────────

/** Persisted code flow graph for one fn/action, keyed by fn/action PID. */
export interface CodeFlowGraph {
  pid: string;
  nodes: Record<string, CodeFlowNode>;
  wires: Record<string, CodeFlowWire>;
}

export interface CodeFlowNode {
  nodeType: string;    // "if" | "elif" | "else" | "return" | "code-block" | "for" | "while"
}

export interface CodeFlowWire {
  from: string;        // source node PID
  fromPort: string;    // "flow-out" | "br0" | "br1" | "br2"
  to: string;          // target node PID
  toPort: string;      // "flow-in"
}

// ── Component def types (unchanged) ──────────────────────────

/** Component source-level definition — parsed from .omni DSL declarations. */
export interface ComponentSourceDef {
  name: string;
  filePath: string;
  signals: Array<DefEntry>;
  props: Array<DefEntry>;
  events: Array<{ pid: string; name: string; payloadType: string }>;
  fns: Array<FnDef>;
}

/** Runtime-only canvas state (not persisted to project.json). */
export interface ComponentCanvasState {
  elements: import("../types/element").ElementData[];
  wires: import("../types/wire").WireData[];
  rootOrder: string[];
}

/** Component definition — parsed from .omni source, cached here */
export type ComponentDef = ComponentSourceDef & Partial<ComponentCanvasState>;

export interface DefEntry {
  pid: string;
  name: string;
  type: string;
  defaultValue?: unknown;
}

export interface FnDef {
  pid: string;
  name: string;
  params: Array<{ name: string; type: string }>;
  returnType: string;
}
