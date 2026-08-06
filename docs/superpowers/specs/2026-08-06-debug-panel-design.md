# 调试面板设计（Design Canvas 调试台）

> 日期：2026-08-06 ｜ 状态：设计已确认 ｜ 方案：升级现有正则覆盖管线（方案 A）

## 1. 背景与目标

Design Canvas 中 Artboard 旁有一个浮动面板 `ComponentDefPanel`（画布内，Header 中间是 🔧 调试按钮）。它当前是「组件定义管理」面板：Props / Signals / Computed 三段的增删改，调试能力只有 signal 行的内联输入框。

**目标**：把面板重构成**以调试为主**的工具——开发 OmniLang 的 `@if` / `@for` / `@empty` 等视图语法时，临时改信号值驱动预览组件即时变化，无需保存落盘。调试的同时要求「看得舒服、不干扰思路、让思路更清晰」。

**现状机制**（保留管线、升级能力）：

```
🔧 点击 → debugMode → signal 行输入框
→ store.debugSignalValues
→ useWasmCompile（150ms debounce）
→ applyDebugSignalOverrides()：正则替换生成源码中 `signal name: Type = 默认值` 文本
→ WASM 重编译 → 预览更新
```

现状局限：只有组件 signal 可调；数组值被当字符串包引号；跨组件同名信号被 `flatDebug` 扁平化互相覆盖；无校验；无快捷候选。

## 2. 决策记录（用户逐项拍板）

| # | 决策 | 选择 |
|---|------|------|
| D1 | 调试覆盖范围 | **prop + signal + store 全覆盖**（组件 signal、组件 prop、内联 store、`@store` 引用的全局 store） |
| D2 | 数组输入方式 | **类型感知解析**（按声明类型转 OmniLang 字面量） |
| D3 | 面板定位 | **调试台为主，定义管理次之**（默认调试视图，Header 切换回定义管理） |
| D4 | 三分类形式 | **标签页 Tab**：定义 / 信号调试 / 界面信号调试；**默认「界面信号调试」** |
| D5 | 可选值来源 | **代码提取 + 类型默认**（代码提取 = 从 view 树 @if/@for 条件中引用该信号的字面量；后经用户修正为「操作条件本身」，见 D6） |
| D6 | 条件分支交互 | **操作条件本身**——「选择让哪个条件成真」，不做值候选提取（条件形态千变万化：`count > 0`、裸 bool、复合条件，提取候选做不干净） |
| D7 | 信息架构 | **信号为中心**（真实项目 review 验证）：界面驱动信号一张卡，卡内列出该信号的全部界面用途 + 调试控件；多 @if 链按信号聚合 |
| D8 | 实现方案 | **方案 A：升级现有正则覆盖管线**（零侵入 codegen、不碰 Rust/wasm）；方案 B（Rust 侧编译参数）记入长期路线 |
| D9 | 输入框与快捷控件 | 输入框永远是主角（自由键入，支持 OmniLang 字面量透传）；快捷控件（胶囊/步进）点选**即生效**并回填输入框 |
| D10 | 复杂类型数组 | struct 数组无法文本输入 → 「**代码样本**」胶囊：从组件 fn 赋值字面量提取真实样本（如 DataTable 的 `initRows()` 10 行） |
| D11 | store qualified 比较 | 合成规则支持 `Store.sig op v` 带限定名形式（`NotificationStore.unreadCount == 0`） |
| D12 | 无 else 单分支 @if | 单分支为**二态开关**（成真/成假），非多分支多选一 |

## 3. 语法研究结论（设计依据）

来自 `docs/omniview/15-grammar-view.md` 与真实项目 review（`test/projects/learn-omniview/src`）：

- **@if 链**：`@if expr { ... } (@else if expr { ... })* (@else { ... })?`，支持 `mode: "fade"|"wait"|"none"`。一个组件**可有多个 @if 链**，可嵌套。
- **@for**：`@for <item> in <expr> key={expr} [scroll]? { ... } @empty { ... }`。collection 可以是信号、`Store.sig`、**表达式**（`items.filter(...)`）。
- **@empty**：空列表回退分支——空数组 `[]` 即可触发。
- **view 级 vs fn 级**：界面渲染结构只由 **view 树的 @if/@for 驱动**；fn 内的 if 是逻辑分支，不驱动渲染结构（DataTable 验证）——**界面驱动分析只看 view 树**（`condition` / `for-in` / `while` 设计节点）。
- 信号在界面中的全部用途：条件（@if/@while）、列表（@for）、插值 `{sig}`、双向绑定 `bind:value`、条件类名 `class: {k: v}`、样式 `style: {k: v}`、computed 依赖、@effect 依赖、子组件 prop 传递。

