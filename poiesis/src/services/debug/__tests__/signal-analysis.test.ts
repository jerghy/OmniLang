// 信号分析器（spec 4.2）：对组件源码文本聚合信号 → 界面用途 + 多 @if 链 + @for。
import { describe, it, expect } from "vitest";
import { analyzeSignalUsage } from "../signal-analysis";

const SRC = `
component NotificationsPanel {
    signal badgeVisible: bool = false;
    signal count: i32 = 0;
    @store NotificationStore;

    view {
        <h3>
            Notifications
            @if badgeVisible {
                <span>New</span>
            }
        </h3>
        @if NotificationStore.unreadCount == 0 {
            <p>No notifications</p>
        } @else {
            <p>Total: {NotificationStore.unreadCount}</p>
        }
        @for item in NotificationStore.notifications {
            <span>{item}</span>
        }
        <p>Count: {count} · badge {badgeVisible}</p>
        <input bind:value={input} />
    }
}
`;

const SIGNALS = [{ key: "badgeVisible", type: "bool" }, { key: "count", type: "i32" }];
const STORE_SIGNALS = [
  { key: "NotificationStore.unreadCount", type: "i32" },
  { key: "NotificationStore.notifications", type: "Array<String>" },
];

describe("analyzeSignalUsage", () => {
  it("界面信号：@if 条件出现 → conditions 链提取（含多分支）", () => {
    const { interfaceSignals } = analyzeSignalUsage(SRC, SIGNALS, STORE_SIGNALS);
    const badge = interfaceSignals.find((s) => s.key === "badgeVisible");
    expect(badge?.conditions.length).toBe(1);
    expect(badge?.conditions[0].branches[0].exprText).toBe("badgeVisible");
    // 单分支无 else → isElse 只一条
    expect(badge?.conditions[0].branches).toHaveLength(1);
  });

  it("!sig 取反：comparisons 带 negated 标志（F1 修复）", () => {
    const src = `component C {\n signal badgeVisible: bool = false;\n view {\n  @if !badgeVisible {\n   <span>New</span>\n  }\n }\n}`;
    const { interfaceSignals } = analyzeSignalUsage(src, [{ key: "badgeVisible", type: "bool" }], []);
    const sig = interfaceSignals.find((s) => s.key === "badgeVisible");
    expect(sig?.conditions[0].branches[0].exprText).toBe("!badgeVisible");
    expect(sig?.conditions[0].branches[0].comparisons).toEqual([
      { sig: "badgeVisible", op: null, negated: true },
    ]);
  });

  it("store qualified 信号：== 比较 + 多分支链 + 插值计数", () => {
    const { interfaceSignals } = analyzeSignalUsage(SRC, SIGNALS, STORE_SIGNALS);
    const unread = interfaceSignals.find((s) => s.key === "NotificationStore.unreadCount");
    expect(unread?.conditions[0].branches.map((b) => b.isElse)).toEqual([false, true]);
    expect(unread?.conditions[0].branches[0].comparisons).toEqual([
      { sig: "NotificationStore.unreadCount", op: "==", literal: "0" },
    ]);
    expect(unread?.uses.interpolations).toBe(1);
  });

  it("@for collection 信号 → loops（含 @empty 标志）", () => {
    const { interfaceSignals } = analyzeSignalUsage(SRC, SIGNALS, STORE_SIGNALS);
    const items = interfaceSignals.find((s) => s.key === "NotificationStore.notifications");
    expect(items?.loops.length).toBe(1);
    expect(items?.loops[0].collectionText).toContain("NotificationStore.notifications");
    expect(items?.loops[0].hasEmpty).toBe(false);
  });

  it("纯数据信号（插值/bind）→ dataSignals；插值与绑定计数", () => {
    const { dataSignals } = analyzeSignalUsage(SRC, SIGNALS, STORE_SIGNALS);
    const count = dataSignals.find((s) => s.key === "count");
    expect(count?.uses.interpolations).toBe(1);
    const badge = dataSignals.find((s) => s.key === "badgeVisible");
    expect(badge).toBeUndefined(); // badgeVisible 驱动 @if → 界面信号
  });

  it("bind:value 绑定计数", () => {
    const src = `component C {\n signal input: String = "";\n view {\n  <input bind:value={input} />\n }\n}`;
    const { dataSignals } = analyzeSignalUsage(src, [{ key: "input", type: "String" }], []);
    expect(dataSignals[0]?.uses.binds).toContain("value");
  });

  it("复杂条件（函数调用）→ comparisons 空 + 标记 complex", () => {
    const src = `component C {\n signal items: Array<String> = [];\n view {\n  @if items.len() > 0 {\n   <p>has</p>\n  }\n }\n}`;
    const { interfaceSignals } = analyzeSignalUsage(src, [{ key: "items", type: "Array<String>" }], []);
    const items = interfaceSignals[0];
    // 条件含函数调用 → 该分支 comparisons 为空（UI 据此置灰胶囊）
    expect(items?.conditions[0].branches[0].comparisons).toEqual([]);
  });

  it("computed 依赖计入 uses.computedDeps", () => {
    const src = `component C {\n signal count: i32 = 0;\n computed double: i32 = count * 2;\n view { <p>{double}</p> }\n}`;
    const { dataSignals } = analyzeSignalUsage(src, [{ key: "count", type: "i32" }], []);
    expect(dataSignals[0]?.uses.computedDeps).toBe(1);
  });

  it("回归：信号名含 else 子串（elsesEnabled）不误判为 else 分支", () => {
    // 旧实现 view.slice(...).includes("else") 把 @if elsesEnabled 判成 else，
    // 触发孤立 @else 防御整段丢弃 → @if 不生效（信号落 dataSignals）。
    const src = `component C {\n signal elsesEnabled: bool = false;\n view {\n  @if elsesEnabled {\n   <p>new</p>\n  }\n }\n}`;
    const { interfaceSignals, dataSignals } = analyzeSignalUsage(src, [{ key: "elsesEnabled", type: "bool" }], []);
    const sig = interfaceSignals.find((s) => s.key === "elsesEnabled");
    expect(sig?.conditions[0].branches).toHaveLength(1);
    expect(sig?.conditions[0].branches[0].isElse).toBe(false);
    expect(sig?.conditions[0].branches[0].exprText).toBe("elsesEnabled");
    expect(dataSignals.find((s) => s.key === "elsesEnabled")).toBeUndefined();
  });
});
