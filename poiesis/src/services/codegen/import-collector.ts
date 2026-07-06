// poiesis/src/omni-gen/import-collector.ts
// Collects import statements from component tags, store refs, and cross-tab function references.

import type { CodeTab } from "../../core/types";

export function collectImports(
  pascalCaseTags: string[],
  storeRefs: string[],
  codeTabs: CodeTab[],
): string[] {
  const imports: string[] = ["import omniview;"];

  for (const tag of pascalCaseTags) {
    imports.push(`import { ${tag} } from "./components/${tag}";`);
  }

  for (const storeName of storeRefs) {
    imports.push(`import { ${storeName} } from "./stores/${storeName}";`);
  }

  // Cross-tab function references
  const logicFnMap = new Map<string, string>(); // fnName → tabLabel
  for (const tab of codeTabs) {
    if ((tab.type === "logic" || tab.type === "event") && tab.id !== "main") {
      for (const el of tab.graph.elements) {
        if (el.tag === "customFn" && el.config) {
          const fnName: unknown = el.config.fnName;
          if (typeof fnName === "string") {
            logicFnMap.set(fnName, tab.label);
          }
        }
      }
    }
  }

  return imports;
}