## 4. 架构设计

### 4.1 数据模型（store 改造）

替换现有 `debugSignalValues`：

```ts
// store/types.ts
interface DebugValueEntry {
  text: string;    // 输入框原始文本（回显用；非法输入也保留，标红待修正）
  valid: boolean;  // 类型校验结果——false 不参与编译
}
debugValues: Record<ownerKey, Record<name, DebugValueEntry>>
// ownerKey：组件名（prop/signal）| store 名 | 内联 store "Component.Store"
```

- `setDebugValue(ownerKey, name, text)` → 解析+校验 → 存 entry
- `clearDebugValues()` → 全部重置（「重置全部」按钮）
- 退出调试模式 / 切换组件 Tab：值保留（按 ownerKey 分组天然隔离），但可一键重置

解析值不存 store（解析结果只存在于编译瞬间）；输入框回显的是**用户敲的文本**。

### 4.2 信号分析器（新模块 `services/debug/signal-analysis.ts`）

输入：当前组件设计树（view 节点 `condition` / `for-in` / `while` 元素）+ `componentDefs` + `storeDefs` / `inlineStoreDefs`。

输出（按信号聚合）：

```ts
interface SignalUsage {
  key: string;                  // 信号 key："status" | "Store.sig" | "Comp.Store.sig"
  declaredType: string;         // 声明类型（i32 / Array<String> / RowItem[] …）
  owner: string;                // 组件名 | store 名
  conditions: IfChain[];        // 该信号参与的所有 @if 链
  loops: ForUsage[];            // 该信号参与的所有 @for
  uses: {                       // 用途计数（界面徽章）
    interpolations: number;
    binds: string[];            // bind:value 等
    classGates: number;
    styleRefs: number;
    computedDeps: number;
  };
}

interface IfChain {
  branches: IfBranch[];         // @if / @else if / @else 分支
}
interface IfBranch {
  exprText: string;             // 条件表达式文本（else 为 null）
  isElse: boolean;
  comparisons: Comparison[];    // 从条件中解析出的「信号-运算符-字面量」
}
interface Comparison {
  sig: string;                  // 信号 key（可带 Store 限定）
  op: "==" | "!=" | ">" | ">=" | "<" | "<=" | null;  // null = 裸 bool
  literal?: string;             // 比较字面量（若有）
}
```

分析规则：
- 遍历 view 树 `condition` 节点 → `props.condition` 文本（如 `count <= 20`）→ 解析参与的信号（正则识别 `[A-Za-z_]\w*` 与 `Store.sig` 限定名）与比较形式
- `for-in` 节点 → `props.collection` 文本 → 参与信号；collection 含 `filter`/`map`/函数调用 → `viaTransform: true`（标注「经转换」）
- 一个信号出现在多个链 → 同一信号卡内多链平铺（链间空行分隔）
- 用途计数：插值 `{sig}`、`bind:value={sig}`、`class: {k: sig}`、`style: {k: sig}`、computed 表达式依赖、`@effect(sig)` 依赖

### 4.3 覆盖管线（新模块 `services/debug/debug-overrides.ts` + useWasmCompile 接线）

**文件级键控**（修复现有 flatDebug 跨组件同名冲突）：

```
ownerKey → 目标文件：
  组件 X         → src/components/X.omni（X === "main" → src/main.omni）
  独立 store S   → src/stores/S.omni
  内联 Comp.S    → src/components/Comp.omni
```

替换逻辑（按 (文件, 名字) 精确定位，同一文件内重名编译器本就会报错，天然无冲突）：

```
(signal|prop)\s+<name>\s*:\s*\S+\s*=\s*[^;]+;  →  $1 调试字面量;
无默认值行（signal x: Type;）                    →  补 " = 调试字面量;"
```

