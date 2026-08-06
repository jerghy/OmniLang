// poiesis/src/services/codegen/boolean-attrs.ts
// HTML 布尔属性全集（source-emitter 原私有集合 + autocomplete）。
// parser-view（裸布尔属性解析）与 source-emitter（发射）共用。
export const BOOLEAN_ATTRS: ReadonlySet<string> = new Set([
  "disabled", "required", "readonly", "checked", "selected",
  "hidden", "multiple", "autofocus", "novalidate",
  "async", "defer", "ismap", "itemscope", "open",
  "loop", "muted", "controls", "autoplay", "autocomplete",
]);
