/** Block palette — unified single code block for the wire canvas.
 *  State management (signal/computed/store) moved to StatePanel.
 *  Function definitions moved to FunctionLibrary.
 *  Only the 💻 logic code block remains here. */

import { useEditorStore, type ElementData } from "../store/editor";

const DEFAULT_SOURCE = `// 💻 逻辑代码块
// 输入端口从参数自动生成，输出端口从返回值生成

action handleEvent(event) {
    // 在此编写事件处理逻辑

}`;

const BLOCK_TYPES = [
  { tag: "action", label: "代码块", desc: "编写逻辑代码，处理事件和数据流", icon: "💻", ports: [
    { name: "call", direction: "in" as const, type: "any", label: "调用入口" },
    { name: "result", direction: "out" as const, type: "any", label: "返回结果" },
  ]},
];

let _seq = 1;

function createBlock(tag: string): ElementData {
  const id = `cb-${tag}-${Date.now()}-${_seq++}`;
  const def = BLOCK_TYPES.find(b => b.tag === tag);
  return {
    id, nodeType: "codeblock" as const, tag,
    props: { name: "newBlock" },
    children: [], parentId: null,
    ports: (def?.ports || []).map(p => ({ name: p.name, direction: p.direction, type: p.type, label: p.label })),
    isFolded: false,
    source: DEFAULT_SOURCE,
  };
}

export { BLOCK_TYPES, createBlock };

export function BlockPalette() {
  const addElement = useEditorStore(s => s.addElement);

  return (
    <div style={{ padding: "8px 0", borderTop: "1px solid #E5E7EB" }}>
      <div style={{ padding: "0 8px 6px", fontSize: 10, fontWeight: 600, color: "#9CA3AF", textTransform: "uppercase" }}>
        💻 代码块
      </div>
      {BLOCK_TYPES.map(b => (
        <div
          key={b.tag}
          onClick={() => addElement(createBlock(b.tag))}
          style={{
            padding: "6px 8px", cursor: "pointer", display: "flex", alignItems: "center", gap: 8,
            fontSize: 12, borderRadius: 0, margin: "0 4px",
          }}
          onMouseEnter={e => { e.currentTarget.style.background = "#F3F4F6"; }}
          onMouseLeave={e => { e.currentTarget.style.background = "transparent"; }}
          title={`${b.desc}\n输入: ${b.ports.filter(p => p.direction === "in").map(p => p.label).join(", ") || "无"}\n输出: ${b.ports.filter(p => p.direction === "out").map(p => p.label).join(", ") || "无"}`}
        >
          <span style={{ fontSize: 14 }}>{b.icon}</span>
          <div>
            <span style={{ fontWeight: 500, color: "#374151" }}>{b.label}</span>
            <div style={{ fontSize: 10, color: "#9CA3AF" }}>{b.desc}</div>
          </div>
        </div>
      ))}
    </div>
  );
}
