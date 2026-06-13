/**
 * 统一搜索索引 (Ctrl+Shift+F)。
 *
 * 跨所有信息源搜索:
 *   - 文件（文件名 + 内容）
 *   - 组件面板（OmniView 标签名 + 描述）
 *   - 代码片段（片段名 + 描述 + 关键词）
 *   - 数据连接器（Store 名 + endpoint URL）
 *   - 设计令牌（令牌名 + 值）
 *   - 连线（连线 ID / 端口名）
 */

import { BUILTIN_SNIPPETS } from "../snippets/builtin";
import type { SearchIndexEntry, SearchIndexSnapshot } from "../omniJson/types";

// ── Types ─────────────────────────────────────────

export type SearchSource =
  | "file"
  | "component"
  | "snippet"
  | "connector"
  | "token"
  | "wire"
  | "command";

export interface SearchResult {
  id: string;
  source: SearchSource;
  title: string;
  subtitle: string;
  icon: string;
  /** Action on select (e.g. navigate, insert, focus) */
  action: SearchAction;
  /** Relevance score (0-1) */
  score: number;
}

export interface SearchAction {
  type: "open-file" | "insert-snippet" | "select-element" | "focus-wire" | "navigate-tab" | "run-command";
  payload: string;
  label: string;
}

// ── Index builders ────────────────────────────────

function score(query: string, text: string): number {
  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();

  // 精确匹配
  if (lowerText === lowerQuery) return 1.0;
  // 前缀匹配
  if (lowerText.startsWith(lowerQuery)) return 0.9;
  // 词首字母匹配（首字母缩写）
  const words = lowerText.split(/[\s_-]+/);
  const initials = words.map((w) => w[0] || "").join("");
  if (initials === lowerQuery) return 0.85;
  // 包含匹配
  if (lowerText.includes(lowerQuery)) return 0.7;
  // 模糊匹配（每个查询字符按序出现）
  let qi = 0;
  for (const ch of lowerText) {
    if (ch === lowerQuery[qi]) qi++;
    if (qi >= lowerQuery.length) return 0.5;
  }
  return 0;
}

// ── 文件索引 ─────────────────────────────────────

export function searchFiles(
  query: string,
  files: { name: string; path: string; type: string }[],
  fileContents?: Map<string, string>
): SearchResult[] {
  const results: SearchResult[] = [];

  for (const f of files) {
    if (f.type !== "file") continue;
    const nameScore = score(query, f.name);
    let contentScore = 0;

    if (fileContents && nameScore < 0.8) {
      const content = fileContents.get(f.path) || "";
      contentScore = score(query, content.slice(0, 5000));
    }

    const total = Math.max(nameScore, contentScore * 0.6);
    if (total < 0.3) continue;

    results.push({
      id: `file:${f.path}`,
      source: "file",
      title: f.name,
      subtitle: f.path,
      icon: f.name.endsWith(".omni") ? "📄" : "📝",
      action: { type: "open-file", payload: f.path, label: "打开文件" },
      score: total,
    });
  }

  return results;
}

// ── 组件索引 ─────────────────────────────────────

export function searchComponents(query: string): SearchResult[] {
  const components = [
    "Container", "Flex", "Grid", "Text", "Heading", "Image",
    "Button", "Input", "TextArea", "Select", "List", "ListItem",
    "Card", "Form", "Divider", "Spinner", "Progress", "Alert",
    "Link", "Checkbox", "Radio", "Slider", "Icon", "Badge",
    "Modal", "Dropdown", "Tabs", "Table", "Avatar",
  ];

  const results: SearchResult[] = [];
  for (const comp of components) {
    const s = score(query, comp);
    if (s < 0.3) continue;
    results.push({
      id: `component:${comp}`,
      source: "component",
      title: `<${comp}>`,
      subtitle: "OmniView 组件",
      icon: "🧩",
      action: { type: "select-element", payload: comp, label: "拖入画布" },
      score: s,
    });
  }
  return results;
}

// ── 代码片段索引 ─────────────────────────────────

export function searchSnippets(query: string): SearchResult[] {
  const results: SearchResult[] = [];
  for (const snip of BUILTIN_SNIPPETS) {
    const s = Math.max(
      score(query, snip.label),
      score(query, snip.description)
    );
    if (s < 0.3) continue;
    results.push({
      id: `snippet:${snip.id}`,
      source: "snippet",
      title: snip.label,
      subtitle: `${snip.context} · ${snip.description}`,
      icon: "💡",
      action: { type: "insert-snippet", payload: snip.body, label: "插入片段" },
      score: s,
    });
  }
  return results;
}

// ── 令牌索引 ─────────────────────────────────────

export function searchTokens(
  query: string,
  tokens: { name: string; value: string; category: string }[]
): SearchResult[] {
  const results: SearchResult[] = [];
  for (const tok of tokens) {
    const s = Math.max(score(query, tok.name), score(query, tok.value));
    if (s < 0.3) continue;
    const icon = { colors: "🎨", spacing: "📏", typography: "🔤", radii: "⭕", shadows: "🌑" }[
      tok.category
    ] || "🎨";
    results.push({
      id: `token:${tok.name}`,
      source: "token",
      title: `--${tok.name}`,
      subtitle: `${tok.value} · ${tok.category}`,
      icon,
      action: { type: "navigate-tab", payload: "tokens", label: "查看令牌" },
      score: s,
    });
  }
  return results;
}

// ── 连线索引 ─────────────────────────────────────

