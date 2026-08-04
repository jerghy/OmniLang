// poiesis/src/components/canvas/code/NodeSearchMenu.tsx
// 可搜索节点创建菜单（复刻 ComfyUI NodeSearchBox：自动聚焦/过滤/高亮/键盘导航）。
import { useEffect, useMemo, useRef, useState } from "react";
import { MENU_SECTIONS } from "./flowConstants";
import styles from "./NodeSearchMenu.module.css";

export interface FilteredSection {
  section: string;
  items: Array<{ label: string; tag: string; shortcut?: string }>;
}

/** 子串匹配：中文 label / 英文 tag / 快捷键别名；空 query 返回全部。 */
export function filterMenuItems(query: string): FilteredSection[] {
  const q = query.trim().toLowerCase();
  if (!q) return MENU_SECTIONS.map((s) => ({ section: s.label, items: s.items }));
  return MENU_SECTIONS.map((s) => ({
    section: s.label,
    items: s.items.filter((i) =>
      i.label.toLowerCase().includes(q) ||
      i.tag.toLowerCase().includes(q) ||
      (i.shortcut ?? "").toLowerCase().includes(q),
    ),
  })).filter((s) => s.items.length > 0);
}

/** 命中子串高亮（转义后渲染）。 */
function highlight(text: string, query: string): React.ReactNode {
  const q = query.trim();
  if (!q) return text;
  const idx = text.toLowerCase().indexOf(q.toLowerCase());
  if (idx < 0) return text;
  return (
    <>
      {text.slice(0, idx)}
      <mark className={styles.hit}>{text.slice(idx, idx + q.length)}</mark>
      {text.slice(idx + q.length)}
    </>
  );
}

interface Props {
  x: number;
  y: number;
  onSelect: (tag: string) => void;
  onClose: () => void;
}

export function NodeSearchMenu({ x, y, onSelect, onClose }: Props) {
  const [query, setQuery] = useState("");
  const [activeIdx, setActiveIdx] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const rootRef = useRef<HTMLDivElement>(null);

  const sections = useMemo(() => filterMenuItems(query), [query]);
  const flat = useMemo(() => sections.flatMap((s) => s.items), [sections]);

  // 自动聚焦
  useEffect(() => { inputRef.current?.focus(); }, []);

  // 点击菜单外关闭
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) onClose();
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [onClose]);

  // 过滤变化时重置选中
  useEffect(() => { setActiveIdx(0); }, [query]);

  // 选中项滚动可见
  useEffect(() => {
    const active = listRef.current?.querySelector(`[data-menu-index="${activeIdx}"]`);
    active?.scrollIntoView?.({ block: "nearest" });
  }, [activeIdx]);

  const select = (tag: string) => {
    onSelect(tag);
    onClose();
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") { e.preventDefault(); setActiveIdx((i) => Math.min(i + 1, flat.length - 1)); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setActiveIdx((i) => Math.max(i - 1, 0)); }
    else if (e.key === "Enter") { e.preventDefault(); if (flat[activeIdx]) select(flat[activeIdx].tag); }
    else if (e.key === "Escape") { e.preventDefault(); onClose(); }
  };

  const vw = window.innerWidth;
  const vwH = window.innerHeight;
  const left = Math.min(x, vw - 260);
  const top = Math.min(y, vwH - 320);

  return (
    <div ref={rootRef} className={styles.menu} style={{ left, top }} role="listbox" id="node-search-results" onContextMenu={(e) => e.preventDefault()}>
      <input
        ref={inputRef}
        className={styles.search}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={onKeyDown}
        placeholder="搜索节点…"
        aria-label="搜索节点"
        role="combobox"
        aria-expanded="true"
        aria-controls="node-search-results"
      />
      <div ref={listRef} className={styles.list}>
        {flat.length === 0 ? (
          <div className={styles.empty}>无匹配节点</div>
        ) : (
          sections.map((sec) => (
            <div key={sec.section}>
              <div className={styles.sectionLabel}>{sec.section}</div>
              {sec.items.map((item) => {
                const idx = flat.indexOf(item);
                return (
                  <button
                    type="button"
                    key={item.tag}
                    data-menu-index={idx}
                    role="option"
                    aria-selected={idx === activeIdx}
                    className={`${styles.item} ${idx === activeIdx ? styles.itemActive : ""}`}
                    onClick={() => select(item.tag)}
                    onMouseEnter={() => setActiveIdx(idx)}
                  >
                    <span>{highlight(item.label, query)}</span>
                    {item.shortcut && <span className={styles.shortcut}>{item.shortcut}</span>}
                  </button>
                );
              })}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
