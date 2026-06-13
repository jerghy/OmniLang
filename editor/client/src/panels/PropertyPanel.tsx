/** 属性面板。根据选中元素和元数据协议动态渲染编辑器。*/

import { useEditorStore } from "../store/editor";
import { useUndoStore } from "../store/undo";
import { SetProp } from "../commands/SetProp";
import { COMPONENT_PROTOCOL } from "./ComponentPalette";

// ── Metadata (inline until M2.3 JSON files) ────────

interface PropMeta {
  type: string;
  default?: unknown;
  values?: string[];
  editor: string;
}

interface EventMeta {
  description: string;
  payload: string | null;
}

interface ComponentMeta {
  component: string;
  category: string;
  props: Record<string, PropMeta>;
  events: Record<string, EventMeta>;
  styleProperties: string[];
  children: boolean;
}

const METADATA: Record<string, ComponentMeta> = {
  Container: { component: "Container", category: "Layout", props: {}, events: {}, styleProperties: ["padding","margin","gap","background_color","border_radius","width","height","layout","align","justify"], children: true },
  Flex: { component: "Flex", category: "Layout", props: { direction: {type:"enum",values:["row","column"],default:"row",editor:"select"}, gap: {type:"string",default:"8px",editor:"text"}, align: {type:"enum",values:["start","center","end","stretch"],default:"start",editor:"select"}, justify: {type:"enum",values:["start","center","end","between","around"],default:"start",editor:"select"}, wrap: {type:"enum",values:["nowrap","wrap"],default:"nowrap",editor:"select"} }, events: {}, styleProperties: ["padding","margin","gap","width","height"], children: true },
  Grid: { component: "Grid", category: "Layout", props: { columns: {type:"string",default:"2",editor:"text"}, gap: {type:"string",default:"16px",editor:"text"} }, events: {}, styleProperties: ["padding","margin","gap","width","height"], children: true },
  Text: { component: "Text", category: "Typography", props: { label: {type:"string",default:"Text",editor:"text"}, "test-id": {type:"string",default:"",editor:"text"} }, events: {}, styleProperties: ["font_size","font_weight","color","text_align","line_height"], children: false },
  Heading: { component: "Heading", category: "Typography", props: { level: {type:"enum",values:["1","2","3","4"],default:"1",editor:"select"}, label: {type:"string",default:"Heading",editor:"text"} }, events: {}, styleProperties: ["font_size","font_weight","color"], children: true },
  Button: { component: "Button", category: "Actions", props: { label: {type:"string",default:"Button",editor:"text"}, variant: {type:"enum",values:["primary","secondary","danger","ghost"],default:"primary",editor:"select"}, disabled: {type:"bool",default:false,editor:"checkbox"}, loading: {type:"bool",default:false,editor:"checkbox"}, "test-id": {type:"string",default:"",editor:"text"} }, events: { onPress: {description:"点击",payload:null}, onHover: {description:"悬停",payload:null}, onFocus: {description:"焦点",payload:null} }, styleProperties: ["width","height","padding","margin","background_color","border_radius","font_size","font_weight"], children: true },
  Input: { component: "Input", category: "Forms", props: { value: {type:"string",default:"",editor:"text"}, placeholder: {type:"string",default:"",editor:"text"}, type: {type:"enum",values:["text","password","email","number"],default:"text",editor:"select"}, disabled: {type:"bool",default:false,editor:"checkbox"} }, events: { onChange: {description:"值变化",payload:"String"}, onFocus: {description:"焦点",payload:null}, onBlur: {description:"失焦",payload:null} }, styleProperties: ["width","padding","border_radius","font_size"], children: false },
  Image: { component: "Image", category: "Media", props: { src: {type:"string",default:"",editor:"text"}, alt: {type:"string",default:"",editor:"text"}, width: {type:"string",default:"",editor:"text"}, height: {type:"string",default:"",editor:"text"} }, events: {}, styleProperties: ["width","height","border_radius"], children: false },
  List: { component: "List", category: "Data", props: {}, events: {}, styleProperties: ["padding","margin","gap"], children: true },
  ListItem: { component: "ListItem", category: "Data", props: { key: {type:"string",default:"",editor:"text"} }, events: {}, styleProperties: ["padding","margin"], children: true },
  Card: { component: "Card", category: "Layout", props: {}, events: {}, styleProperties: ["padding","margin","border_radius","shadow","background_color","width","height"], children: true },
  Form: { component: "Form", category: "Forms", props: {}, events: { onSubmit: {description:"提交",payload:null} }, styleProperties: ["padding","gap","width"], children: true },
  Link: { component: "Link", category: "Navigation", props: { href: {type:"string",default:"#",editor:"text"}, label: {type:"string",default:"Link",editor:"text"} }, events: {}, styleProperties: ["color","font_size","font_weight"], children: false },
  Spinner: { component: "Spinner", category: "Feedback", props: { size: {type:"enum",values:["sm","md","lg"],default:"md",editor:"select"} }, events: {}, styleProperties: ["width","height"], children: false },
  Progress: { component: "Progress", category: "Feedback", props: { value: {type:"string",default:"0",editor:"text"}, max: {type:"string",default:"100",editor:"text"} }, events: {}, styleProperties: ["width"], children: false },
  Alert: { component: "Alert", category: "Feedback", props: { type: {type:"enum",values:["info","success","warning","error"],default:"info",editor:"select"}, label: {type:"string",default:"",editor:"text"} }, events: {}, styleProperties: ["padding","border_radius"], children: true },
  Select: { component: "Select", category: "Forms", props: { value: {type:"string",default:"",editor:"text"}, placeholder: {type:"string",default:"",editor:"text"} }, events: { onChange: {description:"值变化",payload:"String"} }, styleProperties: ["width","padding","border_radius"], children: false },
  TextArea: { component: "TextArea", category: "Forms", props: { value: {type:"string",default:"",editor:"text"}, placeholder: {type:"string",default:"",editor:"text"}, rows: {type:"string",default:"3",editor:"text"} }, events: { onChange: {description:"值变化",payload:"String"} }, styleProperties: ["width","padding","border_radius"], children: false },
};

