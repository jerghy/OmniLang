// FlowInspector.tsx
// Right-side panel in CodeFlowCanvas.
// Inline-editing era: syntax nodes (let/assign/store-call/call) and return are edited
// directly on the node in the canvas — this panel is degraded to a live preview
// (single source: formatSyntaxNode) + validation hints. fn param editing is retained.
// Loop 节点（while/for-in）画布静态渲染、props 为权威：面板承担唯一编辑表单。

import { useMemo } from "react";
import { usePoiesisStore } from "../../../store";
import type { CodeblockElementData } from "../../../core/types/element";
import { formatSyntaxNode, type SyntaxNodeProps } from "../../../services/codegen/syntax-nodes";
import { validateSyntaxProps, buildFormulaSource, type FlowScope } from "../../../services/codegen/flow-scope";
import { useFlowScope } from "../../../services/codegen/flow-scope-hook";
import { NodeTextField } from "./fields/NodeField";
import { NodeFormulaField } from "./fields/NodeFormulaField";
import { TAG_CN } from "./flowConstants";

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

// 预览面板共用样式
const HINT_STYLE: React.CSSProperties = {
  fontSize: "var(--ol-font-size-11)",
  color: "var(--ol-text-muted)",
  marginTop: 2,
};

const PRE_STYLE: React.CSSProperties = {
  background: "var(--ol-gray-100)",
  borderRadius: "var(--ol-radius-sm)",
  padding: "6px 8px",
  fontSize: "var(--ol-font-size-12)",
  fontFamily: "var(--ol-font-family-mono)",
  margin: 0,
  overflowX: "auto",
  color: "var(--ol-text)",
};

const SYNTAX_TAGS = ["let", "assign", "store-call", "call"];

interface Props {
  selectedNodeIds: string[];
}

export function FlowInspector({ selectedNodeIds }: Props) {
  const elements = usePoiesisStore((s) => s.elements);
  const setProp = usePoiesisStore((s) => s.setProp);
  const activeTabId = usePoiesisStore((s) => s.designPage.activeTabId);
  const openTabs = usePoiesisStore((s) => s.designPage.openTabs);
  const activeTab = openTabs.find((t) => t.id === activeTabId);
  // Rules of Hooks: unconditional top-level call — must precede the early returns below.
  const scope = useFlowScope(activeTab?.componentName);

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
    return <ReturnPreview el={el} />;
  }

  if (SYNTAX_TAGS.includes(el.tag)) {
    return <SyntaxPreview tag={el.tag} props={el.props ?? {}} scope={scope ?? undefined} />;
  }

  if (el.tag === "while" || el.tag === "for-in") {
    return <LoopForm el={el} setProp={setProp} scope={scope ?? undefined} />;
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
// FN Node Inspector（参数编辑表单，原样保留）
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
// 语法节点预览（内联编辑时代：面板降级为单一来源预览）
// ────────────────────────────────────────────

function SyntaxPreview({ tag, props, scope }: {
  tag: string;
  props: Record<string, unknown>;
  scope?: FlowScope;
}) {
  const code = formatSyntaxNode(tag, props as SyntaxNodeProps);
  const errors = scope ? validateSyntaxProps(tag, props, scope) : [];
  return (
    <div style={S.panel}>
      <div style={S.header}>{TAG_CN[tag] ?? tag}</div>
      <div style={S.field}>
        <div style={S.label}>⚡ 在节点上直接编辑</div>
        <div style={HINT_STYLE}>
          点击节点上的字段直接修改，Enter 或失焦提交
        </div>
      </div>
      <div style={S.field}>
        <div style={S.label}>生成代码（实时预览）</div>
        <pre style={PRE_STYLE}>{code}</pre>
      </div>
      {errors.length > 0 && (
        <div style={S.field}>
          <div style={{ ...S.label, color: "var(--ol-color-danger)" }}>⚠ 校验提示</div>
          {errors.map((e) => (
            <div key={e.field} style={{ fontSize: "var(--ol-font-size-11)", color: "var(--ol-color-danger)", marginTop: 2 }}>
              {e.message}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ────────────────────────────────────────────
// Loop 节点表单（while / for-in）
// ────────────────────────────────────────────
// 与 SyntaxNode 内联编辑相反的契约：loop 画布静态渲染（props 权威、双击豁免），
// 面板是唯一编辑面——while 条件 / for-in 变量+遍历 走字段提交（onCommit → setProp），
// 底部仍给 formatSyntaxNode 实时预览（单一来源）。

function LoopForm({ el, setProp, scope }: {
  el: CodeblockElementData;
  setProp: (id: string, prop: string, value: unknown) => void;
  scope?: FlowScope;
}) {
  const props = (el.props ?? {}) as SyntaxNodeProps;
  const code = formatSyntaxNode(el.tag, props);
  // 公式框候选源：scope 全量候选（面板无 wire 拓扑，不含 letVars；无 scope 退化纯文本）
  const formulaSource = useMemo(() => buildFormulaSource(scope, []), [scope]);
  const set = (prop: string, value: unknown) => setProp(el.id, prop, value);
  return (
    <div style={S.panel}>
      <div style={S.header}>{TAG_CN[el.tag] ?? el.tag}</div>
      {el.tag === "while" ? (
        <div style={S.field}>
          <div style={S.label}>条件</div>
          <NodeFormulaField
            value={props.condition ?? ""}
            candidates={formulaSource}
            onCommit={(v) => set("condition", v)}
            placeholder="循环条件"
            ariaLabel="循环条件"
          />
        </div>
      ) : (
        <>
          <div style={S.field}>
            <div style={S.label}>循环变量</div>
            <NodeTextField
              value={props.varName ?? ""}
              onCommit={(v) => set("varName", v)}
              placeholder="循环变量"
              ariaLabel="循环变量"
            />
          </div>
          <div style={S.field}>
            <div style={S.label}>遍历表达式</div>
            <NodeFormulaField
              value={props.iterable ?? ""}
              candidates={formulaSource}
              onCommit={(v) => set("iterable", v)}
              placeholder="遍历表达式"
              ariaLabel="遍历表达式"
            />
          </div>
        </>
      )}
      <div style={S.field}>
        <div style={S.label}>生成代码（实时预览）</div>
        <pre style={PRE_STYLE}>{code}</pre>
      </div>
    </div>
  );
}

// ────────────────────────────────────────────
// Return 节点预览（表达式在节点上内联编辑，面板显示 source 预览）
// ────────────────────────────────────────────

function ReturnPreview({ el }: { el: CodeblockElementData }) {
  return (
    <div style={S.panel}>
      <div style={S.header}>← 返回</div>
      <div style={S.field}>
        <div style={S.label}>⚡ 在节点上直接编辑</div>
        <div style={HINT_STYLE}>
          点击节点上的字段直接修改，Enter 或失焦提交
        </div>
      </div>
      <div style={S.field}>
        <div style={S.label}>生成代码（实时预览）</div>
        <pre style={PRE_STYLE}>{el.source}</pre>
      </div>
    </div>
  );
}
