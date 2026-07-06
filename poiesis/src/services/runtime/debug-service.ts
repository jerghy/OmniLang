// Legacy debug service stub — runtime services not yet integrated in v2.
export async function startDebugSession(_options: unknown): Promise<void> {
  // TODO(poiesis-v2): integrate debug service
}
export function handleDebugMessage(
  _msg: Record<string, unknown>,
  _setDebugStateChanges: (v: any) => void,
  _setDebugConnected: (v: boolean) => void,
  _setDebugPaused: (v: boolean) => void,
  _setActiveNodeIds: (v: any) => void,
  _setActiveEdgeIds: (v: any) => void,
  _setFnToNodeMap: (v: Record<string, string>) => void,
  _fnToNodeMapRef: { current: Record<string, string> },
): void {
  // stub
}
