// Socket/port types for canvas nodes — v2 stub.
export interface SocketDef {
  id: string;
  label: string;
  type: "input" | "output";
  dataType?: string;
}

export const TYPE_COLORS: Record<string, string> = {};
