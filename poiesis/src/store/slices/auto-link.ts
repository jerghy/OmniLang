// ── 自动中转 signal 生命周期（M3 T4）──
// 元素间连线 = 自动 signal：connectElementLink(源元素, 源槽, 目标元素, 目标槽) →
//   1. 落盘：普通组件 signal（componentDefs[compName].signals，零语法/形状变更）
//   2. 源端：bindings[源槽] = signal 名（输入类槽位双向绑定）
//   3. 目标端：按槽位种类写现有语法（文本/属性 → props 插值；输入 → bindings；
//      列表 → for.collection；条件 → if 分支 condition）
// 命名 = `<源元素id消毒>_<槽语义>`；同槽复用（同 (源,槽) N 条 wire 共享同一 signal）；
// 回收（断开后无其他引用 → 删除 signal）；自动分组 = 运行时命名模式识别（不持久化）。
import type { PoiesisState } from "../types";
import type { ElementData } from "../../core/types/element";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

/** 元素 id 消毒：非 [a-zA-Z0-9_] 字符 → "_"；前导数字前补 "_" */
export function sanitizeElementId(id: string): string {
  const cleaned = id.replace(/[^a-zA-Z0-9_]/g, "_");
  return /^[0-9]/.test(cleaned) ? `_${cleaned}` : cleaned;
}

/**
 * 自动 signal 命名：`<源元素id消毒>_<槽语义>`。
 * 消毒后前缀为空（理论上 uid 不会，防御）→ 回退 `<tag>_<序号>_<槽语义>`。
 */
export function autoSignalName(elementId: string, slotKey: string, tag?: string, seq?: number): string {
  const prefix = sanitizeElementId(elementId);
  const base = prefix || `${tag ?? "el"}_${seq ?? 0}`;
  return `${base}_${slotKey}`;
}

/**
 * 识别自动 signal：命名模式 + 前缀必须对应画布中某元素（消毒后）。
 * 回退形式（tag_序号 前缀）也识别（`<tag>_<seq>_<slot>`）。
 * 手工信号（userName_value 等无对应元素前缀）不识别。
 */
export function isAutoSignal(signalName: string, elements: ElementData[]): boolean {
  const m = signalName.match(/^([a-z0-9_]+)_(label|value|src|href|list|if)$/);
  if (!m) return false;
  const prefix = m[1];
  // 回退形式：`<tag>_<序号>` 前缀（无元素 id 对应，如 tag_0_value）
  if (/^[a-z][a-z0-9]*_\d+$/.test(prefix)) return true;
  return elements.some((el) => sanitizeElementId(el.id) === prefix);
}

/**
 * 已知槽位集合（与 isAutoSignal 正则 / ContentTab slotsFor 对齐）。
 * 未知槽拒绝写入（M6 防御：C1 类槽解析错误若漏到此处，将凭文本兜底落垃圾 prop）。
 */
const KNOWN_SLOT_KEYS = new Set(["label", "value", "src", "href", "list", "if"]);

/** 槽位种类（与 ContentTab slotsFor 对齐）：列表/条件/输入/属性/文本；未知槽 → null（拒绝绑定） */
function slotKindFor(el: ElementData, slotKey: string): "文本" | "输入" | "列表" | "条件" | "属性" | null {
  if (!KNOWN_SLOT_KEYS.has(slotKey)) {
    console.warn(`[auto-link] 未知槽位 "${slotKey}"：跳过绑定（疑似端口解析错误，拒绝垃圾写入）`);
    return null;
  }
  if (el.nodeType === "for" && slotKey === "list") return "列表";
  if (el.nodeType === "conditional" && slotKey === "if") return "条件";
  const tag = "tag" in el ? el.tag : "";
  if (["input", "textarea", "select"].includes(tag ?? "") && slotKey === "value") return "输入";
  if (slotKey === "src" || slotKey === "href") return "属性";
  return "文本";
}

/** 目标端绑定落盘（与 ContentTab pick switch 同源）：文本/属性 → props 插值；输入 → bindings；列表 → collection；条件 → if condition */
function setTargetBinding(get: GetFn, target: ElementData, slotKey: string, name: string): void {
  const kind = slotKindFor(target, slotKey);
  if (!kind) return; // 未知槽防御：早退，不落垃圾 prop
  switch (kind) {
    case "文本":
    case "属性":
      get().setProp(target.id, slotKey, `{${name}}`);
      break;
    case "输入":
      get().setElementBinding(target.id, slotKey, name);
      break;
    case "列表":
      get().updateForBlock(target.id, { collection: name });
      break;
    case "条件": {
      const idx = target.nodeType === "conditional" ? target.branches.findIndex((b) => b.type === "if") : -1;
      get().updateBranchCondition(target.id, idx >= 0 ? idx : 0, name);
      break;
    }
  }
}

