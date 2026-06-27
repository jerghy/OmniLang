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

export interface ElementData {
  id: string;
  nodeType: "view" | "codeblock" | "group";
  tag: string;
  props: Record<string, unknown>;
  style?: ElementStyle;
  children: string[];
  parentId: string | null;
  ports?: PortData[];
  renderMode?: "dom" | "canvas";
  isFolded?: boolean;
  typeAnnotation?: string;
  storeName?: string;
  source?: string;
  outputValue?: unknown;
  position?: { x: number; y: number };
  nodeCategory?: "flow" | "event" | "action" | "data" | "variable" | "code" | "function" | "ui";
  config?: Record<string, unknown>;
  flowOrder?: number;
  componentRef?: string;
}