**类型感知解析器** `parseDebugValue(type, text) → { literal, valid }`：

| 声明类型 | 输入示例 | 转出字面量 | 校验 |
|---|---|---|---|
| `String` | `hello "world"` | `"hello \"world\""` | 任意文本（转义引号） |
| `bool` | `true` / `false` | `true` | 仅这两个词 |
| `i32`/`i64`/`f64`/`f32` | `42`、`3.14` | `42` | 数字正则 |
| `Array<String>` | `a, b, c` | `["a", "b", "c"]` | 逗号分隔，逐项转义 |
| `Array<i32>` | `1, 2, 3` | `[1, 2, 3]` | 逐项数字校验 |
| `Array<struct>` | （不可手动输入） | 代码样本胶囊 | 从 fn 赋值提取 |
| 其他（Map/自定义） | — | 原样透传 | 不校验（提示手写字面量） |

**字面量透传识别**（D9）：输入以 `[` / `{` / `"` / `'` 开头，或整体是数字 / `true` / `false` → 原样透传，不二次转换。效果：`["a","b","c"]`、`{id:1, name:"x"}` 直接可用。

**代码样本提取**（D10）：对 `Array<复合类型>` 信号，扫描组件 fn 源码（design 树 fn 元素文本）中 `name = [ ... ]` 的赋值（正则 + 平衡括号提取数组字面量）→ 作为「样本」候选胶囊（`[样本: initRows() 10行]`），**原样注入**不解析；含函数调用/展开的赋值不可提取（标记跳过）。空数组 `[]` 总是可候选（触发 @empty）。

### 4.4 成真/成假合成规则（D6/D11/D12）

只支持**单信号简单比较**（可带 `Store.` 限定）。合成目标真/假值后写入调试值：

| 条件形式 | 成真 | 成假 |
|---|---|---|
| `sig`（bool） | `true` | `false` |
| `!sig` | `false` | `true` |
| `sig == v` | `v` | bool 取反 / 数字 `v≠0 ? 0 : 1` / 字符串 `v≠"" ? "" : "x"` |
| `sig != v` | 上列「成假」取值 | `v` |
| `sig > n` | `n + 1` | `n` |
| `sig >= n` | `n` | `n - 1` |
| `sig < n` | `n - 1` | `n` |
| `sig <= n` | `n` | `n + 1` |

**边界**：
- 复合条件（`a > 0 && b < 10`）、函数调用（`items.len() > 0`）、非比较表达式 → 分支胶囊**置灰** + tooltip「复杂条件，请在下方输入框手动调」
- else 分支 → 对每个条件各自取「成假」值（交叉冲突极端情况不处理——启发式足够调试用）
- 无 else 单分支 @if（D12）→ 二态开关：成真 / 成假
- 点击写入的是**调试值**（走同一覆盖管线），不落盘、不改源码、退出即还原

### 4.5 UI 设计（ComponentDefPanel 重构）

三 Tab 结构，默认「界面信号调试」（D3/D4）：

```
┌ main · 调试台 · 2条件 1列表 2数据 ┐   ← Header：组件名 + 用途计数徽章 + 🔧 + 折叠
│ [界面信号调试] [信号调试] [定义]    │   ← Tab 栏（默认第一个）
├────────────────────────────────────┤
│ ● 界面信号调试 Tab                  │
│  ┌ status (String) · 条件3 插值1 ┐  │
│  │ @if status == "loading"   ⦿   │  │   ← 分支胶囊：点选驱动成真
│  │ @else if status == "error" ○  │  │
│  │ @else                    ○   │  │
│  │ 值: [loading]            ✓    │  │   ← 输入框（自由输入）+ 校验状态
│  └───────────────────────────────┘  │
│  ┌ rows (Array<RowItem>) · 列表1 ┐  │
│  │ @for row in rows key={row.id} │  │
│  │ @empty 空态                   │  │
│  │ 值: [________]                │  │
│  │ [空] [样本: initRows() 10行]  │  │   ← 数组胶囊：空 / 代码样本 / 手动
│  └───────────────────────────────┘  │
├────────────────────────────────────┤
│ ● 信号调试 Tab                      │
│  ┌ count (i32) · 插值2  computed1 ┐ │
│  │ 值: [0]  − ＋                  │ │   ← 数值：输入框 + 步进
│  └───────────────────────────────┘ │
├────────────────────────────────────┤
│ ● 定义 Tab（现功能原样迁入）        │
│  Props / Signals / Computed 三节    │
└────────────────────────────────────┘
```

