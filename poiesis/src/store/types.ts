// ── Poiesis Store Interface ──

import type { ElementData } from "../core/types/element";
import type { WireData } from "../core/types/wire";
import type { ArchitectureState, BubbleNode, BubbleEdge } from "../core/types/architecture";
import type { DesignPageState, Breadcrumb, TabType } from "../core/types/design-page";
import type { PidEntry } from "../core/types/pid";
import type { StoreDefinition, StoreSignalDef, StoreGetterDef, StoreActionDef } from "../core/types/store";
import type { ComponentDef, ProjectJson, NodeMeta, WireMeta } from "../core/types/project-json";
import type { StyleProperty } from "../core/types/style";
import type {
  DesignDefinition,
  DesignToken,
  DesignClass,
  VariantDefinition,
  ElementVariantBinding,
} from "../core/types/design";
import type { UndoStack } from "./undo-slice";

export interface PoiesisState {
  // ── Theme ──
  theme: "light" | "dark" | "system";
  resolvedTheme: "light" | "dark";
  setTheme: (theme: "light" | "dark" | "system") => void;
  // ── Design system (was extends DesignSlice, now inlined) ──
  activeDesign: DesignDefinition;
  elementVariants: Record<string, ElementVariantBinding>;
  loadDesign: (design: DesignDefinition) => void;
  addToken: (name: string, category: string, tokenType: DesignToken["tokenType"], value: string) => void;
  updateToken: (id: string, updates: Partial<DesignToken>) => void;
  removeToken: (id: string) => void;
  addDesignClass: (name: string) => void;
  updateDesignClass: (id: string, updates: Partial<DesignClass>) => void;
  removeDesignClass: (id: string) => void;
  addVariant: (name: string) => void;
  updateVariant: (id: string, updates: Partial<VariantDefinition>) => void;
  removeVariant: (id: string) => void;
  toggleInheritedProp: (variantId: string, propName: string) => void;
  bindVariantToElement: (elementId: string, variantName: string, dimensionValues: Record<string, string>) => void;
  unbindVariantFromElement: (elementId: string) => void;
  // ── Page routing ──
  currentPage: "architecture" | "design";

  // ── Architecture page ──
  architecture: ArchitectureState;

  // ── Design page ──
  designPage: DesignPageState;

  // ── Current tab's canvas data (active tab's working copy) ──
  elements: ElementData[];
  wires: WireData[];
  rootOrder: string[];
  selectedElementIds: string[];

  /** Per-tab canvas data for persistence across tab switches.
   *  Keyed by tab ID. Null entries will be loaded from .omni parse on first switch. */
  tabData: Record<string, {
    elements: ElementData[];
    wires: WireData[];
    rootOrder: string[];
    selectedElementIds: string[];
    filePath?: string;
    codeFlowVisible?: boolean;
    codeFlowElementId?: string | null;
    selectedActionPid?: string | null;
  }>;

  // ── Component definitions (parsed from .omni, cached) ──
  componentDefs: Record<string, ComponentDef>;

  // ── PID index ──
  pidIndex: Record<string, PidEntry>;

  // ── Project ──
  projectDir: string | null;
  projectName: string;
  isDirty: boolean;
  activeFilePath: string | null;
  activeFileContent: string | null;

  // ── Mode ──
  mode: "design" | "code";
  compileTarget: string;
  switchMode(mode: "design" | "code"): void;

  // ── Compilation ──
  compileErrors: Array<{ code: string; message: string; line: number }>;
  previewUrl: string | null;

   // ── Editor metadata (for project.json save/load) ──
  nodeMeta: Record<string, NodeMeta>;
  wireMeta: Record<string, WireMeta>;
  canvasStates: Record<string, { zoom: number; panX: number; panY: number }>;
  panels: { leftWidth: number; rightWidth: number };

  // ── Undo/redo (keyed by tabId) ──
  undoStacks: Record<string, UndoStack>;

  // ── Store definitions ──
  storeDefs: Record<string, StoreDefinition>;
  storeReferences: Record<string, string[]>;

  // ── Actions ──

  // Architecture
  setArchMode: (mode: "overall" | "local") => void;
  expandBubble: (bubbleId: string) => void;
  collapseBubble: (bubbleId: string) => void;
  focusComponent: (componentId: string | null) => void;
  selectBubbleByPid: (pid: string | null) => void;
  setArchCamera: (camera: { x: number; y: number; zoom: number }) => void;
  setBubblesAndEdges: (bubbles: BubbleNode[], edges: BubbleEdge[]) => void;

