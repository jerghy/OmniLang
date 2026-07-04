// Connection validators — v2 stub.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function canConnect(_from: any, _to: any): boolean {
  return true;
}
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function validateConnection(_from: any, _to: any): Array<{ valid: boolean; reason?: string }> {
  return [{ valid: true }];
}
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function getAllPorts(_element: any): any[] {
  return [];
}
