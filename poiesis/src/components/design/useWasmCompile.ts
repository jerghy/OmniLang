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

        // Build path mappings
        const pathMapping: Record<string, string> = {
          "./design.omni": "src/design.omni",
        };
        if (componentDefs) {
          for (const compName of Object.keys(componentDefs)) {
            pathMapping[`./components/${compName}`] = `src/components/${compName}.omni`;
          }
        }

        // Skip compile if sources haven't changed
        const sourcesKey = JSON.stringify(sources);
        if (sourcesKey === lastSourcesRef.current) {
          setIsCompiling(false);
          return;
        }
        lastSourcesRef.current = sourcesKey;

        const result = rustCompiler.resolveStylesMulti(sources, pathMapping);

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
  }, [elements, wires, rootOrder, activeDesign, componentDefs, codeFlowVisible]);

  return { output, isCompiling, errors };
}

export { useWasmCompile };
