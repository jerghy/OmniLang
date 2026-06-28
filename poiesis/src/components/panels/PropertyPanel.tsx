/** 属性面板。根据选中元素和统一元数据动态渲染编辑器。
 *  编辑自定义组件定义时显示 ComponentMetaSection。*/

import React, { useCallback, useRef, useEffect } from "react";
import type { ElementData } from "../../core/types";
import { useEditorStore } from "../../store/editor";
import { useCodeGraphStore } from "../../services/codegraph/store";
import { compilerCache } from "../../services/compiler/cache";
import {
  getUnifiedComponentMeta,
  useComponentRegistry,
} from "../../store/componentRegistry";
import { getComponentEvents } from "../../data/componentProtocol";
import type { CommandContext } from "../../core/commands/types";

// Section components
import { LayoutSection } from "../inspector/sections/LayoutSection";
import { StyleSection } from "../inspector/sections/StyleSection";
import { EventSection } from "../inspector/sections/EventSection";
import { DataSection } from "../inspector/sections/DataSection";
import { ComponentMetaSection } from "../inspector/sections/ComponentMetaSection";

// ═══════════════════════════════════════════════
// Types
// ═══════════════════════════════════════════════

interface PropMeta {
  type: string;
  default?: unknown;
  values?: string[];
  editor: string;
}

interface EventMeta {
  id?: string;
  description: string;
  payload: string | null;
}

interface ComponentMeta {
  component: string;
  category: string;
  props: Record<string, PropMeta>;
  events: Record<string, EventMeta>;
  children: boolean;
  isCustom?: boolean;
}

// ═══════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════

/** Walk up the element tree to find which component owns an element.
 *  Returns 'App' for top-level elements that are not inside a custom component. */
function findOwningComponent(elementId: string, elements: ElementData[]): string {
  let current = elements.find((e) => e.id === elementId);
  if (!current) return 'App';

  // Walk up to root
  while (current?.parentId) {
    const parent = elements.find((e) => e.id === current!.parentId);
    if (!parent) break;
    current = parent;
  }

  return current?.componentRef || 'App';
}

// ═══════════════════════════════════════════════
// Main Component
// ═══════════════════════════════════════════════

interface PropertyPanelProps {
  ctx: CommandContext;
  onNavigateToCode?: (wireId: string) => void;
}

