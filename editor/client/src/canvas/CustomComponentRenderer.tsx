/** 自定义组件实例渲染器。
 *
 *  解析组件定义中的 {param:xxx} 绑定语法，合并实例 props，
 *  递归渲染内部元素树。支持嵌套子元素和 <Slot/> 标记。
 *  内置递归深度限制防止循环引用导致栈溢出。 */

import React, { useMemo } from "react";
import { useComponentRegistry, type ComponentDefinition } from "../store/componentRegistry";
import { useEditorStore, type ElementData } from "../store/editor";
import { COMPONENT_PROTOCOL } from "../data/componentProtocol";

/** Maximum recursion depth for custom component rendering.
 *  Prevents stack overflow from circular component references. */
const MAX_RENDER_DEPTH = 8;

/** Slot marker — element with this tag in the definition marks where
 *  instance children should be inserted. */
const SLOT_TAG = "Slot";

// ═══════════════════════════════════════════════
// Param Binding Resolver
// ═══════════════════════════════════════════════

function resolveParamBinding(
  value: unknown,
  instanceProps: Record<string, unknown>,
  definitionProps: Record<string, { type: string; default?: unknown }>,
): unknown {
  if (typeof value === "string") {
    const fullMatch = value.match(/^\{param:(\w+)\}$/);
    if (fullMatch) {
      const paramName = fullMatch[1];
      if (paramName in instanceProps) return instanceProps[paramName];
      const schema = definitionProps[paramName];
      if (schema?.default !== undefined) return schema.default;
      return value;
    }
    return value.replace(/\{param:(\w+)\}/g, (_m, name) => {
      if (name in instanceProps) return String(instanceProps[name]);
      const schema = definitionProps[name];
      if (schema?.default !== undefined) return String(schema.default);
      return `{param:${name}}`;
    });
  }
  return value;
}

// ═══════════════════════════════════════════════
// Main Renderer
// ═══════════════════════════════════════════════

interface CustomRendererProps {
  instanceEl: ElementData;
  definition: ComponentDefinition;
  allElements: ElementData[];
  selectedIds: string[];
  onSelect: (id: string | null) => void;
  onDropOnElement: (e: React.DragEvent, parentId: string) => void;
  onDragOverElement: (e: React.DragEvent) => void;
  onContextMenu: (e: React.MouseEvent, id: string) => void;
  isDragging?: boolean;
  isSelected?: boolean;
  RenderElement: React.ComponentType<{
    el: ElementData;
    allElements: ElementData[];
    selectedIds: string[];
    onSelect: (id: string | null) => void;
    onDropOnElement: (e: React.DragEvent, parentId: string) => void;
    onDragOverElement: (e: React.DragEvent) => void;
    onContextMenu: (e: React.MouseEvent, id: string) => void;
    isDragging?: boolean;
    isSelected?: boolean;
    /** Recursion depth guard for custom components */
    _renderDepth?: number;
  }>;
  /** Recursion depth — internal, do not pass directly */
  _renderDepth?: number;
}

