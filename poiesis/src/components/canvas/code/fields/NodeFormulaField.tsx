// poiesis/src/components/canvas/code/fields/NodeFormulaField.tsx
// 表达式公式框（Notion 式）：输入即分词 → token 胶囊渲染；候选菜单选择插入；
// 运算符直插成符号 token 且弹多字符变体；Backspace 整块删除；IME 组合守卫。
// 提交仍是字符串（onCommit），持久化/编译器零改动。光标用 Selection API（不造零宽锚点）。
// 渲染用 dangerouslySetInnerHTML（react-contenteditable 同款受控模式）：React 不管理
// contenteditable 子节点，浏览器对整树文本的整体替换（全选输入/IME 提交）不会与 React
// 对账冲突（children 渲染会抛 NotFoundError）；每次渲染后经 rAF restoreCaret 恢复光标。
// token 间空白以文本段保留（.notionField white-space: pre-wrap 不折叠），draft 与
// textContent 双向一致，编辑不会吞掉用户输入的空格。
import { useEffect, useMemo, useRef, useState, type CompositionEvent, type KeyboardEvent } from "react";
import { tokenizeFormula, operatorCandidates, replaceToken } from "./expression-trigger";
import { NodeCandidateMenu, type CandidateItem } from "./NodeCandidateMenu";
import styles from "./NodeField.module.css";

export interface FormulaCandidateSource {
  /** operand 上下文候选（query 前缀过滤，空 query 返回全量） */
  filter(query: string): CandidateItem[];
  /** member 上下文候选（storeName 点后） */
  storeSignals(storeName: string): CandidateItem[];
}

interface Props {
  value: string;
  onCommit: (v: string) => void;
  candidates?: FormulaCandidateSource; // 缺省退化纯文本输入（无分词无弹层）
  placeholder?: string;
  ariaLabel?: string;
  className?: string;
  invalid?: boolean;
  onCancel?: () => void;
  onBlur?: () => void;
}

interface MenuState {
  x: number;
  y: number;
  groups: Array<{ label: string; items: CandidateItem[] }>;
  activeIdx: number;
  /** 选中候选的替换区间（token 起止）；null = 光标处插入（兜底） */
  replace: { start: number; end: number } | null;
}

const BLOCK_DELETE_KINDS = new Set(["identifier", "member", "operator", "paren"]);
const HTML_ESCAPES: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };

/** HTML 转义（dangerouslySetInnerHTML 渲染用户输入——防注入；token 文本/空白段均转义）。 */
function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => HTML_ESCAPES[c]!);
}

/** 在文本节点间按 offset 恢复光标（Selection API——CodeCard 零宽锚点教训的规避）。 */
function restoreCaret(el: HTMLElement, offset: number): void {
  const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
  let node: Text | null;
  let remaining = offset;
  let last: Text | null = null;
  while ((node = walker.nextNode() as Text | null)) {
    last = node;
    const len = node.textContent?.length ?? 0;
    if (remaining <= len) {
      const range = document.createRange();
      range.setStart(node, remaining);
      range.collapse(true);
      const sel = window.getSelection();
      sel?.removeAllRanges();
      sel?.addRange(range);
      return;
    }
    remaining -= len;
  }
  if (last) {
    const range = document.createRange();
    range.setStart(last, last.textContent?.length ?? 0);
    range.collapse(true);
    window.getSelection()?.removeAllRanges();
    window.getSelection()?.addRange(range);
  }
}

/** 当前光标在 textContent 内的字符偏移（无选区/光标在编辑面外时回退到末尾）。 */
function caretOffset(el: HTMLElement): number {
  const sel = window.getSelection();
  if (!sel || sel.rangeCount === 0 || !el.contains(sel.anchorNode)) return el.textContent?.length ?? 0;
  const range = sel.getRangeAt(0);
  const pre = range.cloneRange();
  pre.selectNodeContents(el);
  pre.setEnd(range.startContainer, range.startOffset);
  return pre.toString().length;
}

