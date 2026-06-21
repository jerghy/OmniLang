# OmniLang: `omnic analyse` as Gatekeeper for `omnic run`

**日期**: 2026-06-21
**目标**: 使 `omnic analyse` 成为 `omnic run -t omniview` 的可靠守门人。如果 analyse 通过，run 必须成功。

---

## 当前状态

`omnic analyse` 运行：解析 → 语义（符号、名称解析、依赖、验证）→ IR 降低 → 5 条 lint 规则。IR 降低已经存在（`lib.rs` 第 751-758 行），操作于单模块和合并后的多模块 IR 上。然而，管道遗漏了许多会导致运行时静默失败的故障模式类别。这份设计通过三个互补的新组件来消除这些差距。

## 架构概览

```
omnic analyse
    │
    ├─ 解析 (词法分析器 + 解析器)
    ├─ 语义第 1-4 阶段 (符号、名称、依赖、验证) [E0001-E0006]
    ├─ 类型检查 第 5 阶段 [T0001-T0012]                         ← 第 2 节（AST 级别）
    │   └─ 第 5a 阶段：降低前 AST 检查 [E0020 移至此处]           ← NaN/Inf 在降低之前被捕获
    ├─ IR 降低（已存在于 lib.rs:751-758）                         ← 在降低期间，E0020 在此发出
    ├─ NotYetImplemented IR 变体                                  ← 第 1 节推动因素
    ├─ 合约验证 [E0007-E0032]                                     ← 第 1 节（IR 级别）
    │   ├─ Store 合约 (E0007-E0009)
    │   ├─ Component 合约 (E0010-E0014)
    │   ├─ View 合约 (E0015-E0018, E0030)
    │   ├─ Expr 合约 (E0019, E0021-E0022)   （E0020 移至降低阶段）
    │   ├─ Motion 合约 (E0023-E0024)
    │   ├─ Codegen 合约 (E0025)              （E0026 移至代码生成断言）
    │   └─ 跨文件合约 (E0027-E0029, E0031)
    │
    └─ 结果：成功 → IR 有效 + 类型正确 + 跨文件一致
                      │
                      ▼
                omnic run
                      │
                      ├─ 代码生成（带有崩溃安全处理 + E0026 NID 断言）  ← 第 3 节
                      ├─ 写入文件
                      └─ 打开浏览器
```

---

## 第 1 节：IR → 代码生成合约系统

### 概念

位于 IR 降低和代码生成之间的验证层。合约是 IR 必须满足的不变量，以便 OmniView 代码生成能够产生正确的 JS/HTML/CSS。如果任何合约失败，分析将报告错误，并且不会进行代码生成。

### 合约 Trait

```rust
pub trait Contract {
    fn code(&self) -> &'static str;        // "E0007"
    fn name(&self) -> &'static str;        // "store-field-exists"
    fn domain(&self) -> Domain;            // Store | Component | View | Expr | Motion | Codegen | Cross
    fn severity(&self) -> Severity;        // Error | Warning
    fn description(&self) -> &'static str;
    fn check(&self, ir: &IRProgram, files: &HashMap<String, String>,
             diags: &mut Vec<ContractViolation>);
}
```

### 合约注册表（26 份合约）

