import { useCallback } from "react";
import { usePoiesisStore } from "../../store";
import { useCodeGraphStore } from "../../services/codegraph/store";
import { extractComponentName } from "../../core/utils";
import { compileProject } from "../../services/compiler/compiler";
import { compilerCache } from "../../services/compiler/cache";
import { api } from "../../commands";
import { parseCodeBlocks } from "../../services/parser/omni-regex-parser";
import type { CommandContext } from "../../core/commands/types";
import type { ElementData } from "../../core/types";

export function useCompile(
  _ctx: CommandContext,
  target: string,
  runStatus: { running: boolean },
  setFnToNodeMap: (map: Record<string, string>) => void,
  setStatus: (s: { text: string; isError: boolean }) => void,
) {
  const setCompileErrors = usePoiesisStore((s) => s.setCompileErrors);

  const handleCompile = useCallback(async () => {
    const store = usePoiesisStore.getState();
    const cg = useCodeGraphStore.getState();
    const codeTabs = cg.toSerializable().tabs;
    const componentName = extractComponentName(store.activeFileContent || "");

    const compileResult = await compileProject({
      elements: store.elements,
      wires: store.wires,
      codeTabs,
      componentName,
      rootOrder: store.rootOrder,
      activeFileContent: store.activeFileContent || "",
      target,
      isRunning: runStatus.running,
    });

    if (!compileResult) return;

    const { content, compileResponse } = compileResult;

    if (compileResult.generatedSource && compileResult.validationPassed) {
      store.setActiveFileContent(content);
    }

    if (!compileResult.validationPassed && compileResult.validationErrors.length > 0) {
      setCompileErrors(compileResult.validationErrors);
      setStatus({ text: "⚠️ 生成的代码验证失败，使用原始内容编译", isError: false });
    }

    setStatus({ text: "编译中...", isError: false });

    const errs = compileResponse.errors || [];
    setCompileErrors(errs);

    useCodeGraphStore.getState().switchTab("main");
    useCodeGraphStore.getState().setElementsAndWires([], []);
    try {
      // Parse code blocks via local TS parser (replaces Python /api/parse-*)
      const cbData = { success: true, blocks: parseCodeBlocks(content) };

      const codeElements: ElementData[] = [];
      const seenIds = new Set<string>();

      if (cbData.success && cbData.blocks) {
        const portDefs: Record<string, Array<{ name: string; direction: "in" | "out"; type: string }>> = {
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
          const parentStore = b.parentStore || "";
          const id = b.id;
          if (b.blockType === "action" || b.blockType === "onPress" || b.blockType === "onChange") continue;
          if (seenIds.has(id)) continue;
          seenIds.add(id);
          const el: ElementData = {
            id,
            nodeType: "codeblock",
            tag: b.blockType,
            props: { name: b.name, parentStore, typeAnnotation: b.typeAnnotation },
            children: [],
            parentId: parentStore ? `cb-store-${parentStore}` : null,
            ports: portDefs[b.blockType] || [],
            isFolded: false,
          };
          codeElements.push(el);
        }
        const existingStoreIds = new Set(codeElements.map((e) => e.id));
        for (const el of codeElements) {
          const pid = el.parentId;
          if (pid && pid.startsWith("cb-store-") && !existingStoreIds.has(pid)) {
            existingStoreIds.add(pid);
            const storeName = pid.replace("cb-store-", "");
            codeElements.push({
              id: pid,
              nodeType: "codeblock",
              tag: "store",
              props: { name: storeName },
              children: [el.id],
              parentId: null,
              ports: portDefs.store,
              isFolded: false,
            });
          }
        }
      }

      const newFnToNodeMap: Record<string, string> = {};
      const componentName2 = extractComponentName(content);
      if (cbData.success && cbData.blocks) {
        for (const block of cbData.blocks) {
          if (block.blockType === "fn") {
            const compName = block.parentStore || componentName2;
            newFnToNodeMap[`${compName}.${block.name}`] = block.id;
          }
        }
      }
      setFnToNodeMap(newFnToNodeMap);

      useCodeGraphStore.getState().setElementsAndWires(codeElements, []);
      usePoiesisStore.getState().setActiveFileContent(content);
    } catch {
      /* canvas not critical */
    }

    const hasErrors = errs.some((e: { severity: string }) => e.severity === "error");
    const warns = errs.filter((e: { severity: string }) => e.severity === "warning").length;
    if (hasErrors) setStatus({ text: "❌ 编译失败", isError: true });
    else {
      setStatus({
        text: `✅ 已编译 — ${new Date().toLocaleTimeString()}${warns ? ` (${warns} 条警告)` : ""}`,
        isError: false,
      });
      if (runStatus.running && compileResponse.files) {
        api.compile.writeRunFiles(compileResponse.files).catch(() => {});
      }
      const s = usePoiesisStore.getState();
      if (s.elements.length > 0) {
        const cg = useCodeGraphStore.getState();
        const ct = cg.toSerializable().tabs;
        const out = compilerCache.compileProject(
          {
            designElements: s.elements,
            designWires: s.wires,
            codeTabs: ct,
            componentName,
            rootOrder: s.rootOrder,
          },
          true,
        );
        if (out)
          window.dispatchEvent(
            new CustomEvent("omni-srcdoc-update", { detail: { srcdoc: out.srcdoc } }),
          );
      }
    }
  }, [target, runStatus.running, setCompileErrors, setFnToNodeMap, setStatus]);

  return { handleCompile };
}
