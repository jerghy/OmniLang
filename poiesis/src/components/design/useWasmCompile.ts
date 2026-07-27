// poiesis/src/components/design/useWasmCompile.ts
// React hook that listens to Zustand store changes, generates .omni source files,
// debounces 150ms, compiles via WASM, and returns structured output for React rendering.

import { useState, useEffect, useRef } from "react";
import { usePoiesisStore } from "../../store";
import { useElements, useRootOrder, useComponentDefs } from "../../store/hooks";
import { rustCompiler } from "../../commands/rustCompiler";
import type { StyleResolveResult } from "../../commands/rustCompiler";
import { generateOmniFiles, generateOmniFragment } from "../../services/codegen/pipeline";
import { emitDesignSource } from "../../services/codegen/design-emitter";
import type { CodeTab } from "../../core/types/codegraph";

export interface OmniViewOutput {
  css: string;
  component_css: Record<string, string>;
  element_tree: ElementTreeNode[];
  element_classes: Record<string, string[]>;
  element_tags: Record<string, string>;
  nid_map: Record<string, string>;
  warnings: string[];
}

export interface ElementTreeNode {
  nid: string;
  tag: string;
  text?: string | null;
  children: ElementTreeNode[];
  /** Inline styles from style: { } — applied directly as HTML style attribute */
  inlineStyle?: Record<string, string>;
}