| 代码 | 域 | 名称 | Sev | 预防 |
|------|-----|------|-----|------|
| E0007 | Store | `store-field-exists` | Error | `store.wrongField` → JS ReferenceError |
| E0008 | Store | `store-alias-resolves` | Error | `@store Missing` → undefined |
| E0009 | Store | `store-default-is-literal` | Warn | `"<runtime expr>"` 字符串（仅限字面量默认值应通过） |
| E0010 | Component | `component-tag-exists` | Error | `<Missing />` → 空 div |
| E0011 | Component | `signal-name-resolves` | Error | `{nonexistent}` → undefined（需要步行者作用域跟踪） |
| E0012 | Component | `mutation-target-exists` | Error | `x = 5` 其中 x 不是 signal（需要作用域） |
| E0013 | Component | `function-ref-resolves` | Error | `@onPress={badFn}` → undefined（需要作用域） |
| E0014 | Component | `event-emit-valid` | Warn | `emit missingEvent`（需要作用域） |
| E0015 | View | `view-not-empty-child` | Warn | 没有内容的非 void 元素（豁免：br、img、input、hr、meta、link 等） |
| E0016 | View | `component-tag-format` | Warn | `<comp>` 不是 `<Comp>`（豁免已知的 HTML 标签如 `<div>`） |
| E0017 | View | `for-template-vars-in-scope` | Error | @for 模板 eval() 引用未定义（需要作用域） |
| E0018 | View | `bind-target-is-signal` | Warn | `bind:value={computed}` → 写入只读（豁免 store.field 模式） |
| E0019 | Expr | `no-not-yet-implemented` | Error | NYI 表达式 → 静默 null |
| E0020 | Lower | `style-value-finite` | Error | **在降低期间发出**，在 NaN→0 转换发生之前，而不是作为 IR 合约 |
| E0021 | Expr | `no-empty-ternary-both` | Warn | 两侧都为 null 的三元表达式（豁免合法模式如 `{cond ? <x/> : null}`） |
| E0022 | Expr | `no-null-iterable` | Warn | `@for x in null`（仅捕获字面量 null) |
| E0023 | Motion | `motion-ref-exists` | Error | `@onPress={missingMotion}`（需要作用域） |
| E0024 | Motion | `keyframe-from-has-to` | Error | 关键帧有 `from` 没有 `to` |
| E0025 | Codegen | `global-signal-uniqueness` | Error | 跨 component/store 的信号名称冲突 |
| E0026 | Codegen | `nid-determinism` | Assert | **代码生成 `debug_assert!`**，不是 IR 合约 —— NID 在 IR 中不存在 |
| E0027 | Cross | `import-resolves` | Error | 跨文件导入目标缺失 |
| E0028 | Cross | `prop-names-match` | Warn | 父组件传递了未知的 prop（豁免 HTML 属性如 `class`、`id`、`data-*`） |
| E0029 | Cross | `logic-view-boundary` | Error | 逻辑文件导入视图文件 |
| E0030 | View | `single-root-child` | Error | 视图块有多个根子节点 —— 只有一个能被降低，其他的会静默丢失 |
| E0031 | Store | `computed-ref-valid` | Error | Store 计算属性引用的信号存在 |
| E0032 | Codegen | `two-way-binding-safety` | Warn | `bind:value` 在同时有 `@onChange` 的信号上（可能导致循环） |

### 关键推动因素：`NotYetImplementedIR` + E0020 降低阶段检查

**NotYetImplementedIR**：向 `ExprIR` 添加一个显式变体，以便将静默 null 降级变为错误：

```rust
// 在 ir/mod.rs 中，添加到 ExprIR 枚举：
NotYetImplementedIR {
    feature: String,
    source_line: u32,
    source_col: u32,
},
```

`lower_expr` 通配符发出此变体，而非 `LiteralExpr { Null }`。E0019 捕获所有实例。

**E0020 必须在降低阶段发出**，而不是作为 IR 合约。`ir/mod.rs` 第 991-1001 行中的 NaN→0 转换在合约验证器看到 IR *之前*就静默地对值进行了消毒处理。修复：
1. 在 `lower_expr` 中的 NaN 转换之前添加一个 `diagnostics.push()` 调用
2. 从合约注册表中移除 E0020（它不是一个合约 —— 它是一个降低阶段诊断）
3. 降低在 E0019 和 E0020 被捕获后继续进行，但因为它们是错误，分析会报告它们

