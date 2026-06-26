// core/types/style.ts
// Shared style type definitions and helpers.

export interface StyleProperty {
  name: string;
  value: string;
}

export interface StyleEntry {
  id: string;
  type: "class" | "variable" | "inline";
  name: string;
  properties: StyleProperty[];
}

// ─── Style property name set (for migration + parsing) ─
export const STYLE_PROPERTY_NAMES = new Set([
  // From STYLE_PROP_CSS_MAP legacy
  "background_color", "border_radius", "font_size", "font_weight",
  "text_align", "shadow", "padding", "margin", "gap",
  "width", "height", "color",
  // Additional CSS properties (kebab-case + camelCase aliases)
  "display", "position", "opacity", "overflow", "overflow-x", "overflow_x", "overflowX",
  "overflow-y", "overflow_y", "overflowY", "z-index", "z_index", "zIndex",
  "box-shadow", "box_shadow", "boxShadow",
  "border", "border-color", "border_color", "borderColor",
  "font-family", "font_family", "fontFamily",
  "line-height", "line_height", "lineHeight",
  "letter-spacing", "letter_spacing", "letterSpacing",
  "text-decoration", "text_decoration", "textDecoration",
  "text-transform", "text_transform", "textTransform",
  "white-space", "white_space", "whiteSpace",
  "word-break", "word_break", "wordBreak",
  "min-width", "min_width", "minWidth",
  "max-width", "max_width", "maxWidth",
  "min-height", "min_height", "minHeight",
  "max-height", "max_height", "maxHeight",
  "transform", "transition", "animation", "cursor",
  "box-sizing", "box_sizing", "boxSizing",
  "flex-direction", "flex_direction", "flexDirection",
  "flex-wrap", "flex_wrap", "flexWrap",
  "flex-grow", "flex_grow", "flexGrow",
  "flex-shrink", "flex_shrink", "flexShrink",
  "flex-basis", "flex_basis", "flexBasis",
  "justify-content", "justify_content", "justifyContent",
  "align-items", "align_items", "alignItems",
  "align-self", "align_self", "alignSelf",
  "grid-template-columns", "grid_template_columns", "gridTemplateColumns",
  "grid-template-rows", "grid_template_rows", "gridTemplateRows",
  "grid-column", "grid_column", "gridColumn",
  "grid-row", "grid_row", "gridRow",
  "background-image", "background_image", "backgroundImage",
  "transition-property", "transition_property", "transitionProperty",
  "transition-duration", "transition_duration", "transitionDuration",
  "transition-timing-function", "transition_timing_function", "transitionTimingFunction",
  "animation-name", "animation_name", "animationName",
  "animation-duration", "animation_duration", "animationDuration",
  "animation-iteration-count", "animation_iteration_count", "animationIterationCount",
  "animation-direction", "animation_direction", "animationDirection",
  "animation-fill-mode", "animation_fill_mode", "animationFillMode",
  "animation-timing-function", "animation_timing_function", "animationTimingFunction",
]);

/** Variable name "--primary-color" → class name "_v-primary-color" */
export function variableToClassName(varName: string): string {
  return "_v-" + varName.slice(2); // strip --, prefix _v-
}

/** Class name "_v-primary-color" → variable name "--primary-color" */
export function classNameToVariable(className: string): string | null {
  if (className.startsWith("_v-")) {
    return "--" + className.slice(3);
  }
  return null;
}
