/** TypeScript mirrors of editor/server/annotation/types.py */

export type CodeTabType = "logic" | "event";
export type ViewRenderMode = "dom" | "canvas";

export interface ViewMarker {
  elementId: string;
  lineNumber: number;
  isFolded: boolean;
  renderMode: ViewRenderMode;
}

export interface NodeEntry {
  id: string;
  tag: string;
  attrs: Record<string, string>;
  posX: number;
  posY: number;
  lineNumber: number;
}

export interface WireEntry {
  id: string;
  fromId: string;
  fromPort: string;
  toId: string;
  toPort: string;
  isCollapsed: boolean;
  collapseGroupId: string;
  lineNumber: number;
}

export interface Chain {
  name: string;
  startLine: number;
  endLine: number;
  nodes: NodeEntry[];
  wires: WireEntry[];
}

export interface AnnotationFile {
  codeTabType: CodeTabType;
  codeTabLine: number;
  viewMarkers: ViewMarker[];
  chains: Chain[];
}

export interface AnnotationValidationError {
  code: string;
  message: string;
  line: number;
}

export interface AnnotatedLoadResponse {
  success: boolean;
  source?: string;
  annotations?: AnnotationFile;
  validationErrors?: AnnotationValidationError[];
  error?: string;
}

export interface AnnotatedSaveResponse {
  ok: boolean;
  path?: string;
  error?: string;
  validationErrors?: AnnotationValidationError[];
}

export interface EditorTomlState {
  active_file: string;
  active_mode: string;
  open_files: string[];
  left_open: boolean;
  right_width: number;
  viewports: Record<string, { zoom: number; scroll_x: number; scroll_y: number }>;
}
