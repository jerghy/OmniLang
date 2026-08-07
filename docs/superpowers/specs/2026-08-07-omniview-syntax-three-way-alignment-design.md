# OmniView 语法挖掘三方对齐设计

**日期**: 2026-08-07
**状态**: Approved（用户三节确认：范围=全面三方对齐；矛盾处置=文档对齐编译器（保守）；方法=脚本骨架驱动；域 7=一次全收）
**依据**: 语义集维护约定（新增语法必须先登记）+ 编译器事实源

## 背景与目标

近两周实现大量新语法（@drag/@dragEnd 意图指令 ADR-073、style 直写通道 D1/D2、view 属性三态 ADR-068、@ref 语义绑定全集），demo 覆盖审计与文档已随实现跟进（d4d1d0e6 等提交），但**语义集登记册（`semantic-universe.md`）未同步**——@drag 标注"尚非登记册成员，演进见后续 spec 同步"。本次通过系统挖掘编译器实际支持的语法全集，完成三方对齐：

1. **语义集登记册**（`docs/superpowers/specs/semantic-universe.md`）——补域 7
2. **语法文档**（`docs/omniview/`，16 份）——核对补漏
3. **展示项目**（`test/projects/learn-omniview/src/`，102 文件）——补展示

**原则**：文档/demo/登记册对齐编译器现状（编译器是 ground truth）；确认的编译器 bug 只列清单不修（用户拍板：保守）。

## 挖掘框架

### 编译器事实源（ground truth）

| 事实源 | 内容 |
|--------|------|
| `crates/omnilang-core/src/parser/view/element.rs` | 元素属性/事件/@ref/@drag/bind:/style:/class:/属性三态解析 |
| `crates/omnilang-core/src/parser/top_level/motion.rs` | 19 动画属性白名单 + 8 触发器 |
| `crates/omnilang-core/src/semantic/validate_*.rs` | 验证器（array 3 方法、drag 6 验证项、ref ops、bindings、a11y） |
| `crates/omnilang-core/src/ir/types.rs` | RefOpIR 10 ops、DragBindingIR、EventHandlerIR、语义绑定 |
| `crates/omnilang-core/src/codegen/shared/event_names.rs` | 54 canonical 事件名 |
| `crates/omnilang-core/src/parser/design.rs` | 13 token 类型 |
| `omnilang/src/codegen/`（三目标） | 直写通道、@for 委派、class 映射、语义绑定发射 |
| `omnilang/src/codegen/shared/direct_write.rs` | 信号域分类 D1/D2/B2 |

### 三方参照

1. 语义集登记册（6 域：可动画属性/数组操作/事件/RefOp/design token/值绑定）——**缺域 7**
2. docs/omniview 16 份文档——@drag/属性三态已覆盖（d4d1d0e6、ADR-068 文档），重点核对 @for 委派、直写通道行为、语义绑定全集
3. learn-omniview demo——覆盖审计 0 缺口（基于旧登记册 6 域），新域入册后需重审

## 差异清单机制

1. 基线：5 个验证脚本全绿（已跑：semantic-universe 0 / vocabulary 0 / syntax-lifecycle 0 / docs NOTE-only / demo-coverage 0 缺口 6 豁免）
2. 按 6 域逐域核对脚本断言盲区（脚本已断言的域不再人工复核）
3. 从 parser 元素解析点顺藤摸瓜挖登记册外语法
4. 产出差异清单（语法点 / 编译器事实 / 三方缺口定位）→ 交付时附审

## 域 7：元素指令与绑定（一次全收，8 成员）

| 成员 | 语义 | 事实源 | 实现状态 |
|------|------|--------|---------|
| `@drag` | 拖拽意图指令（指针捕获编译器选路） | `parse_event_prop` + `validate_drag.rs` + `DragBindingIR` | 支持（三目标） |
| `@dragEnd` | 拖拽结束回调 | 同上 | 支持（三目标） |
| `@ref={name}` | 元素引用绑定 | `ref_binding` + `RefDeclIR` | 支持 |
| `focus={sig}` | 声明式焦点绑定 | `SEMANTIC_BINDINGS` 常量（parser/view/element.rs） | 支持 |
| `scroll-x={sig}` | 声明式水平滚动绑定 | 同上 | 支持 |
| `scroll-y={sig}` | 声明式垂直滚动绑定 | 同上 | 支持 |
| 动态 style 绑定（直写通道） | `style: { prop: sig }` 动态条目 → D1/D2 直写 | `codegen/shared/direct_write.rs` | 支持（三目标） |
| 属性三态 | `class="x"` 裸布尔 / `prop={expr}` 动态属性 | `HTML_BOOLEAN_ATTRS`（19 项，omniview codegen）+ `is_expr` 判定 | 支持 |

