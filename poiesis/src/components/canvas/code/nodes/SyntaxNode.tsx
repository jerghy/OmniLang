// poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx
// 语法节点内联编辑版：节点即编辑面（Unity VS 交互精神）。
// 草稿在 NodeField 内部，提交走 onEditProp → setProp；props 仍是唯一事实来源。
import { Handle, Position } from "@xyflow/react";
import React, { useMemo } from "react";
import { formatSyntaxNode, type SyntaxNodeProps } from "../../../../services/codegen/syntax-nodes";
import { NodeTextField, NodeSelect, NodeChipToggle, NodeArgsRows } from "../fields/NodeField";
import { validateSyntaxProps, filterCandidates, type FlowScope } from "../../../../services/codegen/flow-scope";
import styles from "./SyntaxNode.module.css";

const OMNI_TYPES = ["String", "i32", "i64", "f64", "f32", "bool", "Array<String>", "Array<i32>", "Array<f64>", "Map<String, String>"];
const ASSIGN_OPTS = ["=", "+=", "-=", "*=", "/=", "%="].map((op) => ({ value: op, label: op }));
const TYPE_OPTS = OMNI_TYPES.map((t) => ({ value: t, label: t }));

interface Props {
  data: {
    id: string;
    label: string;
    tag: string;
    props?: Record<string, unknown>;
    scope?: FlowScope;
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

// store 名：即时提交文本框（原生 select 无法承载任意 store 名；选中即联动重置 action）。
const StoreNameField = ({ value, onCommit }: { value: string; onCommit: (v: string) => void }) => (
  <input
    data-field-kind="store-name"
    className={`nodrag ${styles.storeName}`}
    value={value}
    aria-label="store 名"
    placeholder="store 名"
    onChange={(e) => onCommit(e.target.value)}
  />
);

const SyntaxNode = React.memo(function SyntaxNode({ data }: Props) {
  const { tag, props = {}, scope, onEditProp } = data;
  const id = data.id ?? "el";
  const set = (prop: string, value: unknown) => onEditProp?.(id, prop, value);

  const errors = useMemo(
    () => (scope ? validateSyntaxProps(tag, props, scope) : []),
    [tag, props, scope],
  );
  const errorFields = useMemo(() => new Set(errors.map((e) => e.field)), [errors]);

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

  const targetPlaceholder = tag === "assign"
    ? scope?.signals[0] ? `e.g. ${scope.signals[0].name} / ThemeStore.mode` : "e.g. count / ThemeStore.mode"
    : undefined;

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
              <NodeTextField
                value={(props.value as string) ?? ""}
                onCommit={(v) => set("value", v)}
                placeholder="初始值（可放表达式）"
                ariaLabel="初始值"
                className={styles.grow}
              />
            </div>
          </>
        )}

        {tag === "assign" && (
          <div className={styles.row}>
            <NodeTextField
              value={(props.target as string) ?? ""}
              onCommit={(v) => set("target", v)}
              placeholder={targetPlaceholder}
              ariaLabel="目标"
              invalid={errorFields.has("target")}
              className={styles.grow}
            />
            <NodeSelect value={(props.operator as string) ?? "="} options={ASSIGN_OPTS} onCommit={(v) => set("operator", v)} ariaLabel="运算符" />
            <NodeTextField
              value={(props.value as string) ?? ""}
              onCommit={(v) => set("value", v)}
              placeholder="表达式"
              ariaLabel="表达式"
              className={styles.grow}
            />
          </div>
        )}

        {tag === "store-call" && (
          <>
            <div className={styles.row}>
              <StoreNameField
                value={(props.storeName as string) ?? ""}
                onCommit={(v) => { set("storeName", v); set("action", ""); }}
              />
              <span className={styles.op}>.</span>
              <NodeSelect
                value={(props.action as string) ?? ""}
                options={actionCandidates}
                onCommit={(v) => set("action", v)}
                ariaLabel="action"
                allowCustom
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
                allowCustom
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

      {errors.length > 0 && (
        <div className={styles.warnBar}>
          {errors.map((e) => <div key={e.field}>{e.message}</div>)}
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
