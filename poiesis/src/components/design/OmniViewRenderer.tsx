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
import { useLiveCodePreview } from "../../hooks/useLiveCodePreview";
import { emitDesignSource } from "../../services/codegen/design-emitter";
import type { ElementData, ViewElementData } from "../../core/types/element";
import type { ComponentDef } from "../../core/types/project-json";

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

  // Build expanded element map + track IDs that are inside custom components
  const { expandedElements, customChildIds } = useMemo(() => {
    const registry: Record<string, ComponentDef> = componentDefs ?? {};
    const map = new Map<string, ElementData>(elements.map((e) => [e.id, e]));
    const innerIds = new Set<string>();

    const expanded = new Set<string>();
    let foundNew = true;
    while (foundNew) {
      foundNew = false;
      for (const [id, el] of [...map]) {
        if (!("tag" in el)) continue; // skip group elements
        const compRef: string | undefined = "componentRef" in el ? (el.componentRef || (registry[el.tag] ? el.tag : undefined)) : (registry[el.tag] ? el.tag : undefined);
        if (!compRef || expanded.has(id)) continue;
        expanded.add(id);
        const def = registry[compRef];
        const defElements = def && Array.isArray((def as unknown as Record<string, unknown>).elements)
          ? (def as unknown as Record<string, unknown>).elements as ElementData[]
          : null;
        if (!defElements || defElements.length === 0) continue;

        const idMap = new Map<string, string>();
        let idx = 0;
        for (const defEl of defElements) {
          idMap.set(defEl.id, `${id}__${idx++}`);
        }

        const newChildIds: string[] = [];
        for (const defEl of defElements) {
          if (!("children" in defEl && "tag" in defEl)) continue;
          const newId = idMap.get(defEl.id)!;
          // Mark as inside custom component (non-selectable)
          innerIds.add(newId);
          const isRoot = !defEl.parentId || !defElements.some((d) => d.id === defEl.parentId);
          const cloned: ViewElementData = {
            id: newId,
            parentId: isRoot ? id : (idMap.get(defEl.parentId || "") || defEl.parentId),
            nodeType: "view",
            tag: defEl.tag || "div",
            props: defEl.props || {},
            children: (defEl.children || []).map((cid) => idMap.get(cid) || cid),
            // Copy style (classRefs + inlineProperties) so canvas preview reflects
            // design-system class bindings and inline styles applied to the component.
            style: defEl.style ? { ...defEl.style, classRefs: [...(defEl.style.classRefs ?? [])], inlineProperties: [...(defEl.style.inlineProperties ?? [])] } : undefined,
          };
          map.set(newId, cloned);
          if (isRoot) newChildIds.push(newId);
        }

        const updated = { ...map.get(id)! };
        if ("children" in updated) {
          updated.children = [...updated.children, ...newChildIds];
        }
        map.set(id, updated);
        foundNew = true;
      }
    }

    return { expandedElements: map, customChildIds: innerIds };
  }, [elements, componentDefs]);

  // Live component .omni source for WASM compilation
  const componentSource = useLiveCodePreview();

  // Build design.omni source from activeDesign
  const designSource = useMemo(() => {
    if (!activeDesign?.name) return null;
    if (!activeDesign.tokens?.length && !activeDesign.classes?.length && !activeDesign.variants?.length) return null;
    return emitDesignSource(activeDesign);
  }, [activeDesign]);

  // WASM CSS resolution: resolve design-system styles + component styles
  const wasmStyleResult = useMemo(() => {
    if (!componentSource || !designSource || !rustCompiler.isReady) return null;
    const sources: Record<string, string> = {
      "src/design.omni": designSource,
      "src/main.omni": componentSource,
    };
    const pathMapping: Record<string, string> = {
      "./design.omni": "src/design.omni",
    };
    const result = rustCompiler.resolveStylesMulti(sources, pathMapping);
    if (!result.success) return null;
    return result;
  }, [componentSource, designSource]);

  // Build classMap from WASM result (element id → class names)
  const classMap = useMemo(() => {
    if (!wasmStyleResult) return null;
    const map = new Map<string, string[]>();
    for (const [elId, classes] of Object.entries(wasmStyleResult.element_classes)) {
      map.set(elId, classes);
    }
    return map;
  }, [wasmStyleResult]);

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
