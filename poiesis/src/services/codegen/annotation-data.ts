// poiesis/src/services/codegen/annotation-data.ts
// Build PID annotations from view tree + tab type, and create AnnotationStrategy from data.
// Replaces the old // @edit system with // @pid:xxx PIDs.

import type { ViewNode, AnnotationData, AnnotationStrategy, PidAnnotation } from "./types";
import type { PidMap } from "../../core/types/pid";
import { generatePid } from "../../services/pid/pid-index";

/**
 * Walk ViewNode tree recursively and build AnnotationData with PID annotations.
 *
 * For each ViewNode (excluding text/interpolation which are inline), a PID is generated
 * using the centralized generatePid() from pid-index.ts (format: `v{compIdx}_v{seq}`).
 *
 * The PidAnnotation maps the node's kind to an appropriate PidBlockType and stores
 * the node's tag as blockName.
 *
 * The codeTabDirective is set to `// @omni-file type=${tabType}`.
 *
 * @param viewNodes     The list of root ViewNodes.
 * @param tabType       The current code tab type.
 * @param componentIndex The file-level index of the component (1-based).
 * @param existingPidMap Optional map of existing PIDs to preserve across regenerations.
 */
export function buildPidAnnotations(
  viewNodes: ViewNode[],
  tabType: "logic" | "event",
  componentIndex: number = 1,
  existingPidMap?: PidMap,
): AnnotationData {
  let pidCounter = 0;

  function nextPidSeq(): number {
    return ++pidCounter;
  }

  /** Map a ViewNode kind to the most specific PidBlockType. */
  function getPidBlockType(kind: ViewNode["kind"]): "view" | "if" | "for" | "slot" {
    switch (kind) {
      case "if":   return "if";
      case "for":  return "for";
      case "slot": return "slot";
      default:     return "view";
    }
  }

  const pidAnnotations = new Map<string, PidAnnotation>();

  function walk(nodes: ViewNode[]): void {
    for (const node of nodes) {
      if (node.kind === "text" || node.kind === "interpolation") continue;

      const seq = nextPidSeq();
      const blockType = getPidBlockType(node.kind);
      // Generate PID using the centralized generatePid() from pid-index.ts.
      // For view nodes this produces `v{compIdx}_v{seq}` (view prefix + view sub-prefix).
      const generatedPid = generatePid(componentIndex, "view", seq);

      // Check if we already have a PID for this node path in the existing map
      let finalPid = generatedPid;
      if (existingPidMap && existingPidMap.status === "ok") {
        for (const [existingPid, entry] of Object.entries(existingPidMap.entries)) {
          if (entry.blockName === node.path || entry.blockName === (node.kind === "element" ? node.tag : node.kind)) {
            // Reuse existing PID for same path to preserve stability
            finalPid = existingPid;
            break;
          }
        }
      }

      pidAnnotations.set(node.path, {
        pid: finalPid,
        blockType,
        blockName: node.kind === "element" ? node.tag : node.kind,
      });

      // Walk children based on kind
      switch (node.kind) {
        case "element":
          walk(node.children);
          break;
        case "if":
          for (const branch of node.branches) {
            walk(branch.body);
          }
          if (node.fallback) walk(node.fallback);
          break;
        case "for":
          walk(node.children);
          if (node.emptyBody) walk(node.emptyBody);
          break;
        case "slot":
          walk(node.children);
          break;
        // text and interpolation are skipped above
      }
    }
  }

  walk(viewNodes);

  // type=logic is the default — omit the directive for cleaner output
  return {
    codeTabDirective: tabType === "logic" ? "" : `// @omni-file type=${tabType}`,
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
