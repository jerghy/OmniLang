// FlowInspector.tsx
// Right-side panel in CodeFlowCanvas that edits selected node properties:
// fn / return / let / assign / store-call / call.
// Reusable across store action editing and component fn editing.

import { usePoiesisStore } from "../../../store";
import type { CodeblockElementData } from "../../../core/types/element";
import { ASSIGN_OPERATORS } from "../../../services/codegen/syntax-nodes";

const OMNI_TYPES = [
  "String", "i32", "i64", "f64", "f32",
  "bool", "Array<String>", "Array<i32>", "Array<f64>",
  "Map<String, String>",
];

const S = {
  panel: {
    width: 240,
    minWidth: 200,
    height: "100%",
    borderLeft: "1px solid var(--ol-border)",
    background: "var(--ol-gray-50)",
    overflow: "auto",
    fontSize: "var(--ol-font-size-12)",
  } as React.CSSProperties,

  header: {
    padding: "8px 12px",
    fontSize: "var(--ol-font-size-11)",
    fontWeight: "var(--ol-font-weight-600)",
    color: "var(--ol-text-secondary)",
    textTransform: "uppercase",
    letterSpacing: "0.05em",
    borderBottom: "1px solid var(--ol-border-subtle)",
  } as React.CSSProperties,

  field: {
    padding: "6px 12px",
    borderBottom: "1px solid var(--ol-border-subtle)",
  } as React.CSSProperties,

  label: {
    fontSize: "var(--ol-font-size-11)",
    color: "var(--ol-text-muted)",
    marginBottom: 3,
  } as React.CSSProperties,

  input: {
    width: "100%",
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    padding: "3px 6px",
    fontSize: "var(--ol-font-size-12)",
    fontFamily: "var(--ol-font-family-mono)",
    background: "var(--ol-surface)",
    color: "var(--ol-text)",
    outline: "2px solid transparent",
    boxSizing: "border-box" as const,
  } as React.CSSProperties,

  select: {
    width: "100%",
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    padding: "3px 4px",
    fontSize: "var(--ol-font-size-12)",
    fontFamily: "var(--ol-font-family-mono)",
    background: "var(--ol-surface)",
    color: "var(--ol-text)",
    outline: "2px solid transparent",
    cursor: "pointer",
    boxSizing: "border-box" as const,
  } as React.CSSProperties,

  btn: {
    padding: "2px 8px",
    fontSize: "var(--ol-font-size-11)",
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    background: "var(--ol-surface)",
    color: "var(--ol-text-secondary)",
    cursor: "pointer",
  } as React.CSSProperties,

  removeBtn: {
    border: "none",
    background: "transparent",
    color: "var(--ol-text-muted)",
    cursor: "pointer",
    fontSize: 14,
    padding: "0 2px",
  } as React.CSSProperties,
};

interface Props {
  selectedNodeIds: string[];
}

export function FlowInspector({ selectedNodeIds }: Props) {
  const elements = usePoiesisStore((s) => s.elements);
  const setProp = usePoiesisStore((s) => s.setProp);
  const setElementSource = usePoiesisStore((s) => s.setElementSource);
  const storeReferences = usePoiesisStore((s) => s.storeReferences);

  if (selectedNodeIds.length !== 1) {
    return (
      <div style={S.panel}>
        <div style={{ padding: 16, color: "var(--ol-text-muted)", textAlign: "center" }}>
          {selectedNodeIds.length === 0
            ? "Select a node to edit"
            : "Select one node at a time"}
        </div>
      </div>
    );
  }

  const el = elements.find((e) => e.id === selectedNodeIds[0] && e.nodeType === "codeblock") as CodeblockElementData | undefined;
  if (!el) {
    return (
      <div style={S.panel}>
        <div style={{ padding: 16, color: "var(--ol-text-muted)", textAlign: "center" }}>
          Node not found
        </div>
      </div>
    );
  }

  if (el.tag === "fn") {
    return <FnInspector el={el} setProp={setProp} />;
  }

  if (el.tag === "return") {
    return <ReturnInspector el={el} setElementSource={setElementSource} />;
  }

  if (el.tag === "let") {
    return <LetInspector el={el} setProp={setProp} />;
  }

  if (el.tag === "assign") {
    return <AssignInspector el={el} setProp={setProp} />;
  }

  if (el.tag === "store-call") {
    return <StoreCallInspector el={el} setProp={setProp} storeReferences={storeReferences} />;
  }

  if (el.tag === "call") {
    return <CallInspector el={el} setProp={setProp} />;
  }

  return (
    <div style={S.panel}>
      <div style={{ padding: 16, color: "var(--ol-text-muted)", textAlign: "center" }}>
        No properties for "{el.tag}"
      </div>
    </div>
  );
}

// ────────────────────────────────────────────
// FN Node Inspector
// ────────────────────────────────────────────

