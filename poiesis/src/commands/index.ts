import { invoke } from "@tauri-apps/api/core";

export interface CompileResult {
  success: boolean;
  output: string;
  errors: string[];
}

export async function compileProject(projectPath: string, target: string): Promise<CompileResult> {
  return invoke<CompileResult>("compile_project", { projectPath, target });
}

export async function projectExport(projectPath: string, outputDir: string): Promise<string> {
  return invoke<string>("project_export", { projectPath, outputDir });
}

export async function fileWatch(path: string): Promise<string> {
  return invoke<string>("file_watch", { path });
}
