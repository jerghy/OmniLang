# ADR-002: 编辑器范式演进

## Status

Accepted

## Context

Poiesis 是 OmniLang 语言的可视化编程编辑器。编辑器经历了从积木式编程到文本编辑的范式转换，每一次转换都是对前一代根本性限制的响应。

**时间线**：
- **v3 CodeCard**（~2026-07-15）：扁平积木架构——Lexical 风格的词法卡片。`poiesis-codecard.md` 记录了 4 次光标方案失败后废弃。
- **v4 Block Editor**（~2026-07-20）：从 Blockly 开始 → 替换为自定义 SVG 积木渲染。Blockly 的输入速度 vs 导航权衡被证明无法接受。
- **v4.1**（~2026-07-23）：自定义 SVG 积木、C 形侧臂布局、DOM 驱动选择——视觉改进但维持积木范式。
- **v5 CodeBlock Editor**（~2026-07-25）：Monaco 文本编辑 + WASM 语义依赖分析。彻底放弃积木范式。
- **未来（方案 3）**：结构投影编辑器——Monaco + AST 感知（暂不实施，见 `poiesis-structural-projection-editor.md`）。

## Decision Drivers

1. **光标模型**：v3 的词法卡片无法提供可靠的多节点光标导航——4 种方案全部失败
2. **布局稳定性**：v4 积木编辑器中 Scratch 风格的叠加布局在 If-arm 渲染中产生不稳定
3. **输入速度**：积木编辑器中的 drag-and-drop 比文本输入慢；Blockly 的导航 vs 输入速度权衡不令人满意
4. **视觉整洁度**：连接线、块重叠和复杂的 SVG 使 v4.1 画布视觉杂乱
5. **编译器集成**：Monaco 支持语法高亮、括号匹配、IME 和诊断——这些在积木编辑器中都需要手动实现

## Decision

**采用 v5 CodeBlock Editor：Monaco Editor 文本编辑 + WASM 语义依赖分析。**

- **文本编辑**：`CodeBlockEditor.tsx` 和 `CodeEditorModal.tsx` 中 2 个文件使用 Monaco（非 11+ 个积木文件）
- **WASM 集成**：`omnilang-wasm` 通过 `useWasmCompile.ts`（260 行）在浏览器内编译 `.omni` 源码
- **语义分析**：`omnilang::analyse_dependencies()` 提取符号 + 依赖——在 WASM 可用时实时运行
- **退化模式**：`AnalysisStatus` 类型有三个状态——`loading`、`ready`、`unavailable`。WASM 无法初始化时编辑器正常降级
- **可视化辅助**：CodeFlow Canvas（XYFlow + React Flow）渲染代码连接图——非积木式编辑，仅代码流可视化

**被废弃**：
- 37 个 v3/v4 积木编辑器文件在 v5 清理提交中删除
- Blockly 知识已废弃
- 自定义 SVG 积木组件（`CodeBlockNode.tsx`、`ConditionNode.tsx`）作为遗留代码保留用于代码流可视化，不作为编辑界面

## Consequences

### Positive

- **输入速度**：文本编辑 = 所有开发者的原生速度
- **语法高亮**：Monaco 原生支持——无需自己实现
- **IME 支持**：对非英文用户至关重要的 Monaco 内置功能
- **撤销历史**：Monaco 内置；积木编辑器需要自定义实现
- **可维护性**：2 个文件（Monaco 集成）替代 37 个积木文件
- **WASM 退化**：优雅降级而非崩溃

### Negative

- **失去了可视化编辑的独特性**：Poiesis 现在主要是一个带编译预览的文本编辑器
- **Monaco 包大小**：~16MB 压缩 JS（通过代码分割缓解——实现计划 P7）
- **WASM 二进制大小**：1.2MB，启动时加载（增量缓存计划 P8 降低延迟）
- **结构投影推迟**：Monaco + AST 感知的长期愿景（方案 3）是"暂不实施"

### Neutral

- **CodeFlow Canvas 保留**：可视化代码流图仍在——但现在只读/导航，非编辑
- **TypeScript 解析器保留**：用于 PID 注释 round-trip 和样式解析——可能被 WASM 替换（见 P30 评估）

## Implementation Strategy

### Blast Radius

v5 迁移已完成。积木编辑器文件已删除。**该决策已实施**——逆转将需要完全重建积木 UI（中等爆炸半径，高工作量）。

### Reversibility

**可逆性级别**：Low（已删除 37 个积木编辑器文件；恢复需要重建）

**回滚可行性**：Monaco 集成是轻微的——2 个文件。文本编辑是标准模式。如果可视化编辑需要以不同形式回归，可以在保留 Monaco 的基础上建设（结构投影方案 3 方向），而非取而代之。

### Social Cost

**学习曲线**：Low——Monaco 是标准编辑器组件。文本编辑是通用的。
**清晰度评估**：文本编辑+编译预览的模式是广泛理解的（VS Code + Live Preview 模式）。不会比积木编辑更令人困惑。

## Alternatives Considered

### CodeCard v3（词法卡片）

**被拒绝**：4 种光标方案全部失败。词法卡片无法处理多节点选择和跨块光标移动。

### Blockly / Scratch 风格积木（v4）

**被拒绝**：drag-and-drop 输入速度比文本慢。布局不稳定（If-arm 渲染）。Blockly 的导航 vs 输入权衡无法接受。

### 自定义 SVG 积木（v4.1）

**被拒绝**：视觉上比 Blockly 好但维持了积木范式的根本性限制。C 形侧臂和重叠检查增加了视觉复杂度而没有解决输入效率。

### 结构投影编辑器（方案 3）

**延迟**：Monaco + AST 感知的长期愿景。技术上可行但实施成本高。当前优先修复编译器基础设施（见实现计划 Sprint 1-4）。

## Pragmatic Enforcer Analysis

**Mode**: Balanced

**Necessity Assessment**: 9/10
- 积木编辑器（v3/v4）有根本性限制——光标模型、输入速度、布局稳定性
- 等待成本：继续修补无法工作的编辑器会延迟所有其他开发
- v5 选择是经过 3 次范式转换后对可靠性的追求

**Complexity Assessment**: 3/10
- Monaco Editor 是现成组件——集成代码 2 个文件
- 积木编辑器更复杂——37 个文件、自定义 SVG 渲染、连接线路由、光标模型

**Pragmatic Score**:
- 必要性: 9/10
- 复杂度: 3/10
- 比率: 0.33 ✅ **Ratio < 1.5（出色）**

**Recommendation**: ✅ 批准决策——v5 是显著简化。Monaco 是更简单的方案，而非更复杂的。

## Validation

**Acceptance Criteria**:
- [x] Monaco Editor 在 CodeBlockEditor 和 CodeEditorModal 中可用
- [x] WASM 语义分析在加载时返回符号和依赖
- [x] 编辑器在 WASM 不可用时优雅降级
- [x] 37 个积木编辑器文件在清理提交中删除
- [ ] Monaco 代码分割已实施（实现计划 P7）
- [ ] WASM 增量缓存已接入（实现计划 P8）

## References

- [[poiesis-codecard]] — v3 失败教训
- [[poiesis-codeblock-editor-v5]] — v5 设计决策
- [[poiesis-structural-projection-editor]] — 长期愿景
- [architecture-review-2026-07-26](../reviews/full-workspace-review-2026-07-26.md) — 全工作区评审
- [[ADR-001]] — 编译器管线架构
- [[ADR-003]] — 代码生成目标优先级
