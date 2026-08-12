// poiesis/src/components/design/action-verbs.ts
// M3 T1 行为卡最小动词集：行为卡（当[事件]→[动作][参数]）的动作定义表。
// code 模板是 T2 接线（applyBehaviorCard）的契约——保持最小可编译、可读。
// 语义事件映射（click→onPress 等）在 T2 落盘，此处为意图层。

export interface BehaviorAction {
  label: string;
  params: Array<{ key: string; label: string; kind: "signal" | "page" | "text" | "none" }>;
  code: (ctx: Record<string, string>) => string;
}

export const ACTION_VERBS = ["点击", "提交", "删除", "编辑", "取消", "加", "减", "跳转", "切换", "关闭"];

export const ACTIONS: Record<string, BehaviorAction> = {
  click:    { label: "点击",   params: [{ key: "target", label: "目标元素", kind: "signal" }], code: (c) => `${c.target}.click();` },
  submit:   { label: "提交",   params: [], code: () => `emit submit();` },
  delete:   { label: "删除",   params: [{ key: "target", label: "目标元素", kind: "signal" }], code: (c) => `${c.target}.remove();` },
  edit:     { label: "编辑",   params: [{ key: "target", label: "目标元素", kind: "signal" }], code: (c) => `edit(${c.target});` },
  cancel:   { label: "取消",   params: [], code: () => `emit cancel();` },
  add:      { label: "加",     params: [{ key: "target", label: "目标信号", kind: "signal" }, { key: "amount", label: "步长", kind: "text" }], code: (c) => `${c.target} = ${c.target} + (${c.amount || "1"});` },
  subtract: { label: "减",     params: [{ key: "target", label: "目标信号", kind: "signal" }, { key: "amount", label: "步长", kind: "text" }], code: (c) => `${c.target} = ${c.target} - (${c.amount || "1"});` },
  navigate: { label: "跳转",   params: [{ key: "page", label: "目标页面", kind: "page" }], code: (c) => `navigate("${c.page}");` },
  toggle:   { label: "切换",   params: [{ key: "target", label: "目标信号", kind: "signal" }], code: (c) => `${c.target} = !${c.target};` },
  close:    { label: "关闭",   params: [], code: () => `emit close();` },
};
