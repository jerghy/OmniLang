/** 画布面板 — 显示 .omni 源文件列表，点击加载到画布编辑。
 *
 *  不再使用 .omni.json 项目文件格式，直接读写 .omni 源代码文件。
 *  .omni 源码是唯一真相源，打开时解析为视觉元素，保存通过主工具栏操作。 */

import { useState, useEffect, useCallback } from "react";
import { useEditorStore } from "../store/editor";
import { useComponentRegistry } from "../store/componentRegistry";
import { api } from "../api/client";
import { parseOmniSource } from "../services/codegen";

interface OmniFile {
  name: string;
  path: string;
}

export function CanvasPanel() {
  const [omniFiles, setOmniFiles] = useState<OmniFile[]>([]);
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState("");

  // Scan for .omni files (recursive, up to 3 levels deep)
  const scanFiles = useCallback(async () => {
    setLoading(true);
    setOmniFiles([]);
    try {
      const files: OmniFile[] = [];
      const scanned = new Set<string>();

      const scanDir = async (dirPath: string, depth: number): Promise<void> => {
        if (depth > 3 || scanned.has(dirPath)) return;
        scanned.add(dirPath);
        try {
          const listing = await api.listDir(dirPath);
          const entries = listing?.entries || [];
          for (const e of entries) {
            if (e.name.endsWith(".omni")) {
              const fullPath = dirPath === "." ? e.name : `${dirPath}/${e.name}`;
              files.push({ name: fullPath, path: fullPath });
            }
          }
          // Recurse into subdirectories
          if (depth < 3) {
            for (const d of entries.filter((x: { type: string }) => x.type === "directory")) {
              const subPath = dirPath === "." ? d.name : `${dirPath}/${d.name}`;
              await scanDir(subPath, depth + 1);
            }
          }
        } catch { /* skip inaccessible dirs */ }
      };

      await scanDir(".", 1);
      setOmniFiles(files);
      if (files.length === 0) setStatus("未找到 .omni 文件");
      else setStatus(`${files.length} 个 .omni 文件`);
    } catch {
      setStatus("扫描失败");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    scanFiles();
  }, [scanFiles]);

  // Open a .omni source file — main.omni loads to main canvas, others open as tabs
  const handleOpen = useCallback(async (file: OmniFile) => {
    setStatus(`加载 ${file.name}...`);
    try {
      const result = await api.readFile(file.path);
      if (!result.content) {
        setStatus(`❌ 文件为空: ${file.name}`);
        return;
      }

      // Parse .omni source into design elements
      const parsed = parseOmniSource(result.content);
      const elements = parsed.designElements || [];
      const wires = parsed.designWires || [];

      // main.omni → 加载到主画布（切回主画布 tab）
      if (file.name.endsWith("main.omni")) {
        useEditorStore.setState((prev) => {
          const newTabStates = { ...prev.tabStates };
          const cur = prev.activeDocId;
          if (cur && cur.startsWith("file:")) {
            newTabStates[cur] = { elements: prev.elements, wires: prev.wires };
          }
          return {
            activeDocId: null,
            elements,
            wires,
            rootOrder: elements.filter((e) => !e.parentId).map((e) => e.id),
            tabStates: newTabStates,
            docBackup: null,  // reset backup since main canvas content changed
            selectedElementIds: [],
          };
        });
        import("../store/undo").then((m) => m.useUndoStore.getState().clear()).catch(() => {});
        setStatus(`✅ 已加载到主画布: ${file.name}`);
        return;
      }

      // 其他 .omni 文件 → 新开标签页
      const tabKey = `file:${file.path}`;

      useEditorStore.setState((prev) => {
        const newTabStates = { ...prev.tabStates };
        const cur = prev.activeDocId;
        if (cur && cur.startsWith("file:")) {
          newTabStates[cur] = { elements: prev.elements, wires: prev.wires };
        }

        // Backup main canvas when switching from it
        const newDocBackup = cur === null && !prev.docBackup
          ? { elements: prev.elements, wires: prev.wires }
          : prev.docBackup;

        newTabStates[tabKey] = { elements, wires };

        return {
          activeDocId: tabKey,
          elements,
          wires,
          rootOrder: elements.filter((e) => !e.parentId).map((e) => e.id),
          tabStates: newTabStates,
          docBackup: newDocBackup,
          openTabs: prev.openTabs.includes(tabKey)
            ? prev.openTabs
            : [...prev.openTabs, tabKey],
          selectedElementIds: [],
        };
      });

      // Clear undo stack when switching context
      import("../store/undo").then((m) => m.useUndoStore.getState().clear()).catch(() => {});

      // 加载自定义组件定义（从 src/components/*.omni 解析，metadata 在 // @component / @prop / @event 注释中）
      api.loadComponentDefinitions().then((res) => {
        if (res.definitions && Object.keys(res.definitions).length > 0) {
          const existing = useComponentRegistry.getState().definitions;
          const merged = { ...existing };
          for (const [name, raw] of Object.entries(res.definitions)) {
            const data = raw as Record<string, unknown>;
            const source = data.source as string || "";
            if (!source) continue;

            const parsed = parseOmniSource(source);
            const meta = parsed.componentMeta || {};

            const elements = parsed.designElements || [];
            const wires = parsed.designWires || [];

            const propSchema: Record<string, unknown> = (meta.propSchema as Record<string, unknown>) || {};
            const dslProps = parsed.props || [];
            for (const p of dslProps) {
              if (!propSchema[p.name]) {
                propSchema[p.name] = { type: p.type, default: p.default, editor: "text" };
              }
            }

            const eventSchema: Record<string, unknown> = (meta.eventSchema as Record<string, unknown>) || {};
            const dslEvents = parsed.events || [];
            for (const e of dslEvents) {
              if (!eventSchema[e.name]) {
                eventSchema[e.name] = { id: e.name, description: "", payload: null };
              }
            }

            merged[name] = {
              id: (meta.id as string) || name,
              name,
              category: (meta.category as string) || "Custom",
              description: (meta.description as string) || "",
              props: propSchema,
              events: eventSchema,
              styleProperties: (meta.styleProperties as string[]) || [],
              children: (meta.children as boolean) ?? true,
              elements,
              wires,
              createdAt: new Date().toISOString(),
              updatedAt: new Date().toISOString(),
              storage: "file",
            };
          }
          useComponentRegistry.setState({ definitions: merged } as any);
        }
      }).catch(() => {});

      setStatus(`✅ 已打开: ${file.name}`);
    } catch (e) {
      setStatus(`❌ 加载失败: ${String(e)}`);
    }
  }, []);

  return (
    <div style={{ padding: 8, fontSize: 12, display: "flex", flexDirection: "column", height: "100%" }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", marginBottom: 6, gap: 4 }}>
        <span style={{ fontWeight: 600, fontSize: 11, color: "#6B7280", textTransform: "uppercase", flex: 1 }}>
          📄 源码文件
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
        ) : omniFiles.length === 0 ? (
          <div style={{ textAlign: "center", color: "#9CA3AF", fontSize: 11, padding: "12px 0" }}>
            {status || "未找到 .omni 文件"}
          </div>
        ) : (
          omniFiles.map((f) => (
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
              <span style={{ fontSize: 14 }}>📝</span>
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

    </div>
  );
}
