# OmniView 语法审计——编译器 bug 清单（待批，不修）

**原则**：本次审计只列不改（用户拍板：文档对齐编译器保守处置）。B2 已排期修复（eb651652）。

**审计日期**：2026-08-07（Task 4，三向对齐：docs/omniview ↔ 编译器 ↔ 运行时产物）
**验证工具**：`omnic 0.2.0`（`omnic analyse` = 静态分析；`omnic build -t omniview` = OmniView 编译；产物核对生成 JS/HTML）

| # | 语法点 | 编译器事实 | 文档宣称 | 差异 | 建议 |
|---|--------|-----------|---------|------|------|
| B1 | `open` 布尔属性 | parser 15 关键字无 open；`HTML_BOOLEAN_ATTRS`（12 项）与 `emit_element_props` 已知属性白名单（~70 项）均无 open（修复前快照——修复后为 13 项含 open，见 B2） | 06-templates.md 布尔属性示例使用 `<details open={isOpen}>` | **实测编译通过**（`omnic analyse` No issues found，exit 0；`omnic build -t omniview` 成功，React 目标亦正确透传）→ 按核对判据「编译通过 = 文档示例合法」，**差异关闭，文档零改动** | 关闭（见 B2 遗留） |
| B2 | 未知属性绑定（open 为文档可见实例） | OmniView 目标**静默丢弃**：open 落入未知属性 fallback（`omnilang/src/codegen/omniview/view/element.rs` `emit_element_props` `_ =>` 分支），静态发射 `data-open="{isOpen}"`，**不生成任何 `__omni_effect`**；生成的运行时无通用 data-* 绑定解析器（setAttribute/dataset 零消费；data-for-key 为 @for 克隆专用）→ `<details>` 运行时**恒关闭**；静态 `<details open>` 同样降级为 `data-open="true"`。全程零诊断（parse/analyse/build 均无警告）。React 目标 `<details open={isOpen}>` 正确透传。同一通道影响所有白名单外属性（如 `data-tag={count}`）；`expr_to_static_str` 为空时整条属性静默删除（`continue`） | 06-templates.md 布尔属性示例将其与 `checked`/`disabled`/`readonly`/`required` 并列（后者均正常生成 `el.attr = !!signal` 效果） | **编译器行为缺陷**（零诊断静默退化模式）：文档示例合法（编译判据），但 OmniView 目标上绑定语义丢失 | 已修复：eb651652（① open 入册 HTML_BOOLEAN_ATTRS 13 项走 el.open = !!() 分支 + ③ fallback 动态绑定 warning——裸未知属性名 + 非 data-/aria- 前缀）；② 未实施（data-*/aria-* 前缀保持静默降级——有意设计，用户自定义透传通道） |
| B3 | variant 参数值 class 误报 flood（管线修复暴露） | `expand_variant` **有意**返回 variant 名 + dimension 值进 class（`omnilang/src/codegen/shared/design.rs:190`；测试 `expand_variant_returns_classes` 明确断言 `["Button","primary","sm"]`），但 `emit_element_props` class 验证循环（`omniview/view/element.rs:316-332`）只查 `design_class_names`（`design.classes` 注册集——variant 名与参数值不在其中）→ **数百条误报 warning**（learn-omniview 实测：`class 'Button' referenced ... is not defined` ×118、`'sm'` ×92、`'primary'` ×45 等）。渲染正确（variant CSS 展开正常，真机验证样式无误）——纯诊断噪音 | 无（文档未声称 variant 参数是 class） | **编译器 warning 逻辑缺陷**（误报）：variant 展开产物未从 class 验证豁免。B2 管线修复（eb651652）使本已存在的警告可见——暴露此缺陷 | 待批。修复方向：class 验证循环豁免 variant 展开产物（`expand_variant` 返回的类名不参与「未定义 class」验证——它们由 variant CSS 生成承担），或 `expand_variant` 返回的类名并入 `design_class_names` 注册 |

**核对附注**（非 bug，记录备查）：
- `omnic build`/`analyse` 偶发环境级 warning：`[omni] warning: skipping inaccessible entry: IO error for operation on : 系统找不到指定的路径。 (os error 3)`（空路径，疑似目录自动发现探测）——不影响编译结果，未入清单。
- 其余语法点抽查（@if/@for/@empty、bind:value/bind:checked、emit、@motion 8 触发器、@store/getter/action、variant/@themes/@default-theme、import 四形态 + pub 可见性、a11y 块、@drag/@dragEnd、focus/scroll-x/scroll-y、直写通道）**均与编译器行为一致**，无失实。
