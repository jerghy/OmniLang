// core/types/element.ts
// Shared element and port type definitions.

import type { ElementStyle } from "./style";
import type { CodeOp } from "./code-op";

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
  /** Two-way data bindings: prop name → signal name, e.g. { "value": "input", "checked": "isDone" } */
  bindings?: Record<string, string>;
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
  operations?: CodeOp[];
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

export interface ConditionalBlockData extends ElementDataBase {
  nodeType: "conditional";
  /** 条件分支列表，按 @if → @else if → @else 顺序 */
  branches: Array<{
    type: "if" | "elif" | "else";
    condition: string;       // @else 时为空字符串 ""
    children: string[];      // 该分支内的子元素 ID
  }>;
  /** 所有分支子元素的扁平列表（兼容现有 children 遍历模式） */
  children: string[];
  isFolded?: boolean;
  position?: { x: number; y: number };
}

/** view 级 @for 循环块（2026-08-06：parser-view 补 @for 解析，闭环调试列表卡） */
export interface ForBlockData extends ElementDataBase {
  nodeType: "for";
  /** 循环项绑定名：`@for item in items` 的 item */
  itemName: string;
  /** 遍历集合表达式：`items` / `TodoStore.items` / `items.filter(...)` */
  collection: string;
  /** key 表达式（`key={item.id}`），可选 */
  keyExpr?: string;
  /** 循环体子元素 ID */
  children: string[];
  /** @empty 分支子元素 ID（可选） */
  emptyChildren: string[];
  isFolded?: boolean;
  position?: { x: number; y: number };
}

export type ElementData = ViewElementData | CodeblockElementData | GroupElementData | ConditionalBlockData | ForBlockData;
