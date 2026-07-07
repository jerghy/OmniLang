// poiesis/src/services/codegen/wire-analyzer.ts
// Analyzes wire connections between elements to extract signal/ref/dep bindings.

import type { ElementData, WireData } from "../../core/types";
import type { BindingResult } from "./types";

export function analyzeWires(elements: ElementData[], wires: WireData[]): BindingResult {
  const signalBindings = new Map<string, Array<{ elementId: string; propName: string }>>();
  const storeRefs = new Map<string, string[]>();
  const computedDeps = new Map<string, string[]>();

  const elMap = new Map(elements.map(e => [e.id, e]));

  for (const w of wires) {
    const src = elMap.get(w.fromId);
    const tgt = elMap.get(w.toId);
    if (!src || !tgt) continue;
    if (!("tag" in src) || !("tag" in tgt)) continue;

    // signal.value → view.prop
    if ((src.tag === "signal" || src.tag === "computed") && w.fromPort === "value" && tgt.nodeType === "view") {
      const name = src.props.name as string;
      if (!signalBindings.has(name)) signalBindings.set(name, []);
      signalBindings.get(name)!.push({ elementId: tgt.id, propName: w.toPort });
    }
    // store.items → view
    if (src.tag === "store" && w.fromPort === "items" && tgt.nodeType === "view") {
      const name = src.props.name as string;
      if (!storeRefs.has(name)) storeRefs.set(name, []);
      storeRefs.get(name)!.push(tgt.id);
    }
    // → computed.deps
    if (tgt.tag === "computed" && w.toPort === "deps") {
      const srcName = src.props.name as string;
      const tgtName = tgt.props.name as string;
      if (!computedDeps.has(tgtName)) computedDeps.set(tgtName, []);
      computedDeps.get(tgtName)!.push(srcName);
    }
  }

  // unwired props
  const unwiredProps: BindingResult["unwiredProps"] = [];
  const wiredProps = new Set<string>();
  for (const [, bindings] of signalBindings) {
    for (const b of bindings) wiredProps.add(`${b.elementId}:${b.propName}`);
  }
  for (const el of elements.filter(e => e.nodeType === "view")) {
    for (const key of Object.keys(el.props)) {
      if (!wiredProps.has(`${el.id}:${key}`) && !key.startsWith("on")) {
        unwiredProps.push({ elementId: el.id, tag: el.tag, propName: key });
      }
    }
  }

  return { signalBindings, storeRefs, computedDeps, unwiredProps };
}
