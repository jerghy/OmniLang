/** 文件浏览器 — 支持目录导航、返回上级、.omni 文件高亮。*/

import { useState, useCallback } from "react";
import { usePoiesisStore } from "../../store";
import { api } from "../../commands";

// NOTE: 'files' is not in PoiesisState yet — migration needed
type FileEntry = { name: string; path: string; type: "file" | "directory"; ext?: string };

function getFiles(): FileEntry[] {
  return (usePoiesisStore.getState() as unknown as Record<string, unknown>).files as FileEntry[] || [];
}

function setFiles(files: FileEntry[]): void {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (usePoiesisStore as any).setState({ files });
}

export function FileExplorer() {
  const files = getFiles();
  const activePath = usePoiesisStore((s) => s.activeFilePath);
  const openFile = usePoiesisStore((s) => s.openFile);
  const [currentDir, setCurrentDir] = useState<string[]>(["."]);

  const handleClick = useCallback(async (entry: FileEntry) => {
    if (entry.type === "directory") {
      const result = await api.project.listDir(entry.path);
      if (result.entries) {
        setCurrentDir([...currentDir, entry.path]);
        setFiles(result.entries);
      }
    } else {
      const result = await api.project.readFile(entry.path);
      if (result.content) {
        openFile(entry.path, result.content);
      }
    }
  }, [currentDir, openFile]);

  const handleBack = useCallback(async () => {
    if (currentDir.length <= 1) return;
    const newDir = currentDir.slice(0, -1);
    setCurrentDir(newDir);
    const parentPath = newDir[newDir.length - 1] || ".";
    const result = await api.project.listDir(parentPath);
    if (result.entries) {
      setFiles(result.entries);
    }
  }, [currentDir]);

  const dirs = files.filter((f) => f.type === "directory");
  const omniFiles = files.filter((f) => f.type === "file" && f.name.endsWith(".omni"));
  const otherFiles = files.filter((f) => f.type === "file" && !f.name.endsWith(".omni"));

  return (
    <div style={{ padding: 8, fontSize: 13, display: "flex", flexDirection: "column", height: "100%" }}>
      {/* Header + breadcrumb */}
      <div style={{ display: "flex", alignItems: "center", marginBottom: 6, gap: 4 }}>
        {currentDir.length > 1 && (
          <button onClick={handleBack}
            style={{ background: "none", border: "1px solid #e5e7eb", borderRadius: 3, cursor: "pointer", fontSize: 11, padding: "1px 4px", color: "#6B7280" }}
            title="返回上级目录">
            ← 返回
          </button>
        )}
        <span style={{ fontWeight: 600, fontSize: 11, color: "#6B7280", textTransform: "uppercase", flex: 1 }}>
          文件
        </span>
        <span style={{ fontSize: 10, color: "#d1d5db" }}>
          {currentDir.length > 1 ? currentDir[currentDir.length - 1] : "根目录"}
        </span>
      </div>

      {/* Directory navigation */}
      <div style={{ flex: 1, overflow: "auto" }}>
        {dirs.map((d) => (
          <div
            key={d.path}
            onClick={() => handleClick(d)}
            style={{ padding: "3px 6px", cursor: "pointer", borderRadius: 4, color: "#374151", fontSize: 12 }}
            onMouseEnter={(e) => { e.currentTarget.style.background = "#F3F4F6"; }}
            onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; }}
          >
            📁 {d.name}
          </div>
        ))}

        {omniFiles.length > 0 && (
          <div style={{ marginTop: 4, fontSize: 10, color: "#9CA3AF", textTransform: "uppercase", padding: "0 6px", marginBottom: 2 }}>
            .omni
          </div>
        )}
        {omniFiles.map((f) => (
          <div
            key={f.path}
            onClick={() => handleClick(f)}
            style={{
              padding: "3px 6px", cursor: "pointer", borderRadius: 4, fontSize: 12,
              background: f.path === activePath ? "#EEF2FF" : "transparent",
              color: f.path === activePath ? "#4F46E5" : "#374151",
              fontWeight: f.path === activePath ? 600 : 400,
            }}
            onMouseEnter={(e) => { if (f.path !== activePath) e.currentTarget.style.background = "#F3F4F6"; }}
            onMouseLeave={(e) => { if (f.path !== activePath) e.currentTarget.style.background = "transparent"; }}
          >
            📄 {f.name}
          </div>
        ))}

        {otherFiles.length > 0 && (
          <>
            <div style={{ marginTop: 6, fontSize: 10, color: "#9CA3AF", textTransform: "uppercase", padding: "0 6px", marginBottom: 2 }}>
              其他
            </div>
            {otherFiles.map((f) => (
              <div
                key={f.path}
                onClick={() => handleClick(f)}
                style={{
                  padding: "3px 6px", cursor: "pointer", borderRadius: 4, fontSize: 12,
                  background: f.path === activePath ? "#EEF2FF" : "transparent",
                }}
                onMouseEnter={(e) => { if (f.path !== activePath) e.currentTarget.style.background = "#F3F4F6"; }}
                onMouseLeave={(e) => { if (f.path !== activePath) e.currentTarget.style.background = "transparent"; }}
              >
                📄 {f.name}
              </div>
            ))}
          </>
        )}

      </div>
    </div>
  );
}
