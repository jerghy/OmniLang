// components/panels/inspector/PropsSection.tsx
// Editable form for element properties (id, class binding, label, etc.).
// All relevant props render as chip-based segment editors ({expr} segments
// become purple chips); the id row keeps a plain PropRow input.

import { useState, useCallback, useRef, useEffect } from "react";
import { usePoiesisStore } from "../../../store";
import type { ElementData } from "../../../core/types/element";
import type { SignalPickerItem } from "./SignalSlashPicker";
import { SignalSlashPicker } from "./SignalSlashPicker";
import { BOOLEAN_ATTRS } from "../../../services/codegen/boolean-attrs";
import styles from "./PropsSection.module.css";

const KNOWN_PROPS = ["id", "label", "class", "name", "type", "placeholder", "value", "src", "href"];

// Module-level WeakMap: stores chip slash-insert callbacks keyed by input element,
// instead of polluting DOM elements with ad-hoc `__chipSlashInsert` properties.
const chipSlashCallbacks = new WeakMap<HTMLElement, (signalName: string) => void>();

// ── Chip segment types & helpers ──────────────────────────────────────────

type ChipSegment =
  | { type: "text"; value: string }
  | { type: "expr"; value: string }; // value is the name inside { }

function parseChipSegments(label: string): ChipSegment[] {
  if (!label) return [];
  const segments: ChipSegment[] = [];
  const regex = /\{([^}]+)\}|([^{]+)/g;
  let match: RegExpExecArray | null;
  while ((match = regex.exec(label)) !== null) {
    if (match[1] !== undefined) {
      segments.push({ type: "expr", value: match[1] });
    } else if (match[2]) {
      segments.push({ type: "text", value: match[2] });
    }
  }
  return segments.length > 0 ? segments : [{ type: "text", value: label }];
}

function serializeSegments(segments: ChipSegment[]): string {
  return segments
    .map((s) => (s.type === "expr" ? `{${s.value}}` : s.value))
    .join("");
}

// ── Shared chip styles ─────────────────────────────────────────────────────

const CHIP_BASE: React.CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: 4,
  padding: "3px 8px",
  fontSize: 12,
  fontFamily: "var(--dp-font-mono, monospace)",
  borderRadius: 12,
  border: "1px solid",
  whiteSpace: "nowrap",
  lineHeight: "18px",
};

const EXPR_CHIP_STYLE: React.CSSProperties = {
  ...CHIP_BASE,
  background: "var(--ol-color-primary-subtle)",
  color: "var(--ol-color-primary)",
  borderColor: "var(--ol-color-primary-muted)",
};

const CHIP_DELETE_BTN: React.CSSProperties = {
  border: "none",
  background: "none",
  cursor: "pointer",
  fontSize: 13,
  padding: 0,
  lineHeight: 1,
  marginLeft: 2,
  opacity: 0.5,
};

function buildLocalProps(element: ElementData): Record<string, string | boolean> {
  const initial: Record<string, string | boolean> = {};
  if (!("props" in element)) return initial;
  // KNOWN_PROPS + BOOLEAN_ATTRS：布尔属性（disabled/checked/...）不在
  // KNOWN_PROPS 里，但必须在面板可见可编辑（D9）。
  for (const key of new Set([...KNOWN_PROPS, ...BOOLEAN_ATTRS])) {
    const val = element.props[key];
    if (val !== undefined) {
      initial[key] = typeof val === "boolean" ? val : String(val);
    }
  }
  return initial;
}

interface SlashState {
  propName: string;
  inputEl: HTMLInputElement;
  /** Cursor position where "/" was typed (for replacement range) */
  slashIndex: number;
}

interface PropsSectionProps {
  element: ElementData;
  signalItems?: SignalPickerItem[];
}

