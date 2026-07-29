# ADR-005: 澄清：TypeScript 管线不是编译器，是编辑器往返翻译层

## Status

Accepted (2026-07-27)

> This ADR is cited as authoritative by ADR-007, agent_docs/reference.md, and multiple workspace reviews.

## Context

在 2026-07-26 的全工作区架构评审中，多位专家将 Poiesis 编辑器的 TypeScript 管线（poiesis/src/services/codegen/，~10,778 行）描述为"双编译器"或"第二个 Omni 编译器"。这一表述在技术上是错误的——TS 管线不产生任何目标语言的编译输出，它只是编辑器的解析器+源码发射器，负责 .omni 源文本 ↔ 编辑器数据模型的往返转换。

真正的 OmniLang 编译器只有 Rust 端的 omnilang crate（~62,000 行），包含完整的 Lexer→Parser→Semantic→IR→Codegen 管线，原始设计支持 9 个目标后端（经 [[ADR-003]] 缩减为 3 个 P0 活跃目标）。

正确理解两者的关系：

1. Rust 编译器（omnilang/）：.omni 源文本 → Lexer → Parser → Semantic → IR → Codegen → JS/HTML/CSS/React/Flutter/...
2. TS 管线（poiesis/src/services/codegen/）：编辑器数据模型 ↔ .omni 源文本（双向往返）
   - 读取方向：.omni 源文本 → Tokenizer(17 TokenKind) → Parser → ViewNode 树 → 编辑器可视化积木
   - 写入方向：编辑器可视化积木 → SourceEmitter → .omni 源文本

两者的关键差异：
- Rust 解析器：58 种 TokenKind，完整的语言规范解析
- TS 解析器：17 种 TokenKind，仅覆盖编辑器需要的语法子集
- TS 管线不产生任何编译产物，它只是编辑器读写 .omni 文件的翻译层

虽然"双编译器"表述不准确，但核心风险是真实的：两个独立的解析器实现（TS 和 Rust）需要手动保持同步，存在语义漂移风险。这是 7/11 专家共同识别的问题，但应该准确描述为"双解析器"而非"双编译器"。

## Decision

在架构文档和评审中统一使用以下术语：

1. "OmniLang 编译器" = 仅指 Rust 端的 omnilang crate，包含完整的编译管线
2. "编辑器管线"或"TS 解析器/发射器" = 指 Poiesis 的 TypeScript 代码生成管线
3. 禁止使用"双编译器"、"TS 编译器"等误导性表述
4. "双解析器问题" = 准确描述两个独立解析器实现需要同步维护的风险

这一术语规范适用于：架构评审报告、ADR、设计文档、代码注释、以及 AI 辅助架构分析。

## Consequences

正面影响：
- 后续架构评审不会再将 TS 管线误判为"第二个编译器"
- "双解析器"的表述更精确地指向真正的问题：两套解析器的手动同步
- 新人或 AI 分析时不会产生"为什么需要两个编译器"的困惑
- 明确了代码生成（Rust 编译器负责）和源码发射（TS 管线负责）的职责边界

需要跟进：
- 更新 full-workspace-review-2026-07-26.md 中的相关措辞，将"双编译器"改为"双解析器"
- 在 poiesis/src/services/codegen/README.md 中明确标注"这不是编译器，是编辑器往返翻译层"
- 在 omnilang/README.md 中明确标注"OmniLang 的唯一官方编译器"
- 确保后续 AI 辅助分析（包括架构评审子代理）能读取到这篇 ADR

## Decision Drivers

- 7/11 architecture review specialists independently identified the dual-implementation risk
- Mislabeling as "dual compiler" inflated perceived complexity and confused review findings
- Precise terminology is required for AI-assisted architecture analysis to produce accurate results

## Validation

- [x] ADR-007 (Three-Layer Review Methodology) references this decision as settled
- [x] agent_docs/reference.md "Decided Decisions" list includes this terminology
- [x] full-workspace-review-2026-07-26.md has updated "双编译器" → "双解析器"
- [x] No subsequent architecture review has mischaracterized the TS pipeline as a compiler

## References

- [[ADR-004-architecture-review-2026-07-26]] — originating review
- [[ADR-007-three-layer-review-methodology]] — cites this as settled terminology
- [[subagent-reliability-issues]] — subagent error patterns in terminology confusion

## Date

2026-07-26 (proposed), 2026-07-27 (accepted)
