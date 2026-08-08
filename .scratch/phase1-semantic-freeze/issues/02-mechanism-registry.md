# 02-mechanism-registry: 机制级登记 + C2 决策（检查点 1B）

Status: ready-for-agent

## 背景

@store / @mount / @effect / @unmount 已实现（**机制级**）但从未在语义全集登记（CONTEXT.md 对照表：设计域 5 Stores → 无对应）。另：08-08 评审 **C2 待决策**——legacy 硬错误是否覆盖组件实例（`<Child @onClick={f}/>` 目前绕过）。

## 任务

- [ ] @store / @mount / @effect / @unmount 登记进语义全集
- [ ] C2 决策记录（评审推荐：**是**——覆盖组件实例；决策后 codegen 侧保持一致）

## 验收

- 语义全集 10 域全登记
- C2 决策落地（含 codegen 侧一致 + 测试）

## 关联

- ROADMAP.md 1B
- `docs/superpowers/specs/semantic-universe.md`
- 08-08 评审 Compiler Architect C2（`ir/lower/mod.rs:345` 相关）

## Comments
