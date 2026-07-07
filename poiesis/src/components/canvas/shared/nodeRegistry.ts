// Node registry — legacy adapter, functionality merged into store.

export interface NodeConfig {
  nodeType: string;
  label: string;
  category: string;
  defaultPorts?: Array<{ name: string; direction: "in" | "out"; type: string }>;
  minWidth?: number;
  minHeight?: number;
}

export const nodeRegistry: Record<string, NodeConfig> = {};
export function getNodeConfig(_tag: string): NodeConfig | undefined {
  return undefined;
}
