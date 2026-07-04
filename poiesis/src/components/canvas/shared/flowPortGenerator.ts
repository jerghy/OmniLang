import type { FlowPortConfig } from "../../../core/types/node-flow";
import {
  FLOW_NODE_PORTS,
  FLOW_NODE_COLORS,
} from "../../../core/types/node-flow";

export function getFlowPorts(nodeType: string): FlowPortConfig[] {
  return FLOW_NODE_PORTS[nodeType as keyof typeof FLOW_NODE_PORTS] || [];
}

export function getFlowNodeColor(
  nodeType: string,
): { bg: string; border: string } {
  return (
    FLOW_NODE_COLORS[nodeType as keyof typeof FLOW_NODE_COLORS] || {
      bg: "#F8FAFC",
      border: "#94A3B8",
    }
  );
}
