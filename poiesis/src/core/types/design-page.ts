// Design page types — tabbed editor with Z-axis dual-layer canvas

import type { PidMap } from "./pid";

export interface Breadcrumb {
  type: "architecture" | "component" | "code-layer";
  label: string;
  id: string;          // pid or bubble id
  filePath?: string;   // .omni file path
}

export type TabType = "component" | "store" | "code" | "design";

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
  codeFlowVisible: boolean;
  codeFlowElementId: string | null;
  breadcrumbs: Breadcrumb[];
}