export function NodeFormulaField({ value, onCommit, candidates, placeholder, ariaLabel, className, invalid, onCancel, onBlur }: Props) {
  const [draft, setDraft] = useState(value);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [composing, setComposing] = useState(false);
  const focusedRef = useRef(false);
  const dirtyRef = useRef(false);
  const composingRef = useRef(false);
  const editRef = useRef<HTMLDivElement>(null);

  const tokens = useMemo(() => tokenizeFormula(draft), [draft]);

  // 渲染 HTML：IME 组合期/退化模式为原样文本（防 React 重渲染覆盖组合输入）；
  // 其余按 token 段 + token 间空白文本段拼接（textContent 与 draft 双向一致）。
  const html = useMemo(() => {
    if (composing || !candidates || tokens.length === 0) return escapeHtml(draft);
    let out = "";
    let pos = 0;
    for (const t of tokens) {
      if (t.start > pos) out += escapeHtml(draft.slice(pos, t.start));
      out += `<span data-token-kind="${t.kind}" class="${BLOCK_DELETE_KINDS.has(t.kind) ? styles.tokenCapsule : styles.tokenPlain}">${escapeHtml(t.text)}</span>`;
      pos = t.end;
    }
    if (pos < draft.length) out += escapeHtml(draft.slice(pos));
    return out;
  }, [composing, candidates, draft, tokens]);

  // 外部 value 变化（提交成功/重渲染）时同步草稿，但聚焦输入中不打断
  useEffect(() => {
    if (!focusedRef.current) setDraft(value);
  }, [value]);

  const commit = () => {
    if (dirtyRef.current && draft !== value) onCommit(draft);
    dirtyRef.current = false;
    setMenu(null);
  };
  const cancel = () => {
    focusedRef.current = false;
    dirtyRef.current = false;
    setMenu(null);
    setDraft(value);
    onCancel?.();
  };

  /** 候选上下文：光标所在 token → 候选源（语义与旧 getTriggerContext 对齐：member/operator/operand）。
   *  member 判定优先按字符（光标在 "." 后）：trailing dot 时 "." 常被 tokenizer 归入 unknown 段。 */
  const candidatesFor = (text: string, offset: number): { groups: MenuState["groups"]; replace: MenuState["replace"] } | null => {
    if (composingRef.current || !candidates) return null;
    const toks = tokenizeFormula(text);
    const at = toks.find((t) => offset >= t.start && offset <= t.end);
    // member 上下文：光标在 "." 之后（Store. 前缀或 Store.signal 中途）。store 名取 "." 前最长标识符；
    // 前缀不匹配时（如 "a.b" 光标在 member 末尾）兜底取 member token 的 store 段。
    if (text[offset - 1] === ".") {
      const prefix = text.slice(0, offset - 1);
      const storeMatch = prefix.match(/[A-Za-z_]\w*$/);
      let storeName = storeMatch?.[0] ?? "";
      let start = storeName ? offset - 1 - storeName.length : offset;
      if (!storeName && at?.kind === "member") {
        storeName = at.text.slice(0, at.text.indexOf("."));
        start = at.start;
      }
      if (!storeName) return null;
      const items = candidates.storeSignals(storeName);
      if (items.length === 0) return null;
      return { groups: [{ label: `${storeName} 信号`, items }], replace: { start, end: offset } };
    }
    if (at?.kind === "member") {
      // 光标在 member token 内（非点后）：最后一个 "." 之后的段当操作数过滤（如 "a.b" → filter("b")）
      const tailAt = at.text.lastIndexOf(".");
      const token = tailAt >= 0 ? at.text.slice(tailAt + 1) : at.text;
      const items = candidates.filter(token);
      if (items.length === 0) return null;
      return { groups: [{ label: "候选", items }], replace: { start: at.start + tailAt + 1, end: at.end } };
    }
    if (at?.kind === "operator") {
      const items = operatorCandidates(at.text);
      if (items.length === 0) return null;
      return { groups: [{ label: "运算符", items }], replace: { start: at.start, end: at.end } };
    }
    // operand：标识符前缀过滤；数字/字符串字面量内不弹（字面量无补全语义——Enter 提交契约的基础）
    if (at?.kind === "identifier") {
      const items = candidates.filter(at.text);
      if (items.length === 0) return null;
      return { groups: [{ label: "候选", items }], replace: { start: at.start, end: at.end } };
    }
    if (at && (at.kind === "number" || at.kind === "string")) return null;
    // 表达式边界（空 token 全量）：括号/空白后/任意符号处
    const items = candidates.filter("");
    if (items.length === 0) return null;
    return { groups: [{ label: "候选", items }], replace: { start: offset, end: offset } };
  };

  /** 输入 → 更新草稿 + token 渲染 + 候选弹层 */
  const onInput = () => {
    const el = editRef.current;
    if (!el) return;
    const next = el.textContent ?? "";
    const offset = caretOffset(el);
    setDraft(next);
    dirtyRef.current = true;
    if (composingRef.current) return; // IME 组合期不弹层
    // 渲染替换 innerHTML 会丢选区：重渲染后经 rAF 按原 offset 恢复光标（textContent 双向一致）
    requestAnimationFrame(() => { if (editRef.current) restoreCaret(editRef.current, offset); });
    const ctx = candidatesFor(next, offset);
    if (!ctx) { setMenu(null); return; }
    const rect = el.getBoundingClientRect();
    setMenu({ x: rect.left, y: rect.bottom + 4, groups: ctx.groups, activeIdx: 0, replace: ctx.replace });
  };

  /** 选中候选：替换光标所在 token 区间；关闭弹层；光标移到替换后 */
  const selectItem = (item: CandidateItem) => {
    if (!menu) return;
    const el = editRef.current;
    const offset = el ? caretOffset(el) : draft.length;
    const r = menu.replace;
    const { value: v, cursor } = r
      ? replaceToken(draft, r.start, r.end, item.value)
      : { value: draft.slice(0, offset) + item.value + draft.slice(offset), cursor: offset + item.value.length };
    setDraft(v);
    setMenu(null);
    requestAnimationFrame(() => {
      if (editRef.current) { restoreCaret(editRef.current, cursor); editRef.current.focus(); }
    });
  };

  /** Backspace 整块删除：光标在胶囊 token 边界时删整个 token（文本类逐字符）；菜单打开时走导航 */
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (!menu) {
      if (e.key === "Enter") { e.preventDefault(); commit(); return; }
      if (e.key === "Escape") { e.preventDefault(); cancel(); return; }
      if (e.key === "Backspace" && !composingRef.current) {
        const el = editRef.current;
        if (!el) return;
        const offset = caretOffset(el);
        const toks = tokenizeFormula(draft);
        const at = toks.find((t) => offset === t.end && BLOCK_DELETE_KINDS.has(t.kind));
        if (at) {
          e.preventDefault();
          const v = draft.slice(0, at.start) + draft.slice(at.end);
          setDraft(v);
          requestAnimationFrame(() => { if (editRef.current) restoreCaret(editRef.current, at.start); });
        }
      }
      return;
    }
    const flat = menu.groups.flatMap((g) => g.items);
    if (e.key === "ArrowDown") { e.preventDefault(); setMenu((m) => (m ? { ...m, activeIdx: Math.min(m.activeIdx + 1, flat.length - 1) } : m)); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setMenu((m) => (m ? { ...m, activeIdx: Math.max(m.activeIdx - 1, 0) } : m)); }
    else if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); selectItem(flat[menu.activeIdx] ?? { value: "", label: "" }); }
    else if (e.key === "Escape") { e.preventDefault(); setMenu(null); } // 只关弹层，草稿保留
  };

  return (
    <>
      <div
        ref={editRef}
        contentEditable
        suppressContentEditableWarning
        data-field-kind="formula"
        role="combobox"
        aria-label={ariaLabel}
        aria-expanded={menu ? true : undefined}
        aria-controls={menu ? "node-candidate-results" : undefined}
        aria-invalid={invalid || undefined}
        className={`nodrag ${styles.field} ${styles.notionField} ${invalid ? styles.invalid : ""} ${className ?? ""}`}
        data-placeholder={placeholder}
        dangerouslySetInnerHTML={{ __html: html }}
        onFocus={() => { focusedRef.current = true; }}
        onBlur={() => {
          const wasDirty = dirtyRef.current;
          commit();
          focusedRef.current = false;
          if (!wasDirty) setDraft(value);
          onBlur?.();
        }}
        onCompositionStart={() => { composingRef.current = true; setComposing(true); }}
        onCompositionEnd={(e: CompositionEvent<HTMLDivElement>) => {
          composingRef.current = false;
          setComposing(false);
          // 组合结束：DOM 文本与组合结果对齐（jsdom 的 CompositionEvent data 为空串，跳过以免清空）
          if (e.data) { editRef.current!.textContent = e.data; }
          onInput();
        }}
        onInput={onInput}
        onKeyDown={onKeyDown}
      />
      {menu && (
        <NodeCandidateMenu
          x={menu.x}
          y={menu.y}
          groups={menu.groups}
          activeIdx={menu.activeIdx}
          onSelect={selectItem}
          onMouseEnter={(idx) => setMenu((m) => (m ? { ...m, activeIdx: idx } : m))}
          onClose={() => setMenu(null)}
        />
      )}
    </>
  );
}
