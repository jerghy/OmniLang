// editor/client/src/omni-gen/annotation-data.ts
// Build AnnotationData from view tree + tab type, and create AnnotationStrategy from data.

import type { ViewNode, AnnotationData, AnnotationStrategy, EditAnnotation } from "./types";

/**
 * Walk ViewNode tree recursively and build AnnotationData.
 *
 * For each ViewNode, an EditAnnotation is created:
 * - `"// @edit"` for normal nodes
 * - `"// @edit folded"` if `node.isFolded` is true
 * - `"// @edit render=canvas"` if `node.renderMode === "canvas"` (folded takes precedence)
 *
 * The codeTabDirective is set to `// @omni-file type=${tabType}`.
 */
export function buildAnnotationData(
  viewNodes: ViewNode[],
  tabType: "logic" | "event",
): AnnotationData {
  const editAnnotations = new Map<string, EditAnnotation>();

  function walk(nodes: ViewNode[], depth: number): void {
    for (const node of nodes) {
      let line: string;
      if (node.isFolded) {
        line = "// @edit folded";
      } else if (node.renderMode === "canvas") {
        line = "// @edit render=canvas";
      } else {
        line = "// @edit";
      }

      editAnnotations.set(node.path, { line, indent: depth });

      if (node.children.length > 0) {
        walk(node.children, depth + 1);
      }
    }
  }

  walk(viewNodes, 0);

  return {
    codeTabDirective: `// @omni-file type=${tabType}`,
    editAnnotations,
  };
}

/**
 * Create an AnnotationStrategy from AnnotationData.
 *
 * - `beforeFile()` returns the codeTabDirective.
 * - `beforeViewNode(node, depth)` looks up `node.path` in the editAnnotations map.
 *   If found, returns the annotation line. If not found, returns null.
 */
export function createAnnotationStrategy(data: AnnotationData): AnnotationStrategy {
  return {
    beforeFile: () => data.codeTabDirective,
    beforeViewNode: (node: ViewNode, _depth: number): string | null => {
      const ann = data.editAnnotations.get(node.path);
      return ann ? ann.line : null;
    },
  };
}
