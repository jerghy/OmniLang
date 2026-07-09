// Design page types — tabbed editor with Z-axis dual-layer canvas

import type { PidMap } from "./pid";

export interface Breadcrumb {
  type: "architecture" | "component" | "code-layer";
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

/**
 * Get tab type for a file path, preferring explicit metadata when available.
 * Falls back to the heuristic-based detectTabType.
 */
export function getTabType(filePath: string, fileTypeMap?: FileTypeMap): TabType {
  if (fileTypeMap && filePath in fileTypeMap) {
    return fileTypeMap[filePath];
  }
  return detectTabType(filePath);
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
  designView: "tokens" | "classes" | "variants";  // persisted sub-tab selection
}
