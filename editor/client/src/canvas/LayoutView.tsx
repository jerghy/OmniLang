/** DOM 布局画布渲染器。OmniView 属性 → CSS 样式映射。*/

import React, { useCallback } from "react";
import { useEditorStore, type ElementData } from "../store/editor";
import { useUndoStore } from "../store/undo";
import { AddElement } from "../commands/AddElement";
import type { CommandContext } from "../commands/types";
import { uid } from "../utils/id";

const TAG_MAP: Record<string, keyof HTMLElementTagNameMap> = {
  Container: "div", Flex: "div", Grid: "div", Stack: "div", Spacer: "div",
  Text: "span", Heading: "h2", Image: "img", Link: "a", Divider: "hr",
  Button: "button", Input: "input", TextArea: "textarea",
  Checkbox: "input", Select: "select", List: "ul", ListItem: "li",
  Spinner: "div", Progress: "progress", Alert: "div", Card: "div", Form: "form",
};

const VOID_TAGS = new Set(["input", "img", "hr", "br", "progress"]);

// ── OmniView prop → CSS mapping ──────────────────

function omniPropsToStyle(el: ElementData): React.CSSProperties {
  const p = el.props;
  const style: React.CSSProperties = {};

  // Layout direction
  if (p.direction === "row") { style.display = "flex"; style.flexDirection = "row"; }
  else if (p.direction === "column") { style.display = "flex"; style.flexDirection = "column"; }
  else if (el.tag === "Flex" || el.tag === "Container" || el.tag === "Grid") {
    style.display = "flex";
    style.flexDirection = el.tag === "Flex" ? "row" : "column";
  }

  // Gap
  if (p.gap) style.gap = String(p.gap);

  // Padding & Margin
  if (p.padding) style.padding = String(p.padding);
  if (p.margin) style.margin = String(p.margin);

  // Alignment
  if (p.align) style.alignItems = String(p.align);
  if (p.justify) style.justifyContent = String(p.justify);
  if (p.wrap === "wrap") style.flexWrap = "wrap";

  // Dimensions
  if (p.width) style.width = String(p.width);
  if (p.height) style.height = String(p.height);

  // Colors
  if (p.background_color) style.backgroundColor = String(p.background_color);
  if (p.color) style.color = String(p.color);

  // Typography
  if (p.font_size) style.fontSize = String(p.font_size);
  if (p.font_weight) style.fontWeight = String(p.font_weight);

  // Borders & Radius
  if (p.border_radius) style.borderRadius = String(p.border_radius);
  if (p.border) style.border = String(p.border);

  // Shadow
  if (p.shadow === "md") style.boxShadow = "0 4px 6px rgba(0,0,0,0.1)";
  else if (p.shadow) style.boxShadow = "0 2px 4px rgba(0,0,0,0.1)";

  // Container defaults
  if (el.tag === "Container" || el.tag === "Card") {
    if (!p.padding) style.padding = "16px";
    if (el.tag === "Card") {
      style.background = style.background || "#fff";
      style.borderRadius = style.borderRadius || "8px";
      style.boxShadow = style.boxShadow || "0 2px 8px rgba(0,0,0,0.08)";
    }
  }

  // Text defaults
  if (el.tag === "Text") {
    style.fontSize = style.fontSize || "14px";
  }

  // Button defaults
  if (el.tag === "Button") {
    style.padding = style.padding || "8px 16px";
    style.borderRadius = style.borderRadius || "6px";
    style.border = "1px solid #d1d5db";
    style.background = style.background || "#fff";
    style.cursor = "pointer";
    style.fontSize = style.fontSize || "13px";
    if (p.variant === "primary") { style.background = "#4F46E5"; style.color = "#fff"; style.borderColor = "#4F46E5"; }
    else if (p.variant === "danger") { style.background = "#FEE2E2"; style.color = "#DC2626"; style.borderColor = "#FECACA"; }
    else if (p.variant === "ghost") { style.background = "transparent"; style.borderColor = "transparent"; }
  }

  // Input defaults
  if (el.tag === "Input") {
    style.padding = style.padding || "6px 10px";
    style.border = "1px solid #d1d5db";
    style.borderRadius = style.borderRadius || "4px";
    style.fontSize = style.fontSize || "13px";
    style.minWidth = "120px";
  }

  // List defaults
  if (el.tag === "List") {
    style.listStyle = "none";
    style.padding = style.padding || "0";
    style.display = "flex";
    style.flexDirection = "column";
    style.gap = style.gap || "8px";
  }
  if (el.tag === "ListItem") {
    style.padding = style.padding || "4px 0";
  }

  // Heading defaults
  if (el.tag === "Heading") {
    const lvl = Number(p.level) || 1;
    const sizes = { 1: "24px", 2: "20px", 3: "16px", 4: "14px" };
    style.fontSize = style.fontSize || ((sizes as Record<number, string>)[lvl] || "16px");
    style.fontWeight = "600";
    style.margin = style.margin || "0 0 8px 0";
  }

  return style;
}

// ── Main Component ───────────────────────────────

