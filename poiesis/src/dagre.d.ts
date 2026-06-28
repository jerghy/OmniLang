declare module 'dagre' {
  export function layout(graph: unknown): void;
  export namespace graphlib {
    class Graph {
      constructor(attrs?: unknown);
      setGraph(attrs: unknown): void;
      setDefaultEdgeLabel(callback: () => unknown): void;
      setNode(id: string, attrs: unknown): void;
      setEdge(v: string, w: string, attrs?: unknown): void;
      node(id: string): unknown;
    }
  }
}
