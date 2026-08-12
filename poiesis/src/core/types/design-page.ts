// Design page types — tabbed editor with Z-axis dual-layer canvas

import type { PidMap } from "./pid";
import type { OmniViewOutput } from "../../components/design/useWasmCompile";

/** 语境面板主面板标签（[样子] 后置）。[内容] = 选中元素的接口（睡看印记、醒接/断，M3 起可接）。 */
export type PanelTab = "behavior" | "content" | "structure";

export interface Breadcrumb {
  type: "architecture" | "component";
  label: string;
  id: string;          // pid or bubble id
  filePath?: string;   // .omni file path
}

export type TabType = "component" | "store" | "code" | "design";

/** Explicit file-to-tab-type map for overriding the heuristic. */
export type FileTypeMap = Record<string, TabType>;

/** Detect editor tab type from file path. */
export function detectTabType(filePath: string): TabType {
  const normalized = filePath.replace(/\\/g, "/");
  if (normalized.endsWith("design.omni") || normalized.includes("/design.omni")) return "design";
  if (normalized.includes("/stores/") || normalized.startsWith("stores/")) return "store";
  if (normalized.includes("/lib/") || normalized.startsWith("lib/")) return "code";
  return "component";
}

export interface DesignTab {
  id: string;
  filePath: string;        // relative to project root, e.g. "src/components/TodoItem.omni"
  componentName: string;
  isDirty: boolean;
  pidMap: PidMap;
  tabType: TabType;        // determines which editor to show
}

export interface DesignPageState {
  openTabs: DesignTab[];
  activeTabId: string | null;
  tabOrder: string[];       // drag-reordered tab IDs
  previewOutput: OmniViewOutput | null;
  /** 整页醒（呼吸点 / [内容] 自动醒）。瞬态视图状态，不持久化。 */
  awake: boolean;
  /** 主面板活动标签。 */
  mainPanelTab: PanelTab;
  breadcrumbs: Breadcrumb[];
  designView: "tokens" | "classes" | "variants";  // persisted sub-tab selection
  /** Store drawer panel state */
  storePanelVisible: boolean;
  storePanelActiveStore: string | null;
  /** [数据节点] 开关：源节点显隐（醒态下有效）。瞬态视图状态，不持久化。 */
  dataNodesVisible: boolean;
}