**E0026 必须是代码生成断言**，而不是 IR 合约。NID 在代码生成期间通过 `omniview.rs` 中的 `next_nid()` 进行分配 —— 它们不存在于 IR 中。修复：
1. 在代码生成中添加一个 `debug_assert!`，验证 NID 在分配到相同标签时是确定性的
2. 从合约注册表中移除 E0026
3. 保留代码作为第 3b 节的一部分（代码生成崩溃安全）

### 共享遍历 + 作用域跟踪基础设施

单个 IR 遍历器（`ExprWalker`、`StmtWalker`、`ViewWalker`），所有合约共享。遍历器维护一个**作用域栈**用于上下文感知检查：

```rust
struct ContractContext {
    /// 当前组件（我们正在遍历其 IR 的那个）
    current_component: Option<String>,
    /// 当前 store（我们正在遍历其 IR 的那个）
    current_store: Option<String>,
    /// 我们是否在 @for 循环体内（带有循环变量名）
    for_item_name: Option<String>,
    /// 当前函数是否是 async
    is_async: bool,
    /// 循环深度（用于 break/continue 检查）
    loop_depth: u32,
}
```

遍历器在进入/退出组件和 store IR 节点时推送/弹出作用域。这使得像 E0011（signal-name-resolves）这样的合约能够将 `IdentifierExpr { name: "count" }` 解析到正确的组件作用域：

```
Verifier.walk(ir, ctx) {
  enter_component("App")    → ctx.current_component = Some("App")
    on_identifier_expr        → 检查符号表["App"].signals["count"]？
    on_mutation_stmt           → 检查突变目标是否在 App 的 signals 中
  exit_component()

  enter_store("TodoStore") → ctx.current_store = Some("TodoStore")
    on_store_field_access     → 检查字段是否在 TodoStore 的字段中
  exit_store()

  enter_for("item")        → ctx.for_item_name = Some("item")
    on_identifier_expr        → "item" 解析为循环变量，其他解析为外部作用域
  exit_for()
}
```

需要作用域感知的关键合约：E0011、E0012、E0013、E0014、E0017、E0023。

### 文件

```
omni-compiler/src/ir/
  contract.rs          ← 新增：Contract trait + Violation + 注册表 + 验证器 + ContractContext
  walk.rs              ← 新增：ExprWalker, StmtWalker, ViewWalker（带有作用域栈）
  mod.rs               ← 导出 + NotYetImplementedIR 变体 + 降低期间 E0020 诊断

omni-compiler/src/lib.rs  ← 将 verify() 接入 omni_analyse() + compile_omniview()
```

约 1,200 行新代码。

---

## 第 2 节：类型验证层（语义第 5 阶段）

### 概念

在语义分析中添加类型检查阶段。根据类型声明验证表达式：参数数量、类型兼容性、字段存在性、运算符有效性。

### 类型表示

```rust
pub enum TypeValue {
    I32, I64, F64, String, Bool, Void, Null,
    Array(Box<TypeValue>),
    Map(Box<TypeValue>, Box<TypeValue>),
    Optional(Box<TypeValue>),
    Function(Vec<TypeValue>, Box<TypeValue>),
    Named(String),   // 已解析的用户定义类型
    Unknown,         // 推断失败 —— 对一切兼容
}
```

### 兼容性规则

- 精确匹配：`i32` → `i32`
- 数值扩展：`i32` → `f64`、`i32` → `i64`、`i64` → `f64`
- Null 对 Optional：`null` → `T?`
- Array/Map/Function 结构等价
- `Unknown` 与一切兼容（无假阳性）
- 其他一切：不兼容 → 错误

### 类型环境

```rust
pub struct TypeEnv {
    scopes: Vec<HashMap<String, TypeValue>>,  // 带有作用域栈的变量
    structs: HashMap<String, StructInfo>,      // 结构体定义 → 字段类型
    functions: HashMap<String, TypeValue>,     // fn 名称 → Function 类型
    current_return: Option<TypeValue>,         // 用于 Return 检查
}
```

### StructInfo（来自代理 2 的补充）