  // Design page
  openTab: (filePath: string, componentName: string, tabType?: TabType) => void;
  closeTab: (tabId: string) => void;
  switchTab: (tabId: string) => void;
  reorderTabs: (tabOrder: string[]) => void;
  markTabDirty: (tabId: string, dirty: boolean) => void;
  toggleCodeFlow: (elementId: string | null, focusPoint?: { x: number; y: number }) => void;
  setBreadcrumbs: (breadcrumbs: Breadcrumb[]) => void;
  setPreviewOutput: (output: import("../components/design/useWasmCompile").OmniViewOutput | null) => void;

  // Element/wire CRUD
  addElement: (el: ElementData) => void;
  removeElement: (id: string) => void;
  setProp: (id: string, prop: string, value: unknown) => void;
  getProp: (id: string, prop: string) => unknown;
  selectElement: (id: string | null, multi?: boolean) => void;
  clearSelection: () => void;
  addWire: (w: WireData) => void;
  removeWire: (id: string) => void;
  setElementSource: (id: string, source: string) => void;
  setElementPosition: (id: string, pos: { x: number; y: number }) => void;
  setElementsAndWires: (elements: ElementData[], wires: WireData[]) => void;
  moveElement: (elementId: string, newParentId: string | null, targetSiblingId: string | null, position: "before" | "after") => void;
  setElementClassRefs: (elementId: string, classRefs: string[]) => void;
  setElementInlineStyles: (elementId: string, properties: StyleProperty[]) => void;
  reorderElement: (elementId: string, targetSiblingId: string, position: "before" | "after") => void;

  // Undo/redo
  undo: () => void;
  redo: () => void;

  // Component defs
  setComponentDef: (name: string, def: ComponentDef) => void;
  removeComponentDef: (name: string) => void;
  getComponentDef: (name: string) => ComponentDef | undefined;

  // Custom component lifecycle (used by ComponentPalette)
  defineComponent: (name: string, def: Record<string, unknown>) => void;
  removeComponent: (name: string) => void;
  enterComponentCanvas: (name: string) => void;

  // Project
  setProjectDir: (dir: string) => void;
  setProjectName: (name: string) => void;
  resetEditorState: () => void;
  openFile: (path: string, content: string) => void;
  setActiveFileContent: (content: string) => void;
  markDirty: () => void;
  markClean: () => void;
  setCompileErrors: (errors: Array<{ code: string; message: string; line: number }>) => void;
  setPreviewUrl: (url: string | null) => void;

  // PID index
  setPidIndex: (index: Record<string, PidEntry>) => void;
  updatePidEntry: (pid: string, entry: PidEntry) => void;

  // Node/wire metadata
  setNodeMeta: (pid: string, meta: NodeMeta) => void;
  setWireMeta: (wireId: string, meta: WireMeta) => void;
  removeNodeMeta: (pid: string) => void;
  removeWireMeta: (wireId: string) => void;
  setCanvasState: (filePath: string, state: { zoom: number; panX: number; panY: number }) => void;
  setPanelSize: (side: "left" | "right", width: number) => void;

  // Navigation
  navigateTo: (page: "architecture" | "design") => void;

  // New project
  newProject: () => void;

  // Project.json load/save helpers
  toProjectJson: () => ProjectJson;
  loadFromProjectJson: (json: ProjectJson) => void;

  // Store lifecycle
  loadStoreDef: (def: StoreDefinition) => void;
  removeStoreDef: (name: string) => void;

  // Signal CRUD
  addStoreSignal: (storeName: string, signal: StoreSignalDef) => void;
  updateStoreSignal: (storeName: string, pid: string, updates: Partial<StoreSignalDef>) => void;
  removeStoreSignal: (storeName: string, pid: string) => void;

  // Getter CRUD
  addStoreGetter: (storeName: string, getter: StoreGetterDef) => void;
  updateStoreGetter: (storeName: string, pid: string, updates: Partial<StoreGetterDef>) => void;
  removeStoreGetter: (storeName: string, pid: string) => void;

  // Action CRUD
  addStoreAction: (storeName: string, action: StoreActionDef) => void;
  updateStoreAction: (storeName: string, pid: string, updates: Partial<StoreActionDef>) => void;
  removeStoreAction: (storeName: string, pid: string) => void;

  // PID + references
  registerStorePids: (storeDef: StoreDefinition) => void;
  registerStoreRef: (componentName: string, storeName: string) => void;
  unregisterStoreRef: (componentName: string, storeName: string) => void;
  unregisterAllStoreRefs: (componentName: string) => void;
}
