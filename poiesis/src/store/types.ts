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

/** 调试值条目：text=输入框回显原文，valid=类型校验结果（false 不参与编译） */
export interface DebugValueEntry {
  text: string;
  valid: boolean;
}

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

  // ── 调试值覆盖（Task 1+ 新模型；旧模型已于 Task 7 删除）──
  /** 调试值覆盖（ownerKey = 组件名 | store 名 | 内联 "Comp.Store"）——调试专用，不 dirty 项目 */
  debugValues: Record<string, Record<string, DebugValueEntry>>;
  /** 调试总开关：off 时调试值不参与编译但输入保留（对比原始渲染） */
  debugOverridesEnabled: boolean;

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

  // ── Inline store definitions (component-scoped, keyed by "ComponentName.StoreName") ──
  inlineStoreDefs: Record<string, StoreDefinition>;
  loadInlineStoreDef: (def: StoreDefinition) => void;
  removeInlineStoreDef: (key: string) => void;

  // ── Code flow graphs（F1 物化：fn 名 → 解析侧图 {elements, wires}）──
  // 运行时形状为解析侧产物；持久化形状（CodeFlowGraph）在 toProjectJson 时经
  // buildCodeFlowGraphs 转换，保存格式不变。
  codeFlowGraphs: Record<string, { elements: import("../core/types/element").CodeblockElementData[]; wires: import("../core/types/wire").WireData[] }>;

  /** 生命周期图（pid 键）——materializeFnGraphs 生命周期合并循环的读取源。
   *  形状与 codeFlowGraphs 同构（元素/线均含生命周期入口元素）。 */
  lifecycleGraphs: Record<string, { elements: import("../core/types/element").CodeblockElementData[]; wires: import("../core/types/wire").WireData[] }>;

  /** Codeflow 节点位置持久化缓存：元素 id（PID）→ {x,y}，从 project.json 读回。
   *  materializeFnGraphs 物化缺 position 的新元素时优先取此值，否则行布局兜底。 */
  persistedPositions: Record<string, { x: number; y: number }>;

  /** Codeflow 分组缓存：fn 名 → 分组数组（从 project.json 读回，画布编辑时经 setCodeFlowGroups 写入）。 */
  codeFlowGroups: Record<string, import("../core/types/project-json").CodeFlowGroup[]>;
  setCodeFlowGroups: (fnName: string, groups: import("../core/types/project-json").CodeFlowGroup[]) => void;

  // ── Code tab（lib 文件）──
  libDefs: Record<string, import("../services/codegen/types").LibFileDef>;
  setLibDef: (filePath: string, def: import("../services/codegen/types").LibFileDef) => void;
  materializeLibGraphs: (filePath: string) => void;

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
  setStorePanelVisible: (visible: boolean) => void;
  setStorePanelActiveStore: (name: string | null) => void;

  // Element/wire CRUD
  addElement: (el: ElementData) => void;
  removeElement: (id: string) => void;
  setProp: (id: string, prop: string, value: unknown) => void;
  getProp: (id: string, prop: string) => unknown;
  /** Set or remove a two-way data binding on a view element (e.g. bind:value={signalName}).
   *  Pass signalName = null to remove the binding. */
  setElementBinding: (id: string, propName: string, signalName: string | null) => void;
  selectElement: (id: string | null, multi?: boolean) => void;
  clearSelection: () => void;
  addWire: (w: WireData) => void;
  removeWire: (id: string) => void;
  setElementSource: (id: string, source: string) => void;
  setElementPosition: (id: string, pos: { x: number; y: number }) => void;
  /** 进入组件 codeflow 时：把该组件的 fn 图合并进 elements/wires（pid 去重，已存在不覆盖，无 undo）。 */
  materializeFnGraphs: (componentName: string) => void;
  setElementsAndWires: (elements: ElementData[], wires: WireData[]) => void;
  moveElement: (elementId: string, newParentId: string | null, targetSiblingId: string | null, position: "before" | "after") => void;
  setElementClassRefs: (elementId: string, classRefs: string[]) => void;
  setElementInlineStyles: (elementId: string, properties: StyleProperty[]) => void;
  reorderElement: (elementId: string, targetSiblingId: string, position: "before" | "after") => void;

  // Conditional block CRUD
  addConditionalBlock: (parentId: string) => void;
  addConditionalBranch: (blockId: string, type: "elif" | "else") => void;
  updateBranchCondition: (blockId: string, branchIndex: number, condition: string) => void;
  addChildToBranch: (blockId: string, branchIndex: number, childId: string) => void;
  removeChildFromBranch: (blockId: string, branchIndex: number, childId: string) => void;
  removeConditionalBranch: (blockId: string, branchIndex: number) => void;

  // For block CRUD (view @for, 2026-08-06)
  addForBlock: (parentId: string) => void;
  updateForBlock: (blockId: string, updates: { itemName?: string; collection?: string; keyExpr?: string }) => void;
  addChildToEmpty: (blockId: string, childId: string) => void;
  removeChildFromEmpty: (blockId: string, childId: string) => void;

  // Undo/redo
  undo: () => void;
  redo: () => void;

  // Component defs
  setComponentDef: (name: string, def: ComponentDef) => void;
  removeComponentDef: (name: string) => void;
  getComponentDef: (name: string) => ComponentDef | undefined;

  // ── Component signal CRUD + debug ──
  addComponentSignal: (compName: string, signal: { pid: string; name: string; type: string; defaultValue?: unknown }) => void;
  updateComponentSignal: (compName: string, pid: string, updates: Partial<{ name: string; type: string; defaultValue: unknown }>) => void;
  removeComponentSignal: (compName: string, pid: string) => void;
  setDebugValue: (ownerKey: string, name: string, text: string, valid: boolean) => void;
  clearDebugValues: () => void;
  clearDebugValue: (ownerKey: string, name: string) => void;
  setDebugOverridesEnabled: (enabled: boolean) => void;
  /** 用户添加的数据值候选（字面量数组）——调试辅助状态，不 dirty 项目 */
  debugValueCandidates: Record<string, Record<string, string[]>>;
  addDebugValueCandidate: (ownerKey: string, name: string, literal: string) => void;
  removeDebugValueCandidate: (ownerKey: string, name: string, literal: string) => void;

  // ── Component prop CRUD ──
  addComponentProp: (compName: string, prop: { pid: string; name: string; type: string; defaultValue?: unknown }) => void;
  updateComponentProp: (compName: string, pid: string, updates: Partial<{ name: string; type: string; defaultValue: unknown }>) => void;
  removeComponentProp: (compName: string, pid: string) => void;

  // ── Component computed CRUD ──
  addComponentComputed: (compName: string, computed: { pid: string; name: string; returnType: string; expression: string }) => void;
  updateComponentComputed: (compName: string, pid: string, updates: Partial<{ name: string; returnType: string; expression: string }>) => void;
  removeComponentComputed: (compName: string, pid: string) => void;

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

  // ── Inline store Signal CRUD ──
  addInlineStoreSignal: (storeKey: string, signal: StoreSignalDef) => void;
  updateInlineStoreSignal: (storeKey: string, pid: string, updates: Partial<StoreSignalDef>) => void;
  removeInlineStoreSignal: (storeKey: string, pid: string) => void;

  // ── Inline store Getter CRUD ──
  addInlineStoreGetter: (storeKey: string, getter: StoreGetterDef) => void;
  updateInlineStoreGetter: (storeKey: string, pid: string, updates: Partial<StoreGetterDef>) => void;
  removeInlineStoreGetter: (storeKey: string, pid: string) => void;

  // ── Inline store Action CRUD ──
  addInlineStoreAction: (storeKey: string, action: StoreActionDef) => void;
  updateInlineStoreAction: (storeKey: string, pid: string, updates: Partial<StoreActionDef>) => void;
  removeInlineStoreAction: (storeKey: string, pid: string) => void;

  // PID + references
  registerStorePids: (storeDef: StoreDefinition) => void;
  registerStoreRef: (componentName: string, storeName: string) => void;
  unregisterStoreRef: (componentName: string, storeName: string) => void;
  unregisterAllStoreRefs: (componentName: string) => void;
}
