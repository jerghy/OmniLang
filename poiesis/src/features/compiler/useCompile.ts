import { useCallback } from "react";
import { usePoiesisStore } from "../../store";
import { serializeCodeTabs } from "../../services/codegraph/store";
import { extractComponentName } from "../../core/utils";
import { compileProject } from "../../services/compiler/compiler";
import { compilerCache } from "../../services/compiler/cache";
import { api } from "../../commands";
import { parseCodeBlocks } from "../../services/parser/omni-regex-parser";
import type { ElementData } from "../../core/types";
import { createLogger } from "../../lib/utils/logger";

const log = createLogger("useCompile");

export function useCompile(
  target: string,
  runStatus: { running: boolean },
  setFnToNodeMap: (map: Record<string, string>) => void,
  setStatus: (s: { text: string; isError: boolean }) => void,
) {
  const setCompileErrors = usePoiesisStore((s) => s.setCompileErrors);

  const handleCompile = useCallback(async () => {
    const store = usePoiesisStore.getState();
    const codeTabs = serializeCodeTabs().tabs;
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

    usePoiesisStore.getState().switchTab("main");
    usePoiesisStore.getState().setElementsAndWires([], []);
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
            parentId: parentStore ? `cb-store-${parentStore}` : null,
            ports: portDefs[b.blockType] || [],
            source: "",
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
              source: "",
              parentId: null,
              ports: portDefs.store,
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

      usePoiesisStore.getState().setElementsAndWires(codeElements, []);
      usePoiesisStore.getState().setActiveFileContent(content);
    } catch (err) {
      // Non-critical: code block parsing failure doesn't block compilation.
      // Still log so we can diagnose parser bugs in production.
      log.warn('code block parsing failed:', err);
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
        api.compile.writeRunFiles(compileResponse.files).catch((e) => log.warn("Failed to write run files:", e));
      }
      // Populate cache from already-compiled output instead of re-running the pipeline
      if (compileResult.generatedSource && compileResponse.files) {
        const files = compileResponse.files;
        const srcdoc = [
          "<!DOCTYPE html>",
          '<html lang="en">',
          "<head>",
          '<meta charset="UTF-8">',
          '<meta name="viewport" content="width=device-width, initial-scale=1.0">',
          "<title>OmniView Preview</title>",
          "<style>",
          "*,*::before,*::after{box-sizing:border-box}",
          "body{margin:0;padding:8px;font-family:system-ui,sans-serif}",
          (files['index.css'] || ''),
          "</style>",
          "</head>",
          "<body>",
          (files['index.html'] || ''),
          "<script>",
          (files['index.js'] || ''),
          "</script>",
          "</body>",
          "</html>",
        ].join("\n");
        compilerCache.cachePut(compileResult.generatedSource, { srcdoc, omniSource: compileResult.generatedSource });
        window.dispatchEvent(
          new CustomEvent("omni-srcdoc-update", { detail: { srcdoc } }),
        );
      }
    }
  }, [target, runStatus.running, setCompileErrors, setFnToNodeMap, setStatus]);

  return { handleCompile };
}
