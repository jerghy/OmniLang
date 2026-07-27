// Code canvas node flow types

/** The four code canvas node types */
export type FlowNodeType = "function-header" | "skeleton" | "code-block" | "return";

/** Control flow skeleton variants */
export type SkeletonType = "if" | "else" | "for" | "while";

export interface FlowPortConfig {
  direction: "in" | "out";
  position: "top" | "bottom" | "left" | "right";
  portType: "flow" | "data";
  label?: string;
}
