// core/types/wire.ts
// Shared wire/connection type definition.

export interface WireData {
  id: string;
  fromId: string;
  fromPort: string;
  toId: string;
  toPort: string;
  isCollapsed?: boolean;
  collapseGroupId?: string;
}
