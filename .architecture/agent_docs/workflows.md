# Workflow Procedures — 分步操作流程

> AI 助理执行具体操作时的分步指南。每个流程包含前置条件、执行步骤、验证方法和常见陷阱。

## 架构审查工作流

### 全工作区审查（三层方法论）

本项目采用 ADR-007 定义的三层审查方法论：

```
Layer 1（广度）→ 11 专家并行独立评审
Layer 2（深度）→ 4-6 专家代码级验证 + 纠正 Layer 1 错误
Layer 3（执行）→ 基于 Layer 2 验证结果制定实施计划
```

**前置条件**：
- `.architecture/members.yml` 已配置
- `.architecture/config.yml` 务实模式设置已确认
- 有明确的审查范围（全工作区 / 版本号 / 功能名 / 组件名）

**执行步骤**：

1. **确定范围**——从用户请求提取审查目标。如未明确，提一个澄清问题。
2. **加载配置**——读取 `config.yml` 和 `members.yml`，确定活跃成员集合。
3. **预分析（可选）**——对宽范围审查，用 `git log` 和 `Glob` 组装约 500 token 的共享简报。
4. **Layer 1 派发**——所有活跃成员在同一响应中并行派发 `Agent()`。使用技能提示模板。
5. **Layer 1 聚合**——交叉分析（≥3 专家识别的主题）、冲突表面化、优先级分桶。保存到 `.architecture/reviews/<filename>.md`。
6. **Layer 2 派发**——4-6 位技术专家，聚焦代码级验证。每个专家接受精确的文件:行指令。
7. **Layer 2 聚合**——纠正 Layer 1 的事实错误。产出具体的修复方案和代码量估算。保存到 `.architecture/reviews/deep-review-<date>.md`。
8. **Layer 3 制定**——使用 `superpowers:writing-plans` 技能编写实施计划。每个任务含确切文件路径、代码变更、测试步骤。保存到 `docs/superpowers/plans/<filename>.md`。

**常见陷阱**：
- Layer 1 的专家可能产生约 10-20% 的事实错误——Layer 2 必须纠正
- 子代理倾向于根据 grep 结果做表层判断——必须要求阅读实际代码
- 子代理驱动模式下约 33% 的 agent 会产生重大误判——主会话需交叉验证

### 专项审查

对单个专家视角的深度审查，使用 `/ai-software-architect:specialist-review`。

### 审查后行动

1. 为关键发现创建 ADR：`/ai-software-architect:create-adr`
2. 为重校准创建 ADR 跟踪：`/ai-software-architect:architecture-recalibration`
3. 状态检查：`/ai-software-architect:architecture-status`

---

## ADR 创建工作流

**前置条件**：
- 有明确的架构决策需要记录
- 了解备选方案和权衡

**执行步骤**：

1. **收集上下文**——了解决策内容、解决的问题、备选方案、权衡。
2. **确定编号**——`ls .architecture/decisions/adrs/ | grep ADR- | tail -1`，新编号 = 最高 + 1。
3. **清理输入**——文件名转 kebab-case：空格→连字符，移除特殊字符，限制 80 字符。
4. **检查务实模式**——从 `config.yml` 读取。若启用，ADR 中需含 Pragmatic Enforcer 分析。
5. **填写模板**——使用 `templates/adr-template.md`：Status → Context → Decision Drivers → Decision → Consequences → Implementation → Alternatives → Validation。
6. **务实模式分析**——必要性评分（0-10）、复杂度评分（0-10）、比率（目标 <1.5）、建议。
7. **保存**——`.architecture/decisions/adrs/ADR-XXX-kebab-case-title.md`。
8. **报告**——简要告知用户 ADR 编号、标题、关键决策和下一步。

**ADR 创建时机**：
- ✅ 技术选型（框架、数据库、语言）
- ✅ 架构模式（微服务、事件驱动）
- ✅ 基础设施决策（云提供商、部署方式）
- ✅ 安全方案（认证、加密）
- ❌ 实现细节（函数名、变量名）
- ❌ 临时决策
- ❌ 影响范围有限的次要决策

---

## 设置流程

**仅在新项目首次使用时执行。**

1. 运行 `/ai-software-architect:setup-architecture`
2. 验证目录结构：`ls .architecture/`
3. 审查并更新 `principles.md` 以匹配项目上下文
4. 根据需要自定义 `members.yml`（本项目已添加 rust_expert、typescript_react_specialist、compiler_architect 三位语言专家）
5. 审查 `config.yml` 中的务实模式设置
6. 创建 `agent_docs/` 文件（本目录）

---

## 重校准工作流

将架构审查发现转化为可执行实施计划。

**前置条件**：
- 已有完成的架构审查（Layer 1 + Layer 2）
- 发现已按严重性分类

**执行步骤**：

1. **收集发现**——从审查文档提取所有 CRITICAL 和 HIGH 发现。
2. **技术验证**——对每个发现进行代码级验证（如本项目的 Layer 2）。
3. **制定实施计划**——使用 `superpowers:writing-plans` 技能。按 Phase 组织，每任务精确到文件:行。
4. **创建跟踪 ADR**——记录推迟或分阶段的决策。
5. **更新 deferrals.md**——若有项目推迟到后续 Phase。
6. **制定重校准计划**——使用 `templates/recalibration_plan.md` 模板，保存到 `recalibration/`。

---

## 本项目特有流程

### OmniLang 编译器测试

```bash
cd D:\space\OmniLang
cargo test -p omnilang --lib          # 核心测试（预期 656/0）
cargo test -p omnilang --test cross_target  # 跨目标测试
cargo test -p omnilang --test golden_parse  # Golden 文件解析
cargo clippy --lib -p omnilang -- -D warnings  # Lint 检查
```

### Poiesis 前端测试

```bash
cd D:\space\OmniLang\poiesis
npx vitest run                         # 单元测试（预期 20 文件/144 测试通过）
npx tsc --noEmit                       # 类型检查
```

### 构建验证

```bash
cargo build --workspace                # 全工作区 Rust 构建
cd poiesis && npm run build            # 前端构建
```