interface LayoutViewProps {
  ctx: CommandContext;
  hideCodeBlocks?: boolean;
}

export function LayoutView({ ctx, hideCodeBlocks = false }: LayoutViewProps) {
  const elements = useEditorStore((s) => s.elements);
  const selectedId = useEditorStore((s) => s.selectedElementId);
  const selectElement = useEditorStore((s) => s.selectElement);
  const undoExec = useUndoStore((s) => s.execute);
  const viewRoots = elements.filter((e) => !e.parentId && e.nodeType !== "codeblock");
  const codeRoots = hideCodeBlocks ? [] : elements.filter((e) => !e.parentId && e.nodeType === "codeblock");
  const hasContent = viewRoots.length > 0 || codeRoots.length > 0;

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    const tag = e.dataTransfer.getData("application/omni-component");
    if (!tag) return;
    // Provide default props for empty-drag elements so they are visible
    const props: Record<string, unknown> = {};
    if (tag === "Text") props.label = "Text";
    else if (tag === "Heading") props.label = "标题";
    else if (tag === "Button") props.label = "按钮";
    else if (tag === "Input") props.placeholder = "输入框";
    const el: ElementData = {
      id: uid(), nodeType: "view", tag,
      props, children: [], parentId: null,
    };
    undoExec(new AddElement(el), ctx);
  }, [undoExec, ctx]);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
  }, []);

  return (
    <div onDrop={handleDrop} onDragOver={handleDragOver} style={{ padding: 16, minHeight: "100%" }}>
      {!hasContent ? (
        <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", color: "#9ca3af", fontSize: 13 }}>
          <div style={{ textAlign: "center" }}>
            <div style={{ fontSize: 32, marginBottom: 8 }}>🎨</div>
            <div>编译 .omni 或在左侧组件面板拖入组件</div>
            <div style={{ fontSize: 11, marginTop: 4, color: "#d1d5db" }}>Ctrl+Enter 编译</div>
          </div>
        </div>
      ) : (
        <>
          {/* View tree */}
          {viewRoots.map((el) => React.createElement(RenderElement, { key: el.id, el, allElements: elements, selectedId, onSelect: selectElement }))}

          {/* Code block nodes — rendered as ported cards */}
          {codeRoots.length > 0 && (
            <div style={{ marginTop: 24, borderTop: "1px dashed #e5e7eb", paddingTop: 16 }}>
              <div style={{ fontSize: 11, fontWeight: 600, color: "#6B7280", marginBottom: 12, textTransform: "uppercase" }}>
                🧩 Code Blocks
              </div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 12 }}>
                {codeRoots.map((el) => React.createElement(CodeBlockCard, { key: el.id, el, allElements: elements, selectedId, onSelect: selectElement }))}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}

// ── Recursive Render Components ───────────────────

interface ElementProps {
  el: ElementData;
  allElements: ElementData[];
  selectedId: string | null;
  onSelect: (id: string | null) => void;
}

function RenderElement({ el, allElements, selectedId, onSelect }: ElementProps): React.ReactElement {
  const htmlTag = TAG_MAP[el.tag] || "div";
  const isSelected = el.id === selectedId;
  const isVoid = VOID_TAGS.has(htmlTag);
  const [hovered, setHovered] = React.useState(false);

  const layoutStyle = omniPropsToStyle(el);

  const outlineStyle: React.CSSProperties = {};
  if (isSelected) {
    outlineStyle.outline = "2px solid #4F46E5";
    outlineStyle.outlineOffset = "2px";
    outlineStyle.zIndex = 1;
  } else if (hovered) {
    outlineStyle.outline = "1px dashed #93C5FD";
    outlineStyle.outlineOffset = "1px";
  }

  const style: React.CSSProperties = {
    ...layoutStyle,
    ...outlineStyle,
    position: "relative",
    boxSizing: "border-box",
  };

  // Text content
  let textLabel = "";
  if (el.props.label && typeof el.props.label === "string" && el.children.length === 0) {
    textLabel = el.props.label;
  }

  // Build children
  const childElements: React.ReactNode[] = [];
  if (textLabel) {
    childElements.push(textLabel);
  }
  if (el.children.length > 0) {
    for (const childId of el.children) {
      const child = allElements.find((e) => e.id === childId);
      if (child) {
        childElements.push(React.createElement(RenderElement, { key: child.id, el: child, allElements, selectedId, onSelect }));
      }
    }
  }

  // Empty placeholder for container-like elements
  if (childElements.length === 0 && !isVoid && el.children.length === 0 && el.tag !== "Text") {
    childElements.push(
      React.createElement("span", {
        key: "__empty",
        style: { color: "#d1d5db", fontSize: 10, fontStyle: "italic" },
      }, `<${el.tag}>`)
    );
  }
  // Make empty Text visible (no label = invisible span otherwise)
  if (el.tag === "Text" && childElements.length === 0 && !isVoid) {
    childElements.push(
      React.createElement("span", {
        key: "__empty",
        style: { color: "#D1D5DB", fontSize: 13 },
      }, "Text")
    );
  }

  const commonProps: Record<string, unknown> = {
    key: el.id,
    "data-omni-id": el.id,
    "data-omni-tag": el.tag,
    style,
    onClick: (e: React.MouseEvent) => {
      // For leaf Text elements inside an interactive parent (Button, Container),
      // let the click bubble up so the parent gets selected instead.
      if (el.tag === "Text" && el.parentId) {
        return; // don't stop propagation; parent handles it
      }
      e.stopPropagation();
      onSelect(el.id);
    },
    onMouseEnter: () => setHovered(true),
    onMouseLeave: () => setHovered(false),
  };

  // Map OmniView event prop names to HTML (for display)
  if (el.props.value && el.tag === "Input") commonProps.placeholder = String(el.props.placeholder || "");

  if (isVoid) {
    if (el.tag === "Input" && el.props.value !== undefined) {
      commonProps.defaultValue = String(el.props.value);
    }
    return React.createElement(htmlTag, commonProps as React.HTMLAttributes<HTMLElement>);
  }

  return React.createElement(
    htmlTag,
    commonProps as React.HTMLAttributes<HTMLElement>,
    ...(childElements.length > 0 ? childElements : []),
  );
}

// ── CodeBlock Card Renderer ───────────────────────

const BLOCK_COLORS: Record<string, { bg: string; border: string; dot: string }> = {
  store:    { bg: "#F5F3FF", border: "#7C3AED", dot: "#7C3AED" },
  signal:   { bg: "#FEF3C7", border: "#D97706", dot: "#D97706" },
  computed: { bg: "#DBEAFE", border: "#2563EB", dot: "#2563EB" },
  action:   { bg: "#FCE7F3", border: "#DB2777", dot: "#DB2777" },
};

const BLOCK_ICONS: Record<string, string> = {
  store: "🗄️", signal: "⚡", computed: "🧮", action: "🔄",
};

function CodeBlockCard({ el, allElements, selectedId, onSelect }: ElementProps): React.ReactElement {
  const blockType = (el.tag || "signal") as string;
  const colors = BLOCK_COLORS[blockType] || BLOCK_COLORS.signal;
  const icon = BLOCK_ICONS[blockType] || "📦";
  const isSelected = el.id === selectedId;
  const [hovered, setHovered] = React.useState(false);

  const name = (el.props.name as string) || el.id;
  const typeAnn = (el.props.typeAnnotation as string) || "";
  const parentStore = (el.props.parentStore as string) || "";

  // Child code blocks (e.g., signals inside a store)
  const childBlocks = allElements.filter(e => e.parentId === el.id && e.nodeType === "codeblock");

  return React.createElement("div", {
    key: el.id,
    "data-omni-id": el.id,
    "data-omni-tag": el.tag,
    style: {
      background: colors.bg,
      border: `1.5px solid ${isSelected ? colors.border : `${colors.border}60`}`,
      borderRadius: 10,
      padding: "10px 14px",
      minWidth: 160,
      maxWidth: 260,
      cursor: "pointer",
      position: "relative",
      boxShadow: hovered || isSelected ? "0 2px 8px rgba(0,0,0,0.08)" : "none",
      transition: "box-shadow 0.15s, border-color 0.15s",
    },
    onClick: (e: React.MouseEvent) => { e.stopPropagation(); onSelect(el.id); },
    onMouseEnter: () => setHovered(true),
    onMouseLeave: () => setHovered(false),
  },
    // Port dot (out)
    React.createElement("div", {
      style: {
        position: "absolute", right: -4, top: "50%", transform: "translateY(-50%)",
        width: 8, height: 8, borderRadius: "50%", background: colors.dot, zIndex: 2,
      },
    }),
    // Port dot (in) - only for non-store blocks
    blockType !== "store" && React.createElement("div", {
      style: {
        position: "absolute", left: -4, top: "50%", transform: "translateY(-50%)",
        width: 8, height: 8, borderRadius: "50%", background: "#10B981", zIndex: 2,
      },
    }),
    // Header
    React.createElement("div", { style: { display: "flex", alignItems: "center", gap: 6, marginBottom: 4 } },
      React.createElement("span", { style: { fontSize: 14 } }, icon),
      React.createElement("span", { style: { fontSize: 11, fontWeight: 700, color: "#1F2937" } }, name),
    ),
    // Type annotation
    typeAnn && React.createElement("div", { style: { fontSize: 10, color: "#6B7280", fontFamily: "monospace", marginBottom: 2 } },
      `: ${typeAnn}`,
    ),
    // Parent store
    parentStore && React.createElement("div", { style: { fontSize: 9, color: "#9CA3AF" } },
      `@ ${parentStore}`,
    ),
    // Child blocks
    childBlocks.length > 0 && React.createElement("div", { style: { marginTop: 8, display: "flex", flexWrap: "wrap", gap: 6 } },
      ...childBlocks.map(cb => React.createElement(CodeBlockCard, { key: cb.id, el: cb, allElements, selectedId, onSelect })),
    ),
  );
}