```rust
pub struct StructInfo {
    pub name: String,
    pub fields: Vec<(String, TypeValue)>,
}
```

构建于 `TypeEnv::build_from()` 期间，通过遍历所有模块的 `struct` 定义并解析每个字段的类型注解。通过 `env.structs.get("Point")` 查找以进行字段解析。

### 现有 Lint 规则与 T 诊断的关系

现有的 `TypeMismatch` lint（`lints.rs` 第 175-312 行）仅检查字面量初始化器类型。T 诊断**取代**这种狭隘的检查，并扩展覆盖率。迁移路径：
1. 将新的类型检查器以警告形式与现有的 `TypeMismatch` lint 同时运行
2. 验证无误后，从 `lints.rs` 中移除 `TypeMismatch` lint 规则（其所有情况现在都被 T0001-T0012 覆盖）
3. `MissingReturn` 和 `DeadCode` lint 规则保留 —— 它们不会被 T 诊断取代

### 诊断代码（T0001-T0012）

| 代码 | 检查 | 示例错误 |
|------|------|---------|
| T0001 | 参数数量不匹配 | "foo 期望 2 个参数，得到 3 个" |
| T0002 | 类型不匹配 | "期望 i32，得到 String" |
| T0003 | 字段未找到 | "struct 'Point' 没有字段 'z'" |
| T0004 | 无效的运算符 | "不能相加 i32 和 String" |
| T0005 | 索引类型错误 | "不能对 i32 进行索引" |
| T0006 | 返回类型不匹配 | "期望返回 i32，得到 String" |
| T0007 | 赋值不匹配 | "不能将 String 赋值给类型为 i32 的 'x'" |
| T0008 | If 分支不匹配 | "分支类型不匹配：i32 vs String" |
| T0009 | 数组元素不匹配 | "索引 2 处数组元素类型不匹配" |
| T0010 | 未知类型名称 | "未解析的类型名称 'Foo'" |
| T0011 | 方法未找到 | "'Array<i32>' 没有方法 'pushx'。你是说 'push' 吗？" |
| T0012 | 匹配非穷举 | "match on 'Result' 缺少变体 'Error'" |

T0011 需要 `TypeEnv` 追踪内置方法签名（例如 Array 上的 `.push(T)`、`.pop()`、`.len()`）。对于用户定义的结构体方法，目前在范围外 —— 仅检查内置方法。

T0012 仅适用于具有已知变体的标记联合类型。对于 `match x { ... }`，其中 `x` 的类型为 `Unknown`，检查静默通过。

### 范围外（明确）

- 完整的类型推断（参数需要注解）
- 通用类型参数
- Trait/接口一致性
- 穷举模式匹配
- Null 安全跟踪
- Store getter 返回类型

### 文件

```
omni-compiler/src/semantic/
  types.rs            ← 新增：TypeValue、StructInfo、TypeEnv、infer_type()、compatible()、
                       check_program()、内置方法签名
  mod.rs              ← 添加第 5 阶段：type_checker::check_program()
                       + 第 5a 阶段：E0020 降低前 NaN 检查
```

约 800 行新代码。

---

## 第 3 节：跨文件 + 崩溃安全代码生成

### 3a. 将跨文件导入解析移至分析阶段

将 `validate_cross_file`（`lib.rs` 的第 336-452 行）的导入解析部分移至分析管道。合约 E0027-E0029 覆盖：

- **E0027 `import-resolves`**：每个 `import { X } from "./path"` 解析为目标文件中的已导出符号
- **E0028 `prop-names-match`**：传递给子组件的 props 与声明的 props 匹配
- **E0029 `logic-view-boundary`**：逻辑文件（无 `import omniview`）不能导入视图文件

### 3b. 代码生成崩溃安全

代码生成中获得清晰的错误，而非静默的破碎 JS：