**语义域定义**（草稿）：元素属性层语义域——**声明在元素上的指令与绑定**（元素可拖拽 / 元素可被引用 / 声明式 DOM 副作用 / 动态样式 / 属性形态），与事件（域 3）和命令式 RefOp（域 4）互补：指令表达意图、绑定表达声明、RefOp 表达命令。

## 脚本扩展

- `verify-semantic-universe.py`：
  - `DOMAIN_TITLES` 加域 7（`"元素指令与绑定"`）
  - `range(1, 7)` → `range(1, 8)`
  - 域 7 交叉断言（4 组事实源映射）：
    - @drag/@dragEnd ↔ parser element.rs `parse_event_prop` drag 分派 + `validate_drag.rs` 存在性
    - @ref ↔ `RefDeclIR` / element `ref_binding`
    - focus/scroll-x/scroll-y ↔ `SEMANTIC_BINDINGS` 常量
    - 动态 style 直写 ↔ `codegen/shared/direct_write.rs` 存在性
    - 属性三态 ↔ `HTML_BOOLEAN_ATTRS`（omniview codegen 常量）
  - 白名单小节要求：支持/部分条目 ⊆ 白名单
- `verify-demo-coverage.py`：
  - EXEMPT 更新：capturePointer/releasePointer 豁免理由改写——"@drag 入册，意图形态由 @drag 承担（同一演示语义的意图形态）"
  - 域 7 成员审计接入（usage 函数新增域 7 分支：@drag/@dragEnd/@ref/focus=/scroll-x=/scroll-y=/style 动态条目/裸布尔属性）

## 文档核对清单

- 逐份 vs 编译器事实核对（16 份），已知覆盖良好项不重写：
  - 06-templates：@for 委派细节、属性三态（已覆盖——确认即可）
  - 09-styling：直写通道行为（d4d1d0e6 已加——确认即可）
  - 05-components：@ref/语义绑定全集（已覆盖——确认即可）
  - 15-grammar-view：@drag BNF（d4d1d0e6 已加——确认即可）
  - 00-cheatsheet：@drag/直写（已覆盖——确认即可）
  - 其余文档快速核对
- 登记册域 7 引用的文档链接补写（如适用）

## demo 补展示

- 域 7 成员入册 → `verify-demo-coverage.py` 重跑 → 未展示成员补 demo：
  - 预计缺口：scroll-x 语义绑定（scroll-y 已有）、属性三态裸布尔（ViewAttributesDemo 可能已有——核对）、动态 style 直写（ElementRefDemo 已有）
- demo 改动需过 WASM 编译门禁（`omnic build` learn-omniview）

## 验证门禁

| 门禁 | 通过标准 |
|------|---------|
| `verify-semantic-universe.py` | 0 problem |
| `verify-demo-coverage.py` | 缺口归零 |
| `verify-vocabulary.py` / `verify-syntax-lifecycle.py` / `verify-docs.py` | 0 problem |
| `omnic build`（learn-omniview，OmniView 目标） | WASM 编译通过 |
| cargo test（如脚本依赖 Rust 常量变更——预期不动编译器） | 全绿 |

## 提交策略（分主题）

1. `docs(universe): 域 7 元素指令与绑定登记 + verify-semantic-universe 断言扩展`
2. `docs(omniview): 文档核对补漏`（如有）
3. `feat(demo): learn-omniview 新域成员展示补齐`（含当前未提交的 ElementRefDemo/Overview 直写注释，并入本次）

## 交付物

- 差异清单（挖掘产出）
- 编译器 bug 清单 → `tasks/omniview-syntax-audit-bugs.md`（只列不修）
- 门禁结果汇总

## 范围外

- 编译器 bug 修复（用户拍板：保守，只列清单）
- 语义集登记册现有 6 域内容重审（脚本已断言，不重写）
- 其他目标（vue/dioxus 等）语法盘点（本次聚焦 omniview 语义域）
