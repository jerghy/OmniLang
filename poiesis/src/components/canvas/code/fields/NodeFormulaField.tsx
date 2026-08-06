// poiesis/src/components/canvas/code/fields/NodeFormulaField.tsx
// 表达式公式框（重新设计）：textarea 编辑层 + 覆盖层胶囊渲染。
// 光标/IME/选区浏览器原生；胶囊=私有区标记段 \uE000..\uE001，覆盖层渲染高亮块；
// 结构字符着色；自由文本为普通文本。提交=serializeSegments(markup)（字符串，编译器零改动）。
import { memo, useEffect, useMemo, useRef, useState, type KeyboardEvent, type CompositionEvent, type FormEvent, type MouseEvent as ReactMouseEvent } from "react";
import { parseMarkup, serializeSegments, toMarkup, isBoundary, TRIGGER_CHARS, operatorCandidates, tokenizeFormula, CAPSULE_START, CAPSULE_END } from "./expression-trigger";
import { NodeCandidateMenu, type CandidateItem } from "./NodeCandidateMenu";
import styles from "./NodeFormulaField.module.css";

export interface FormulaCandidateSource {
  filter(query: string): CandidateItem[];
  storeSignals(storeName: string): CandidateItem[];
}

interface Props {
  value: string;
  onCommit: (v: string) => void;
  candidates?: FormulaCandidateSource;
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
  /** 触发字符区间（Escape 落点取触发字符用；选中替换用 triggerOffset..当前光标 动态区间） */
  replace: { start: number; end: number } | null;
  /** 逃生口：菜单打开时触发字符（如 "/"）在 markup 中的偏移 */
  triggerOffset: number;
}

