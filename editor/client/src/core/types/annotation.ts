// core/types/annotation.ts
// Annotation system types — mirrors of editor/server/annotation/types.py.

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

export type CodeTabType_Annotation = "logic" | "event";

export interface AnnotationFile {
  filepath: string;
  codeTabType: CodeTabType_Annotation;
  codeTabLine: number;
  viewMarkers: ViewMarker[];
  chains: Chain[];
  rawSource: string;
}

export interface AnnotatedLoadResponse {
  success: boolean;
  source?: string;
  annotations?: Record<string, unknown>;
  validationErrors?: Array<{ code: string; message: string; line: number }>;
  error?: string;
}

export interface AnnotatedSaveResponse {
  ok: boolean;
  path?: string;
  error?: string;
  validationErrors?: Array<{ code: string; message: string; line: number }>;
}

export interface EditorTomlState {
  active_file?: string;
  active_mode?: string;
  open_files?: string[];
  left_open?: boolean;
  right_width?: number;
  viewports?: Record<string, unknown>;
}
