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
  /** Semantic props only (label, src, placeholder, type, etc.) — no layout or style */
  props: Record<string, unknown>;
  /** References to design system classes, e.g. ["AppDesign.Card", "AppDesign.Row"] */
  classRefs?: string[];
  /** Reference to a design variant binding */
  variantRef?: {
    designName: string;
    variantName: string;
    params: Record<string, string>;
  } | null;
  /** Component-local style class names (defined in component style {} block) */
  componentClasses?: string[];
  /** Inline style ONLY for signal-driven dynamic values */
  inlineStyle?: Record<string, string>;
  /** Deprecated: kept for backward compat during migration */
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