/** memo 自定义比较：函数 props 引用变化不触发重渲染（父节点选中等不重置光标）。 */
export const NodeFormulaField = memo(function NodeFormulaField({ value, onCommit, candidates, placeholder, ariaLabel, className, invalid, onCancel, onBlur }: Props) {
  const [draft, setDraft] = useState(() => toMarkup(value));
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [composing, setComposing] = useState(false);
  const focusedRef = useRef(false);
  const dirtyRef = useRef(false);
  const composingRef = useRef(false);
  const taRef = useRef<HTMLTextAreaElement>(null);
  const menuRef = useRef<MenuState | null>(null);
  // 渲染期写 ref 会被 React 回放/丢弃泄漏——改为 effect 同步（React 保证离散事件的被动
  // effect 在下个输入事件前已 flush，onInput 读到的 menuRef 恒为最新提交的菜单状态）
  useEffect(() => { menuRef.current = menu; });

  // 外部 value 变化（提交成功）时同步草稿；聚焦输入中不打断
  useEffect(() => {
    if (!focusedRef.current) setDraft(toMarkup(value));
  }, [value]);

  const segments = useMemo(() => parseMarkup(draft), [draft]);

  const commit = () => {
    if (dirtyRef.current) onCommit(serializeSegments(draft));
    dirtyRef.current = false;
    setMenu(null);
  };
  const cancel = () => {
    focusedRef.current = false;
    dirtyRef.current = false;
    setMenu(null);
    setDraft(toMarkup(value));
    onCancel?.();
  };

  const caret = (): number => taRef.current?.selectionStart ?? draft.length;

  /** 弹框上下文：光标处最后输入的触发字符 → 候选分组与替换区间。 */
  const openMenu = (markup: string) => {
    const el = taRef.current;
    if (!el || !candidates) return;
    const offset = caret();
    const ch = markup[offset - 1];
    if (!ch || !TRIGGER_CHARS.has(ch) || !isBoundary(markup, offset - 1)) return;
    const rect = el.getBoundingClientRect();
    if (ch === "/") {
      const all = candidates.filter("");
      if (all.length === 0) return;
      const groups: Array<{ label: string; items: CandidateItem[] }> = [];
      if (all.length > 0) groups.push({ label: "候选", items: all });
      const ops = operatorCandidates("/");
      if (ops.length > 0) groups.push({ label: "运算符", items: ops });
      setMenu({ x: rect.left, y: rect.bottom + 4, groups, activeIdx: 0, replace: { start: offset - 1, end: offset }, triggerOffset: offset - 1 });
    } else {
      const items = operatorCandidates(ch);
      if (items.length === 0) return;
      setMenu({ x: rect.left, y: rect.bottom + 4, groups: [{ label: "运算符", items }], activeIdx: 0, replace: { start: offset - 1, end: offset }, triggerOffset: offset - 1 });
    }
  };

  /** 输入（textarea 原生）→ 更新草稿；非组合期判定弹框。 */
  const onInput = (e: FormEvent<HTMLTextAreaElement>) => {
    const el = taRef.current;
    if (!el) return;
    const native = (e as unknown as { nativeEvent?: InputEvent })?.nativeEvent;
    if (native && native.isComposing === false) composingRef.current = false;
    const next = el.value;
    setDraft(next);
    dirtyRef.current = true;
    if (composingRef.current) return;
    if (menuRef.current) {
      // 菜单打开：斜杠触发 → 按触发后文本重新过滤（逃生口依赖候选组空）；运算符触发 → 字符已变则关框
      const m = menuRef.current;
      const offset = caret();
      if (next[m.triggerOffset] === "/") {
        const query = next.slice(m.triggerOffset + 1, offset);
        const items = candidates?.filter(query) ?? [];
        const groups: Array<{ label: string; items: CandidateItem[] }> = [];
        if (items.length > 0) groups.push({ label: "候选", items });
        const ops = operatorCandidates("/");
        if (ops.length > 0) groups.push({ label: "运算符", items: ops });
        if (groups.length === 0) setMenu(null);
        else setMenu({ ...m, groups, activeIdx: 0 });
      } else {
        setMenu(null);
      }
      return;
    }
    const prevCaret = caret();
    if (next[prevCaret - 1] && TRIGGER_CHARS.has(next[prevCaret - 1]!)) openMenu(next);
  };

  /** 逃生口：斜杠菜单、候选组空时 Enter → 触发字符+输入文本 → 引用胶囊（候选外标识符出路）。 */
  const escapeHatch = () => {
    if (!menu) return false;
    const cands: CandidateItem[] = [];
    for (const g of menu.groups) if (g.label === "候选") cands.push(...g.items);
    if (cands.length > 0) return false; // 候选组非空 → 正常选中
    if (draft[menu.triggerOffset] !== "/") return false; // 仅斜杠菜单有逃生口（运算符菜单无）
    const el = taRef.current;
    if (!el) return false;
    const offset = caret();
    const raw = draft.slice(menu.triggerOffset + 1, offset).trim();
    if (!raw) return false;
    const next = draft.slice(0, menu.triggerOffset) + CAPSULE_START + raw + CAPSULE_END + draft.slice(offset);
    setDraft(next);
    setMenu(null);
    const cursor = menu.triggerOffset + CAPSULE_START.length + raw.length + CAPSULE_END.length;
    requestAnimationFrame(() => { if (taRef.current) taRef.current.setSelectionRange(cursor, cursor); });
    return true;
  };

  /** 选中候选：替换 triggerOffset..光标 区间为胶囊（斜杠后已输入文本一并替换；运算符/点击胶囊场景区间即触发字符）。 */
  const selectItem = (item: CandidateItem) => {
    if (!menu) return;
    const el = taRef.current;
    const offset = el ? el.selectionStart ?? draft.length : draft.length;
    const start = menu.triggerOffset;
    const v = draft.slice(0, start) + CAPSULE_START + item.value + CAPSULE_END + draft.slice(offset);
    setDraft(v);
    setMenu(null);
    const cursor = start + CAPSULE_START.length + item.value.length + CAPSULE_END.length;
    requestAnimationFrame(() => { if (taRef.current) { taRef.current.setSelectionRange(cursor, cursor); taRef.current.focus(); } });
  };

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    const m = menu;
    if (m) {
      const flat = m.groups.flatMap((g) => g.items);
      if (e.key === "ArrowDown") { e.preventDefault(); setMenu((s) => (s ? { ...s, activeIdx: Math.min(s.activeIdx + 1, flat.length - 1) } : s)); }
      else if (e.key === "ArrowUp") { e.preventDefault(); setMenu((s) => (s ? { ...s, activeIdx: Math.max(s.activeIdx - 1, 0) } : s)); }
      else if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        if (escapeHatch()) return; // 候选空且斜杠菜单 → 引用胶囊
        if (flat.length === 0) return;
        selectItem(flat[m.activeIdx]!);
      }
      else if (e.key === "Escape") {
        e.preventDefault();
        // Esc 关框：触发字符落点为运算符胶囊（原样提交，/ 除法语义由编译器解释）
        const r = m.replace;
        if (r) {
          const ch = draft.slice(r.start, r.end);
          if (TRIGGER_CHARS.has(ch) && !ch.includes(CAPSULE_START)) {
            setDraft(draft.slice(0, r.start) + CAPSULE_START + ch + CAPSULE_END + draft.slice(r.end));
          }
        }
        setMenu(null);
      }
      return;
    }
    if (e.key === "Enter") { e.preventDefault(); commit(); return; }
    if (e.key === "Escape") { e.preventDefault(); cancel(); return; }
    if (e.key === "Backspace" && !composingRef.current) {
      const el = taRef.current;
      if (!el) return;
      const offset = el.selectionStart ?? draft.length;
      if (el.selectionEnd !== offset) return; // 有选区走原生删除
      const seg = segments.find((s) => s.kind === "capsule" && offset >= s.start && offset <= s.end);
      if (seg && offset > seg.start) {
        // 光标在胶囊标记段内或末尾 → 整块删除
        e.preventDefault();
        const v = draft.slice(0, seg.start) + draft.slice(seg.end);
        setDraft(v);
        requestAnimationFrame(() => { if (taRef.current) taRef.current.setSelectionRange(seg.start, seg.start); });
      }
    }
  };

  /** 点击胶囊 → 定位光标 + 弹该类型候选（引用→全量过滤，运算符→运算符候选）。 */
  const onCapsuleMouseDown = (seg: (typeof segments)[number], e: ReactMouseEvent) => {
    e.preventDefault();
    const el = taRef.current;
    if (!el) return;
    el.focus();
    el.setSelectionRange(seg.end, seg.end);
    if (!candidates) return;
    const rect = el.getBoundingClientRect();
    // 胶囊类型以 tokenizer 为单一来源：operator token → 运算符候选；其余 → 引用候选
    const tok = tokenizeFormula(seg.text)[0];
    const ops = tok?.kind === "operator" ? operatorCandidates(tok.text) : [];
    if (ops.length > 0) {
      setMenu({ x: rect.left, y: rect.bottom + 4, groups: [{ label: "运算符", items: ops }], activeIdx: 0, replace: { start: seg.start, end: seg.end }, triggerOffset: seg.start });
      return;
    }
    const items = candidates.filter(seg.text);
    if (items.length === 0) return;
    setMenu({ x: rect.left, y: rect.bottom + 4, groups: [{ label: "候选", items }], activeIdx: 0, replace: { start: seg.start, end: seg.end }, triggerOffset: seg.start });
  };

  return (
    <>
      <div className={`nodrag ${styles.wrap} ${className ?? ""}`} data-field-kind="formula" role="combobox" aria-expanded={menu ? true : undefined} aria-controls={menu ? "node-candidate-results" : undefined} aria-invalid={invalid || undefined} aria-label={ariaLabel}>
        {/* 覆盖层：胶囊块 + 结构字符着色 + 自由文本透明（textarea 文字透明，内容由覆盖层呈现） */}
        <div className={styles.overlay} aria-hidden>
          {segments.map((s, i) =>
            s.kind === "capsule" ? (
              <span key={i} data-seg-kind="capsule" className={styles.capsule} onMouseDown={(e) => onCapsuleMouseDown(s, e)}>{s.text}</span>
            ) : s.kind === "struct" ? (
              <span key={i} data-seg-kind="struct" className={styles.struct}>{s.text}</span>
            ) : (
              <span key={i} data-seg-kind="text" className={styles.text}>{s.text}</span>
            )
          )}
        </div>
        <textarea
          ref={taRef}
          className={composing ? styles.inputComposing : styles.input}
          value={draft}
          placeholder={placeholder}
          data-placeholder={placeholder}
          onFocus={() => { focusedRef.current = true; }}
          onBlur={() => {
            const wasDirty = dirtyRef.current;
            commit();
            focusedRef.current = false;
            if (!wasDirty) setDraft(toMarkup(value));
            onBlur?.();
          }}
          onCompositionStart={() => { composingRef.current = true; setComposing(true); }}
          onCompositionEnd={(e: CompositionEvent<HTMLTextAreaElement>) => {
            composingRef.current = false;
            setComposing(false);
            onInput(e);
          }}
          onInput={onInput}
          onKeyDown={onKeyDown}
        />
      </div>
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
}, (prev, next) => {
  return (
    prev.value === next.value &&
    prev.candidates === next.candidates &&
    prev.placeholder === next.placeholder &&
    prev.ariaLabel === next.ariaLabel &&
    prev.className === next.className &&
    prev.invalid === next.invalid
  );
});
