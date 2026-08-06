// 信号分析器（spec 4.2）：对组件源码文本做词法级分析，按信号聚合其界面用途。
// 只看 view 树（@if/@else/@for/@empty）+ 插值/bind/class/style/computed 依赖；
// fn 内的 if 是逻辑分支不驱动渲染结构，不参与分析。

import type { ConditionComparison, ComparisonOp } from "./synthesize-condition";

export interface IfBranch {
  exprText: string | null;   // else 分支为 null
  isElse: boolean;
  comparisons: ConditionComparison[];  // 空 = 复杂条件（UI 置灰胶囊）
}
export interface IfChain { branches: IfBranch[] }
export interface ForUsage {
  collectionText: string;
  hasEmpty: boolean;
  viaTransform: boolean;     // collection 含 filter/map/函数调用
}
export interface SignalUsage {
  key: string;
  declaredType: string;
  conditions: IfChain[];
  loops: ForUsage[];
  uses: { interpolations: number; binds: string[]; classGates: number; styleRefs: number; computedDeps: number };
}
export interface SignalDecl { key: string; type: string }

const COMPARISON_RE = /^(.+?)\s*(==|!=|>=|<=|>|<)\s*([^&|!]+)$/;

/** expr 文本是否提及该信号 key（标识符边界匹配，避免 item ⊂ items 误报） */
function mentionsKey(expr: string | null, key: string): boolean {
  if (!expr) return false;
  // 信号 key 只含 \w 与点（Store.sig），无需转义
  const re = new RegExp(`(^|[^\\w.])${key}([^\\w]|$)`);
  return re.test(expr);
}