interface FnInspectorProps {
  el: CodeblockElementData;
  setProp: (id: string, prop: string, value: unknown) => void;
}

function FnInspector({ el, setProp }: FnInspectorProps) {
  const fnName = (el.props?.name as string) || "";
  const paramsStr = (el.props?.params as string) || "";
  const parsedParams: Array<{ name: string; type: string }> = paramsStr
    ? paramsStr.split(",").map((p) => {
        const [pName, pType] = p.trim().split(":").map((s) => s.trim());
        return { name: pName || "", type: pType || "String" };
      })
    : [];

  const handleNameChange = (name: string) => {
    setProp(el.id, "name", name);
  };

  const handleParamChange = (index: number, field: "name" | "type", value: string) => {
    const newParams = [...parsedParams];
    newParams[index] = { ...newParams[index], [field]: value };
    const newParamsStr = newParams.map((p) => `${p.name}: ${p.type}`).join(", ");
    setProp(el.id, "params", newParamsStr);
  };

  const handleAddParam = () => {
    const newParams = [...parsedParams, { name: "param", type: "String" }];
    const newParamsStr = newParams.map((p) => `${p.name}: ${p.type}`).join(", ");
    setProp(el.id, "params", newParamsStr);
  };

  const handleRemoveParam = (index: number) => {
    const newParams = parsedParams.filter((_, i) => i !== index);
    const newParamsStr = newParams.map((p) => `${p.name}: ${p.type}`).join(", ");
    setProp(el.id, "params", newParamsStr);
  };

  return (
    <div style={S.panel}>
      <div style={S.header}>ƒ Function</div>

      {/* Name */}
      <div style={S.field}>
        <div style={S.label}>Name</div>
        <input aria-label="Function name" style={S.input} value={fnName}
          onChange={(e) => handleNameChange(e.target.value)} />
      </div>

      {/* Parameters */}
      <div style={S.field}>
        <div style={{ ...S.label, display: "flex", justifyContent: "space-between" }}>
          <span>Parameters</span>
          <button type="button" style={S.btn} onClick={handleAddParam}>+ Add</button>
        </div>
        {parsedParams.map((p, i) => (
          <div key={p.name + "-" + p.type} style={{ display: "flex", gap: 4, marginBottom: 4, alignItems: "center" }}>
            <input
              aria-label="Parameter name"
              style={{ ...S.input, width: 72 }}
              value={p.name}
              placeholder="name"
              onChange={(e) => handleParamChange(i, "name", e.target.value)}
            />
            <span style={{ color: "var(--ol-text-muted)", fontSize: 12 }}>:</span>
            <select
              aria-label="Parameter type"
              style={{ ...S.select, flex: 1 }}
              value={p.type}
              onChange={(e) => handleParamChange(i, "type", e.target.value)}
            >
              {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
              {!OMNI_TYPES.includes(p.type) && <option value={p.type}>{p.type}</option>}
            </select>
            <button type="button" style={S.removeBtn} onClick={() => handleRemoveParam(i)}>×</button>
          </div>
        ))}
        {parsedParams.length === 0 && (
          <div style={{ color: "var(--ol-text-muted)", fontSize: "var(--ol-font-size-11)" }}>
            No parameters
          </div>
        )}
      </div>
    </div>
  );
}

// ────────────────────────────────────────────
// Return Node Inspector
// ────────────────────────────────────────────

interface ReturnInspectorProps {
  el: CodeblockElementData;
  setElementSource: (id: string, source: string) => void;
}

function ReturnInspector({ el, setElementSource }: ReturnInspectorProps) {
  const source = el.source || "";

  return (
    <div style={S.panel}>
      <div style={S.header}>← Return</div>
      <div style={S.field}>
        <div style={S.label}>Expression</div>
        <input
          style={S.input}
          value={source}
          placeholder="e.g. items.len()"
          aria-label="Expression"
          onChange={(e) => setElementSource(el.id, e.target.value)}
        />
      </div>
    </div>
  );
}

// ────────────────────────────────────────────
// Syntax node inspectors (let / assign / store-call / call)
// ────────────────────────────────────────────

const SYNTAX_FIELD: React.CSSProperties = { ...S.field };
const SYNTAX_LABEL: React.CSSProperties = { ...S.label };
const SYNTAX_INPUT: React.CSSProperties = { ...S.input };
const SYNTAX_SELECT: React.CSSProperties = { ...S.select };

function LetInspector({ el, setProp }: { el: CodeblockElementData; setProp: FnInspectorProps["setProp"] }) {
  const name = (el.props?.name as string) || "";
  const mutable = !!el.props?.mutable;
  const varType = (el.props?.varType as string) || "";
  const value = (el.props?.value as string) || "";
  return (
    <div style={S.panel}>
      <div style={S.header}>let 变量声明</div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>变量名</div>
        <input aria-label="变量名" style={SYNTAX_INPUT} value={name}
          onChange={(e) => setProp(el.id, "name", e.target.value)} />
      </div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>类型（可选）</div>
        <select aria-label="类型" style={SYNTAX_SELECT} value={varType}
          onChange={(e) => setProp(el.id, "varType", e.target.value || undefined)}>
          <option value="">（推断）</option>
          {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
          {varType && !OMNI_TYPES.includes(varType) && <option value={varType}>{varType}</option>}
        </select>
      </div>
      <div style={SYNTAX_FIELD}>
        <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--ol-font-size-12)" }}>
          <input type="checkbox" aria-label="可变 (mut)" checked={mutable}
            onChange={(e) => setProp(el.id, "mutable", e.target.checked)} />
          可变 (mut)
        </label>
      </div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>初始值</div>
        <input aria-label="初始值" style={SYNTAX_INPUT} value={value}
          placeholder="e.g. 0 / [] / items.filter(...)"
          onChange={(e) => setProp(el.id, "value", e.target.value)} />
      </div>
    </div>
  );
}

function AssignInspector({ el, setProp }: { el: CodeblockElementData; setProp: FnInspectorProps["setProp"] }) {
  const target = (el.props?.target as string) || "";
  const operator = (el.props?.operator as string) || "=";
  const value = (el.props?.value as string) || "";
  return (
    <div style={S.panel}>
      <div style={S.header}>assign 赋值</div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>目标（signal/变量/store 信号）</div>
        <input aria-label="目标" style={SYNTAX_INPUT} value={target}
          placeholder="e.g. count / ThemeStore.mode"
          onChange={(e) => setProp(el.id, "target", e.target.value)} />
      </div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>运算符</div>
        <select aria-label="运算符" style={SYNTAX_SELECT} value={operator}
          onChange={(e) => setProp(el.id, "operator", e.target.value)}>
          {ASSIGN_OPERATORS.map((op) => <option key={op} value={op}>{op}</option>)}
        </select>
      </div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>表达式</div>
        <input aria-label="表达式" style={SYNTAX_INPUT} value={value}
          placeholder="e.g. count + step"
          onChange={(e) => setProp(el.id, "value", e.target.value)} />
      </div>
    </div>
  );
}

function StoreCallInspector({ el, setProp, storeReferences }: {
  el: CodeblockElementData;
  setProp: FnInspectorProps["setProp"];
  storeReferences: Record<string, string[]>;
}) {
  const storeName = (el.props?.storeName as string) || "";
  const action = (el.props?.action as string) || "";
  const args = (el.props?.args as string) || "";
  const storeNames = Object.keys(storeReferences);
  return (
    <div style={S.panel}>
      <div style={S.header}>store 调用</div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>store 名</div>
        <input aria-label="store 名" list="syntax-store-names" style={SYNTAX_INPUT} value={storeName}
          placeholder="e.g. TodoStore"
          onChange={(e) => setProp(el.id, "storeName", e.target.value)} />
        <datalist id="syntax-store-names">
          {storeNames.map((n) => <option key={n} value={n} />)}
        </datalist>
      </div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>action</div>
        <input aria-label="action" style={SYNTAX_INPUT} value={action}
          placeholder="e.g. add"
          onChange={(e) => setProp(el.id, "action", e.target.value)} />
      </div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>参数（逗号分隔）</div>
        <input aria-label="参数" style={SYNTAX_INPUT} value={args}
          placeholder="e.g. text, 2"
          onChange={(e) => setProp(el.id, "args", e.target.value)} />
      </div>
    </div>
  );
}

function CallInspector({ el, setProp }: { el: CodeblockElementData; setProp: FnInspectorProps["setProp"] }) {
  const fnName = (el.props?.fnName as string) || "";
  const args = (el.props?.args as string) || "";
  const emit = !!el.props?.emit;
  return (
    <div style={S.panel}>
      <div style={S.header}>call 调用</div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>函数名</div>
        <input aria-label="函数名" style={SYNTAX_INPUT} value={fnName}
          placeholder="e.g. close / onSelect / console_log"
          onChange={(e) => setProp(el.id, "fnName", e.target.value)} />
      </div>
      <div style={SYNTAX_FIELD}>
        <div style={SYNTAX_LABEL}>参数（逗号分隔）</div>
        <input aria-label="参数" style={SYNTAX_INPUT} value={args}
          placeholder="e.g. item"
          onChange={(e) => setProp(el.id, "args", e.target.value)} />
      </div>
      <div style={SYNTAX_FIELD}>
        <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: "var(--ol-font-size-12)" }}>
          <input type="checkbox" aria-label="emit 事件" checked={emit}
            onChange={(e) => setProp(el.id, "emit", e.target.checked)} />
          emit 事件
        </label>
      </div>
    </div>
  );
}
