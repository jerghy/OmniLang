// editor/client/src/omni-gen/annotation-data.ts
// Build PID annotations from view tree + tab type, and create AnnotationStrategy from data.
// Replaces the old // @edit system with // @pid:xxx PIDs.

import type { ViewNode, AnnotationData, AnnotationStrategy, PidAnnotation } from "./types";

let pidCounter = 0;

function nextPidSeq(): number {
  return ++pidCounter;
}

/**
 * Walk ViewNode tree recursively and build AnnotationData with PID annotations.
 *
 * For each ViewNode (excluding text/interpolation which are inline), a PID is generated:
 * - Format: `c{compIndex}_v{seq}` where seq is a sequential counter
 * - The PidAnnotation stores the pid, blockType ("view"), and the node's tag as blockName
 *
 * The codeTabDirective is set to `// @omni-file type=${tabType}`.
 *
 * @param viewNodes     The list of root ViewNodes.
 * @param tabType       The current code tab type.
 * @param componentIndex The file-level index of the component (1-based).
 */
export function buildPidAnnotations(
  viewNodes: ViewNode[],
  tabType: "logic" | "event",
  componentIndex: number = 1,
): AnnotationData {
  pidCounter = 0; // reset for consistency
  const pidAnnotations = new Map<string, PidAnnotation>();

  function walk(nodes: ViewNode[]): void {
    for (const node of nodes) {
      if (node.kind === "text" || node.kind === "interpolation") continue;

      const seq = nextPidSeq();
      const pid = `c${componentIndex}_v${seq}`;

      pidAnnotations.set(node.path, {
        pid,
        blockType: "view",
        blockName: node.tag,
      });

      if (node.children.length > 0) {
        walk(node.children);
      }

      // Handle conditional branches
      if (node.branches) {
        for (const branch of node.branches) {
          walk(branch.body);
        }
      }
      if (node.fallback) {
        walk(node.fallback);
      }
      if (node.emptyBody) {
        walk(node.emptyBody);
      }
    }
  }

  walk(viewNodes);

  return {
    codeTabDirective: `// @omni-file type=${tabType}`,
    pidAnnotations,
  };
}

/**
 * Create an AnnotationStrategy from AnnotationData.
 *
 * - `beforeFile()` returns the codeTabDirective.
 * - `beforeViewNode(node, depth)` looks up `node.path` in the pidAnnotations map.
 *   If found, returns the PidAnnotation. If not found, returns null.
 */
export function createAnnotationStrategy(data: AnnotationData): AnnotationStrategy {
  return {
    beforeFile: () => data.codeTabDirective,
    beforeViewNode: (node: ViewNode, _depth: number): PidAnnotation | null => {
      const ann = data.pidAnnotations.get(node.path);
      return ann ?? null;
    },
  };
}
