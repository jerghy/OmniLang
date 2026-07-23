import { useEffect, useRef } from "react";
import { usePoiesisStore } from "../../store";
import { rustCompiler } from "../../commands/rustCompiler";
import { generateOmniFiles, generateOmniFragment } from "../../services/codegen/pipeline";
import { emitDesignSource } from "../../services/codegen/design-emitter";
import { useDebugWindow } from "./useDebugWindow";
import { Logger } from "../../lib/utils/logger";

const log = new Logger({ prefix: "DebugCompile" });

/** Simple hash for change detection. */
function hashStr(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) {
    h = ((h << 5) - h + s.charCodeAt(i)) | 0;
  }
  return h;
}

/**
 * When debug window is open, re-compiles the full project (editor_mode=true, debug_mode=true)
 * on every canvas change (150ms debounce) and pushes result to the debug window.
 *
 * Hot-replace strategy:
 * - First load (windowReadyCounter increment): full page replacement via debug:compile-result
 * - Subsequent edits: component-level hot-replace via debug:hot-replace
 */
export function useDebugCompile() {
  const { isOpen, windowReadyCounter, pushCompileResult, pushHotReplace } = useDebugWindow();
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const prevHashes = useRef<Record<string, number>>({});
  const needFullReload = useRef(true);

  // Watch the same elements/wires/rootOrder/activeDesign as useWasmCompile
  const elements = usePoiesisStore((s) => s.elements);
  const wires = usePoiesisStore((s) => s.wires);
  const rootOrder = usePoiesisStore((s) => s.rootOrder);
  const activeDesign = usePoiesisStore((s) => s.activeDesign);
  const componentDefs = usePoiesisStore((s) => s.componentDefs);

  // Track isOpen transitions: when the debug window is reopened
  // (closed → open), force a full page reload. Without this, reopening
  // uses the hot-replace path which relies on (0, eval) that cannot
  // access IIFE-scoped runtime functions (__omni_signal, etc.),
  // causing the debug window to break with a blank page.
  const prevIsOpen = useRef(isOpen);
  useEffect(() => {
    if (isOpen && !prevIsOpen.current) {
      needFullReload.current = true;
    }
    prevIsOpen.current = isOpen;
  }, [isOpen]);

  // When debug window reconnects (windowReadyCounter increments), force full reload
  useEffect(() => {
    if (windowReadyCounter > 0) {
      needFullReload.current = true;
    }
  }, [windowReadyCounter]);

  useEffect(() => {
    if (!isOpen) return;
    if (!rustCompiler.isReady) return;
    if (elements.length === 0) return;

    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      try {
        // Mirror useWasmCompile's codeTab construction — codeTabs needs graph property
        const codeTabs = [{
          id: "main", type: "logic" as const, label: "Main", icon: "📄",
          graph: { elements, wires }, variables: [],
        }];

        const mainResult = generateOmniFiles({
          designElements: elements,
          designWires: wires,
          codeTabs,
          componentName: "main",
          rootOrder,
          filePath: "src/main.omni",
          designName: activeDesign?.name
            && (activeDesign?.tokens?.length || activeDesign?.classes?.length || activeDesign?.variants?.length)
            ? activeDesign.name : undefined,
        });

        const sources: Record<string, string> = {};
        if (mainResult.value) Object.assign(sources, mainResult.value);

        // Generate source for each custom component
        if (componentDefs) {
          for (const [compName, compDef] of Object.entries(componentDefs)) {
            if (!compDef.elements || compDef.elements.length === 0) continue;
            const compResult = generateOmniFragment({
              componentName: compName,
              designElements: compDef.elements,
              designWires: compDef.wires || [],
              codeTabs: [{ id: "fragment", type: "logic", label: "Fragment", icon: "📄",
                graph: { elements: compDef.elements, wires: compDef.wires || [] }, variables: [] }],
              rootOrder: compDef.rootOrder,
              fromDefinition: true,
            });
            if (compResult.value) {
              sources[`src/components/${compName}.omni`] = compResult.value;
            }
          }
        }

        // Add design.omni
        const designSrc = activeDesign?.name
          && (activeDesign?.tokens?.length || activeDesign?.classes?.length || activeDesign?.variants?.length)
          ? emitDesignSource(activeDesign)
          : null;
        if (designSrc) sources["src/design.omni"] = designSrc;

        const output = rustCompiler.compileForDebugWindow(sources);
        if (!output) return;

        if (needFullReload.current) {
          // First load / window reconnect — full page replacement
          pushCompileResult({ ...output, nidMap: output.nidMap });
          // Cache component hashes for future diffing
          const newHashes: Record<string, number> = {};
          for (const [name, comp] of Object.entries(output.components)) {
            newHashes[name] = hashStr(comp.js);
          }
          prevHashes.current = newHashes;
          needFullReload.current = false;
        } else {
          // Hot update — diff components and send only changed ones
          const changed: Record<string, { js: string }> = {};
          for (const [name, comp] of Object.entries(output.components)) {
            const h = hashStr(comp.js);
            if (prevHashes.current[name] !== h) {
              changed[name] = comp;
              prevHashes.current[name] = h;
            }
          }
          if (Object.keys(changed).length > 0) {
            pushHotReplace(changed);
          }
        }
      } catch (e) {
        log.error("Compile failed:", e);
      }
    }, 150);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [isOpen, windowReadyCounter, elements, wires, rootOrder, activeDesign, pushCompileResult, pushHotReplace]);
}
