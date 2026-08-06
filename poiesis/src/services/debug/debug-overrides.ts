// 文件级调试覆盖管线（spec 4.3）：ownerKey → 目标文件精确定位，
// 正则替换 (signal|prop) <name>: Type = 值 → 调试字面量；无默认值补 = 值。
// 文件级键控修复旧 flatDebug 拍平导致的跨组件同名信号冲突。

/**
 * DebugValueEntry 同构本地定义：store/types.ts 的同名 interface 未 export
 * （Task 1 遗留），此处本地定义避免触碰既有文件；字段与 store 模型一致。
 */
interface DebugValueEntry {
  text: string;
  valid: boolean;
}

export interface OverrideCtx {
  componentName: string;
  /** 独立 store 名 → 文件路径（含内联 store 的组件文件由前缀推导，不入此表） */
  storePaths: Record<string, string>;
}

export function resolveOwnerFile(ownerKey: string, ctx: OverrideCtx): string | null {
  // 内联 store "Comp.Store" → 组件文件
  if (ownerKey.includes(".")) {
    const comp = ownerKey.slice(0, ownerKey.indexOf("."));
    return comp === "main" ? "src/main.omni" : `src/components/${comp}.omni`;
  }
  // 独立 store
  if (ownerKey in ctx.storePaths) return ctx.storePaths[ownerKey];
  // 组件
  if (ownerKey === ctx.componentName) {
    return ownerKey === "main" ? "src/main.omni" : `src/components/${ownerKey}.omni`;
  }
  return null;
}

function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** 替换串中转义 $，防止调试字面量里的 $ 被 replace 当作分组引用 */
function escapeReplaceLiteral(str: string): string {
  return str.replace(/\$/g, "$$");
}

function replaceDecl(source: string, name: string, literal: string): string {
  const kw = "(signal|prop)";
  // 有默认值：捕获类型组 $2 保留原样，只换 = 后的值段。
  // 类型组用惰性 [^=;\n]+?：可跨空格捕获泛型（Map<String, String>），
  // 且停在首个 = 前、由 \s*=\s* 吞掉等号前空格（贪婪版会吞尾空格产生双空格）。
  const withDefault = new RegExp(
    `${kw}\\s+${escapeRegex(name)}\\s*:\\s*([^=;\\n]+?)\\s*=\\s*[^;\\n]+;`,
    "g",
  );
  if (withDefault.test(source)) {
    withDefault.lastIndex = 0;
    return source.replace(withDefault, `$1 ${name}: $2 = ${escapeReplaceLiteral(literal)};`);
  }
  // 无默认值：signal x: Type;（类型可能带泛型，用非贪婪到 ; 前）——补 = 值，类型保留
  const noDefault = new RegExp(
    `${kw}\\s+${escapeRegex(name)}\\s*:\\s*([^;\\n]+);`,
    "g",
  );
  noDefault.lastIndex = 0;
  return source.replace(noDefault, `$1 ${name}: $2 = ${escapeReplaceLiteral(literal)};`);
}

export function applyDebugOverrides(
  sources: Readonly<Record<string, string>>,
  debugValues: Readonly<Record<string, Record<string, DebugValueEntry>>>,
  ctx: OverrideCtx,
): Record<string, string> {
  const result: Record<string, string> = {};
  for (const path of Object.keys(sources)) {
    let text = sources[path];
    for (const [ownerKey, entries] of Object.entries(debugValues)) {
      const targetFile = resolveOwnerFile(ownerKey, ctx);
      if (targetFile !== path) continue;
      for (const [name, entry] of Object.entries(entries)) {
        if (!entry.valid) continue;
        const replaced = replaceDecl(text, name, entry.text);
        if (replaced !== text) text = replaced;
      }
    }
    result[path] = text;
  }
  return result;
}
