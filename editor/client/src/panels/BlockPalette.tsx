/** Block palette — quick-add code block types.
 *  Used in both design mode sidebar and code mode FunctionLibrary panel.
 *  Each type creates a code block with proper ports, nodeCategory, and source template. */

import { useEditorStore } from "../store/editor";
import type { ElementData } from "../core/types";
import { useUndoStore } from "../store/undo";
import { uid } from "../utils/id";
import { getNodeConfig } from "../canvas/nodeRegistry";
import { AddElement } from "../core/commands/AddElement";
import type { CommandContext } from "../core/commands/types";

// ── Source templates ───────────────────────────

const SOURCE_TEMPLATES: Record<string, (name: string) => string> = {
  signal: (name) => `signal ${name}: i32 = 0`,
  computed: (name) => `computed ${name}() -> i32 {
  0
}`,
  store: (name) => `store ${name} {
  // 在此定义 store 内的 signal
}`,
};

// ── Block type definitions ─────────────────────

interface BlockTypeDef {
  tag: string;
  label: string;
  desc: string;
  icon: string;
  color: string;
}

const BLOCK_TYPES: BlockTypeDef[] = [
  { tag: "signal",   label: "Signal",   desc: "响应式状态变量，可读写", icon: "⚡", color: "#D97706" },
  { tag: "computed", label: "Computed", desc: "从其他值派生计算",      icon: "🧮", color: "#2563EB" },
  { tag: "store",    label: "Store",    desc: "数据集合容器",           icon: "🗄️", color: "#7C3AED" },
];

let _seq = 1;

/**
 * Create a code block element with proper ports/nodeCategory from nodeRegistry.
 * Auto-selects the new element so PropertyPanel shows immediately.
 */
function createBlock(tag: string, name?: string): ElementData {
  const id = `cb-${tag}-${Date.now()}-${_seq++}`;
  const label = name || `${tag}_${_seq}`;
  const cfg = getNodeConfig(tag);

  // Clone ports from nodeRegistry, mapping "any" data ports to "i32" for signal/computed
  const ports = cfg.defaultPorts.map((p) => ({
    name: p.name,
    direction: p.direction,
    portType: p.portType,
    type: (p.portType !== "flow" && p.type === "any" && (tag === "signal" || tag === "computed")) ? "i32" : p.type,
    label: p.label,
  }));

  return {
    id,
    nodeType: "codeblock",
    tag,
    props: {
      name: label,
      parentStore: "",
      typeAnnotation: tag === "signal" || tag === "computed" ? "i32" : "",
    },
    children: [],
    parentId: null,
    ports,
    isFolded: false,
    source: SOURCE_TEMPLATES[tag] ? SOURCE_TEMPLATES[tag](label) : "",
    nodeCategory: cfg.category,
  };
}

export { BLOCK_TYPES, createBlock };

// ── Component ─────────────────────────────────

interface BlockPaletteProps {
  /** Called after a block is created (for App.tsx to also select it) */
  onBlockCreated?: (id: string) => void;
  /** Command context for undo support. If omitted, falls back to direct store.addElement. */
  ctx?: CommandContext;
}

export function BlockPalette({ onBlockCreated, ctx }: BlockPaletteProps) {
  const addElement = useEditorStore((s) => s.addElement);
  const undoExec = useUndoStore((s) => s.execute);

  const handleClick = (tag: string) => {
    const el = createBlock(tag);
    if (ctx) {
      // Use Command pattern for undo support
      undoExec(new AddElement(el), ctx);
    } else {
      addElement(el);
    }
    onBlockCreated?.(el.id);
  };

  return (
    <div style={{ padding: "8px 4px", borderTop: "1px solid #E5E7EB" }}>
      <div style={{ padding: "0 4px 6px", fontSize: 10, fontWeight: 600, color: "#6B7280", textTransform: "uppercase", letterSpacing: "0.5px" }}>
        🧩 逻辑节点
      </div>
      {BLOCK_TYPES.map((b) => (
        <div
          key={b.tag}
          onClick={() => handleClick(b.tag)}
          style={{
            padding: "5px 8px", cursor: "pointer", display: "flex", alignItems: "center", gap: 8,
            fontSize: 12, borderRadius: 6, margin: "1px 0",
            transition: "background 0.1s",
          }}
          onMouseEnter={(e) => { e.currentTarget.style.background = "#F3F4F6"; }}
          onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; }}
          title={b.desc}
        >
          <span style={{
            fontSize: 14, width: 22, height: 22, display: "flex", alignItems: "center",
            justifyContent: "center", borderRadius: 4,
            background: `${b.color}18`,
          }}>{b.icon}</span>
          <div style={{ flex: 1, minWidth: 0 }}>
            <span style={{ fontWeight: 500, color: "#1F2937" }}>{b.label}</span>
            <div style={{ fontSize: 10, color: "#9CA3AF", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{b.desc}</div>
          </div>
        </div>
      ))}
    </div>
  );
}
