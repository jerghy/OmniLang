# agent_docs — AI Assistant 操作指南

> 补充 `AGENTS.md` 的详细操作流程。AGENTS.md 提供全局指引，本目录提供分步执行细节。

## 文件导航

| 文件 | 用途 | 何时阅读 |
|------|------|---------|
| [workflows.md](workflows.md) | 分步操作流程 | 需要执行审查、创建 ADR、进行重校准时 |
| [reference.md](reference.md) | 高级主题和故障排除 | 遇到务实模式配置、子代理误判、CI 集成问题 |

## 项目速览

**名称**：OmniLang 工作区
**仓库**：`D:\space\OmniLang`
**技术栈**：Rust（omnilang 编译器 63K 行）+ TypeScript/React（Poiesis 编辑器 42K 行）+ Tauri 2 + WASM
**架构框架版本**：1.6.0
**ADR 数量**：10（ADR-001 → ADR-010）
**架构团队成员**：11（含 Rust/TS/Compiler 三位语言专家）

## 快速入口

### 我想启动架构审查
→ [workflows.md § 架构审查工作流](workflows.md#架构审查工作流)
→ 简版：`/ai-software-architect:architecture-review <target>`

### 我想创建 ADR
→ [workflows.md § ADR 创建工作流](workflows.md#adr-创建工作流)
→ 简版：`/ai-software-architect:create-adr`

### 我想查看当前架构状态
→ `/ai-software-architect:architecture-status`

### 我想了解务实模式
→ [reference.md § 务实守卫模式](reference.md#务实守卫模式)
→ 配置文件：`.architecture/config.yml`

### 子代理评审结果不可靠怎么办
→ [reference.md § 子代理可靠性](reference.md#子代理可靠性)
→ 本项目实测数据：实现代理 67% 失败率，审查代理 33% 重大误判率

## 项目约定

- **提交格式**：`<type>: <description>`（conventional commits）
- **分支策略**：main 分支 + feature 分支
- **语言**：中文注释和文档优先
- **测试要求**：Rust `cargo test --lib` 通过，TS `npx vitest run` 通过
- **FS 权限**：Tauri FS 全覆盖是设计意图（ADR-004 决策 2），不可缩小

## 相关文档

- [AGENTS.md](../templates/AGENTS.md) — 跨平台 AI 助理指令
- [principles.md](../principles.md) — 核心架构原则
- [members.yml](../members.yml) — 架构审查团队成员
- [config.yml](../config.yml) — 框架配置（含务实模式）
- [deferrals.md](../deferrals.md) — 推迟项追踪
