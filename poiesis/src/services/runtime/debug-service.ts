// Legacy debug service stub — runtime services not yet integrated in v2.
export async function startDebugSession(_options: unknown): Promise<void> {
  // TODO(poiesis-v2): integrate debug service
}
export function handleDebugMessage(
  _msg: Record<string, unknown>,
  _setDebugStateChanges: (v: unknown) => void,
  _setDebugConnected: (v: boolean) => void,
  _setDebugPaused: (v: boolean) => void,
  _setActiveNodeIds: (v: unknown) => void,
  _setActiveEdgeIds: (v: unknown) => void,
  _setFnToNodeMap: (v: Record<string, string>) => void,
  _fnToNodeMapRef: { current: Record<string, string> },
): void {
  // stub
}
