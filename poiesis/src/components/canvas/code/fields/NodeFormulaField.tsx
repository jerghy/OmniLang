// poiesis/src/components/canvas/code/fields/NodeFormulaField.tsx
// 表达式公式框：NodeTextField 的草稿提交语义 + 触发式自动补全（字符触发 + 上下文候选）。
// 候选插入是草稿的一部分：Enter/blur 才 onCommit（props 仍是唯一事实来源）。
import { useEffect, useRef, useState, type CompositionEvent } from "react";
import { getTriggerContext, operatorCandidates, insertAt, replaceToken, SINGLE_CHAR_OPERATORS } from "./expression-trigger";
import { NodeCandidateMenu, type CandidateItem } from "./NodeCandidateMenu";
import styles from "./NodeField.module.css";

export interface FormulaCandidateSource {
  /** operand 上下文候选（query 前缀过滤，空 query 返回全量前 8） */
  filter(query: string): CandidateItem[];
  /** member 上下文候选（storeName 点后） */
  storeSignals(storeName: string): CandidateItem[];
}

interface Props {
  value: string;
  onCommit: (v: string) => void;
  candidates?: FormulaCandidateSource; // 缺省退化为纯文本输入（无弹层）
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
  // 选中候选的替换区间（tokenStart..cursor）；null = 光标处插入（防御性兜底，当前三上下文均走替换）
  replace: { tokenStart: number; cursor: number } | null;
}