function useWasmCompile(): {
  output: OmniViewOutput | null;
  isCompiling: boolean;
  errors: string[];
} {
  const elements = useElements();
  const rootOrder = useRootOrder();
  const wires = usePoiesisStore((s) => s.wires);
  const activeDesign = usePoiesisStore((s) => s.activeDesign);
  const componentDefs = useComponentDefs();
  const codeFlowVisible = usePoiesisStore((s) => s.designPage.codeFlowVisible);
  const debugSignalValues = usePoiesisStore((s) => s.debugSignalValues);

  const [output, setOutput] = useState<OmniViewOutput | null>(null);
  const [isCompiling, setIsCompiling] = useState(false);
  const [errors, setErrors] = useState<string[]>([]);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastSourcesRef = useRef<string>("");

  useEffect(() => {
    if (!rustCompiler.isReady) return;
    if (elements.length === 0) {
      setOutput(null);
      setIsCompiling(false);
      return;
    }

    // Skip WASM recompilation when CodeFlow is active.
    // OmniViewRenderer stays mounted so output is preserved in store.
    if (codeFlowVisible) return;

    setIsCompiling(true);
    // 150ms debounce
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {

      try {
        const codeTabs: CodeTab[] = [{
          id: "main", type: "logic", label: "Main", icon: "📄",
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
            && (activeDesign.tokens?.length || activeDesign.classes?.length || activeDesign.variants?.length)
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
          && (activeDesign.tokens?.length || activeDesign.classes?.length || activeDesign.variants?.length)
          ? emitDesignSource(activeDesign)
          : null;
        if (designSrc) sources["src/design.omni"] = designSrc;

        // Apply debug signal value overrides before WASM compilation.
        // Returns a new object — the store's original defaults are untouched.
        const effectiveSources =
          Object.keys(debugSignalValues).length > 0
            ? applyDebugSignalOverrides(sources, debugSignalValues)
            : sources;

        // Path mapping is auto-derived by WASM from source keys
        const pathMapping: Record<string, string> = {};

        // Skip compile if sources haven't changed
        const sourcesKey = JSON.stringify(effectiveSources);
        if (sourcesKey === lastSourcesRef.current) {
          setIsCompiling(false);
          return;
        }
        lastSourcesRef.current = sourcesKey;

        const result = rustCompiler.resolveStylesMultiCached(effectiveSources, pathMapping);

        const actualErrors = (result.errors || []).filter(
          (e: any) => e.severity === "error",
        );
        if (actualErrors.length === 0) {
          // Build element UUID → inline styles map from store elements
          const elStyleMap = new Map<string, Record<string, string>>();
          for (const el of elements) {
            const props = (el as any).style?.inlineProperties as Array<{ name: string; segments: Array<{ type: string; value?: string; name?: string }> }> | undefined;
            if (props && props.length > 0) {
              const styles: Record<string, string> = {};
              for (const p of props) {
                const val = p.segments
                  .map((s) => (s.type === "text" ? s.value : s.type === "var-ref" ? `var(--${s.name})` : ""))
                  .join("");
                if (val) styles[p.name] = val;
              }
              elStyleMap.set(el.id, styles);
            }
          }

          // Walk element tree and attach inline styles via nid_map
          const nidMapRaw = (result as StyleResolveResult).nid_map || {};
          const nidMap: Record<string, string> =
            nidMapRaw instanceof Map ? Object.fromEntries(nidMapRaw) : nidMapRaw;
          const elementTree = (result as StyleResolveResult).element_tree || [];

          function attachInlineStyles(nodes: ElementTreeNode[]) {
            for (const node of nodes) {
              const elId = nidMap[node.nid];
              if (elId && elStyleMap.has(elId)) {
                node.inlineStyle = elStyleMap.get(elId);
              }
              if (node.children) attachInlineStyles(node.children as ElementTreeNode[]);
            }
          }
          if (elStyleMap.size > 0) {
            attachInlineStyles(elementTree as ElementTreeNode[]);
          }

          setOutput({
            css: result.design_css,
            component_css: result.component_css,
            element_tree: elementTree,
            element_classes: result.element_classes,
            element_tags: result.element_tags,
            nid_map: nidMapRaw,
            warnings: result.warnings,
          });
          setErrors([]);
        } else {
          setErrors(actualErrors.map((e: any) => e.message));
        }
      } catch (e) {
        setErrors([e instanceof Error ? e.message : String(e)]);
      } finally {
        setIsCompiling(false);
      }
    }, 150);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [elements, wires, rootOrder, activeDesign, componentDefs, codeFlowVisible, debugSignalValues]);

  return { output, isCompiling, errors };
}

/**
 * Apply debug signal value overrides to generated .omni source strings.
 * Uses regex to replace signal default values so the WASM compiler
 * resolves them with debug values instead of stored defaults.
 * Original sources are mutated in-place.
 */
function applyDebugSignalOverrides(
  sources: Readonly<Record<string, string>>,
  debugValues: Record<string, Record<string, unknown>>,
): Record<string, string> {
  // Collect all debug signals into a flat map for efficient lookup
  const flatDebug: Record<string, unknown> = {};
  for (const signals of Object.values(debugValues)) {
    Object.assign(flatDebug, signals);
  }
  if (Object.keys(flatDebug).length === 0) return { ...sources };

  const result: Record<string, string> = {};
  for (const path of Object.keys(sources)) {
    let text = sources[path];
    for (const [sigName, sigValue] of Object.entries(flatDebug)) {
      if (sigValue === undefined || sigValue === null) continue;
      const formatted = formatDebugValue(sigValue);
      // Match: signal name: Type = "old_default"; or signal name: Type = old_default;
      const re = new RegExp(
        `(signal\\s+${escapeRegex(sigName)}\\s*:\\s*\\S+\\s*=\\s*)([^;\\n]+)(;)`,
        'g',
      );
      if (re.test(text)) {
        // Reset lastIndex after test()
        re.lastIndex = 0;
        text = text.replace(re, `$1${formatted}$3`);
      } else {
        // Signal has no default value: signal name: Type;
        const noDefaultRe = new RegExp(
          `(signal\\s+${escapeRegex(sigName)}\\s*:\\s*\\S+)(\\s*;)`,
          'g',
        );
        text = text.replace(noDefaultRe, `$1 = ${formatted}$2`);
      }
    }
    result[path] = text;
  }
  return result;
}

function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function formatDebugValue(value: unknown): string {
  if (typeof value === 'string') {
    // Escape quotes inside the string value
    const escaped = value.replace(/"/g, '\\"');
    return `"${escaped}"`;
  }
  return String(value);
}

export { useWasmCompile };