export function PropsSection({ element, signalItems }: PropsSectionProps) {
  const setProp = usePoiesisStore((s) => s.setProp);

  // Local editing state to avoid store writes on every keystroke
  const [localProps, setLocalProps] = useState<Record<string, string | boolean>>(
    () => buildLocalProps(element)
  );
  const [slash, setSlash] = useState<SlashState | null>(null);

  const prevElementIdRef = useRef(element.id);
  const elementRef = useRef(element);
  // Sync element ref after every render — safe: effect, not render phase
  useEffect(() => {
    elementRef.current = element;
  });
  useEffect(() => {
    const el = elementRef.current;
    if (el.id !== prevElementIdRef.current) {
      prevElementIdRef.current = el.id;
      setLocalProps(buildLocalProps(el));
      // eslint-disable-next-line react-doctor/no-adjust-state-on-prop-change
      setSlash(null);
    }
  }, [element.id]); // setLocalProps, setSlash (useState), buildLocalProps (module-level) are stable

  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const debouncedSetProp = useCallback(
    (id: string, prop: string, value: string | boolean) => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => {
        setProp(id, prop, value);
      }, 200);
    },
    [setProp]
  );

  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);

  const handleChange = useCallback(
    (propName: string, value: string | boolean) => {
      setLocalProps((prev) => ({ ...prev, [propName]: value }));
      debouncedSetProp(element.id, propName, value);
    },
    [element.id, debouncedSetProp]
  );

  // Called by PropRow when "/" is typed
  const handleSlash = useCallback(
    (propName: string, inputEl: HTMLInputElement) => {
      const cursorPos = inputEl.selectionStart ?? inputEl.value.length;
      // The "/" was just typed at cursorPos - 1
      const slashIndex = cursorPos - 1;
      setSlash({ propName, inputEl, slashIndex });
    },
    []
  );

  // Close the picker
  const handleSlashClose = useCallback(() => {
    // Clean up any chip editor insert callback
    if (slash?.inputEl) {
      chipSlashCallbacks.delete(slash.inputEl);
    }
    setSlash(null);
  }, [slash]);

  // Handle selection from picker
  const handleSlashSelect = useCallback(
    (item: SignalPickerItem) => {
      if (!slash) return;
      const { propName, inputEl, slashIndex } = slash;

      // Chip editor: use the WeakMap-stored insert callback
      const chipInsert = chipSlashCallbacks.get(inputEl);
      if (chipInsert) {
        chipSlashCallbacks.delete(inputEl);
        chipInsert(item.insertionText);
        setSlash(null);
        return;
      }

      const currentValue = String(localProps[propName] ?? "");
      // Filter text is everything from "/" to end (user may have typed more chars)
      const newValue = currentValue.slice(0, slashIndex) + `{${item.insertionText}}`;
      // Update local and debounced store
      const updated = { ...localProps, [propName]: newValue };
      setLocalProps(updated);
      debouncedSetProp(element.id, propName, newValue);
      // Close picker and refocus input
      setSlash(null);
      // Set cursor after the inserted {expr}
      const cursorTarget = slashIndex + item.insertionText.length + 2; // +2 for { }
      requestAnimationFrame(() => {
        inputEl.focus();
        inputEl.setSelectionRange(cursorTarget, cursorTarget);
      });
    },
    [slash, localProps, element.id, debouncedSetProp]
  );

  // Compute which props are relevant based on tag
  const relevantProps = getRelevantProps("tag" in element ? element.tag : "div");

  // Derived filter text: everything after "/" in the input's actual value.
  // For the label chip editor, use inputEl.value (segment text) instead of
  // localProps[propName] (full serialized label) so the filter is correct.
  const slashFilter = slash
    ? slash.inputEl.value.slice(slash.slashIndex + 1)
    : "";

  // Compute picker position from active input
  const pickerPosition = slash
    ? (() => {
        const rect = slash.inputEl.getBoundingClientRect();
        return { top: rect.bottom + 4, left: rect.left };
      })()
    : { top: 0, left: 0 };

  return (
    <div style={{ marginBottom: 12 }}>
      <div
        style={{
          fontSize: 12,
          fontWeight: 600,
          color: "var(--ol-text-secondary)",
          textTransform: "uppercase",
          marginBottom: 6,
        }}
      >
        属性
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        {/* Always show id */}
        <PropRow
          label="ID"
          value={String(localProps["id"] ?? element.id)}
          onChange={(v) => handleChange("id", v)}
          onSlash={(el) => handleSlash("id", el)}
        />

        {relevantProps.map((prop) =>
          BOOLEAN_ATTRS.has(prop) ? (
            <BoolPropRow
              key={prop}
              label={prop}
              value={localProps[prop]}
              onChange={(v) => handleChange(prop, v)}
              onSlash={(el) => handleSlash(prop, el)}
            />
          ) : (
            <ChipPropRow
              key={prop}
              label={prop}
              value={localProps[prop] ?? ""}
              onChange={(v) => handleChange(prop, v)}
              onSlash={(el) => handleSlash(prop, el)}
            />
          )
        )}
      </div>

      {/* Signal/Computed/Store slash picker */}
      {slash && signalItems && signalItems.length > 0 && (
        <SignalSlashPicker
          items={signalItems}
          filter={slashFilter}
          onSelect={handleSlashSelect}
          onClose={handleSlashClose}
          position={pickerPosition}
        />
      )}
    </div>
  );
}

