# OmniView Codegen 重构：拆分 element_to_html + 消除 Motion Trigger 重复

> **For agentic workers:** Use superpowers:subagent-driven-development to implement task-by-task.
> **关键原则：纯机械重构，不改变任何生成逻辑。** 代码移动必须逐字精确，生成的 JS/HTML/CSS 必须与重构前完全一致。

**Goal:** 将 `element_to_html`（839行）拆分为 7 个独立方法；将 `emit_motion_triggers_for_element`（676行）中 named/inline 两段重复的 switch 统一为一个参数化函数。

**Architecture:** 纯方法提取（Extract Method）——将现有代码块移动到新的 `fn` 中，不改变内部逻辑。Motion trigger 部分引入一个闭包参数来控制动画创建调用。

**Tech Stack:** Rust, `OmniViewGenerator` impl block.

## Global Constraints

- **输出必须字节级一致** —— 重构前后 `cargo test` 全部 500 个测试必须通过
- 零行为变化，纯代码移动
- 不使用 unwrap/panic，遵循现有模式
- 保留所有现有注释

---

### Task 1: 提取 `emit_element_props`（静态属性 + variant class 名）

**文件:** `omnilang/src/codegen/omniview/view/element.rs`

**提取内容:** 行 129-302（static props → attrs + class_name_parts + design variant 展开 + class 名验证）

**方法签名:**
```rust
fn emit_element_props(&mut self, el: &ElementIR, nid: &str) -> (String, Vec<String>)
// 返回 (attrs_string, class_name_parts)
// attrs_string 以 " data-nid=\"...\"" 开头
```

**在 element_to_html 中替换为:**
```rust
let (mut attrs, class_name_parts) = self.emit_element_props(el, &nid);
```

- [ ] 从 element_to_html 中**精确剪切**行 123-302 到新方法
- [ ] 在 element_to_html 中替换为上述调用
- [ ] `cargo test -p omnilang --features codegen-omniview` — 必须 500 passed
- [ ] Commit: `refactor(omniview): extract emit_element_props from element_to_html`

---

### Task 2: 提取 `emit_element_inline_style`（inline style 处理）

**提取内容:** 行 329-444

**方法签名:**
```rust
fn emit_element_inline_style(&mut self, el: &ElementIR, nid: &str, attrs: &mut String)
// 静态 style 追加到 attrs, 动态 style 写 __omni_effect 到 self.js
```

**替换为:**
```rust
self.emit_element_inline_style(el, &nid, &mut attrs);
```

- [ ] 精确剪切行 329-444
- [ ] 替换调用
- [ ] Run tests → 500 passed
- [ ] Commit: `refactor(omniview): extract emit_element_inline_style`

---

### Task 3: 提取 `emit_element_events`（事件绑定 JS）

**提取内容:** 行 446-576（@for 模板的 data-event-* 属性 + 正常模式的 addEventListener）

**方法签名:**
```rust
fn emit_element_events(&mut self, el: &ElementIR, nid: &str, attrs: &mut String)
// 写事件 JS 到 self.js, @for 模式下追加 data-event-* 到 attrs
```

**替换为:**
```rust
self.emit_element_events(el, &nid, &mut attrs);
```

- [ ] 精确剪切行 446-576
- [ ] 替换调用
- [ ] Run tests → 500 passed
- [ ] Commit: `refactor(omniview): extract emit_element_events`

---

### Task 4: 提取 `emit_element_bindings`（双向绑定）

**提取内容:** 行 578-607（bind:value / bind:checked → DOM↔signal 双向同步）

**方法签名:**
```rust
fn emit_element_bindings(&mut self, el: &ElementIR, nid: &str)
```

**替换为:**
```rust
self.emit_element_bindings(el, &nid);
```

- [ ] 精确剪切行 578-607
- [ ] 替换调用
- [ ] Run tests → 500 passed
- [ ] Commit: `refactor(omniview): extract emit_element_bindings`

