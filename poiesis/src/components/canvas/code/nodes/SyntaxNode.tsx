// poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx
// 语法节点内联编辑版：节点即编辑面（Unity VS 交互精神）。
// 草稿在 NodeField 内部，提交走 onEditProp → setProp；props 仍是唯一事实来源。
import { Handle, Position } from "@xyflow/react";
import React, { useMemo } from "react";
import { formatSyntaxNode, type SyntaxNodeProps, ASSIGN_OPERATORS } from "../../../../services/codegen/syntax-nodes";
import { NodeTextField, NodeSelect, NodeChipToggle, NodeArgsRows } from "../fields/NodeField";
import { NodeFormulaField, type FormulaCandidateSource } from "../fields/NodeFormulaField";
import { countUnbalancedParens } from "../fields/expression-trigger";
import { validateSyntaxProps, filterCandidates, buildExpressionCandidates, buildTargetOptions, type FlowScope } from "../../../../services/codegen/flow-scope";
import styles from "./SyntaxNode.module.css";

const OMNI_TYPES = ["String", "i32", "i64", "f64", "f32", "bool", "Array<String>", "Array<i32>", "Array<f64>", "Map<String, String>"];
// 首项为空选项（（推断））：未选类型时 select 不显示幻影 "String"（onCommit 侧 "" → undefined）
const TYPE_OPTS = [{ value: "", label: "（推断）" }, ...OMNI_TYPES.map((t) => ({ value: t, label: t }))];

interface Props {
  data: {
    id: string;
    label: string;
    tag: string;
    props?: Record<string, unknown>;
    scope?: FlowScope;
    // FlowCanvas 按 wire 拓扑注入所属 fn 的 let 变量（公式框候选数据源）
    letVars?: Array<{ name: string }>;
    onEditProp?: (id: string, prop: string, value: unknown) => void;
  };
}

// Handle 依赖 ReactFlowProvider（连线渲染）；单元测试直接渲染节点时静默跳过，避免抛错。
class HandlesBoundary extends React.Component<{ children: React.ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  render() {
    return this.state.failed ? null : this.props.children;
  }
}