// ── ChipEditor: chip-based text segment editor ─────────────────────────────

interface ChipEditorProps {
  value: string;
  onChange: (v: string) => void;
  onSlashRequest?: (inputEl: HTMLInputElement) => void;
}

function ChipEditor({ value, onChange, onSlashRequest }: ChipEditorProps) {
  const segments = parseChipSegments(value);
  const [editingIdx, setEditingIdx] = useState<number | null>(null);
  const editInputRef = useRef<HTMLInputElement>(null);
  const textInputRefs = useRef<Map<number, HTMLInputElement>>(new Map());
  const slashPendingRef = useRef(false);

  useEffect(() => {
    if (editingIdx !== null && editInputRef.current) {
      editInputRef.current.focus();
    }
  }, [editingIdx]);

  const handleDelete = useCallback(
    (idx: number) => {
      const updated = [...segments];
      updated.splice(idx, 1);
      onChange(serializeSegments(updated));
    },
    [segments, onChange],
  );

  /** Commit a text-segment input: parse its value and replace the segment.
   *  idx=-1 means the ghost trailing input — append instead of replace. */
  const commitText = useCallback(
    (idx: number) => {
      if (slashPendingRef.current) { slashPendingRef.current = false; return; }
      const input = textInputRefs.current.get(idx);
      if (!input) return;
      const text = input.value;
      if (idx === -1) {
        // Ghost trailing input: append if non-empty, else just clear
        if (text.trim()) {
          const newSegs = parseChipSegments(text);
          if (newSegs.length > 0) {
            onChange(serializeSegments([...segments, ...newSegs]));
          }
        }
        input.value = "";
        return;
      }
      if (!text.trim()) { handleDelete(idx); return; }
      const newSegs = parseChipSegments(text);
      const updated = [...segments];
      updated.splice(idx, 1, ...newSegs);
      onChange(serializeSegments(updated));
    },
    [segments, onChange, handleDelete],
  );

  const handleEditCommit = useCallback(
    (idx: number) => {
      if (slashPendingRef.current) { slashPendingRef.current = false; return; }
      const input = editInputRef.current;
      if (!input || editingIdx === null) return;
      const text = input.value;
      if (!text.trim()) { handleDelete(idx); return; }
      const newSegs = parseChipSegments(text);
      const updated = [...segments];
      updated.splice(idx, 1, ...newSegs);
      onChange(serializeSegments(updated));
      setEditingIdx(null);
    },
    [segments, onChange, editingIdx, handleDelete],
  );

  /** KeyDown for text-segment inline inputs. i=-1 means ghost trailing input. */
  const onTextKeyDown = useCallback(
    (i: number, e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter") { e.preventDefault(); commitText(i); }
      else if (e.key === "/" && onSlashRequest) {
        slashPendingRef.current = true;
        const input = e.currentTarget;
        chipSlashCallbacks.set(input, (signalName: string) => {
          const pos = input.selectionStart ?? input.value.length;
          const slashPos = input.value.lastIndexOf("/", pos);
          const before = slashPos >= 0 ? input.value.slice(0, slashPos) : input.value;
          const after = input.value.slice(pos);
          input.value = before + `{${signalName}}` + after;
          const newSegs = parseChipSegments(input.value);
          if (i === -1) {
            // Ghost: append new segments
            if (newSegs.length > 0) onChange(serializeSegments([...segments, ...newSegs]));
            input.value = "";
          } else {
            const updated = [...segments];
            updated.splice(i, 1, ...newSegs);
            onChange(serializeSegments(updated));
          }
        });
        requestAnimationFrame(() => onSlashRequest(input));
      }
    },
    [commitText, onSlashRequest, segments, onChange],
  );

  /** KeyDown for click-to-edit inputs */
  const onEditKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter") { e.preventDefault(); handleEditCommit(editingIdx!); }
      else if (e.key === "Escape") { setEditingIdx(null); }
      else if (e.key === "/" && onSlashRequest) {
        slashPendingRef.current = true;
        const input = e.currentTarget;
        chipSlashCallbacks.set(input, (signalName: string) => {
          const pos = input.selectionStart ?? input.value.length;
          const slashPos = input.value.lastIndexOf("/", pos);
          const before = slashPos >= 0 ? input.value.slice(0, slashPos) : input.value;
          const after = input.value.slice(pos);
          input.value = before + `{${signalName}}` + after;
          input.focus();
          input.setSelectionRange(before.length + signalName.length + 2, before.length + signalName.length + 2);
        });
        requestAnimationFrame(() => onSlashRequest(input));
      }
    },
    [handleEditCommit, editingIdx, onSlashRequest],
  );

  const hasExpr = segments.some((s) => s.type === "expr");

  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: 4, alignItems: "center", flex: 1 }}>
      {segments.length === 0 && (
        <input
          ref={(el) => { if (el) textInputRefs.current.set(0, el); }}
          type="text"
          defaultValue=""
          onKeyDown={(e) => onTextKeyDown(0, e)}
          onBlur={() => commitText(0)}
          placeholder="输入文本，/ 插入信号"
          aria-label="Text input"
          className={styles.emptyInput}
        />
      )}
      {segments.map((seg, i) => {
        if (editingIdx === i) {
          return (
            <input
              key={seg.type + "-" + seg.value + "-editing"} ref={editInputRef} type="text"
              aria-label="Edit segment"
              defaultValue={seg.type === "expr" ? `{${seg.value}}` : seg.value}
              onKeyDown={onEditKeyDown} onBlur={() => handleEditCommit(i)}
              style={{ ...CHIP_BASE, flex: "0 1 auto", minWidth: 60, outline: "2px solid transparent", borderColor: "var(--ol-color-primary)", background: "var(--ol-surface)", color: "var(--ol-gray-800)" }}
            />
          );
        }
        if (seg.type === "expr") {
          return (
            <button type="button" key={seg.type + "-" + seg.value} style={{ ...EXPR_CHIP_STYLE, cursor: "pointer" }} onClick={(e) => { if ((e.target as HTMLElement).getAttribute('data-action') === 'delete-segment') { handleDelete(i); } else { setEditingIdx(i); } }} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); if ((e.target as HTMLElement).getAttribute('data-action') === 'delete-segment') { handleDelete(i); } else { setEditingIdx(i); } } }} title="点击编辑">
              {`{${seg.value}}`}
              <span data-action="delete-segment" style={{ ...CHIP_DELETE_BTN, color: "var(--ol-color-primary)" }} title="删除">&times;</span>
            </button>
          );
        }
        // Text segment: inline input
        return (
          <input
            key={seg.type + "-" + seg.value}
            ref={(el) => { if (el) textInputRefs.current.set(i, el); else textInputRefs.current.delete(i); }}
            type="text"
            aria-label={`Segment ${i}`}
            defaultValue={seg.value}
            onKeyDown={(e) => onTextKeyDown(i, e)}
            onBlur={() => commitText(i)}
            className={styles.textSegment}
            style={{
              minWidth: hasExpr ? 40 : 120,
              border: hasExpr ? "1px solid transparent" : "1px solid var(--ol-border)",
              borderRadius: hasExpr ? 6 : 4,
            }}
          />
        );
      })}
    </div>
  );
}

