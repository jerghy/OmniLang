# P30b: WASM 解析器替换 TS 解析器可行性评估

> 2026-07-27 | 基于 P30a WASM 解析器导出原型

## 1. 当前状态

### TS 解析器（编辑器往返翻译层）

| 维度 | 数据 |
|------|------|
| 代码量 | ~8,070 行（15+ 文件） |
| TokenKind | 17 种 |
| 入口 | `parseOmniSource(source: string): ParseResult` |
| 返回值 | `ParseResult`（designElements, designWires, codeTabs, signals, props, events, computeds, storeDef...） |
| 调用方 | 5 处（App.tsx ×2, DesignTabBar.tsx ×2, DesignTabContent.tsx ×1） |
| 核心文件 | parser.ts (1,454 行), parser-view.ts (790 行), source-emitter.ts (743 行) |
| 测试覆盖 | ~10%（P29 评估值） |

### WASM 解析器（P30a 新增）

| 维度 | 数据 |
|------|------|
| 入口 | `parseSource(source: string): JsValue` |
| 返回值 | `{ success: bool, program: Program, errors: ParseError[] }` |
| Program 类型 | Rust AST：modules → components/stores/functions/tagged_unions |
| 二进制大小 | ~1.2MB（omnilang-wasm） |
| 测试 | 5 个单元测试 |

## 2. 核心发现：数据模型不兼容

TS 解析器和 WASM 解析器返回**完全不同**的数据结构：

```
TS:  source → tokenize → Parser.parse → ParseResult
                                           ├── designElements: ElementData[]   ← 编辑器 UI 元素
                                           ├── designWires: WireData[]         ← 连线数据
                                           ├── codeTabs: CodeTab[]             ← 代码面板
                                           ├── signals/props/events/computeds  ← 组件元数据
                                           ├── storeDef/inlineStoreDefs        ← Store 定义
                                           ├── codeFlowGraphs                  ← 代码流程图
                                           └── pidMap                          ← PID 注解

WASM: source → tokenize → Parser::parse → Program
                                            ├── modules: FxHashMap<String, ModuleIR>
                                            │     ├── components: FxHashMap<String, ComponentIR>
                                            │     ├── stores: FxHashMap<String, StoreIR>
                                            │     ├── functions: FxHashMap<String, FunctionIR>
                                            │     └── ...
                                            └── source_map: FxHashMap<String, SourceMapEntry>
```

**TS 解析器不仅仅是解析器**——它同时做了：
1. 语法解析（与 Rust 解析器功能重叠）
2. **编辑器数据模型构建**（designElements, wires, codeFlowGraphs）
3. **PID 注解解析**（`// @pid:xxx` 注释 → PidMap）
4. **代码流图编译**（fn body → CodeblockElementData + WireData）

WASM 解析器只做 #1，且输出的 `Program` AST 与编辑器的 `ParseResult` 是完全不同的类型系统。

## 3. 替换路径分析

### 路径 A：直接替换（不可行）

将 `parseOmniSource` 改为调用 WASM `parseSource`：
- ❌ `ParseResult` 类型不兼容——WASM 返回 `Program`，编辑器消费 `ParseResult`
- ❌ 需要 adapter 层将 `Program` → `ParseResult`，这不是简单的类型映射
- ❌ PID 注解解析、代码流图编译仍需 TS 侧处理

### 路径 B：WASM 解析 + TS Adapter（高成本）

用 WASM 解析器替换 TS 解析器的**语法解析部分**，保留 TS 侧的数据模型构建：
- 替换：`tokenize + Parser.parse` → WASM `parseSource`
- 保留：编辑器数据模型构建（design elements, wires, code flow graphs）
- ⚠️ 需要实现 `Program AST → ParseResult` 的完整 adapter（估计 1,500-2,500 行）
- ⚠️ adapter 必须覆盖 58 种 TokenKind 的所有 AST 节点 → 编辑器模型映射
- ⚠️ 每次语言特性变更需要同步更新 adapter

**工作量估计**：3-4 周（adapter 实现 + 测试 + 迁移 5 个调用方）

### 路径 C：保留双解析器 + Golden CI（当前策略 ✅）

- TS 解析器继续维护（~8,000 行）
- WASM 解析器仅用于 Golden 文件一致性验证（CI 自动检测漂移）
- 51 个 Golden 文件 + TS round-trip 测试（P17）提供安全网
- 每次语言变更后运行双解析器对比测试

**工作量**：0（已实现）

## 4. 成本收益分析

| 维度 | 路径 B（替换） | 路径 C（保留双解析器） |
|------|--------------|---------------------|
| 初期工作量 | 3-4 周 | 0 |
| 每次语言变更成本 | adapter 同步（可能 1-3 天） | TS 解析器手动同步（当前成本） |
| 风险 | adapter bug 导致编辑器损坏 | 语法漂移（已有 Golden CI 缓解） |
| 收益 | 统一解析器，消除语义漂移根源 | 零风险，零新增代码 |
| WASM 下载成本 | 每次启动 ~1.2MB（可能影响冷启动） | 无 |
| TS 解析器维护 | 逐步废弃 | 继续维护 |

## 5. 建议：保留双解析器（路径 C）

**推荐决策**：保留双解析器架构，WASM 解析器用于 CI 验证而非编辑器运行时替换。

**理由**：

1. **数据模型鸿沟过大**：TS 解析器输出的是编辑器专用的 `ParseResult`（含 designElements、wires、codeFlowGraphs），不是通用的 AST。WASM 解析器输出的是编译器 IR 前置的 `Program`。两者之间的 adapter 不是简单的类型转换，而是语义重建——这本身就是容易引入 bug 的复杂逻辑。

2. **TS 解析器的编辑器特定功能无法被 WASM 解析器替代**：PID 注解解析、代码流图编译、设计元素提取这些是编辑器独有的需求，Rust 编译器根本不需要这些概念。替换解析器后这些逻辑仍需保留在 TS 侧。

3. **Golden CI 已经缓解了核心风险**：51 个 Golden 文件 + P5b TS 解析测试 + P17 round-trip 测试构成了有效的语法漂移检测网。如果 TS 解析器产生 Rust 解析器无法解析的输出，CI 会捕获。

4. **YAGNI**：当前双解析器架构是 7/11 专家识别的 #1 风险，但缓解措施（P5+P17）已经就位。在没有实际语法漂移事故的情况下，投入 3-4 周做替换的边际收益不足以证明成本。

5. **P30 延期条件已满足**：实现计划明确规定——"若 P5 或 P17 已延迟，P30 延期到 Phase 2"。虽然 P5+P17 未延迟，但评估结论是「保留双解析器」本身是有效的决策结果。

## 6. 后续行动

1. ✅ **P30a 已完成**：WASM `parse_source` 导出可用
2. ✅ **本评估完成**：决策记录归档
3. 📋 **Phase 2 重新评估触发条件**：
   - 发生至少 1 次语法漂移事故（TS 解析器输出导致编译器报错）
   - 或语言规范稳定（6 个月无新增语法特性）
   - 或编辑器需要离线解析能力（无 Tauri 后端时使用 WASM 解析器）

---

> 本评估基于 P30a 原型实现和 TS 解析器源码分析。数据采集于 2026-07-27。
