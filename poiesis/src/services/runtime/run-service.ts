// services/runtime/run-service.ts
// Run / Vite DevServer lifecycle — extracted 1:1 from App.tsx handleRun + handleStop.

import { api } from "../../commands";
import { useEditorStore } from "../../store/editor";
import { useCodeGraphStore } from "../../services/codegraph/store";
import { extractComponentName } from "../../core/utils";
import type { ElementData } from "../../core/types";

/**
 * Compile the active file and start the Vite dev server.
 * Returns the run result (success + url) or null if compilation failed.
 */
export async function handleRun(
  target: string,
  setStatus: (s: { text: string; isError: boolean }) => void,
  setCompileErrors: (e: unknown[]) => void,
  setRunLoading: (v: boolean) => void,
  setRunStatus: (s: { running: boolean; url: string | null }) => void,
  setFnToNodeMap: (m: Record<string, string>) => void,
): Promise<void> {
  const content = useEditorStore.getState().activeFileContent;
  if (!content) {
    setStatus({ text: "请先打开文件", isError: true });
    return;
  }

  setRunLoading(true);
  setStatus({ text: "编译并启动运行...", isError: false });

  // 1. Compile (with debug mode enabled for runtime tracking)
  const result = await api.compile.compile(content, target, undefined, true);
  const errs = result.errors || [];
  const hasErrors = errs.some(
    (e: { severity: string }) => e.severity === "error",
  );
  if (hasErrors) {
    setCompileErrors(errs);
    setStatus({ text: "❌ 编译失败 — 无法启动", isError: true });
    setRunLoading(false);
    return;
  }

  // 2. Parse view tree & code blocks for reactive canvas
  setCompileErrors([]);
  try {
    const [vRes, cbRes] = await Promise.all([
      fetch("/api/parse-view", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ source: content }),
      }),
      fetch("/api/parse-codeblocks", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ source: content }),
      }),
    ]);
    await vRes.json();
    const cbData = await cbRes.json();

    const codeElements: ElementData[] = [];
    const seenIds = new Set<string>();
    if (cbData.success && cbData.blocks) {
      const portDefs: Record<
        string,
        Array<{ name: string; direction: "in" | "out"; type: string }>
      > = {
        store: [{ name: "items", direction: "out", type: "any" }],
        signal: [
          { name: "value", direction: "out", type: "i32" },
          { name: "set", direction: "in", type: "i32" },
        ],
        computed: [
          { name: "value", direction: "out", type: "i32" },
          { name: "deps", direction: "in", type: "any" },
        ],
      };
      for (const b of cbData.blocks) {
        if (
          b.blockType === "action" ||
          b.blockType === "onPress" ||
          b.blockType === "onChange"
        )
          continue;
        const parentStore = b.parentStore || "";
        const id = b.id;
        if (seenIds.has(id)) continue;
        seenIds.add(id);
        codeElements.push({
          id,
          nodeType: "codeblock",
          tag: b.blockType,
          props: {
            name: b.name,
            parentStore,
            typeAnnotation: b.typeAnnotation,
          },
          children: [],
          parentId: parentStore ? `cb-store-${parentStore}` : null,
          ports: portDefs[b.blockType] || [],
          isFolded: false,
        });
      }
    }

    // Build fnToNodeMap: semantic ID → canvas element ID for execution highlighting
    const newFnToNodeMap: Record<string, string> = {};
    const componentName = extractComponentName(content);
    if (cbData.success && cbData.blocks) {
      for (const block of cbData.blocks) {
        if (block.blockType === "fn") {
          const compName = block.parentStore || componentName;
          newFnToNodeMap[`${compName}.${block.name}`] = block.id;
        }
      }
    }
    setFnToNodeMap(newFnToNodeMap);

    useCodeGraphStore.getState().setElementsAndWires(codeElements, []);
    useEditorStore.getState().setSourceCode(content);
  } catch {
    /* non-critical */
  }

  // 3. Start Vite dev server with compiled files
  setStatus({ text: "启动开发服务器...", isError: false });
  const runResult = await api.compile.startRun(result.files || {}, true);

  if (runResult.success && runResult.url) {
    setRunStatus({ running: true, url: runResult.url });
    setStatus({
      text: `▶ 运行中 — ${runResult.url}`,
      isError: false,
    });
    window.open(runResult.url, "_blank");
  } else {
    setStatus({
      text: `❌ 启动失败: ${runResult.error || "未知错误"}`,
      isError: true,
    });
  }
  setRunLoading(false);
}

/**
 * Stop the running Vite dev server and clean up runtime state.
 */
export async function handleStop(
  setStatus: (s: { text: string; isError: boolean }) => void,
  setRunStatus: (s: { running: boolean; url: string | null }) => void,
  setDebugConnected: (v: boolean) => void,
  setDebugStateChanges: (v: Array<{ signalId: string; name: string; value: unknown; timestamp: number }>) => void,
  setActiveNodeIds: (v: Set<string>) => void,
  setActiveEdgeIds: (v: Set<string>) => void,
  setDebugPaused: (v: boolean) => void,
  setFnToNodeMap: (m: Record<string, string>) => void,
): Promise<void> {
  setStatus({ text: "停止开发服务器...", isError: false });
  await api.compile.stopRun();
  setRunStatus({ running: false, url: null });
  setStatus({ text: "⏹ 已停止", isError: false });
  // Clean up debug state
  setDebugConnected(false);
  setDebugStateChanges([]);
  setActiveNodeIds(new Set());
  setActiveEdgeIds(new Set());
  setDebugPaused(false);
  setFnToNodeMap({});
}
