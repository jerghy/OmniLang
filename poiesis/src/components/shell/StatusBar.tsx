// ═══════════════════════════════════════════════
// Shared StatusBar — present in both design and code modes
// ═══════════════════════════════════════════════

import { S } from "./theme";

export interface StatusBarProps {
  status: { text: string; isError: boolean };
  isDirty: boolean;
  projectName?: string;
  projectDir: string | null;
  compileErrors?: Array<unknown>;
}

export function StatusBar({
  status,
  isDirty,
  projectName,
  projectDir,
  compileErrors,
}: StatusBarProps) {
  return (
    <div style={S.statusBar}>
      <span style={{ color: status.isError ? "#DC2626" : status.text.startsWith("⚠") ? "#D97706" : "#6B7280" }}>{status.text}</span>
      <span style={{ color: "#9ca3af" }}>
        {projectName}{isDirty ? " ●" : ""}
        {projectDir ? ` | 📁 ${projectDir}` : ""}
        {compileErrors && compileErrors.length > 0 ? ` — ${compileErrors.length} 条错误` : ""}
      </span>
    </div>
  );
}
