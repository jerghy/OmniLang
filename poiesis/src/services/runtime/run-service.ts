// Legacy run service stub — runtime services not yet integrated in v2.
export async function runProject(_projectDir: string): Promise<void> {
  // TODO(poiesis-v2): integrate run service
}
export async function handleRun(
  _target: string,
  _setStatus: (s: { text: string; isError: boolean }) => void,
  _setCompileErrors: (e: unknown[]) => void,
  _setRunLoading: (v: boolean) => void,
  _setRunStatus: (v: any) => void,
  _setFnToNodeMap: (v: Record<string, string>) => void,
): Promise<void> {
  // stub
}
export async function handleStop(
  _setStatus: (s: { text: string; isError: boolean }) => void,
  _setRunStatus: (v: any) => void,
  _setDebugConnected: (v: boolean) => void,
  _setDebugStateChanges: (v: any) => void,
  _setActiveNodeIds: (v: any) => void,
  _setActiveEdgeIds: (v: any) => void,
  _setDebugPaused: (v: boolean) => void,
  _setFnToNodeMap: (v: Record<string, string>) => void,
): Promise<void> {
  // stub
}