1. **`expr_to_js()` 中的 `unreachable!()` 错误变体** —— 意外的 ExprIR 变体 → 清晰的错误消息，而非静默的破碎 JS
2. **NID 分配确定性断言** —— 在测试构建中 `debug_assert!` NID 确定性
3. **信号名称完备性** —— 在发出 `.value` 之前 `debug_assert!(self.signal_names.contains(name))`

约 80 行新代码。

---

## 影响摘要

| 组件 | 新文件 | 行数 | 新诊断 |
|------|---------|------|--------------|
| 第 1 节：合约 | `ir/contract.rs`, `ir/walk.rs` | ~1,200 | E0007-E0032（26 份合约） |
| 第 2 节：类型 | `semantic/types.rs` | ~800 | T0001-T0012（12 条诊断） |
| 第 3 节：跨文件 + 崩溃安全 | `lib.rs`, `omniview.rs` 中的更改 | ~120 | 3 份跨文件合约 + E0020 降低诊断 + E0026 代码生成断言 |
| **总计** | **3 个新文件，约 6 处修改** | **~2,100** | **38 项新诊断**

### 迁移策略

1. 所有新检查最初作为**警告**添加（不阻塞代码生成）
2. 对照 453 个现有测试运行。验证无误后，提升为**错误**。
3. 现有的 `omnic run` 输出保持不变 —— 合约在代码生成*之前*运行，而不是之后。

### 不覆盖的内容

| 项目 | 原因 |
|------|------|
| 开发模式 `__omni_assert()` 运行时 | 对调试有价值，但与 `analyse`→`run` 的保证无关。独立的未来特性。 |
| 代码生成输出的正则验证 | 被合约 + 崩溃安全性取代 —— 后者防止问题，而非事后检测。 |
| 浏览器环境 polyfill | 编译器不应为旧浏览器进行 polyfill。框架问题。 |
| 运行时性能检查 | 不在范围内。关注的是正确性，而非性能。 |
| 视觉回归测试 | 合约确保结构正确性，而非像素级视觉输出。 |
| 完整的类型推断 | 参数需要类型注解；推断是局部的、尽力而为的。 |
| 通用/特质系统 | 范围过大；可以在未来的设计迭代中分层添加。 |
| `DeadCode` / `MissingReturn` 的扩展 | 现有的 lint 规则仍然足够；不是运行时崩溃的来源。 |
| Map 字面量键降低格式 | 非字符串键使用 Debug trait —— 一个边缘情况，不会使 JS 崩溃。 |
| Stagger/Interrupt IR 值不匹配 | 仅限 motion；被 motion 合约（E0023-E0024）部分覆盖。 |
| 信号方法调用降低为标识符 | 使用位置上的 `signal.set()` 返回标识符，而非突变 —— 一个边缘情况。 |

### 审查结果：23 → 26 份合约，10 → 12 条类型诊断

对抗性审查发现了几个需要在最终确定前修复的问题：

- **E0020** 被移至降低阶段（不是 IR 合约）—— NaN→0 转换在合约验证之前发生
- **E0026** 被移至代码生成断言（不是 IR 合约）—— NID 在 IR 中不存在
- **添加了 E0030**：多根视图子节点 —— 只有第一个能被降低，其他的会静默丢失
- **添加了 E0031**：Store 计算属性引用有效性 —— 计算属性引用不存在的信号
- **添加了 E0032**：双向绑定安全性 —— `bind:value` + 同一信号上的 `@onChange` 可能导致循环
- **添加了 T0011**：方法未找到 —— `arr.pushx()` 被捕获而非静默失败
- **添加了 T0012**：匹配非穷举 —— 标记联合类型的缺失变体
- **步行者作用域跟踪**：明确定义了用于上下文感知合约的 `ContractContext`
- **现有 lint 重叠**：澄清了 T 诊断取代 `TypeMismatch` lint；`MissingReturn` 和 `DeadCode` 保留
- **行数估算修正**：从约 1,650 行调整为约 2,100 行（更准确的合约大小 + 作用域基础设施）
