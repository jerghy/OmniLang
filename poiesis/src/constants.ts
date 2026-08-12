// ── Shared constants for Poiesis editor ──

export const DEFAULT_PROJECT_NAME = "未命名项目";

export const DEFAULT_PANEL_LAYOUT = { leftWidth: 240, rightWidth: 300 } as const;

/** 值型控件标签集合（B1：源 = 值型控件/源节点/fn 出口三类；显示元素只作目标——设计稿 §15.3）。
 *  单一来源：connectElementLink 源端校验与点名清单收窄共用，防漂移。 */
export const VALUE_CONTROL_TAGS = new Set(["input", "textarea", "select", "slider"]);