---

### Task 5: 提取 `emit_element_motion_attrs`（motion data-* 属性 + trigger JS 调用）

**提取内容:** 行 634-720（trigger data-* 属性 + in-view/scroll/drag 子属性 + `self.emit_motion_triggers_for_element` 调用）

**方法签名:**
```rust
fn emit_element_motion_attrs(&mut self, el: &ElementIR, nid: &str, attrs: &mut String)
```

**替换为:**
```rust
self.emit_element_motion_attrs(el, &nid, &mut attrs);
```

- [ ] 精确剪切行 634-720
- [ ] 替换调用
- [ ] Run tests → 500 passed
- [ ] Commit: `refactor(omniview): extract emit_element_motion_attrs`

---

### Task 6: 提取 `emit_element_motion_bindings`（driven-by + paused + states）

**提取内容:** 行 722-770（@driven-by 进度绑定 + @paused 暂停覆盖 + states: 动画状态机）

**方法签名:**
```rust
fn emit_element_motion_bindings(&mut self, el: &ElementIR, nid: &str)
```

**替换为:**
```rust
self.emit_element_motion_bindings(el, &nid);
```

- [ ] 精确剪切行 722-770
- [ ] 替换调用
- [ ] Run tests → 500 passed
- [ ] Commit: `refactor(omniview): extract emit_element_motion_bindings`

---

### Task 7: 提取 `emit_element_html_close`（标签输出 + children + css_only + source map）

**提取内容:** 行 781-923（void tag 处理 + 正常标签输出 + children 递归 + css_only 截断 + 插值 effect + class map binding + source map）

**方法签名:**
```rust
fn emit_element_html_close(
    &mut self, el: &ElementIR, nid: &str, html_tag: &str,
    attrs: &str, class_name_parts: &[String],
    interp_effects: &[String], indent: usize,
) -> String
```

**替换为:**
```rust
let result = self.emit_element_html_close(
    el, &nid, &html_tag, &attrs, &class_name_parts, &interp_effects, indent
);
return result;
```

- [ ] 精确剪切行 781-923
- [ ] 替换调用。注意：`interp_effects` 来自 element_to_html 中保留的 pre-scan（行 609-632，保留在原函数中不移动）
- [ ] Run tests → 500 passed
- [ ] Commit: `refactor(omniview): extract emit_element_html_close`

---

### Task 8: 消除 motion_triggers.rs 中的重复代码

**文件:** `omnilang/src/codegen/omniview/motion_triggers.rs`

**问题:** 行 36-398（named motion ref，`if let Some(ref motion_name)`）和行 399-673（inline motion，`else if !kf_override.is_empty()`）是两个几乎完全重复的 switch 块。唯一的区别是动画创建调用：

- Named: `__omni_motion_{name}(el, {{ trigger: "{trigger}" }})`
- Inline: `Omni.animate(el, {kf}, {{ {opts} }})`

**方案:** 将每种 trigger 类型的 JS 模板提取为独立方法，接受一个 `anim_creation_call: &str` 参数。

**新方法列表（每个 trigger 类型一个）:**

```rust
/// 为 @enter trigger 生成 JS（mount JS + 自动 play）
fn emit_trigger_enter(&mut self, nid: &str, trigger_str: &str, anim_call: &str, is_inline: bool) {
    if !is_inline {
        self.enter_trigger_nids.push((nid.to_string(), /* motion_name */, "enter".into()));
    }
    let target = if is_inline { &mut self.mount_js } else { &mut self.mount_js };
    target.push_str(&format!(
        "// @enter trigger\n\
         (function() {{\n\
         \x20 const el = __omni_el(\"{nid}\");\n\
         \x20 if (el) {{\n\
         \x20\x20 const anim = {anim_call};\n\
         \x20\x20 __omni_animations.set(\"{nid}_{trigger}\", anim);\n\
         \x20\x20 if (el.offsetParent !== null) anim.play();\n\
         \x20 }}\n\
         }})();\n",
        nid = nid, trigger = trigger_str, anim_call = anim_call
    ));
}
```

