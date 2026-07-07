// core/types/element.ts
// Shared element and port type definitions.

import type { ElementStyle } from "./style";

export interface PortData {
  name: string;
  direction: "in" | "out";
  portType?: "flow" | "data";
  type: string;
  label?: string;
  configurable?: boolean;
  defaultValue?: unknown;
}

interface ElementDataBase {
  id: string;
  parentId: string | null;
}

export interface ViewElementData extends ElementDataBase {
  nodeType: "view";
  tag: string;
  props: Record<string, unknown>;
  style?: ElementStyle;
  children: string[];
  ports?: PortData[];
  renderMode?: "dom" | "canvas";
  isFolded?: boolean;
  typeAnnotation?: string;
  storeName?: string;
  position?: { x: number; y: number };
  componentRef?: string;
}

export interface CodeblockElementData extends ElementDataBase {
  nodeType: "codeblock";
  tag: string;
  props: Record<string, unknown>;
  ports?: PortData[];
  source: string;
  outputValue?: unknown;
  position?: { x: number; y: number };
  nodeCategory?: "flow" | "event" | "action" | "data" | "variable" | "code" | "function" | "ui";
  config?: Record<string, unknown>;
  flowOrder?: number;
}

export interface GroupElementData extends ElementDataBase {
  nodeType: "group";
  children: string[];
  isFolded?: boolean;
}

export type ElementData = ViewElementData | CodeblockElementData | GroupElementData;
