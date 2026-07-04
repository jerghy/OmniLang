// Architecture page types — bubble graph

export type ArchMode = "overall" | "local";

export type BubbleLayer =
  | "interface"      // blue — props + events
  | "composition"    // green — parent/child components
  | "store-deps"     // red — store data deps
  | "code-deps"      // purple — lib/pub fn calls
  | "internal";      // grey — signal/computed/fn/style

export interface BubbleNode {
  id: string;
  pid: string;
  type: "ui" | "store" | "code";
  label: string;
  depth: number;              // 0 = top-level three bubbles, 1 = page/store file, 2+ = nested
  children: string[];         // child bubble IDs
  parentId: string | null;
  expanded: boolean;
  position: { x: number; y: number };
  /** Only set in local architecture mode */
  layer?: BubbleLayer;
  /** Component has basic elements (div, button, etc.) — double-click opens design page */
  isLeaf: boolean;
}

export interface BubbleEdge {
  id: string;
  source: string;    // bubble ID
  target: string;    // bubble ID
  label?: string;    // e.g. "items", "add()", "inc()"
  edgeType: "composition" | "store-read" | "store-write" | "code-call" | "event";
}

export interface ArchitectureState {
  mode: ArchMode;
  bubbles: BubbleNode[];
  edges: BubbleEdge[];
  focusComponentId: string | null;
  expandedBubbleIds: string[];
  selectedPid: string | null;
  camera: { x: number; y: number; zoom: number };
}
