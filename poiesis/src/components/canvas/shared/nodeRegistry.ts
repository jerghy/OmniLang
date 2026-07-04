// Node registry — legacy adapter, functionality merged into store.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const nodeRegistry: Record<string, any> = {};
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function getNodeConfig(_tag: string): any {
  return undefined;
}
