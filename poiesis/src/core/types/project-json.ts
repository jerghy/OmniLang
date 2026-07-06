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
  version: 2;

  /** Node metadata keyed by PID. Alice adds c5, Bob adds c6 → auto-merge. */
  nodes: Record<string, NodeMeta>;

  /** Wire metadata keyed by wire ID. Same auto-merge property. */
  wires: Record<string, WireMeta>;

  architecture: ArchitectureMeta;

  /** Canvas viewport per .omni file, keyed by relative path */
  canvasStates: Record<string, CanvasViewport>;

  panels: PanelLayout;
}

export interface NodeMeta {
  pos: { x: number; y: number };
  folded?: boolean;
  /** migrated from old @edit render=canvas annotation */
  renderMode?: "dom" | "canvas";
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

/** Component definition — parsed from .omni source, cached here */
export interface ComponentDef {
  name: string;
  filePath: string;
  signals: Array<DefEntry>;
  props: Array<DefEntry>;
  events: Array<{ pid: string; name: string; payloadType: string }>;
  fns: Array<FnDef>;
  /** Runtime-only canvas data (set by useSaveProject, not persisted to project.json). */
  elements?: import("../types/element").ElementData[];
  wires?: import("../types/wire").WireData[];
  rootOrder?: string[];
}

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
