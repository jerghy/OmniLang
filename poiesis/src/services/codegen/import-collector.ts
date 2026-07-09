// poiesis/src/services/codegen/import-collector.ts
// Collects import statements from component tags, store refs, and cross-tab function references.

import type { CodeTab } from "../../core/types";

export function collectImports(
  pascalCaseTags: string[],
  storeRefs: string[],
  codeTabs: CodeTab[],
  designName?: string,
  hasDesignClassUsage?: boolean,
): string[] {
  const imports: string[] = ["import omniview;"];

  // Only import design if name is provided AND it's actually used
  if (designName && hasDesignClassUsage) {
    imports.push(`import { ${designName} } from "./design.omni";`);
  }

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
        if (!("tag" in el)) continue;
        if (el.tag === "customFn" && "config" in el && el.config) {
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
