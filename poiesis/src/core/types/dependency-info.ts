// Dependency analysis types shared between WASM pipeline, DependencyPanel,
// FlowCanvas (store.nodeMeta), and ArchitecturePage (bubble graph edges).

/** Full dependency analysis result for a single code block. */
export interface DependencyInfo {
  codeBlockPid: string;
  reads:  DepEntry[];
  writes: DepEntry[];
  calls:  CallEntry[];
}

export interface DepEntry {
  name: string;
  kind: "param" | "signal" | "computed" | "prop" | "store" | "local" | "function" | "unknown";
  sourceLine: number;
}

export interface CallEntry {
  storeName: string;
  actionName?: string;
  sourceLine: number;
}