// ── Main Component ───────────────────────────────

import type { CommandContext } from "../commands/types";

interface PropertyPanelProps {
  ctx: CommandContext;
  onNavigateToCode?: (wireId: string) => void;
}

export function PropertyPanel({ ctx, onNavigateToCode }: PropertyPanelProps) {
  const elements = useEditorStore((s) => s.elements);
  const selectedId = useEditorStore((s) => s.selectedElementId);
  const undoExec = useUndoStore((s) => s.execute);

  const el = elements.find((e) => e.id === selectedId);
  if (!el) {
    return (
      <div style={{ padding: 16, color: "#9ca3af", textAlign: "center" }}>
        <div style={{ fontSize: 24, marginBottom: 8 }}>🔍</div>
        <div>选择画布元素以编辑属性</div>
      </div>
    );
  }

  const meta = METADATA[el.tag];
  if (!meta) {
    return <div style={{ padding: 16 }}><strong>{`<${el.tag}>`}</strong><div style={{color:"#9ca3af",fontSize:12,marginTop:4}}>无元数据</div></div>;
  }

  const handleChange = (prop: string, value: unknown) => {
    undoExec(new SetProp(el.id, prop, value), ctx);
  };

  return (
    <div style={{ padding: 12, overflow: "auto", flex: 1, fontSize: 13 }}>
      <h3 style={{ margin: "0 0 8px", fontSize: 14 }}>
        {`<${el.tag}>`}
        {el.nodeType !== "view" && <span style={{fontSize:10,color:"#6b7280",marginLeft:8}}>{el.nodeType}</span>}
      </h3>

      {/* Props */}
      {Object.keys(meta.props).length > 0 && (
        <>
          <div style={{ fontSize: 11, color: "#6b7280", textTransform: "uppercase", margin: "12px 0 4px" }}>属性</div>
          {Object.entries(meta.props).map(([name, config]) => (
            <PropEditor key={name} name={name} config={config} value={el.props[name] ?? config.default} onChange={(v) => handleChange(name, v)} />
          ))}
        </>
      )}

      {/* Events */}
      {Object.keys(meta.events).length > 0 && (
        <>
          <div style={{ fontSize: 11, color: "#6b7280", textTransform: "uppercase", margin: "12px 0 4px" }}>事件</div>
          {Object.entries(meta.events).map(([name, config]) => (
            <div key={name} style={{ marginBottom: 4 }}>
              <EventEditor name={name} config={config} value={el.props[name]} onChange={(v) => handleChange(name, v)} />
              {onNavigateToCode && (
                <button
                  onClick={() => {
                    const wireId = `wire-${el.id}-${name}-${Date.now()}`;
                    const handlerId = `handler-${el.id}-${name}`;
                    const handlerName = `on${name.replace(/^on/, "").charAt(0).toUpperCase()}${name.replace(/^on/, "").slice(1)}`;

                    // Determine payload type from event metadata
                    const payloadType = config.payload || "any";

                    // Get the UUID for the event port (Phase A — stable wire storage)
                    const eventPortId = COMPONENT_PROTOCOL[el.tag]?.events?.[name]?.id || name;

                    // Create a handler action node with typed input port for the event payload
                    ctx.addElement({
                      id: handlerId,
                      nodeType: "codeblock",
                      tag: "action",
                      props: { name: handlerName, parentStore: "", typeAnnotation: "" },
                      children: [],
                      parentId: null,
                      ports: [
                        { name: "call", direction: "in", type: payloadType, label: config.payload ? `${name}(${config.payload})` : name },
                        { name: "result", direction: "out", type: "any", label: "返回结果" },
                      ],
                      isFolded: false,
                      source: `// 💻 代码块: ${handlerName}
// 接收事件: ${config.payload ? `${name}(payload: ${config.payload})` : `${name}()`}
// 描述: ${config.description}
action ${handlerName}(event) {
    // 在此编写事件处理逻辑
    ${config.payload ? `// 事件数据类型: ${config.payload}` : "// 无事件数据"}
}`,
                    });
                    // Create wire from element event port (UUID) to handler
                    ctx.addWire({
                      id: wireId,
                      fromId: el.id,
                      fromPort: eventPortId,
                      toId: handlerId,
                      toPort: "call",
                    });
                    // Also set the event prop for the .omni export
                    undoExec(new SetProp(el.id, name, handlerName), ctx);
                    // Jump to code mode with this wire highlighted
                    onNavigateToCode(wireId);
                  }}
                  style={{
                    width: "100%", padding: "3px 8px", marginTop: 2,
                    fontSize: 10, fontWeight: 600,
                    background: "#EEF2FF", color: "#4F46E5",
                    border: "1px solid #C7D2FE", borderRadius: 4,
                    cursor: "pointer", textAlign: "center",
                  }}
                  title={`设置 ${name} 事件 → 创建 Action 代码块${config.payload ? ` (接收 ${config.payload} 类型参数)` : ""}`}
                >
                  🔗 连线到代码块 ({name})
                </button>
              )}
            </div>
          ))}
        </>
      )}

      {/* Style */}
      {meta.styleProperties.length > 0 && (
        <>
          <div style={{ fontSize: 11, color: "#6b7280", textTransform: "uppercase", margin: "12px 0 4px" }}>样式</div>
          {meta.styleProperties.map((prop) => (
            <StyleEditor key={prop} name={prop} value={el.props[prop]} onChange={(v) => handleChange(prop, v)} />
          ))}
        </>
      )}

      {/* ID info */}
      <div style={{ marginTop: 16, paddingTop: 8, borderTop: "1px solid #e5e7eb", fontSize: 10, color: "#9ca3af" }}>
        ID: {el.id.slice(0, 8)}...
      </div>
    </div>
  );
}

// ── Sub Editors ───────────────────────────────────

function PropEditor({ name, config, value, onChange }: {
  name: string; config: PropMeta; value: unknown; onChange: (v: unknown) => void;
}) {
  switch (config.editor) {
    case "text":
      return (
        <label style={labelStyle}>
          <span style={labelTextStyle}>{name}</span>
          <input type="text" value={String(value ?? "")} onChange={(e) => onChange(e.target.value)} style={inputStyle} />
        </label>
      );
    case "select":
      return (
        <label style={labelStyle}>
          <span style={labelTextStyle}>{name}</span>
          <select value={String(value ?? config.default)} onChange={(e) => onChange(e.target.value)} style={{...inputStyle, background:"#fff"}}>
            {(config.values || []).map((v) => <option key={v} value={v}>{v}</option>)}
          </select>
        </label>
      );
    case "checkbox":
      return (
        <label style={{ ...labelStyle, flexDirection: "row", alignItems: "center", gap: 8 }}>
          <input type="checkbox" checked={Boolean(value ?? false)} onChange={(e) => onChange(e.target.checked)} />
          <span style={{ fontSize: 12 }}>{name}</span>
        </label>
      );
    default:
      return null;
  }
}

function EventEditor({ name, config, value, onChange }: {
  name: string; config: EventMeta; value: unknown; onChange: (v: unknown) => void;
}) {
  return (
    <label style={labelStyle}>
      <span style={labelTextStyle}>{name} <span style={{color:"#9ca3af",fontSize:10}}>({config.description})</span></span>
      <select value={String(value ?? "")} onChange={(e) => onChange(e.target.value || null)} style={{...inputStyle, background:"#fff"}}>
        <option value="">无</option>
        <option value="__new_fn__">＋ 新建函数...</option>
        <option value="__inline__">＋ 内联函数...</option>
      </select>
    </label>
  );
}

function StyleEditor({ name, value, onChange }: {
  name: string; value: unknown; onChange: (v: unknown) => void;
}) {
  if (name.includes("color")) {
    return (
      <label style={labelStyle}>
        <span style={labelTextStyle}>{name}</span>
        <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
          <input type="color" value={String(value ?? "#000000")} onChange={(e) => onChange(e.target.value)} style={{ width: 28, height: 24, border: "none", cursor: "pointer" }} />
          <input type="text" value={String(value ?? "")} onChange={(e) => onChange(e.target.value)} style={{...inputStyle, flex:1}} />
        </div>
      </label>
    );
  }
  return (
    <label style={labelStyle}>
      <span style={labelTextStyle}>{name}</span>
      <input type="text" value={String(value ?? "")} onChange={(e) => onChange(e.target.value)} style={inputStyle} placeholder="e.g. 16px" />
    </label>
  );
}

// ── Styles ────────────────────────────────────────

const labelStyle: React.CSSProperties = { display: "flex", flexDirection: "column", marginTop: 6 };
const labelTextStyle: React.CSSProperties = { fontSize: 11, color: "#6b7280", marginBottom: 2 };
const inputStyle: React.CSSProperties = { padding: "3px 6px", border: "1px solid #d1d5db", borderRadius: 4, fontSize: 12, width: "100%", boxSizing: "border-box" };