export function searchWires(
  query: string,
  wires: { id: string; fromId: string; toId: string; fromPort: string; toPort: string }[]
): SearchResult[] {
  const results: SearchResult[] = [];
  for (const w of wires) {
    const haystack = `${w.id} ${w.fromId} ${w.toId} ${w.fromPort} ${w.toPort}`;
    const s = score(query, haystack);
    if (s < 0.3) continue;
    results.push({
      id: `wire:${w.id}`,
      source: "wire",
      title: `${w.fromPort} → ${w.toPort}`,
      subtitle: `${w.fromId} → ${w.toId}`,
      icon: "🔗",
      action: { type: "focus-wire", payload: w.id, label: "定位连线" },
      score: s,
    });
  }
  return results;
}

// ── Command 索引 ─────────────────────────────────

const BUILTIN_COMMANDS: { id: string; title: string; shortcut: string }[] = [
  { id: "compile", title: "编译当前文件", shortcut: "Ctrl+Enter" },
  { id: "undo", title: "撤销", shortcut: "Ctrl+Z" },
  { id: "save", title: "保存文件", shortcut: "Ctrl+S" },
  { id: "toggle-wire", title: "切换连线模式", shortcut: "" },
  { id: "toggle-snippets", title: "切换代码片段面板", shortcut: "" },
  { id: "toggle-tokens", title: "切换设计令牌面板", shortcut: "" },
  { id: "toggle-code", title: "切换设计/代码模式", shortcut: "" },
  { id: "deploy", title: "一键发布", shortcut: "" },
  { id: "search", title: "统一搜索", shortcut: "Ctrl+Shift+F" },
  { id: "focus-order", title: "显示焦点顺序", shortcut: "" },
];

export function searchCommands(query: string): SearchResult[] {
  const results: SearchResult[] = [];
  for (const cmd of BUILTIN_COMMANDS) {
    const s = Math.max(score(query, cmd.title), score(query, cmd.id));
    if (s < 0.3) continue;
    results.push({
      id: `command:${cmd.id}`,
      source: "command",
      title: cmd.title,
      subtitle: cmd.shortcut || "无快捷键",
      icon: "⌨️",
      action: { type: "run-command", payload: cmd.id, label: "执行" },
      score: s,
    });
  }
  return results;
}

// ── 统一搜索入口 ─────────────────────────────────

export interface SearchContext {
  files: { name: string; path: string; type: string }[];
  fileContents?: Map<string, string>;
  wires?: { id: string; fromId: string; toId: string; fromPort: string; toPort: string }[];
  tokens?: { name: string; value: string; category: string }[];
  /** M6: Precomputed search index from .omni.json (optional). */
  precomputedIndex?: SearchIndexSnapshot | null;
}

export function unifiedSearch(query: string, ctx: SearchContext): SearchResult[] {
  if (!query.trim()) return [];

  const allResults: SearchResult[] = [
    ...searchFiles(query, ctx.files, ctx.fileContents),
  ];

  // M6: Use precomputed index for components/snippets/commands if available
  if (ctx.precomputedIndex?.entries) {
    allResults.push(...searchPrecomputedIndex(query, ctx.precomputedIndex.entries));
  } else {
    allResults.push(...searchComponents(query));
    allResults.push(...searchSnippets(query));
    allResults.push(...searchCommands(query));
  }

  if (ctx.tokens) {
    allResults.push(...searchTokens(query, ctx.tokens));
  }
  if (ctx.wires) {
    allResults.push(...searchWires(query, ctx.wires));
  }

  // 按分数降序排列
  allResults.sort((a, b) => b.score - a.score);

  // 每组源最多显示 5 条
  const seen = new Map<SearchSource, number>();
  const deduped: SearchResult[] = [];
  for (const r of allResults) {
    const count = seen.get(r.source) || 0;
    if (count >= 5) continue;
    seen.set(r.source, count + 1);
    deduped.push(r);
  }

  return deduped;
}

// ── Precomputed index search (M6) ───────────────────

function searchPrecomputedIndex(query: string, entries: SearchIndexEntry[]): SearchResult[] {
  const lowerQuery = query.toLowerCase();
  const results: SearchResult[] = [];
  void lowerQuery; // Used below via closure in filter loops

  for (const entry of entries) {
    // Fast matching against precomputed keywords
    let bestScore = 0;
    for (const kw of entry.keywords) {
      const s = score(query, kw);
      if (s > bestScore) bestScore = s;
    }
    // Also score against title directly
    bestScore = Math.max(bestScore, score(query, entry.title));
    if (bestScore < 0.3) continue;

    const source: SearchSource = entry.source as SearchSource;
    results.push({
      id: entry.id,
      source,
      title: entry.title,
      subtitle: entry.subtitle,
      icon: entry.icon || SOURCE_ICON[source] || "📦",
      action: {
        type: entry.actionType as SearchResult["action"]["type"],
        payload: entry.actionPayload,
        label: source === "component" ? "选择元素" : source === "snippet" ? "插入片段" : "执行",
      },
      score: bestScore,
    });
  }

  return results;
}

const SOURCE_ICON: Partial<Record<SearchSource, string>> = {
  file: "📄",
  component: "🧩",
  snippet: "💡",
  connector: "🔌",
  token: "🎨",
  wire: "🔗",
  command: "⌨️",
};

// ── Source display config ─────────────────────────

export const SOURCE_CONFIG: Record<
  SearchSource,
  { label: string; color: string }
> = {
  file: { label: "文件", color: "#2563EB" },
  component: { label: "组件", color: "#7C3AED" },
  snippet: { label: "片段", color: "#059669" },
  connector: { label: "连接器", color: "#D97706" },
  token: { label: "令牌", color: "#DC2626" },
  wire: { label: "连线", color: "#4F46E5" },
  command: { label: "命令", color: "#6B7280" },
};