// ── BoolPropRow: 布尔属性行（开关 ↔ 表达式互斥视图） ────────────────────────

function BoolPropRow({
  label,
  value,
  onChange,
  onSlash,
}: {
  label: string;
  value: string | boolean | undefined;
  onChange: (v: boolean | string) => void;
  onSlash?: (inputEl: HTMLInputElement) => void;
}) {
  const isExpr = typeof value === "string";
  const [editingExpr, setEditingExpr] = useState(false);
  const [exprText, setExprText] = useState("");
  // "/" 按下置 true 的闩锁（镜像 ChipEditor slashPendingRef）：picker 交互期间
  // （mousedown 移焦 → blur）的 onBlur 不得提交插入前文本——否则行先以中间值
  // 提交并退回芯片态、输入框卸载，随后的选中插入落在已卸载组件上被丢弃。
  // 闩锁由 blur 守卫或插入回调两者先到者消费。
  const slashPendingRef = useRef(false);

  const commitExpr = useCallback(() => {
    const text = exprText.trim();
    if (text) onChange(text.startsWith("{") ? text : `{${text}}`);
    else onChange(true); // 空输入 → 回开关态（开）
    setEditingExpr(false);
    setExprText("");
  }, [exprText, onChange]);

  if (isExpr) {
    // 表达式形态：紫色芯片（点击编辑 / × 删除回开关态）。
    // 点击目标带 data-action="delete-segment"（× 号）→ 删除回开关态，否则进编辑
    // ——与 ChipEditor 芯片同一模式（键盘 Enter/Space 同判）。
    return (
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span style={{ fontSize: 12, color: "var(--ol-text-secondary)", minWidth: 50, flexShrink: 0 }}>{label}</span>
        <button type="button"
          onClick={(e) => {
            if ((e.target as HTMLElement).getAttribute("data-action") === "delete-segment") onChange(true);
            else { setExprText(value.slice(1, -1)); setEditingExpr(true); }
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              if ((e.target as HTMLElement).getAttribute("data-action") === "delete-segment") onChange(true);
              else { setExprText(value.slice(1, -1)); setEditingExpr(true); }
            }
          }}
          style={{ ...EXPR_CHIP_STYLE, cursor: "pointer" }} title="点击编辑">
          {value}
          <span data-action="delete-segment" style={CHIP_DELETE_BTN} title="删除">&times;</span>
        </button>
      </div>
    );
  }
  if (editingExpr) {
    return (
      <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span style={{ fontSize: 12, color: "var(--ol-text-secondary)", minWidth: 50, flexShrink: 0 }}>{label}</span>
        <input type="text" aria-label={`${label}-expression`} value={exprText}
          autoFocus
          onChange={(e) => setExprText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") commitExpr();
            else if (e.key === "Escape") setEditingExpr(false);
            else if (e.key === "/" && onSlash) {
              e.preventDefault();
              slashPendingRef.current = true;
              const input = e.currentTarget;
              // 复用 chipSlashCallbacks WeakMap 机制（父级 handleSlashSelect 分发）。
              // "/" 不进值域（受控输入）：记录按下时光标位置，picker 选中后按
              // 「前缀 + 裸信号名 + 后缀」光标插入——与 ChipEditor 文本段
              // （onEditKeyDown）语义一致；花括号由 commitExpr 提交时统一包裹，
              // 避免整段包花括号产生 {前缀{sig}} 非法源码（I1）。
              const slashIndex = input.selectionStart ?? input.value.length;
              chipSlashCallbacks.set(input, (signalName: string) => {
                slashPendingRef.current = false; // 插入成功 → 消费闩锁，Enter/blur 正常提交
                const current = input.value;
                setExprText(current.slice(0, slashIndex) + signalName + current.slice(slashIndex));
                requestAnimationFrame(() => input.focus());
              });
              requestAnimationFrame(() => onSlash(input));
            }
          }}
          onBlur={() => {
            // blur 竞态守卫（镜像 ChipEditor slashPendingRef）：picker 交互期间
            // 的 blur 不提交中间文本、不退出编辑态——等选中回调消费闩锁。
            if (slashPendingRef.current) { slashPendingRef.current = false; return; }
            commitExpr();
          }}
          style={{ flex: 1, padding: "3px 6px", fontSize: 12, border: "1px solid var(--ol-color-primary)", borderRadius: 4, fontFamily: "var(--dp-font-mono, monospace)" }} />
      </label>
    );
  }
  // 开关形态：label 只包住开关（一个原生控件），「表达式」按钮在 label 外——
  // 避免一个 label 关联多个控件（a11y）。
  const isOn = value === true;
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
      <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span style={{ fontSize: 12, color: "var(--ol-text-secondary)", minWidth: 50, flexShrink: 0 }}>{label}</span>
        <input type="checkbox" aria-label={`${label}-toggle`} checked={isOn}
          onChange={(e) => onChange(e.target.checked)} />
      </label>
      <button type="button" onClick={() => setEditingExpr(true)} style={{ fontSize: 11, color: "var(--ol-text-secondary)", border: "1px solid var(--ol-border)", borderRadius: 4, padding: "1px 6px", background: "transparent", cursor: "pointer" }}>
        表达式
      </button>
    </div>
  );
}

