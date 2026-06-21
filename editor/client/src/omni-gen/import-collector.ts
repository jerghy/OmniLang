// editor/client/src/omni-gen/import-collector.ts
// Collects import statements from component tags, store refs, and cross-tab function references.

import type { CodeTab } from "../codegraph/types";

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
    if (tab.type === "logic" && tab.id !== "main") {
      for (const el of tab.graph.elements) {
        if (el.tag === "customFn" && el.config?.fnName) {
          logicFnMap.set(el.config.fnName as string, tab.label);
        }
      }
    }
  }

  return imports;
}
