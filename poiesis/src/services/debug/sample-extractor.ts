// 代码样本提取（spec D10）：对 Array<复合类型> 信号，从组件 fn 源码中
// `name = [ ... ]` 的赋值提取数组字面量作为调试候选（struct 数组无法文本输入）。
// 原样注入不解析；含函数调用/展开的赋值跳过（无法确定静态内容）。

export interface SampleCandidate {
  label: string;
  literal: string;
}

const MAX_SAMPLES = 3;

/** 提取 `name = [` 后到配平 `]` 的数组字面量文本；失败（含调用/展开/无法配平）返回 null */
function extractArrayLiteral(fnSource: string, name: string): string | null {
  // 匹配 name = [ 或 name = [ ... （允许空白）
  const startRe = new RegExp(`\\b${name}\\s*=\\s*\\[`);
  const start = fnSource.search(startRe);
  if (start < 0) return null;
  const openIdx = fnSource.indexOf("[", start);
  if (openIdx < 0) return null;

  // 平衡括号扫描
  let depth = 0;
  let inString = false;
  let quote = "";
  for (let i = openIdx; i < fnSource.length; i++) {
    const ch = fnSource[i];
    if (inString) {
      if (ch === "\\") { i++; continue; }
      if (ch === quote) inString = false;
      continue;
    }
    if (ch === '"' || ch === "'") { inString = true; quote = ch; continue; }
    if (ch === "[") { depth++; continue; }
    if (ch === "]") {
      depth--;
      if (depth === 0) {
        const literal = fnSource.slice(openIdx, i + 1);
        // 展开运算符 ... 无法确定静态内容，跳过（测试契约：赋值含展开 → 跳过）
        if (literal.includes("...")) return null;
        // 字面量内不允许出现函数调用/方法调用（含 ( 或 .filter 等）
        if (/[a-zA-Z_]\s*\(/.test(literal)) return null;
        if (/\.\w+\(/.test(literal)) return null;
        return literal;
      }
    }
  }
  return null;
}

export function extractAssignSamples(fnSources: string[], signalName: string): SampleCandidate[] {
  const seen = new Set<string>();
  const samples: SampleCandidate[] = [];
  for (const src of fnSources) {
    if (samples.length >= MAX_SAMPLES) break;
    const literal = extractArrayLiteral(src, signalName);
    if (!literal || seen.has(literal)) continue;
    seen.add(literal);
    const itemCount = (literal.match(/\{[^}]*\}/g) || []).length;
    // label：函数名（从 "fn <name>" 或调用处推）——简化为项数
    const label = `样本 ${itemCount} 项`;
    samples.push({ label, literal });
  }
  return samples;
}