function ChipPropRow({
  label,
  value,
  onChange,
  onSlash,
}: {
  label: string;
  value: string | boolean;
  onChange: (v: string) => void;
  onSlash?: (inputEl: HTMLInputElement) => void;
}) {
  return (
    <div style={{ display: "flex", alignItems: "flex-start", gap: 6 }}>
      <span
        style={{
          fontSize: 12,
          color: "var(--ol-text-secondary)",
          minWidth: 50,
          flexShrink: 0,
          paddingTop: 4,
        }}
      >
        {label}
      </span>
      <ChipEditor
        value={String(value)}
        onChange={onChange}
        onSlashRequest={onSlash}
      />
    </div>
  );
}

function PropRow({
  label,
  value,
  onChange,
  onSlash,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  onSlash?: (inputEl: HTMLInputElement) => void;
}) {
  const inputRef = useRef<HTMLInputElement>(null);

  // Track the filter text after "/" when picker is potentially open.
  // We don't manage picker state here — the parent does.
  // But we need to let the parent know about text changes after "/".
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "/" && onSlash) {
        // Let the input update first, then notify
        const input = e.currentTarget;
        requestAnimationFrame(() => {
          onSlash(input);
        });
      }
    },
    [onSlash]
  );

  return (
    <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
      <span
        style={{
          fontSize: 12,
          color: "var(--ol-text-secondary)",
          minWidth: 50,
          flexShrink: 0,
        }}
      >
        {label}
      </span>
      <input
        ref={inputRef}
        type="text"
        aria-label={label}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={handleKeyDown}
        style={{
          flex: 1,
          padding: "3px 6px",
          fontSize: 12,
          border: "1px solid var(--ol-border)",
          borderRadius: 4,
          outline: "2px solid transparent",
          fontFamily: "var(--dp-font-mono, monospace)",
        }}
        onFocus={(e) => {
          e.target.style.borderColor = "var(--ol-color-primary)";
        }}
        onBlur={(e) => {
          e.target.style.borderColor = "var(--ol-border)";
        }}
      />
    </label>
  );
}

function getRelevantProps(tag: string): string[] {
  const map: Record<string, string[]> = {
    input: ["type", "placeholder", "value", "name", "checked", "disabled", "required", "readonly"],
    button: ["label", "type", "name", "disabled"],
    img: ["src", "alt"],
    a: ["href", "label", "target", "disabled"],
    textarea: ["placeholder", "value", "name", "rows", "disabled", "readonly", "required"],
    select: ["name", "disabled", "required"],
    label: ["label"],
    h1: ["label"], h2: ["label"], h3: ["label"], p: ["label"],
  };
  return map[tag] ?? ["label"];
}
