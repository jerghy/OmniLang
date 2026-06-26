/** Code-mode property panel — shows node configuration for the selected code graph node.
 *  Only renders when activeMode === "code". */

import React, { useEffect, useState } from "react";
import { useCodeGraphStore } from "../codegraph/store";
import { getNodeConfig, getAllNodeConfigs } from "../ui/canvas/shared/nodeRegistry";
import { createCodeGraphContext } from "../codegraph/context";
import { AddElement } from "../core/commands/AddElement";
import type { ElementData } from "../core/types";
import type { FnParam, SignalDefinition } from "../codegraph/types";

// ═══════════════════════════════════════════════════
// CodePropertyPanel — main component
// ═══════════════════════════════════════════════════

export function CodePropertyPanel() {
  const tabs = useCodeGraphStore((s) => s.tabs);
  const activeTabId = useCodeGraphStore((s) => s.activeTabId);
  const selectedIds = useCodeGraphStore((s) => s.selectedElementIds);
  const setProp = useCodeGraphStore((s) => s.setProp);

  const activeTab = tabs.find((t) => t.id === activeTabId) || tabs[0];
  const elements = activeTab?.graph.elements || [];

  const selectedEl = selectedIds.length === 1
    ? elements.find((e) => e.id === selectedIds[0])
    : null;

  if (!selectedEl) {
    return (
      <div style={{ padding: 12, fontSize: 11, color: "#9CA3AF" }}>
        点击节点查看属性
      </div>
    );
  }

  return (
    <div style={{ padding: 12, fontSize: 12, overflow: "auto" }}>
      <NodeEditor element={selectedEl} onUpdate={(prop, value) => {
        setProp(selectedEl.id, prop, value);
      }} />
    </div>
  );
}

// ═══════════════════════════════════════════════════
// NodeEditor — renders the appropriate editor by tag
// ═══════════════════════════════════════════════════