设计原则：
1. **信号为中心**：每个界面驱动信号一张卡，卡头 = 信号名 + 类型 + 用途徽章（条件 N · 列表 N · 插值 N · 绑定 N · computed N）——「看的时候思路更清晰」：一眼知道信号在界面扮演什么角色
2. **分支胶囊**：@if 链的每个分支一个胶囊，条件文本为标签（等宽小字截断），点选即驱动成真并高亮；当前值徽章与胶囊联动
3. **胶囊与输入框联动**：点胶囊 → 合成真值 → 回填输入框（可再手改）；手改值 → 胶囊高亮取消
4. **折叠不丢信息**：Header 计数徽章（2条件 1列表 2数据）——收成细条仍知道有什么可调
5. **密度低、留白多**：一行一个信号，行高宽松——「看的时候不影响思路」
6. 配色走 `--ol-*` token；调试激活态主色强调，与定义管理灰色调区分

### 4.6 错误处理

| 场景 | 行为 |
|---|---|
| 输入非法（数字信号输 `abc`） | 输入框红框 + 右侧 ⚠（tooltip 说明），**不触发编译**，文本保留待修正 |
| 复杂条件分支点选 | 胶囊置灰 + tooltip「复杂条件，请在输入框手动调」 |
| 覆盖目标缺失（信号已删/改名） | 该调试值行灰显「信号已不存在」提示，不参与编译 |
| 代码样本提取失败（赋值含函数调用） | 样本胶囊不出现（不展示失败信息，安静降级） |

## 5. 测试计划

**单元测试**（新模块）：
- `parseDebugValue`：全类型表逐项（String 转义 / bool 严格 / 数字 / 数组 / 透传识别 / struct 数组拒绝）
- 合成规则表：8 种条件形式 × 真/假 全项断言
- `applyDebugOverrides`：文件级键控（两组件同名 signal 各替换各的）、无默认值补值、字符串转义、store 文件定位、内联 store 定位
- `signal-analysis`：多 @if 链聚合、store qualified 识别、view 树 vs fn 图（fn 内 if 不参与）、用途计数、filter/map 转换标注
- 代码样本提取：平衡括号、含函数调用跳过

**组件测试**：
- 三 Tab 渲染、默认「界面信号调试」
- 分支胶囊点选 → 调试值写入 + 输入框回填
- 复杂条件胶囊置灰
- 非法输入 → 红框 + 不编译

**现有测试适配**：`debugSignalValues` → `debugValues` 字段迁移（useWasmCompile 相关测试）。

## 6. 不做（遗留项）

- **预览联动高亮**：hover 调试值行 → 预览中引用该信号的元素呼吸高亮（需 signal→nid 引用映射，成本中等，后续增强）
- **computed 覆盖**：computed 是派生值，覆盖其 signal 源更自然（YAGNI）
- **反向确定当前渲染分支**：运行时信息 WASM 输出不含，不做
- **方案 B（Rust 侧编译参数覆盖）**：记录为长期路线（结构最正确，代价是 wasm crate 重编译 + 编译器 API 变更，本次不做）

## 7. 涉及文件（预估）

| 文件 | 动作 |
|---|---|
| `poiesis/src/store/types.ts` / `store/index.ts` / `slices/component-lifecycle.ts` | `debugValues` 字段替换 + setter/clear |
| `poiesis/src/services/debug/signal-analysis.ts` | 新建：信号聚合分析 |
| `poiesis/src/services/debug/debug-overrides.ts` | 新建：类型解析 + 文件级覆盖 + 合成规则 + 样本提取 |
| `poiesis/src/components/design/useWasmCompile.ts` | 接线新覆盖管线 |
| `poiesis/src/components/design/ComponentDefPanel.tsx` | 三 Tab 重构 + 调试卡 UI |
| `poiesis/src/components/design/ComponentDefPanel.css` | 新样式 |
| 相关 `__tests__` | 新建 + 适配 |