const SyntaxNode = React.memo(function SyntaxNode({ data }: Props) {
  const { tag, props = {}, scope, letVars, onEditProp } = data;
  const id = data.id ?? "el";
  const set = (prop: string, value: unknown) => onEditProp?.(id, prop, value);

  const errors = useMemo(
    () => (scope ? validateSyntaxProps(tag, props, scope) : []),
    [tag, props, scope],
  );
  const errorFields = useMemo(() => new Set(errors.map((e) => e.field)), [errors]);

  const storeCandidates = (scope?.stores ?? []).map((s) => s.name);
  const actionCandidates = useMemo(() => {
    const storeName = (props.storeName as string) ?? "";
    const store = scope?.stores.find((s) => s.name === storeName);
    return (store?.actions ?? []).map((a) => ({ value: a.name, label: a.params.length ? `${a.name}(${a.params.map((p) => `${p.name}: ${p.type}`).join(", ")})` : a.name }));
  }, [scope, props.storeName]);

  const fnCandidates = useMemo(() => {
    const base = [
      ...(scope?.fns ?? []).map((f) => ({ name: f.name, type: "fn" })),
      ...(scope?.events ?? []).map((e) => ({ name: e.name, type: "event" })),
    ];
    return filterCandidates("", base).map((c) => ({ value: c.name, label: c.name }));
  }, [scope]);

  // 公式框候选源：scope 全量候选扁平化 + store 信号分组（member 上下文）。
  // 候选插入是草稿的一部分（NodeFormulaField 内部处理），SyntaxNode 只管 onCommit → set("value", v)。
  const formulaSource = useMemo<FormulaCandidateSource | undefined>(() => {
    if (!scope) return undefined;
    const all = buildExpressionCandidates(scope, letVars ?? []);
    const flat = all.flatMap((g) => g.items);
    const flatStore = all.find((g) => g.label === "store 信号")?.items ?? [];
    return {
      filter: (q) => {
        const t = q.trim();
        if (!t) return flat.slice(0, 8);
        return flat.filter((c) => c.value.startsWith(t)).slice(0, 8);
      },
      storeSignals: (storeName) => flatStore.filter((c) => c.value.startsWith(`${storeName}.`)).slice(0, 8),
    };
  }, [scope, letVars]);

  // 括号配对轻校验（spec §4.5：只提示不阻塞；跳过字符串字面量内的括号）
  const parenIssue = useMemo(() => {
    if (tag !== "assign" && tag !== "let") return null;
    const v = (props.value as string) ?? "";
    if (!v) return null;
    const n = countUnbalancedParens(v);
    if (n > 0) return { field: "value", message: `表达式有 ${n} 个未闭合的括号` };
    if (n < 0) return { field: "value", message: `表达式有 ${-n} 个多余的右括号` };
    return null;
  }, [tag, props.value]);
  const allErrors = parenIssue ? [...errors, parenIssue] : errors;

  const code = useMemo(() => formatSyntaxNode(tag, props as SyntaxNodeProps), [tag, props]);

  return (
    <div
      className={`flow-node ${styles.node}`}
      data-node-type={tag}
      data-syntax-code={code}
    >
      {/* 头部：tag 中文标签 */}
      <div className={styles.header}>{data.label}</div>

      <div className={styles.body}>
        {tag === "let" && (
          <>
            <div className={styles.row}>
              <NodeChipToggle checked={!!props.mutable} onChange={(v) => set("mutable", v)} label="mut" ariaLabel="mut" />
              <NodeTextField
                value={(props.name as string) ?? ""}
                onCommit={(v) => set("name", v)}
                placeholder="变量名"
                ariaLabel="变量名"
                invalid={errorFields.has("name")}
                className={styles.grow}
              />
              <NodeSelect value={(props.varType as string) ?? ""} options={TYPE_OPTS} onCommit={(v) => set("varType", v || undefined)} ariaLabel="类型" allowCustom />
            </div>
            <div className={styles.row}>
              <NodeFormulaField
                value={(props.value as string) ?? ""}
                candidates={formulaSource}
                onCommit={(v) => set("value", v)}
                placeholder="初始值（变量/值/运算）"
                ariaLabel="初始值"
                className={styles.grow}
              />
            </div>
          </>
        )}

        {tag === "assign" && (
          <div className={styles.row}>
            <NodeSelect
              value={(props.target as string) ?? ""}
              options={scope ? buildTargetOptions(scope, letVars ?? []) : []}
              onCommit={(v) => set("target", v)}
              ariaLabel="目标"
              invalid={errorFields.has("target")}
              allowCustom
              className={styles.grow}
            />
            <NodeSelect value={(props.operator as string) ?? "="} options={ASSIGN_OPERATORS.map((op) => ({ value: op, label: op }))} onCommit={(v) => set("operator", v)} ariaLabel="运算符" />
            <NodeFormulaField
              value={(props.value as string) ?? ""}
              candidates={formulaSource}
              onCommit={(v) => set("value", v)}
              placeholder="表达式（变量/值/运算）"
              ariaLabel="表达式"
              className={styles.grow}
            />
          </div>
        )}

        {tag === "store-call" && (
          <>
            <div className={styles.row}>
              {/* store 名：NodeTextField 草稿提交（blur/Enter），提交时才联动重置 action；datalist 提供候选补全 */}
              <NodeTextField
                value={(props.storeName as string) ?? ""}
                onCommit={(v) => { set("storeName", v); set("action", ""); }}
                list="syntax-store-candidates"
                placeholder="store 名"
                ariaLabel="store 名"
                className={styles.storeName}
              />
              <datalist id="syntax-store-candidates">
                {storeCandidates.map((n) => <option key={n} value={n} />)}
              </datalist>
              <span className={styles.op}>.</span>
              <NodeSelect
                value={(props.action as string) ?? ""}
                options={actionCandidates}
                onCommit={(v) => set("action", v)}
                ariaLabel="action"
                invalid={errorFields.has("action")}
                allowCustom
                className={styles.grow}
              />
            </div>
            <NodeArgsRows
              values={((props.args as string) ?? "").split(",").map((s) => s.trim()).filter((s) => s !== "")}
              onChange={(vals) => set("args", vals.filter((v) => v !== "").join(", "))}
              placeholder="参数"
            />
          </>
        )}

        {tag === "call" && (
          <>
            <div className={styles.row}>
              {props.emit ? <span className={styles.op}>emit</span> : null}
              <NodeSelect
                value={(props.fnName as string) ?? ""}
                options={fnCandidates}
                onCommit={(v) => set("fnName", v)}
                ariaLabel="函数名"
                invalid={errorFields.has("fnName")}
                allowCustom
                className={styles.grow}
              />
              <NodeChipToggle
                checked={!!props.emit}
                onChange={(v) => set("emit", v)}
                label="emit"
                ariaLabel="emit 事件"
              />
            </div>
            <NodeArgsRows
              values={((props.args as string) ?? "").split(",").map((s) => s.trim()).filter((s) => s !== "")}
              onChange={(vals) => set("args", vals.filter((v) => v !== "").join(", "))}
              placeholder="参数"
            />
          </>
        )}
      </div>

      {/* 预览：formatSyntaxNode 单一来源（data-syntax-code 属性 + 节点底部文本） */}
      <div className={styles.codePreview}>{code}</div>

      {allErrors.length > 0 && (
        <div className={styles.warnBar}>
          {allErrors.map((e) => <div key={e.field}>{e.message}</div>)}
        </div>
      )}

      <HandlesBoundary>
        <Handle type="target" position={Position.Top} id="flow-in" style={{ background: "var(--ol-text-muted)" }} />
        <Handle type="source" position={Position.Bottom} id="flow-out" style={{ background: "var(--ol-text-muted)" }} />
      </HandlesBoundary>
    </div>
  );
});
export { SyntaxNode };
