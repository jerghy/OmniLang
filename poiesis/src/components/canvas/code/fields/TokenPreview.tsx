// poiesis/src/components/canvas/code/fields/TokenPreview.tsx
// 表达式只读 token 预览：节点非编辑态的表达式显示与 Notion 式编辑态同款结构
// （胶囊/符号/文本），表达式结构一眼可见。无 candidates 上下文时纯文本 fallback。
// 渲染统一取 value.slice(t.start, t.end) 原文：Task 7 契约中 string token 的 text
// 不含引号（start/end 覆盖含引号原文），原文 slice 对 7 种 kind 均与 text 一致或
// 恢复引号，无需按 kind 特判（交接确认的统一方案）。
import { memo } from "react";
import { tokenizeFormula } from "./expression-trigger";
import styles from "./TokenPreview.module.css";

interface Props {
  value: string;
  // Task 9 候选联动（函数命中蓝色）的预留槽——本任务不消费（YAGNI）
  candidates?: { filter(query: string): unknown[] } | null;
  className?: string;
}

const CAPSULE_KINDS = new Set(["identifier", "member", "operator", "paren"]);

export const TokenPreview = memo(function TokenPreview({ value, className }: Props) {
  const tokens = value ? tokenizeFormula(value) : [];
  return (
    <span className={`${styles.root} ${className ?? ""}`} data-testid="token-preview">
      {tokens.length === 0 ? (
        value
      ) : (
        tokens.map((t, i) => (
          <span
            key={`${i}:${t.text}`}
            data-token-kind={t.kind}
            className={CAPSULE_KINDS.has(t.kind) ? styles.capsule : styles.plain}
          >
            {value.slice(t.start, t.end)}
          </span>
        ))
      )}
    </span>
  );
});
