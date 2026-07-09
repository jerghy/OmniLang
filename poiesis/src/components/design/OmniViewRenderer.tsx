// components/design/OmniViewRenderer.tsx
// React direct renderer — walks the element tree, expanding custom components
// inline so their internal elements render correctly. Elements inside custom
// components are non-interactive (cannot be selected individually).
//
// Performance: all element lookups use O(1) Map/Set ops; inline styles are
// pre-computed once per tree; OmniViewElement is React.memo'd for subtree
// skipping; callbacks are useCallback'd for stable references.

import { useMemo, useCallback } from "react";
import { usePoiesisStore } from "../../store";
import { useElements, useRootOrder, useSelectedElementIds, useComponentDefs } from "../../store/hooks";
import { OmniViewElement } from "./OmniViewElement";
import { buildInlineStyleMap } from "./render-utils";
import { rustCompiler } from "../../commands/rustCompiler";
import { generateOmniFiles, generateOmniFragment } from "../../services/codegen/pipeline";
import { expandCustomComponents } from "../../services/codegen/expand-custom-components";
import { emitDesignSource } from "../../services/codegen/design-emitter";
import type { ElementData, ViewElementData } from "../../core/types/element";
import type { CodeTab } from "../../core/types/codegraph";

export function OmniViewRenderer() {
  const elements = useElements();
  const rootOrder = useRootOrder();
  const selectedIds = useSelectedElementIds();
  const selectElement = usePoiesisStore((s) => s.selectElement);
  const toggleCodeFlow = usePoiesisStore((s) => s.toggleCodeFlow);
  const codeFlowVisible = usePoiesisStore((s) => s.designPage.codeFlowVisible);
  // Subscribe to componentDefs so the renderer re-expands when a custom
  // component's definition changes (e.g. after editing and saving MyCard.omni).
  const componentDefs = useComponentDefs();
  // Subscribe to activeDesign so class/style changes reflect on canvas immediately.
  const activeDesign = usePoiesisStore((s) => s.activeDesign);
  // Subscribe to wires for WASM codegen
  const wires = usePoiesisStore((s) => s.wires);

  // Expand custom components for canvas rendering (reuses codegen's expansion)
  const { expandedElements, customChildIds, idMappings } = useMemo(() => {
    const result = expandCustomComponents(elements);
    // Convert to Map for O(1) lookups in OmniViewElement
    const map = new Map<string, ElementData>(result.elements.map((e) => [e.id, e]));
    return { expandedElements: map, customChildIds: result.customChildIds, idMappings: result.idMappings };
  }, [elements, componentDefs]);

  // Generate ALL .omni sources for WASM multi-file resolution
  const allOmniSources = useMemo(() => {
    if (elements.length === 0) return null;

    const codeTabs: CodeTab[] = [{
      id: "main", type: "logic", label: "Main", icon: "📄",
      graph: { elements, wires }, variables: [],
    }];

    try {
      // Main component
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

      // Collect main + store files
      if (mainResult.value) {
        Object.assign(sources, mainResult.value);
      }

      // Generate source for each custom component referenced in the tree
      if (componentDefs) {
        for (const [compName, compDef] of Object.entries(componentDefs)) {
          if (!compDef.elements || compDef.elements.length === 0) continue;
          const compResult = generateOmniFragment({
            componentName: compName,
            designElements: compDef.elements,
            designWires: compDef.wires || [],
            codeTabs: [],
            rootOrder: compDef.rootOrder,
            fromDefinition: true,
          });
          if (compResult.value) {
            sources[`src/components/${compName}.omni`] = compResult.value;
          }
        }
      }

      return sources;
    } catch {
      return null;
    }
  }, [elements, wires, rootOrder, activeDesign, componentDefs]);

  // WASM CSS resolution with ALL component files
  const wasmStyleResult = useMemo(() => {
    if (!allOmniSources || !rustCompiler.isReady) return null;

    const sources: Record<string, string> = { ...allOmniSources };

    // Add design.omni
    const designSrc = activeDesign?.name
      && (activeDesign.tokens?.length || activeDesign.classes?.length || activeDesign.variants?.length)
      ? emitDesignSource(activeDesign)
      : null;
    if (designSrc) {
      sources["src/design.omni"] = designSrc;
    }

    // Build path mappings for cross-file imports
    const pathMapping: Record<string, string> = {
      "./design.omni": "src/design.omni",
    };
    if (componentDefs) {
      for (const compName of Object.keys(componentDefs)) {
        pathMapping[`./components/${compName}`] = `src/components/${compName}.omni`;
      }
    }

    const result = rustCompiler.resolveStylesMulti(sources, pathMapping);
    if (!result.success) return null;
    return result;
  }, [allOmniSources, activeDesign, componentDefs]);

  // Translate WASM class map from original element IDs → expanded canvas IDs
  const classMap = useMemo(() => {
    if (!wasmStyleResult) return null;

    // Build reverse mapping: expandedId → originalDefId
    // (idMappings is: expandedId → originalDefId, same as expandCustomComponents returns)
    const map = new Map<string, string[]>();

    for (const [elId, classes] of Object.entries(wasmStyleResult.element_classes)) {
      // Direct lookup for non-component elements (IDs are unchanged)
      map.set(elId, classes as string[]);
    }

    // For expanded component-internal elements, add entries keyed by expanded ID
    for (const [expandedId, originalId] of idMappings) {
      const classes = wasmStyleResult.element_classes[originalId];
      if (classes) {
        map.set(expandedId, classes as string[]);
      }
    }

    return map;
  }, [wasmStyleResult, idMappings]);

  // Combined CSS for <style> injection (design_css + component_css)
  const combinedCss = useMemo(() => {
    if (!wasmStyleResult) return null;
    const parts: string[] = [];
    if (wasmStyleResult.design_css) parts.push(wasmStyleResult.design_css);
    for (const css of Object.values(wasmStyleResult.component_css)) {
      if (css) parts.push(css);
    }
    return parts.join("\n");
  }, [wasmStyleResult]);

  // Fallback: existing TS-based inline style resolution
  const inlineStyleMap = useMemo(
    () => buildInlineStyleMap(expandedElements, activeDesign),
    [expandedElements, activeDesign],
  );

  // Convert selectedIds array to Set for O(1) .has() lookup
  const selectedIdSet = useMemo(() => new Set(selectedIds), [selectedIds]);

  // Resolve root elements via Map.get() — O(1) instead of Array.find() O(n)
  const rootElements: ViewElementData[] = useMemo(
    () =>
      rootOrder
        .map((id) => expandedElements.get(id))
        .filter((e): e is ViewElementData => e !== undefined && "children" in e && "tag" in e),
    [rootOrder, expandedElements],
  );

  // Stable callbacks — no closures created per render in children
  const handleSelect = useCallback((id: string) => selectElement(id), [selectElement]);

  const handleDoubleClick = useCallback(
    (id: string) => toggleCodeFlow(codeFlowVisible ? null : id),
    [toggleCodeFlow, codeFlowVisible],
  );

  return (
    <div
      data-renderer="omniview"
      style={{
        width: "100%", height: "100%", overflow: "auto",
        padding: 8, boxSizing: "border-box",
      }}
    >
      {combinedCss && <style>{combinedCss}</style>}
      {rootElements.length === 0 ? (
        <div style={{
          display: "flex", alignItems: "center", justifyContent: "center",
          height: "100%", color: "#94a3b8", fontSize: 13,
        }}>
          空画布 — 从组件面板拖入元素
        </div>
      ) : (
        rootElements.map((el) => (
          <OmniViewElement
            key={el.id}
            element={el}
            expandedElements={expandedElements}
            customChildIds={customChildIds}
            selectedIds={selectedIdSet}
            inlineStyleMap={inlineStyleMap}
            classMap={classMap}
            onSelect={handleSelect}
            onDoubleClick={handleDoubleClick}
          />
        ))
      )}
    </div>
  );
}