export function NodeFormulaField({ value, onCommit, candidates, placeholder, ariaLabel, className, invalid, onCancel, onBlur }: Props) {
  const [draft, setDraft] = useState(value);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const focusedRef = useRef(false);
  // 终审修复 5：区分「从未 focus」与「已 blur」两种 focusedRef=false 场景——
  // jsdom/合成事件流从不 focus，React 合成 onChange 不派发时 fallback 是唯一输入路径，
  // 必须放行；真实浏览器 blur 先于 change 的迟发事件才需要守卫（防重开悬空弹层）。
  const everFocusedRef = useRef(false);
  const dirtyRef = useRef(false);
  const composingRef = useRef(false);
  const inputRef = useRef<HTMLInputElement>(null);
  // 原生 change 兜底监听需要访问最新闭包（props.candidates 可能变化），经 ref 转发
  const onInputChangeRef = useRef<(e: React.ChangeEvent<HTMLInputElement>) => void>(() => {});

  // 外部 value 变化（提交成功/重渲染）时同步草稿，但聚焦输入中不打断
  useEffect(() => {
    if (!focusedRef.current) setDraft(value);
  }, [value]);

  // React 受控输入的 onChange 在值未变化时不派发（value tracker 检查），
  // 原生 change 监听兜底：保证"空串 → 空串"这类事件也能触发候选计算（空公式弹全量候选）。
  // 值真实变化时两路径都走同一 onInputChange，结果幂等，无额外副作用。
  useEffect(() => {
    const el = inputRef.current;
    if (!el) return;
    const fallback = () => {
      // 终审修复 5：部分浏览器 blur 先于 change 派发——blur 已 commit 并关闭弹层后，
      // 迟发的兜底 change 不应再触发候选计算重开悬空弹层。
      // 守卫只拦「曾聚焦且已 blur」：从未 focus 的合成事件流必须放行（fallback 是唯一输入路径）。
      if (everFocusedRef.current && !focusedRef.current) return;
      onInputChangeRef.current({ target: el, currentTarget: el } as unknown as React.ChangeEvent<HTMLInputElement>);
    };
    el.addEventListener("change", fallback);
    return () => el.removeEventListener("change", fallback);
  }, []);
  // 每次提交后同步最新处理器到 ref（渲染期写 ref 会被 React 重放丢弃，须在 effect 内写）
  useEffect(() => {
    onInputChangeRef.current = onInputChange;
  });

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

  /** 输入变化 → 触发规则 → 更新草稿 + 弹层状态 */
  const onInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const next = e.target.value;
    const cursor = e.target.selectionStart ?? next.length;
    setDraft(next);
    dirtyRef.current = true;
    if (composingRef.current || !candidates) {
      setMenu(null);
      return;
    }
    const trigger = getTriggerContext(next, cursor);
    // 空公式 = 表达式起点：trigger 判定 none，但组件语义是操作数位置（空 token 弹全量候选）
    const ctx = trigger.kind === "none" && next === "" ? { kind: "operand" as const, token: "", tokenStart: 0 } : trigger;
    if (ctx.kind === "none") { setMenu(null); return; }
    if (ctx.kind === "operator") {
      if (SINGLE_CHAR_OPERATORS.includes(ctx.token)) {
        // 单字符运算符（+ - * / %）已由用户键入进值中：不弹层也不重复插入
        setMenu(null);
        return;
      }
      const items = operatorCandidates(ctx.token);
      if (items.length === 0) { setMenu(null); return; }
      // 走 replaceToken 替换运算符前缀：insertAt 会把已键入的前缀字符（如 "<"）重复插入（"count <<="）
      openMenu(ctx.tokenStart, cursor, [{ label: "运算符", items }], { tokenStart: ctx.tokenStart, cursor });
      return;
    }
    if (ctx.kind === "member") {
      const items = candidates.storeSignals(ctx.storeName ?? "");
      if (items.length === 0) { setMenu(null); return; }
      openMenu(ctx.tokenStart, cursor, [{ label: ctx.storeName ? `${ctx.storeName} 信号` : "成员", items }], { tokenStart: ctx.tokenStart, cursor });
      return;
    }
    // operand：变量/函数候选（前缀过滤）
    const items = candidates.filter(ctx.token);
    if (items.length === 0) { setMenu(null); return; }
    openMenu(ctx.tokenStart, cursor, [{ label: "候选", items }], { tokenStart: ctx.tokenStart, cursor });
  };

  const openMenu = (tokenStart: number, cursor: number, groups: MenuState["groups"], replace: MenuState["replace"]) => {
    const rect = inputRef.current?.getBoundingClientRect();
    if (!rect) { setMenu(null); return; }
    setMenu({ x: rect.left, y: rect.bottom + 4, groups, activeIdx: 0, replace });
  };

  /** 选中候选：替换 token 或光标处插入；关闭弹层，保持聚焦与草稿 */
  const selectItem = (item: CandidateItem) => {
    if (!menu) return;
    const cursor = inputRef.current?.selectionStart ?? draft.length;
    const { value: v, cursor: c } = menu.replace
      ? replaceToken(draft, menu.replace.tokenStart, menu.replace.cursor, item.value)
      : insertAt(draft, cursor, item.value);
    setDraft(v);
    setMenu(null);
    requestAnimationFrame(() => {
      const el = inputRef.current;
      if (el) { el.focus(); el.setSelectionRange(c, c); }
    });
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (!menu) {
      if (e.key === "Enter") { e.preventDefault(); commit(); }
      if (e.key === "Escape") { e.preventDefault(); cancel(); }
      return;
    }
    const flat = menu.groups.flatMap((g) => g.items);
    if (e.key === "ArrowDown") { e.preventDefault(); setMenu((m) => m ? { ...m, activeIdx: Math.min(m.activeIdx + 1, flat.length - 1) } : m); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setMenu((m) => m ? { ...m, activeIdx: Math.max(m.activeIdx - 1, 0) } : m); }
    else if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); selectItem(flat[menu.activeIdx] ?? { value: "", label: "" }); }
    else if (e.key === "Escape") { e.preventDefault(); setMenu(null); } // 只关弹层，草稿保留
  };

  return (
    <>
      <input
        ref={inputRef}
        data-field-kind="formula"
        className={`nodrag ${styles.field} ${invalid ? styles.invalid : ""} ${className ?? ""}`}
        value={draft}
        placeholder={placeholder}
        aria-label={ariaLabel}
        aria-invalid={invalid || undefined}
        aria-expanded={menu ? true : undefined}
        aria-controls={menu ? "node-candidate-results" : undefined}
        role="combobox"
        onFocus={() => { focusedRef.current = true; everFocusedRef.current = true; }}
        onCompositionStart={() => { composingRef.current = true; }}
        onCompositionEnd={(e: CompositionEvent<HTMLInputElement>) => {
          composingRef.current = false;
          onInputChange(e as unknown as React.ChangeEvent<HTMLInputElement>);
        }}
        onChange={onInputChange}
        onBlur={() => {
          const wasDirty = dirtyRef.current;
          commit();
          focusedRef.current = false;
          if (!wasDirty) setDraft(value);
          onBlur?.();
        }}
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
