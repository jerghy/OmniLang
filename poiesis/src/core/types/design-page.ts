// Design page types — tabbed editor with Z-axis dual-layer canvas

import type { PidMap } from "./pid";

export interface Breadcrumb {
  type: "architecture" | "component" | "code-layer";
  label: string;
  id: string;          // pid or bubble id
  filePath?: string;   // .omni file path
}

export interface DesignTab {
  id: string;
  filePath: string;        // relative to project root, e.g. "src/components/TodoItem.omni"
  componentName: string;
  isDirty: boolean;
  pidMap: PidMap;
}

export interface DesignPageState {
  openTabs: DesignTab[];
  activeTabId: string | null;
  tabOrder: string[];       // drag-reordered tab IDs
  codeFlowVisible: boolean;
  codeFlowElementId: string | null;
  breadcrumbs: Breadcrumb[];
}
