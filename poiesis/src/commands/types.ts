/** API 类型定义（与服务端 CompileError/CompileResult 对应）。*/

export interface CompileError {
  file: string;
  line: number;
  col: number;
  endLine?: number;
  endCol?: number;
  severity: "error" | "warning";
  code: string;
  message: string;
  suggestion: string | null;
}

export interface CompileResponse {
  success: boolean;
  files: Record<string, string>;
  diff: Record<string, string>;
  errors: CompileError[];
}

export interface FsEntry {
  name: string;
  path: string;
  type: "file" | "directory";
  ext: string;
}

export interface ProjectInfo {
  dir: string | null;
  target: string;
  mainFile: string | null;
}
