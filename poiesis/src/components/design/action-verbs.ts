// poiesis/src/components/design/action-verbs.ts
// M3 T1 行为卡最小动词集：行为卡（当[事件]→[动作][参数]）的动作定义表。
// M3 T2 修正：语言无元素引用——元素动作（点击/删除/编辑）不可表达，动词集
// 收敛为信号/页面语义 10 项（提交/取消/关闭/加/减/跳转/切换/设置/清空/通知）。
// code 模板是 applyBehaviorCard 落盘的契约（fn 体 source），保持最小可编译、可读。

export interface BehaviorAction {
  label: string;
  params: Array<{ key: string; label: string; kind: "signal" | "page" | "text" | "none" }>;
  code: (ctx: Record<string, string>) => string;
}

export const ACTION_VERBS = ["提交", "取消", "关闭", "加", "减", "跳转", "切换", "设置", "清空", "通知"];

export const ACTIONS: Record<string, BehaviorAction> = {
  submit:   { label: "提交",   params: [], code: () => `emit submit();` },
  cancel:   { label: "取消",   params: [], code: () => `emit cancel();` },
  close:    { label: "关闭",   params: [], code: () => `emit close();` },
  add:      { label: "加",     params: [{ key: "target", label: "目标信号", kind: "signal" }, { key: "amount", label: "步长", kind: "text" }], code: (c) => `${c.target} = ${c.target} + (${c.amount || "1"});` },
  subtract: { label: "减",     params: [{ key: "target", label: "目标信号", kind: "signal" }, { key: "amount", label: "步长", kind: "text" }], code: (c) => `${c.target} = ${c.target} - (${c.amount || "1"});` },
  navigate: { label: "跳转",   params: [{ key: "page", label: "目标页面", kind: "page" }], code: (c) => `navigate("${c.page}");` },
  toggle:   { label: "切换",   params: [{ key: "target", label: "目标信号", kind: "signal" }], code: (c) => `${c.target} = !${c.target};` },
  set:      { label: "设置",   params: [{ key: "target", label: "目标信号", kind: "signal" }, { key: "value", label: "取值", kind: "text" }], code: (c) => `${c.target} = ${c.value};` },
  clear:    { label: "清空",   params: [{ key: "target", label: "目标信号", kind: "signal" }], code: (c) => `${c.target}.clear();` },
  notify:   { label: "通知",   params: [{ key: "message", label: "消息文本", kind: "text" }], code: (c) => `emit notify("${c.message}");` },
};
