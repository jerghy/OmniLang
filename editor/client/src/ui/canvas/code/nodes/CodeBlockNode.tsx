// editor/client/src/canvas/nodes/CodeBlockNode.tsx

import type { NodeProps } from "@xyflow/react";
import { BaseNode } from "./BaseNode";

/**
 * Node renderer for existing code blocks (signal, computed, action, store, fn-call).
 * Extracts ports from element data and delegates to BaseNode.
 */
export function CodeBlockNode(props: NodeProps) {
  return <BaseNode {...props} />;
}
