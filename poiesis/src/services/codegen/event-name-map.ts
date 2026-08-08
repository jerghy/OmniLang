// poiesis/src/services/codegen/event-name-map.ts
// 事件名语义映射（Task 4 事件意图化四链同步）。
//
// Rust 编译器的 ground truth 是
// `crates/omnilang-core/src/codegen/shared/event_names.rs`（canonical_event_name
// + legacy_spelling）——阶段 1 语义化后，@onClick/@click/@onTap/@tap/
// @onMouseEnter/@mouseenter/@onMouseLeave/@mouseleave 已是编译期硬错误（E0006），
// 编辑器在「读侧归一化 + 发射侧防御映射」双端把旧拼写收敛到语义名：
//   click/press/tap 族   → onPress
//   enter/mouseenter 族  → onEnter
//   leave/mouseleave 族  → onLeave
// 其余事件名原样保留（透传）。
//
// 大小写不敏感，镜像 Rust 的 canonical 表（剥 on/On/ON 前缀 + 小写化 stem）。
// 该表与 Rust 侧同步由 TS→Rust 门禁测试（event-emission-wasm-validity.test.ts）
// 守护——TS 发射的事件名必须通过真实 WASM 编译器零 Error。

/**
 * 把任意事件名（含旧 DOM 拼写、任意大小写）映射为语义事件名；
 * 不在映射表内（透传名/自定义名）返回 null。
 * 镜像 Rust event_names.rs 的 canonical + legacy_spelling 语义。
 */
export function toSemanticEventName(raw: string): string | null {
  const stem = raw.replace(/^on/i, "").toLowerCase();
  switch (stem) {
    case "click":
    case "press":
    case "tap":
      return "onPress";
    case "enter":
    case "mouseenter":
      return "onEnter";
    case "leave":
    case "mouseleave":
      return "onLeave";
    default:
      return null;
  }
}

/**
 * 恒等语义化：在映射表内返回语义名，否则原样返回。
 * 读侧归一化与发射侧防御映射统一走这里，保证两端的表永远一致。
 */
export function semanticEventName(name: string): string {
  return toSemanticEventName(name) ?? name;
}