/** 把条件表达式拆成「参与信号 + 比较形式」；只认单信号简单比较 */
function parseComparisons(expr: string, signalKeys: string[]): ConditionComparison[] {
  const trimmed = expr.trim();
  // 裸信号（bool）或 !sig
  const bare = trimmed.match(/^!?\s*([A-Za-z_][\w.]*)$/);
  if (bare) {
    const sig = bare[1];
    if (signalKeys.includes(sig)) return [{ sig, op: null }];
    return [];
  }
  const m = trimmed.match(COMPARISON_RE);
  if (!m) return [];
  const left = m[1].trim();
  const op = m[2] as ComparisonOp;
  const right = m[3].trim();
  // 左边必须是信号；右边必须是无函数调用的字面量/变量（有调用 → 复杂）
  if (!signalKeys.includes(left)) return [];
  if (/[a-zA-Z_]\s*\(/.test(right)) return [];
  // 字面量剥引号：'loading' → loading；数字/bool 原样
  const literal = right.replace(/^["']|["']$/g, "");
  return [{ sig: left, op, literal }];
}

/** 提取 view 块文本（第一个 view { ... } 配平括号） */
function extractViewBlock(source: string): string {
  const start = source.search(/\bview\s*\{/);
  if (start < 0) return "";
  const openIdx = source.indexOf("{", start);
  let depth = 0;
  let inString = false;
  let quote = "";
  for (let i = openIdx; i < source.length; i++) {
    const ch = source[i];
    if (inString) {
      if (ch === "\\") { i++; continue; }
      if (ch === quote) inString = false;
      continue;
    }
    if (ch === '"' || ch === "'") { inString = true; quote = ch; continue; }
    if (ch === "{") depth++;
    if (ch === "}") { depth--; if (depth === 0) return source.slice(openIdx + 1, i); }
  }
  return "";
}

/** 扫描 view 文本，逐段识别 @if 链 / @for / 插值 / bind / class / style */
function scanView(view: string, signalKeys: string[], storeSignalKeys: string[]): {
  ifChains: IfChain[];
  loops: ForUsage[];
  uses: Record<string, { interpolations: number; binds: string[]; classGates: number; styleRefs: number }>;
} {
  const allKeys = [...signalKeys, ...storeSignalKeys];
  const uses: Record<string, { interpolations: number; binds: string[]; classGates: number; styleRefs: number }> = {};
  for (const k of allKeys) uses[k] = { interpolations: 0, binds: [], classGates: 0, styleRefs: 0 };

  const ifChains: IfChain[] = [];
  const loops: ForUsage[] = [];

  // 简化策略：按位置切分指令段（顺序扫描，分别处理 if 链、for、插值计数）
  // ── 1) 提取 @if 链 ──
  const ifSegments: Array<{ expr: string; isElse: boolean }> = [];
  const segmentRe = /@(?:if|else(?:\s*@?if)?)\s*([^{]*?)\s*\{/g;
  let m: RegExpExecArray | null;
  while ((m = segmentRe.exec(view)) !== null) {
    ifSegments.push({ expr: m[1].trim(), isElse: view.slice(segmentRe.lastIndex - m[0].length, segmentRe.lastIndex).includes("else") });
  }
  // 组装链：连续 if/else 段归一条链（段间无其他指令）
  let chain: IfChain | null = null;
  for (const seg of ifSegments) {
    if (!seg.isElse) {
      if (chain) ifChains.push(chain);
      chain = { branches: [] };
    }
    if (!chain) continue;  // 防御：孤立 @else（不应出现）
    const comparisons = parseComparisons(seg.expr, allKeys);
    chain.branches.push({ exprText: seg.expr || null, isElse: seg.isElse, comparisons });
  }
  if (chain) ifChains.push(chain);

  // ── 2) @for ──
  const forRe = /@for\s+([A-Za-z_]\w*)\s+in\s+([^{]*?)\s*(\{|key=)/g;
  while ((m = forRe.exec(view)) !== null) {
    const collectionRaw = m[2].trim();
    // 找该 @for 体后是否有 @empty
    const after = view.slice(forRe.lastIndex);
    const hasEmpty = /@empty\s*\{/.test(after.split(/\}@for|@if/)[0]);
    const viaTransform = /[a-zA-Z_]\s*\(/.test(collectionRaw);
    loops.push({ collectionText: collectionRaw, hasEmpty, viaTransform });
  }

  // ── 3) 插值 / bind / class / style 计数 ──
  const interpRe = /\{([^{}]+)\}/g;
  while ((m = interpRe.exec(view)) !== null) {
    for (const key of allKeys) {
      if (m[1].includes(key)) uses[key].interpolations++;
    }
  }
  const bindRe = /bind:(\w+)=\{([^}]+)\}/g;
  while ((m = bindRe.exec(view)) !== null) {
    for (const key of allKeys) {
      if (m[2].includes(key)) uses[key].binds.push(m[1]);
    }
  }
  const classGateRe = /class:\s*\{([^}]*)\}/g;
  while ((m = classGateRe.exec(view)) !== null) {
    for (const key of allKeys) {
      if (m[1].includes(key)) uses[key].classGates++;
    }
  }
  const styleRe = /style:\s*\{([^}]*)\}/g;
  while ((m = styleRe.exec(view)) !== null) {
    for (const key of allKeys) {
      if (m[1].includes(key)) uses[key].styleRefs++;
    }
  }

  return { ifChains, loops, uses };
}

/** 提取 computed 表达式中的信号依赖（computed x: T = expr;） */
function scanComputedDeps(source: string, signalKeys: string[]): Record<string, number> {
  const deps: Record<string, number> = {};
  for (const key of signalKeys) deps[key] = 0;
  const computedRe = /computed\s+\w+\s*:\s*[^=]+=\s*([^;]+);/g;
  let m: RegExpExecArray | null;
  while ((m = computedRe.exec(source)) !== null) {
    for (const key of signalKeys) {
      if (m[1].includes(key)) deps[key]++;
    }
  }
  return deps;
}

export function analyzeSignalUsage(
  componentSource: string,
  signals: SignalDecl[],
  storeSignals: SignalDecl[],
): { interfaceSignals: SignalUsage[]; dataSignals: SignalUsage[] } {
  const view = extractViewBlock(componentSource);
  const signalKeys = signals.map((s) => s.key);
  const storeKeys = storeSignals.map((s) => s.key);
  const allDecls: SignalDecl[] = [...signals, ...storeSignals];

  const { ifChains, loops, uses } = scanView(view, signalKeys, storeKeys);
  const computedDeps = scanComputedDeps(componentSource, signalKeys);

  const usages: SignalUsage[] = allDecls.map((decl) => {
    const u = uses[decl.key] || { interpolations: 0, binds: [], classGates: 0, styleRefs: 0 };
    return {
      key: decl.key,
      declaredType: decl.type,
      conditions: ifChains.filter((chain) =>
        chain.branches.some((b) =>
          b.comparisons.some((c) => c.sig === decl.key) || mentionsKey(b.exprText, decl.key),
        ),
      ),
      loops: loops.filter((l) => l.collectionText.includes(decl.key)),
      uses: {
        interpolations: u.interpolations,
        binds: u.binds,
        classGates: u.classGates,
        styleRefs: u.styleRefs,
        computedDeps: computedDeps[decl.key] || 0,
      },
    };
  });

  const interfaceSignals = usages.filter((u) => u.conditions.length > 0 || u.loops.length > 0);
  const dataSignals = usages.filter((u) => u.conditions.length === 0 && u.loops.length === 0);
  return { interfaceSignals, dataSignals };
}
