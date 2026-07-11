import { useEffect, useRef } from "react";
import { usePoiesisStore } from "../../store";
import { rustCompiler } from "../../commands/rustCompiler";
import { generateOmniFiles } from "../../services/codegen/pipeline";
import { useDebugWindow } from "./useDebugWindow";
import { Logger } from "../../lib/utils/logger";

const log = new Logger({ prefix: "DebugCompile" });

/**
 * When debug window is open, re-compiles the full project (editor_mode=true, debug_mode=true)
 * on every canvas change (150ms debounce) and pushes result to the debug window.
 */
export function useDebugCompile() {
  const { isOpen, pushCompileResult } = useDebugWindow();
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Watch the same elements/wires/rootOrder as useWasmCompile
  const elements = usePoiesisStore((s) => s.elements);
  const wires = usePoiesisStore((s) => s.wires);
  const rootOrder = usePoiesisStore((s) => s.rootOrder);

  useEffect(() => {
    if (!isOpen || !rustCompiler.isReady) return;
    if (elements.length === 0) return;

    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      try {
        // Generate .omni sources from current editor state
        const result = generateOmniFiles({
          designElements: elements,
          designWires: wires,
          codeTabs: [],
          componentName: "main",
          rootOrder,
        });

        // Collect all .omni source files from result.value
        const sources: Record<string, string> = {};
        if (result.value) {
          for (const [path, content] of Object.entries(result.value)) {
            sources[path] = content;
          }
        }

        // Compile with debug flags (editorMode=true, debugMode=true)
        const output = rustCompiler.compileForDebugWindow(sources);
        if (output) {
          pushCompileResult({ ...output, nidMap: output.nidMap });
        }
      } catch (e) {
        log.error("Compile failed:", e);
      }
    }, 150);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [isOpen, elements, wires, rootOrder, pushCompileResult]);
}