export function CustomComponentRenderer(props: CustomRendererProps): React.ReactElement {
  const {
    instanceEl, definition, allElements, selectedIds, onSelect,
    onDropOnElement, onDragOverElement, onContextMenu,
    isDragging, isSelected, RenderElement,
    _renderDepth = 0,
  } = props;

  // Guard against circular component references
  if (_renderDepth >= MAX_RENDER_DEPTH) {
    return React.createElement("div", {
      key: `overflow-${instanceEl.id}`,
      style: {
        padding: 12, border: "2px solid #F59E0B", borderRadius: 8,
        background: "#FFFBEB", color: "#92400E", fontSize: 11, textAlign: "center",
      },
    }, `⚠ 组件嵌套深度超过限制 (${MAX_RENDER_DEPTH}) — 可能存在循环引用`);
  }

  const instanceProps = (instanceEl.props || {}) as Record<string, unknown>;

  // Resolve definition elements with instance props
  const resolvedElements = useMemo(() => {
    return definition.elements.map((internalEl) => {
      const resolvedProps: Record<string, unknown> = {};
      for (const [key, value] of Object.entries(internalEl.props || {})) {
        resolvedProps[key] = resolveParamBinding(value, instanceProps, definition.props);
      }
      return { ...internalEl, props: resolvedProps };
    });
  }, [definition.elements, definition.props, instanceProps]);

  // Build parent→children index
  const childrenByParent = new Map<string, ElementData[]>();
  for (const defEl of definition.elements) {
    if (!defEl.parentId) continue;
    const list = childrenByParent.get(defEl.parentId) || [];
    list.push(defEl);
    childrenByParent.set(defEl.parentId, list);
  }

  // Root elements (no parentId) and slot element detection
  const rootElements = resolvedElements.filter((el) => !el.parentId);
  const slotElement = resolvedElements.find((el) => el.tag === SLOT_TAG);

  /** Recursive tree renderer — renders definition elements with their children */
  function renderTree(elements: ElementData[]): React.ReactNode[] {
    return elements.map((el) => {
      // Slot element: render instance's children here
      if (el.tag === SLOT_TAG) {
        if (instanceEl.children.length > 0) {
          return React.createElement(React.Fragment, { key: `slot-children-${instanceEl.id}` },
            ...instanceEl.children.map((childId) => {
              const child = allElements.find((e) => e.id === childId);
              if (!child) return null;
              const childSelected = selectedIds.includes(child.id);
              return React.createElement(RenderElement, {
                key: child.id,
                el: child,
                allElements,
                selectedIds,
                onSelect,
                onDropOnElement,
                onDragOverElement,
                onContextMenu,
                isSelected: childSelected,
                _renderDepth: _renderDepth + 1,
              });
            }),
          );
        }
        return React.createElement("div", {
          key: `slot-empty-${instanceEl.id}`,
          style: { minHeight: 16, minWidth: 40 },
        });
      }

      // Get this element's children from definition tree
      const kidIndices = childrenByParent.get(el.id) || [];
      const sortedKids = kidIndices
        .map((k) => resolvedElements.find((re) => re.id === k.id))
        .filter(Boolean) as ElementData[];

      // Render children recursively
      const childNodes = sortedKids.length > 0 ? renderTree(sortedKids) : [];

      return React.createElement(RenderElement, {
        key: `def-${definition.name}-${el.id}`,
        el: {
          ...el,
          // Stable derived IDs to avoid collisions between instances
          id: `ci-${instanceEl.id}-${el.id}`,
          parentId: el.parentId ? `ci-${instanceEl.id}-${el.parentId}` : null,
          // Instance children go through the Slot, not through internal elements
          children: [],
        },
        allElements,
        selectedIds,
        onSelect,
        onDropOnElement,
        onDragOverElement,
        onContextMenu,
        _renderDepth: _renderDepth + 1,
      });
    });
  }

  const renderedChildren = renderTree(rootElements);

  // Wrapper with instance-level styling
  const wrapperStyle: React.CSSProperties = {
    position: "relative",
    boxSizing: "border-box",
    ...(isSelected ? { outline: "2px solid #4F46E5", outlineOffset: "2px", zIndex: 1 } : {}),
    ...(definition.children ? { minHeight: 20 } : {}),
  };

  return React.createElement("div", {
    key: `custom-${instanceEl.id}`,
    "data-omni-id": instanceEl.id,
    "data-omni-tag": definition.name,
    "data-omni-custom": "true",
    style: wrapperStyle,
    onClick: (e: React.MouseEvent) => {
      if (isDragging) return;
      e.stopPropagation();
      onSelect(instanceEl.id);
    },
    onContextMenu: (e: React.MouseEvent) => onContextMenu(e, instanceEl.id),
  }, ...(renderedChildren.length > 0 ? renderedChildren : [
    React.createElement("span", {
      key: "__empty-custom",
      style: { color: "#D1D5DB", fontSize: 10, fontStyle: "italic" },
    }, `<${definition.name}>`),
  ]));
}