/** 目标端重置（回收时清除悬空引用；原始文本不可恢复——WireData 不存，M3 T4 接受） */
function clearTargetBinding(get: GetFn, target: ElementData, slotKey: string): void {
  const kind = slotKindFor(target, slotKey);
  if (!kind) return; // 未知槽防御：早退，不碰垃圾槽
  switch (kind) {
    case "文本":
    case "属性":
      get().setProp(target.id, slotKey, "");
      break;
    case "输入":
      get().setElementBinding(target.id, slotKey, null);
      break;
    case "列表":
      get().updateForBlock(target.id, { collection: "" });
      break;
    case "条件": {
      const idx = target.nodeType === "conditional" ? target.branches.findIndex((b) => b.type === "if") : -1;
      get().updateBranchCondition(target.id, idx >= 0 ? idx : 0, "");
      break;
    }
  }
}

/** 全画布引用扫描：bindings 精确匹配 ∪ props 插值 ∪ for.collection ∪ if 分支 condition */
export function countSignalReferences(elements: ElementData[], signalName: string): number {
  let count = 0;
  for (const el of elements) {
    if ("bindings" in el && el.bindings != null) {
      for (const v of Object.values(el.bindings)) {
        if (v === signalName) count += 1;
      }
    }
    if ("props" in el && el.props != null) {
      for (const v of Object.values(el.props)) {
        if (typeof v === "string" && v === `{${signalName}}`) count += 1;
      }
    }
    if (el.nodeType === "for" && el.collection === signalName) count += 1;
    if (el.nodeType === "conditional") {
      for (const b of el.branches) {
        if (b.condition === signalName) count += 1;
      }
    }
  }
  return count;
}

function generatePid(): string {
  return `pid-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

function generateWireId(): string {
  return `wire-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export function createAutoLinkSlice(set: SetFn, get: GetFn) {
  return {
    /** 元素间连线 → 自动中转 signal：命名/创建（或同槽复用）/源端 bind/目标端绑定/wire */
    connectElementLink: (sourceId: string, sourceSlot: string, targetId: string, targetSlot: string) => {
      const s = get();
      const tab = s.designPage.openTabs.find((t) => t.id === s.designPage.activeTabId);
      const compName = tab?.tabType === "component" ? (tab.componentName ?? null) : null;
      if (!compName) return; // 非组件 tab 无 signal 载体

      const source = s.elements.find((e) => e.id === sourceId);
      const target = s.elements.find((e) => e.id === targetId);
      if (!source || !target) return;

      // 重复 wire guard（同 (源,槽,目标,槽) 已存在 → 幂等返回）
      const dup = s.wires.some(
        (w) => w.fromId === sourceId && w.fromPort === sourceSlot && w.toId === targetId && w.toPort === targetSlot,
      );
      if (dup) return;

      const def = s.componentDefs[compName];
      if (!def) return;

      const name = autoSignalName(sourceId, sourceSlot, "tag" in source ? source.tag : undefined, 0);

      // 同槽复用：已有同名 signal 直接复用；否则创建（普通组件 signal，零语法变更）
      const existing = (def.signals ?? []).find((sig) => sig.name === name);
      if (!existing) {
        s.addComponentSignal(compName, { pid: generatePid(), name, type: "String", defaultValue: '""' });
      }

      // 源端绑定 + 目标端绑定（5 类槽位）
      s.setElementBinding(sourceId, sourceSlot, name);
      setTargetBinding(get, target, targetSlot, name);

      // wire 落盘（addWire 需完整 WireData，id 由调用方生成）
      s.addWire({ id: generateWireId(), fromId: sourceId, fromPort: sourceSlot, toId: targetId, toPort: targetSlot });
    },

    /** 断开连线：wire 删除 + 无引用清理（源 bind 仅当无同 (源,槽) 剩余 wire；signal 无引用即删；目标端重置） */
    disconnectElementLink: (wireId: string) => {
      const s = get();
      const wire = s.wires.find((w) => w.id === wireId);
      if (!wire) return;
      const tab = s.designPage.openTabs.find((t) => t.id === s.designPage.activeTabId);
      const compName = tab?.tabType === "component" ? (tab.componentName ?? null) : null;

      // 先捕获 signal 名（清除前源 bind 还在）；无源 bind → 非自动 wire，仅取消连接
      const source = s.elements.find((e) => e.id === wire.fromId);
      const sigName =
        source && "bindings" in source && source.bindings != null ? source.bindings[wire.fromPort] ?? null : null;

      s.removeWire(wireId);

      if (!sigName) return;
      const target = s.elements.find((e) => e.id === wire.toId);

      // 源端：仅当没有剩余 wire 共享 (源,槽) 才清除 bind（同槽复用共享源绑定）
      const stillShared = s.wires.some(
        (w) => w.fromId === wire.fromId && w.fromPort === wire.fromPort && w.id !== wireId,
      );
      if (!stillShared) s.setElementBinding(wire.fromId, wire.fromPort, null);

      // 目标端：重置被移除 wire 的目标绑定（防悬空 {sig} 插值；引用扫描依赖此步先执行）
      if (target) clearTargetBinding(get, target, wire.toPort);

      // signal 回收：无任何引用（bind/插值/collection/condition）→ 删除
      if (compName) {
        const def = s.componentDefs[compName];
        const signal = (def?.signals ?? []).find((sig) => sig.name === sigName);
        if (signal && countSignalReferences(get().elements, sigName) === 0) {
          s.removeComponentSignal(compName, signal.pid);
        }
      }
    },
  };
}
