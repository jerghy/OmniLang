/** 画布面板 — 显示 .omni.json 画布文件列表，点击加载为标签页。
 *  底部提供保存当前画布按钮，替代原工具栏中的保存/加载按钮。 */

import { useState, useEffect, useCallback } from "react";
import { useEditorStore } from "../store/editor";
import { useComponentRegistry } from "../store/componentRegistry";
import { api } from "../api/client";

interface CanvasFile {
  name: string;
  path: string;
}

export function CanvasPanel() {
  const [canvasFiles, setCanvasFiles] = useState<CanvasFile[]>([]);
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState("");

  const openProject = useEditorStore((s) => s.openProject);
  const activeFilePath = useEditorStore((s) => s.projectPath);

  // Scan for .omni.json files
  const scanFiles = useCallback(async () => {
    setLoading(true);
    setCanvasFiles([]);
    try {
      const listing = await api.listDir(".");
      const entries = listing?.entries || [];
      let jsonFiles = entries
        .filter((e: { name: string }) => e.name.endsWith(".omni.json"))
        .map((e: { name: string; path: string }) => ({ name: e.name, path: e.path || e.name }));

      // Also scan one level of subdirectories
      const subdirs = entries.filter((e: { type: string }) => e.type === "directory");
      for (const dir of subdirs) {
        try {
          const subListing = await api.listDir(dir.name);
          const subEntries = subListing?.entries || [];
          for (const se of subEntries) {
            if (se.name.endsWith(".omni.json")) {
              jsonFiles.push({ name: `${dir.name}/${se.name}`, path: `${dir.name}/${se.name}` });
            }
          }
        } catch { /* skip inaccessible subdirs */ }
      }
      setCanvasFiles(jsonFiles);
      if (jsonFiles.length === 0) setStatus("未找到画布文件");
      else setStatus(`${jsonFiles.length} 个画布文件`);
    } catch {
      setStatus("扫描失败");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    scanFiles();
  }, [scanFiles]);

  // Open a project file
  const handleOpen = useCallback(async (file: CanvasFile) => {
    setStatus(`加载 ${file.name}...`);
    try {
      const result = await api.readFile(file.path);
      if (!result.content) {
        setStatus(`❌ 文件不存在: ${file.name}`);
        return;
      }
      const data = JSON.parse(result.content);
      // v2: validate project file (design.elements + code.elements or legacy elements)
      if (!data.design && !data.elements) {
        setStatus("❌ 项目文件格式无效 — 缺少 design/code 数据");
        return;
      }
      const editorStore = useEditorStore.getState();
      editorStore.openProject(file.path, data);
      setStatus(`✅ 已打开项目: ${file.name}`);
    } catch (e) {
      setStatus(`❌ 加载失败: ${e instanceof SyntaxError ? "格式错误" : String(e)}`);
    }
  }, []);

  // Save current project
  const handleSave = useCallback(async () => {
    const store = useEditorStore.getState();
    const defaultName = store.projectPath
      || `${store.projectName}.omni.json`;

    const name = prompt("保存项目为:", defaultName);
    if (!name) return;

    setStatus(`保存到 ${name}...`);
    try {
      const now = new Date().toISOString();
      const doc: Record<string, unknown> = {
        schemaVersion: "2.0.0",
        meta: { projectName: name.replace(".omni.json", ""), omniVersion: "0.6.0", createdAt: now, updatedAt: now },
        sourceCode: store.sourceCode,
        design: { elements: store.elements, wires: store.wires },
        code: { elements: store.codeElements, wires: store.codeWires },
        components: Object.values(useComponentRegistry.getState().definitions).map((d: any) => ({
          name: d.name,
          source: d.source || "embedded",
          ...(d.source === "embedded" || !d.source ? { definition: d } : {}),
        })),
        canvasLayout: { positions: {} },
        tabState: { openTabs: store.openTabs, activeTab: store.activeDocId || "main" },
        config: store.config,
      };
      const result = await api.saveOmniJson(name, doc);
      if (result.ok) {
        store.markClean();
        useEditorStore.setState({ projectPath: name });
        setStatus(`✅ 已保存: ${name}`);
        scanFiles();
      } else {
        setStatus(`❌ 保存失败: ${result.error || "未知错误"}`);
      }
    } catch (e) {
      setStatus(`❌ 保存失败: ${String(e)}`);
    }
  }, [scanFiles]);

  return (
    <div style={{ padding: 8, fontSize: 12, display: "flex", flexDirection: "column", height: "100%" }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", marginBottom: 6, gap: 4 }}>
        <span style={{ fontWeight: 600, fontSize: 11, color: "#6B7280", textTransform: "uppercase", flex: 1 }}>
          📄 画布文件
        </span>
        <button
          onClick={scanFiles}
          disabled={loading}
          style={{ background: "none", border: "1px solid #e5e7eb", borderRadius: 3, cursor: "pointer", fontSize: 10, padding: "1px 4px", color: "#6B7280" }}
          title="刷新文件列表"
        >
          🔄
        </button>
      </div>

      {/* File list */}
      <div style={{ flex: 1, overflow: "auto", minHeight: 0 }}>
        {loading ? (
          <div style={{ textAlign: "center", color: "#9CA3AF", fontSize: 11, padding: "12px 0" }}>
            ⏳ 扫描中...
          </div>
        ) : canvasFiles.length === 0 ? (
          <div style={{ textAlign: "center", color: "#9CA3AF", fontSize: 11, padding: "12px 0" }}>
            {status || "未找到 .omni.json 文件"}
          </div>
        ) : (
          canvasFiles.map((f) => (
            <div
              key={f.path}
              onClick={() => handleOpen(f)}
              style={{
                padding: "5px 8px", cursor: "pointer", borderRadius: 4, fontSize: 12,
                color: "#374151", display: "flex", alignItems: "center", gap: 6,
                margin: "1px 0",
              }}
              onMouseEnter={(e) => { e.currentTarget.style.background = "#F3F4F6"; }}
              onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; }}
              title={f.path}
            >
              <span style={{ fontSize: 14 }}>🔧</span>
              <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {f.name}
              </span>
              <span style={{ fontSize: 10, color: "#9CA3AF" }}>点击加载</span>
            </div>
          ))
        )}
      </div>

      {/* Status */}
      {status && (
        <div style={{ fontSize: 10, color: status.startsWith("❌") ? "#DC2626" : status.startsWith("✅") ? "#059669" : "#6B7280", padding: "4px 4px 0", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
          {status}
        </div>
      )}

      {/* Save button */}
      <div style={{ borderTop: "1px solid #E5E7EB", paddingTop: 8, marginTop: 8 }}>
        <button
          onClick={handleSave}
          style={{
            width: "100%", padding: "6px 10px", fontSize: 12, fontWeight: 600,
            border: "1px solid #D1D5DB", borderRadius: 6, background: "#fff",
            cursor: "pointer", color: "#374151", display: "flex", alignItems: "center", justifyContent: "center", gap: 6,
          }}
          onMouseEnter={(e) => { e.currentTarget.style.background = "#F9FAFB"; }}
          onMouseLeave={(e) => { e.currentTarget.style.background = "#fff"; }}
        >
          💾 保存当前画布
        </button>
      </div>
    </div>
  );
}
