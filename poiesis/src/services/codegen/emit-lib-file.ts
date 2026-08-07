// poiesis/src/services/codegen/emit-lib-file.ts
// lib 文件发射器：imports → structs → types → fns（spec §6.4）。
// 纯逻辑文件不发射 `import omniview;`（learn-omniview 实证）。

import { CodeWriter } from "./source-emitter";
import { compileCodeFlowGraph } from "./graph-compiler";
import type { LibFileDef } from "./types";
import type { CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types";

/** 发射整个 lib 文件（code tab 保存 / round-trip 共用）。
 *  graphs 按 fn 名键（parse 侧 codeFlowGraphs / 保存侧画布 writeback 均为此形态）。 */
export function emitLibFile(
  def: LibFileDef,
  graphs: Record<string, { elements: CodeblockElementData[]; wires: WireData[] }>,
): string {
  const w = new CodeWriter();
  for (const imp of def.imports) {
    // 纯逻辑文件不发射 import omniview;（存储形态是 token 无空格拼接 "importomniview;"）
    if (imp.replace(/\s+/g, "") === "importomniview;") continue;
    w.emit(imp);
  }
  if (def.imports.length > 0) w.blank();

  const root = emitRootDeclarations(def);

  const blocks: string[] = [];
  for (const f of def.fns) {
    const graph = graphs[f.name];
    if (!graph || !f.pid) continue;
    // def.fns 修饰符（pub/isAsync/declKind）补到 fn 入口元素 props 上——解析侧
    // synthesizeFnEntry 的 propsExtras 未携带 pub/isAsync，若不补，保存会静默剥掉
    // `pub async`（async_demo.omni 实证）。元素自身已有值（画布 FnInspector 改写）
    // 时以元素为准（spread 在后）。
    const elements = graph.elements.map((el) =>
      el.id === f.pid
        ? { ...el, props: { pub: f.pub, isAsync: f.isAsync, declKind: f.declKind ?? "fn", ...el.props } }
        : el,
    );
    const compiled = compileCodeFlowGraph(elements, graph.wires, f.pid);
    if (compiled) blocks.push(compiled);
  }
  // 组装：imports + structs/types + fns（每段恰好一次，段间/fn 间空行分隔）。
  // 注：brief 原稿 `w.emit(root.trimEnd())` 会把 ROOT 段写进 writer，而 parts[1] 又
  // 追加 rootText——ROOT 段会被发射两次（struct 重复定义）。此处 writer 只承载
  // imports，ROOT 段由 parts[1] 单次组装（brief 注释明示的硬性顺序不受影响）。
  const parts = [w.toString().trimEnd(), root.trim(), blocks.join("\n\n")].filter((p) => p.length > 0);
  return parts.join("\n\n") + "\n";
}

/** ROOT 声明发射（lib 文件的 structs/types 段；组件/store 文件头部前置共用）。
 *  不发射 imports——组件/store 的 imports 由 pipeline import-collector 处理。 */
export function emitRootDeclarations(def: LibFileDef): string {
  const w = new CodeWriter();
  for (const s of def.structs) {
    w.emit(`${s.pub ? "pub " : ""}struct${s.nominal ? " nominal" : ""} ${s.name} {`);
    w.indent();
    for (const f of s.fields) {
      if (f.type) w.emit(`${f.name}: ${f.type},`);
    }
    w.dedent();
    w.emit(`}`);
    w.blank();
  }
  for (const t of def.types) {
    w.emit(`${t.pub ? "pub " : ""}type ${t.name} = ${t.value};`);
    w.blank();
  }
  return w.toString();
}