Wait, the naming gets tricky because the `enter_trigger_nids` push needs the `motion_name` for named refs, but for inline ones it doesn't. Let me reconsider.

Actually, the simplest approach is: keep the **outer structure** (loop over triggers, determine trigger_str, kf_override/opts_override), and extract just the **per-trigger-type JS template emission** into methods that take an `anim_call` parameter.

The anim_call is:
- Named ref: `format!("__omni_motion_{motion}(el, {{ trigger: \"{trigger}\" }})", motion=motion_name, trigger=trigger_str)`
- Inline: `format!("Omni.animate(el, {kf}, {{ {opts} }})", kf=kf_override, opts=opts_override)`

But wait — the named ref branch also does extra things like `self.enter_trigger_nids.push(...)`. The inline branch doesn't have this. So the pure parameterized approach doesn't work perfectly for all trigger types.

Better approach: keep the match on `trigger.motion_ref` vs inline at the **top level**, but extract the **JS template generation for each trigger type** into its own method. The two branches still exist but they each call the same template method with different anim_call strings.

Actually, even simpler and less invasive: keep the current two-branch structure (named/inline) but extract the **largest duplicated blocks** (InView, Scroll, Drag) as shared methods. The small triggers (Enter, Hover, Press, Focus, Leave, Animate, Timeline, default) are only ~20 lines each and having them repeated isn't the real problem — the **real problem** is the 125-line Drag handler and the 50-line Scroll handler duplicated identically.

Let me focus on extracting the 3 big duplicated blocks:

**Extract these 3 JS template methods:**

```rust
/// Generate @in-view trigger JS. Used by both named motion ref and inline paths.
fn emit_trigger_inview_js(&mut self, nid: &str, trigger_str: &str, 
    anim_call: &str, el: &ElementIR) { ... }

/// Generate @scroll trigger JS.
fn emit_trigger_scroll_js(&mut self, nid: &str, trigger_str: &str,
    anim_call: &str, el: &ElementIR) { ... }

/// Generate @drag trigger JS.
fn emit_trigger_drag_js(&mut self, nid: &str, trigger_str: &str,
    anim_call: &str, el: &ElementIR, is_horizontal: bool) { ... }
```

And then the tiny triggers (Enter, Hover, Press, Focus, Leave, Animate, Timeline) can be kept inline in the two branches since they're only 5-15 lines each and the duplication is minimal.

- [ ] **Step 1: 提取 `emit_trigger_inview_js`**

复制 InView 的 JS 模板（named ref 路径中的行 127-171 或 inline 路径中的行 437-478——两者完全一致），提取公共逻辑，接受 `anim_call` 参数。

- [ ] **Step 2: 在两个分支中替换为调用**

Named ref InView case → 替换为 `self.emit_trigger_inview_js(nid, trigger_str, &anim_call, el)`
Inline InView case → 同样调用

- [ ] **Step 3: Run tests → 500 passed**

- [ ] **Step 4: 同样提取 `emit_trigger_scroll_js` 和 `emit_trigger_drag_js`**

- [ ] **Step 5: Final tests → 500 passed**

- [ ] **Commit:** `refactor(omniview): extract shared motion trigger JS templates`

---

### 验证检查清单

重构完成后运行：

```bash
# 1. 全量测试
cargo test -p omnilang --features codegen-omniview
# 期望: 500 passed, 0 failed

# 2. 编译检查（无 warning）
cargo check -p omnilang --features codegen-omniview

# 3. 确认 element_to_html 行数
wc -l omnilang/src/codegen/omniview/view/element.rs
# 期望: ~300-350（从 927 降到 ~350）

# 4. 确认 motion_triggers.rs 行数
wc -l omnilang/src/codegen/omniview/motion_triggers.rs
# 期望: ~350-400（从 676 降到 ~400）
```
