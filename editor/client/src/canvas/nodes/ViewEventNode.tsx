// editor/client/src/canvas/nodes/ViewEventNode.tsx

import type { NodeProps } from "@xyflow/react";
import { BaseNode } from "./BaseNode";

/**
 * Node renderer for view elements (Button, Input, etc.) shown on the code canvas.
 * Event ports (onPress, onChange) are rendered as data-out ports.
 */
export function ViewEventNode(props: NodeProps) {
  return <BaseNode {...props} />;
}