function NodeEditor({ element, onUpdate }: {
  element: ElementData;
  onUpdate: (prop: string, value: unknown) => void;
}) {
  const config = getNodeConfig(element.tag);

  return (
    <div>
      {/* Header */}
      <div style={{
        fontSize: 13, fontWeight: 600, marginBottom: 8,
        display: "flex", alignItems: "center", gap: 6,
      }}>
        <span>{config.icon}</span>
        <span>{config.label}</span>
      </div>
      <div style={{ fontSize: 10, color: "#9CA3AF", marginBottom: 12 }}>
        {config.description}
      </div>

      {/* Tag-specific editors */}
      <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
        {config.panelEditor === "literal" && <LiteralEditor element={element} onUpdate={onUpdate} />}
        {config.panelEditor === "signal-selector" && <SignalSelector element={element} onUpdate={onUpdate} />}
        {config.panelEditor === "element-selector" && <ElementSelector element={element} onUpdate={onUpdate} />}
        {config.panelEditor === "function-signature" && <FunctionSignatureEditor element={element} onUpdate={onUpdate} />}
        {config.panelEditor === "return-validator" && <ReturnValidator element={element} />}
        {config.panelEditor === "compare" && <CompareEditor element={element} onUpdate={onUpdate} />}
        {config.panelEditor === "variable" && <VariableEditor element={element} onUpdate={onUpdate} />}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════
// Editor sub-components
// ═══════════════════════════════════════════════════

function LiteralEditor({ element, onUpdate }: {
  element: ElementData;
  onUpdate: (prop: string, value: unknown) => void;
}) {
  const config = element.config || {};
  const value = config.value as string ?? "0";
  const type = config.type as string ?? "i32";

  return (
    <>
      <FieldLabel label="类型" />
      <select
        value={type}
        onChange={(e) => onUpdate("config", { ...config, type: e.target.value })}
        style={selectStyle}
      >
        {["i32", "f64", "bool", "String", "any"].map((t) => (
          <option key={t} value={t}>{t}</option>
        ))}
      </select>

      <FieldLabel label="值" />
      {type === "bool" ? (
        <select
          value={value}
          onChange={(e) => onUpdate("config", { ...config, value: e.target.value })}
          style={selectStyle}
        >
          <option value="true">true</option>
          <option value="false">false</option>
        </select>
      ) : (
        <input
          type={type === "i32" || type === "f64" ? "number" : "text"}
          value={value}
          onChange={(e) => onUpdate("config", { ...config, value: e.target.value })}
          style={inputStyle}
        />
      )}
    </>
  );
}

function SignalSelector({ element, onUpdate }: {
  element: ElementData;
  onUpdate: (prop: string, value: unknown) => void;
}) {
  const config = element.config || {};
  const targetSignal = config.targetSignal as string ?? "";
  const tabs = useCodeGraphStore((s) => s.tabs);
  const activeTabId = useCodeGraphStore((s) => s.activeTabId);
  const activeTab = tabs.find((t) => t.id === activeTabId) || tabs[0];

  const signals = activeTab.graph.elements
    .filter((e) => e.tag === "signal" || e.tag === "computed" || e.tag === "store")
    .map((e) => (e.props.name as string) || e.id);

  return (
    <>
      <FieldLabel label="目标变量" />
      <select
        value={targetSignal}
        onChange={(e) => onUpdate("config", { ...config, targetSignal: e.target.value })}
        style={selectStyle}
      >
        <option value="">-- 选择信号 --</option>
        {signals.map((s) => (
          <option key={s} value={s}>{s}</option>
        ))}
      </select>
      {targetSignal && (
        <div style={{ fontSize: 10, color: "#059669", marginTop: 2 }}>
          → {targetSignal}
        </div>
      )}
    </>
  );
}

function ElementSelector({ element, onUpdate }: {
  element: ElementData;
  onUpdate: (prop: string, value: unknown) => void;
}) {
  const config = element.config || {};
  const targetId = config.targetId as string ?? "";
  const property = config.property as string ?? "label";

  return (
    <>
      <FieldLabel label="目标元素 ID" />
      <input
        value={targetId}
        onChange={(e) => onUpdate("config", { ...config, targetId: e.target.value })}
        placeholder="输入元素 ID"
        style={inputStyle}
      />

      <FieldLabel label="属性" />
      <select
        value={property}
        onChange={(e) => onUpdate("config", { ...config, property: e.target.value })}
        style={selectStyle}
      >
        {["label", "value", "text", "src", "href"].map((p) => (
          <option key={p} value={p}>{p}</option>
        ))}
      </select>
    </>
  );
}

function FunctionSignatureEditor({ element, onUpdate }: {
  element: ElementData;
  onUpdate: (prop: string, value: unknown) => void;
}) {
  const config = element.config || {};
  const fnName = config.fnName as string ?? "";
  const fnParams = (config.fnParams as FnParam[]) || [];
  const fnReturnType = config.fnReturnType as string ?? "void";
  const description = config.description as string ?? "";

  const [newParamName, setNewParamName] = useState("");
  const [newParamType, setNewParamType] = useState("String");

  const addParam = () => {
    const name = newParamName.trim();
    if (!name) return;
    const updatedParams = [...fnParams, { name, type: newParamType }];
    onUpdate("config", { ...config, fnParams: updatedParams });
    setNewParamName("");
  };

  const removeParam = (index: number) => {
    const updatedParams = fnParams.filter((_, i) => i !== index);
    onUpdate("config", { ...config, fnParams: updatedParams });
  };

  return (
    <>
      <FieldLabel label="函数名" />
      <input
        value={fnName}
        onChange={(e) => onUpdate("config", { ...config, fnName: e.target.value })}
        placeholder="如 validateForm"
        style={inputStyle}
      />

      <FieldLabel label="返回类型" />
      <select
        value={fnReturnType}
        onChange={(e) => onUpdate("config", { ...config, fnReturnType: e.target.value })}
        style={selectStyle}
      >
        {["void", "String", "i32", "f64", "bool", "any"].map((t) => (
          <option key={t} value={t}>{t}</option>
        ))}
      </select>

      <FieldLabel label="描述" />
      <input
        value={description}
        onChange={(e) => onUpdate("config", { ...config, description: e.target.value })}
        placeholder="函数功能描述"
        style={inputStyle}
      />

      <FieldLabel label="参数" />
      {fnParams.map((p, i) => (
        <div key={i} style={{
          display: "flex", gap: 4, alignItems: "center",
          padding: "3px 8px", background: "#F3F4F6", borderRadius: 4,
          fontSize: 11, marginBottom: 3,
        }}>
          <span style={{ fontFamily: "monospace", flex: 1 }}>{p.name}: {p.type}</span>
          <button
            onClick={() => removeParam(i)}
            style={{ border: "none", background: "transparent", cursor: "pointer", color: "#DC2626", fontSize: 12 }}
          >
            ✕
          </button>
        </div>
      ))}
      <div style={{ display: "flex", gap: 4 }}>
        <input
          placeholder="参数名"
          value={newParamName}
          onChange={(e) => setNewParamName(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") addParam(); }}
          style={{ flex: 1, ...inputStyle }}
        />
        <select value={newParamType} onChange={(e) => setNewParamType(e.target.value)} style={{ ...selectStyle, width: 80 }}>
          {["String", "i32", "f64", "bool", "any"].map((t) => (
            <option key={t} value={t}>{t}</option>
          ))}
        </select>
        <button
          onClick={addParam}
          style={{
            padding: "3px 8px", fontSize: 11, border: "1px dashed #D1D5DB",
            borderRadius: 4, background: "#fff", cursor: "pointer", color: "#6B7280",
          }}
        >
          +
        </button>
      </div>
    </>
  );
}

function ReturnValidator({ element }: { element: ElementData }) {
  const tabs = useCodeGraphStore((s) => s.tabs);
  const activeTabId = useCodeGraphStore((s) => s.activeTabId);
  const activeTab = tabs.find((t) => t.id === activeTabId) || tabs[0];
  const elements = activeTab.graph.elements;
  const wires = activeTab.graph.wires;

  // ── Find the parent customFn for this return node ──
  // Strategy: walk backward from the return node through the data-flow DAG
  // until we find a customFn node. Uses BFS over incoming wires.
  function findParentCustomFn(returnElId: string): ElementData | null {
    const visited = new Set<string>();
    const queue = [returnElId];
    while (queue.length > 0) {
      const current = queue.shift()!;
      if (visited.has(current)) continue;
      visited.add(current);
      const el = elements.find((e) => e.id === current);
      if (el && el.tag === "customFn" && current !== returnElId) return el;
      // Walk backward: find nodes that feed into `current`
      for (const w of wires) {
        if (w.toId === current && !visited.has(w.fromId)) {
          queue.push(w.fromId);
        }
      }
    }
    return null;
  }

  const parentFn = findParentCustomFn(element.id);
  const fnReturnType = (parentFn?.config?.fnReturnType as string) || "void";

  // ── Find all return nodes in the same function body ──
  // Forward BFS from the customFn to discover all reachable nodes,
  // then filter for "return" nodes.
  function findFunctionReturnNodes(fnId: string): ElementData[] {
    const reachable = new Set<string>();
    const queue = [fnId];
    while (queue.length > 0) {
      const current = queue.shift()!;
      if (reachable.has(current)) continue;
      reachable.add(current);
      for (const w of wires) {
        if (w.fromId === current && !reachable.has(w.toId)) {
          queue.push(w.toId);
        }
      }
    }
    return elements.filter((e) => e.tag === "return" && reachable.has(e.id));
  }

  const siblingReturns = parentFn ? findFunctionReturnNodes(parentFn.id) : [element];

  // ── Type consistency check ──
  // For each return node, check the source type of the wire feeding its "value" port.
  const returnTypeResults: Array<{
    returnId: string;
    sourceType: string | null;
    isConsistent: boolean;
  }> = [];

  for (const ret of siblingReturns) {
    const valueWire = wires.find((w) => w.toId === ret.id && w.toPort === "value");
    if (!valueWire) {
      returnTypeResults.push({ returnId: ret.id, sourceType: null, isConsistent: true });
      continue;
    }
    const sourceEl = elements.find((e) => e.id === valueWire.fromId);
    const sourcePort = sourceEl?.ports?.find((p) => p.name === valueWire.fromPort);
    const sourceType = sourcePort?.type || "any";
    const isConsistent = fnReturnType === "void" || fnReturnType === "any"
      || sourceType === "any"
      || sourceType === fnReturnType
      || (fnReturnType === "i32" && sourceType === "f64")
      || (fnReturnType === "f64" && sourceType === "i32");
    returnTypeResults.push({ returnId: ret.id, sourceType, isConsistent });
  }

  const allConsistent = returnTypeResults.every((r) => r.isConsistent);
  const unconnectedCount = returnTypeResults.filter((r) => r.sourceType === null).length;

  return (
    <div style={{ fontSize: 11 }}>
      <div style={{ color: "#6B7280", marginBottom: 4 }}>
        ↩ 返回节点 — 标记函数返回值
      </div>

      {parentFn ? (
        <div style={{ marginBottom: 8 }}>
          <div style={{ color: "#059669", marginBottom: 2 }}>
            父函数: {(parentFn.config?.fnName as string) || "unnamed"}
          </div>
          <div style={{ color: "#6B7280", fontSize: 10 }}>
            声明返回类型: <span style={{ fontFamily: "monospace", fontWeight: 600 }}>{fnReturnType}</span>
          </div>
        </div>
      ) : (
        <div style={{ color: "#D97706", marginBottom: 8 }}>
          ⚠ 未连接到自定义函数节点
        </div>
      )}

      {/* Return nodes summary */}
      {siblingReturns.length > 0 && (
        <div style={{
          background: "#F9FAFB", borderRadius: 6, padding: 8,
          border: `1px solid ${allConsistent ? "#D1FADF" : "#FECACA"}`,
        }}>
          <div style={{ fontWeight: 600, marginBottom: 6, color: "#374151", fontSize: 10 }}>
            函数内所有 Return 节点 ({siblingReturns.length})
          </div>
          {returnTypeResults.map((r) => {
            const isCurrent = r.returnId === element.id;
            return (
              <div key={r.returnId} style={{
                padding: "3px 6px", marginBottom: 2, borderRadius: 3,
                background: isCurrent ? "#EEF2FF" : "transparent",
                display: "flex", alignItems: "center", gap: 6, fontSize: 10,
              }}>
                <span style={{
                  width: 6, height: 6, borderRadius: "50%",
                  background: r.sourceType === null ? "#D1D5DB"
                    : r.isConsistent ? "#10B981" : "#EF4444",
                  flexShrink: 0,
                }} />
                <span style={{ flex: 1 }}>
                  {isCurrent ? "► " : ""}
                  {r.returnId.slice(0, 12)}…
                </span>
                {r.sourceType ? (
                  <span style={{
                    fontFamily: "monospace", fontSize: 9,
                    color: r.isConsistent ? "#059669" : "#DC2626",
                  }}>
                    {r.sourceType} {r.isConsistent ? "✓" : "≠ " + fnReturnType}
                  </span>
                ) : (
                  <span style={{ color: "#9CA3AF", fontSize: 9 }}>未连接</span>
                )}
              </div>
            );
          })}

          {/* Overall status */}
          <div style={{
            marginTop: 4, padding: "4px 6px", borderRadius: 4,
            fontSize: 10, fontWeight: 500,
            background: allConsistent ? "#ECFDF5" : "#FEF2F2",
            color: allConsistent ? "#065F46" : "#991B1B",
          }}>
            {unconnectedCount > 0 && allConsistent
              ? `✅ 类型一致（${unconnectedCount} 个未连接）`
              : allConsistent
                ? "✅ 所有 return 节点类型一致"
                : "🚫 类型不一致！所有 return 必须匹配函数返回类型"}
          </div>
        </div>
      )}
    </div>
  );
}

function CompareEditor({ element, onUpdate }: {
  element: ElementData;
  onUpdate: (prop: string, value: unknown) => void;
}) {
  const config = element.config || {};
  const op = config.op as string ?? "==";
  const ops = (config.ops as string[]) || ["==", "!=", ">", "<", ">=", "<="];

  return (
    <>
      <FieldLabel label="比较运算符" />
      <select
        value={op}
        onChange={(e) => onUpdate("config", { ...config, op: e.target.value })}
        style={selectStyle}
      >
        {ops.map((o) => (
          <option key={o} value={o}>{o}</option>
        ))}
      </select>
    </>
  );
}

function VariableEditor({ element, onUpdate }: {
  element: ElementData;
  onUpdate: (prop: string, value: unknown) => void;
}) {
  const name = (element.props.name as string) || "";
  const typeAnnotation = (element.props.typeAnnotation as string) || "i32";

  return (
    <>
      <FieldLabel label="变量名" />
      <input
        value={name}
        onChange={(e) => {
          onUpdate("name", e.target.value);
          onUpdate("props", { ...element.props, name: e.target.value });
        }}
        placeholder="变量名称"
        style={inputStyle}
      />

      <FieldLabel label="类型" />
      <select
        value={typeAnnotation}
        onChange={(e) => onUpdate("props", { ...element.props, typeAnnotation: e.target.value })}
        style={selectStyle}
      >
        {["i32", "f64", "bool", "String", "any"].map((t) => (
          <option key={t} value={t}>{t}</option>
        ))}
      </select>

      {element.source && (
        <>
          <FieldLabel label="源码" />
          <pre style={{
            fontSize: 10, background: "#F3F4F6", padding: 6,
            borderRadius: 4, maxHeight: 100, overflow: "auto",
            margin: 0, fontFamily: "monospace",
          }}>
            {element.source}
          </pre>
        </>
      )}
    </>
  );
}

// ═══════════════════════════════════════════════════
// Shared UI helpers
// ═══════════════════════════════════════════════════

function FieldLabel({ label }: { label: string }) {
  return (
    <div style={{ fontSize: 10, color: "#6B7280", fontWeight: 600, marginBottom: 2 }}>
      {label}
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  padding: "4px 8px", fontSize: 11, border: "1px solid #D1D5DB",
  borderRadius: 4, width: "100%", boxSizing: "border-box",
};

const selectStyle: React.CSSProperties = {
  padding: "4px 8px", fontSize: 11, border: "1px solid #D1D5DB",
  borderRadius: 4, width: "100%", boxSizing: "border-box",
  background: "#fff",
};
