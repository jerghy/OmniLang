// core/types/store.ts
// Store editor type definitions — signals, getters, actions, and top-level store.

import type { CodeblockElementData } from "./element";
import type { WireData } from "./wire";

export interface StoreSignalDef {
  pid: string;
  name: string;
  type: string;
  defaultValue?: unknown;
}

export interface StoreGetterDef {
  pid: string;
  name: string;
  returnType: string;
  expression: string;
}

export interface StoreActionDef {
  pid: string;
  name: string;
  params: Array<{ name: string; type: string }>;
  returnType?: string;
  isAsync: boolean;
  elements: CodeblockElementData[];
  wires: WireData[];
}

export interface StoreDefinition {
  pid: string;
  name: string;
  filePath: string;
  imports: string[];
  signals: StoreSignalDef[];
  getters: StoreGetterDef[];
  actions: StoreActionDef[];
}