export function PropertyPanel({ ctx, onNavigateToCode }: PropertyPanelProps) {
  const elements = useEditorStore((s) => s.elements);
  const selectedIds = useEditorStore((s) => s.selectedElementIds);
  const activeDocId = useEditorStore((s) => s.activeDocId);

  const isEditingComponent = activeDocId?.startsWith("comp:") ?? false;
  const editingCompName = isEditingComponent ? activeDocId!.replace("comp:", "") : null;

  // ── Debounced recompile on property change ──
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const debouncedRecompile = useCallback(() => {
    if (debounceTimerRef.current) clearTimeout(debounceTimerRef.current);
    debounceTimerRef.current = setTimeout(() => {
      const store = useEditorStore.getState();
      const cg = useCodeGraphStore.getState();
      const codeTabs = cg.toSerializable().tabs;

      // Determine which component owns the currently selected element
      const compName = selectedIds[0]
        ? findOwningComponent(selectedIds[0], store.elements)
        : 'App';

      const output = compilerCache.compileProject({
        designElements: store.elements,
        designWires: store.wires,
        codeTabs,
        componentName: compName,
        rootOrder: store.rootOrder,
      }, true);

      if (output?.srcdoc) {
        // Dispatch custom event so LayoutView can update the iframe
        window.dispatchEvent(
          new CustomEvent('omni-srcdoc-update', { detail: { srcdoc: output.srcdoc } }),
        );
      }
    }, 300);
  }, [selectedIds]);

  // Cleanup debounce timer on unmount
  useEffect(() => {
    return () => {
      if (debounceTimerRef.current) clearTimeout(debounceTimerRef.current);
    };
  }, []);

  const raw = elements.find((e) => e.id === selectedIds[0]);

  // When editing a component definition and nothing is selected, show component meta editor
  if (!raw && isEditingComponent && editingCompName) {
    return (
      <div style={{ padding: 12, overflow: "auto", flex: 1, fontSize: 13 }}>
        <ComponentMetaSection componentName={editingCompName} />
        <div style={{ padding: 16, color: "#9ca3af", textAlign: "center", marginTop: 16 }}>
          <div style={{ fontSize: 24, marginBottom: 8 }}>🔍</div>
          <div>选择元素以编辑属性</div>
          <div style={{ fontSize: 10, marginTop: 4 }}>或在画布上拖入组件设计内部结构</div>
        </div>
      </div>
    );
  }

  if (!raw) {
    return (
      <div style={{ padding: 16, color: "#9ca3af", textAlign: "center" }}>
        <div style={{ fontSize: 24, marginBottom: 8 }}>🔍</div>
        <div>选择画布元素以编辑属性</div>
      </div>
    );
  }

  const el = { ...raw, props: raw.props || {} };

  // ── Map HTML tags to OmniLang component names for metadata lookup ──
  const HTML_TAG_TO_COMPONENT: Record<string, string> = {
    // Typography
    'p': 'Text', 'span': 'Text',
    'h1': 'Heading', 'h2': 'Heading', 'h3': 'Heading',
    'h4': 'Heading', 'h5': 'Heading', 'h6': 'Heading',
    // Actions
    'button': 'Button',
    // Forms
    'input': 'Input', 'textarea': 'TextArea', 'select': 'Select', 'form': 'Form',
    // Media
    'img': 'Image',
    // Navigation
    'a': 'Link',
    // Data
    'ul': 'List', 'li': 'ListItem',
    // Layout (ambiguous — default to Container for best-effort metadata)
    'div': 'Container', 'section': 'Container', 'header': 'Container',
    'footer': 'Container', 'main': 'Container',
  };
  const resolvedTag = HTML_TAG_TO_COMPONENT[el.tag] || el.tag;

  // Resolve metadata: custom component instances use their definition's metadata
  const unifiedMeta = getUnifiedComponentMeta();
  let meta: ComponentMeta | undefined = unifiedMeta[resolvedTag];

  // If this is a custom component instance, use the definition's metadata
  if (!meta && el.componentRef) {
    const registry = useComponentRegistry.getState();
    const def = registry.definitions[el.componentRef];
    if (def) {
      meta = {
        component: def.name,
        category: def.category,
        props: def.props,
        events: def.events,
        children: def.children,
        isCustom: true,
      };
    }
  }

  if (!meta) {
    return (
      <div style={{ padding: 16 }}>
        <strong>{`<${el.tag}>`}</strong>
        {el.componentRef && <div style={{ color: "#DC2626", fontSize: 11, marginTop: 4 }}>
          ⚠ 组件 "{el.componentRef}" 未找到
        </div>}
        <div style={{ color: "#9ca3af", fontSize: 12, marginTop: 4 }}>无元数据</div>
      </div>
    );
  }

  // ── Merge universal events + component-specific events ──
  // OmniView 采用开放事件模型：任何组件都能绑定任何 onXxx 事件。
  // 通用事件是所有组件都支持的，组件特殊事件则提供更精准的描述和 ID。
  const allEvents: Record<string, EventMeta> = {
    ...getComponentEvents(resolvedTag),
    ...meta.events,
  };

  return (
    <div style={{ padding: 12, overflow: "auto", flex: 1, fontSize: 13 }}>
      {/* Component meta editor when editing definition */}
      {isEditingComponent && editingCompName && (
        <ComponentMetaSection componentName={editingCompName} />
      )}

      <h3 style={{ margin: "0 0 8px", fontSize: 14 }}>
        {meta.isCustom && <span style={{ color: "#4F46E5", marginRight: 4 }}>★</span>}
        {`<${el.tag}>`}
        {el.nodeType !== "view" && (
          <span style={{ fontSize: 10, color: "#6b7280", marginLeft: 8 }}>{el.nodeType}</span>
        )}
        {meta.isCustom && (
          <span style={{ fontSize: 10, color: "#4F46E5", marginLeft: 6, fontWeight: 400 }}>自定义</span>
        )}
      </h3>

      <DataSection element={el} meta={meta} ctx={ctx} onRecompile={debouncedRecompile} />
      <EventSection
        element={el}
        meta={meta}
        allEvents={allEvents}
        ctx={ctx}
        onRecompile={debouncedRecompile}
        onNavigateToCode={onNavigateToCode}
      />
      <StyleSection element={el} ctx={ctx} onRecompile={debouncedRecompile} />
      <LayoutSection element={el} onRecompile={debouncedRecompile} />
    </div>
  );
}

