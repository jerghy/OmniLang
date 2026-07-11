---
name: update-poiesis-dev
description: 开发完 Poiesis 后更新 poiesis-dev 技能知识库。修正错误认知、追加新发现、删除过时内容、改写架构变化。触发词："更新poiesis-dev", "update poiesis dev", "记录到poiesis-dev", "沉淀poiesis经验"。
---

# Update Poiesis Dev

每次完成 Poiesis 开发任务后，将知识变更同步到 `poiesis-dev` 技能中。

## Flowchart

```
开发任务完成
│
├─ [1] 筛选：哪些值得沉淀？
│
├─ [2] 读目标文件（必须先读！）
│
├─ [3] 确定变更类型 + 目标位置
│
├─ [4] 执行变更
│
└─ [5] 自检 + 确认
```

## 核心约束（贯穿全流程）

### C1. 先读后写 —— 铁律

**在 Edit 任何 reference 文件之前，必须先用 Read 打开它。** 不能凭记忆判断"那个文件里大概写了什么"——记忆可能过时、可能记错位置。

### C2. 合并优于堆叠

如果新知识**和已有某个小节相关**，合并到那个小节里——不要在文件末尾追加一个重复主题的孤立段落。

```
❌ confusions.md 已有 §三 Variant 分支继承 → 又在末尾追加 §十四 Variant size.lg 的坑
✅ 把 size.lg 的例子合并到 §三，作为该陷阱的一个具体实例
```

### C3. 一次一处

一个事实只出现在一个文件中。不要为了"保险"把同一个知识点撒到 `confusions.md` 和 `workflow.md` 和 `design-system.md`。

```
❌ "Variant 分支不级联继承" 同时写在 confusions.md 和 design-system.md
✅ confusions.md 写陷阱（为什么会踩），design-system.md 写机制（正确行为是什么）——角度不同不算重复
```

### C4. 修正必须连带清理

修正一个错误事实时，检查文件中是否有**基于这个错误事实推导出的下游结论**，一并修正。

```
修正: "Variant 从 base 继承所有属性" → "Variant 只覆盖 base 同名属性"
连带检查: 是否有段落说 "所以 size.lg 会自动获得 size.md 的 padding"？→ 也修正
```

### C5. 引用代码，不写散文

每条事实尽量附带文件路径和行号。没有代码锚点的泛泛描述，两个月后无法验证真伪。

```
❌ "Variant 的属性合并是通过 store 中的一个函数完成的"
✅ "Variant 属性合并：design-system.ts L37-52 updateVariant() → VariantEditor.tsx 渲染时 base + branch 叠加"
```

### C6. 记录机制，不记录剧情

记录的是**可复用的机制理解**，不是**这次 debug 的故事**。

```
❌ "今天下午花了 2 小时发现 size.lg 的 padding 不对，因为上个月改 design.omni 时漏写了"
✅ "Variant 分支漏定义属性 → 掉到浏览器默认值，不是从同级分支继承"
```

### C7. 修正前确认

当修正的内容**与现有记录矛盾**且你自己不是 100% 确定时，**先向用户确认再改**。宁可多问一句，不要用新错误覆盖旧错误。

```
"confusions.md §三 写的是 X，但这次开发发现似乎是 Y。确认一下：是之前的记录错了，
还是这次的情况比较特殊？"
```

### C8. 上限原则

一次更新不超过 5 条变更。如果值得沉淀的内容超过 5 条，说明该开一个新对话专门做知识整理，而不是在任务结尾顺手追加。

## Step-by-Step

### 1. 筛选：哪些值得沉淀？

用以下 filter 过一遍本次开发的发现：

| 留下 | 丢掉 |
|------|------|
| 推翻了已有记录的认知 | 和已有记录一致，只是又验证了一次 |
| 反直觉的机制行为（下次还会踩） | 显而易见的代码逻辑（读了代码自然知道） |
| 新的文件/API/数据流（架构变化） | 本次任务特有的一次性配置/操作 |
| **可复用的开发/调试方法、效率技巧** | 拼写错误、漏写属性的简单疏忽 |
| 有代码路径引用的具体事实 | 没有文件/行号的泛泛印象 |

**开发经验对应的目标文件**：
| 经验类型 | 写到 |
|---------|------|
| 新文件/新 API/架构变化 | `file-map.md` → `architecture.md` |
| 反直觉的行为/踩坑/API 陷阱 | `confusions.md` |
| **调试技巧/开发流程/效率工具** | **`workflow.md`** |
| Token/Class/Variant 机制变更 | `design-system.md` |

**模糊地带**（以下情况直接跳过，不沉淀）：
- "感觉好像是..." — 不确定就不写
- "以后可能会有用..." — YAGNI，以后再说
- 可以在代码注释或 CLAUDE.md 中表达的 — 不放在 references 里

### 2. 读目标文件

**在确定变更之前，先 Read 目标文件。** 根据筛选结果确定要读哪些：

- **架构/文件变更** → Read `file-map.md` + `architecture.md`
- **机制陷阱/API 坑** → Read `confusions.md`
- **调试技巧/开发流程** → Read `workflow.md` ⚠️ 最容易遗漏！
- **Design System 机制** → Read `design-system.md`

确认：
- 有没有已经覆盖了这个知识点的段落？（→ 合并，不追加）
- 有没有和本次发现矛盾的旧内容？（→ 标记待修正）
- 文件的实际章节编号和结构是什么？（→ 确定合并/追加的准确位置）

### 3. 确定变更类型 + 精确位置

| 类型 | 触发条件 | 操作 |
|------|---------|------|
| **修正** | 发现旧记录有事实错误 | 定位错误段落 → Edit 替换 → 连带清理下游推导 |
| **合并追加** | 新知识和已有小节主题相关 | 在该小节内追加段落/示例，不新建章节 |
| **独立追加** | 新知识和任何已有章节都不相关 | 文件末尾新建小节 |
| **删除** | 文件/API/机制已不存在 | 删除对应条目，修复断裂的序号 |
| **改写** | 架构实质性变化，整个章节需要刷新 | Read 全章节 → Write 替换 |

### 4. 执行变更

通用规则：
- 改动后**不 Read 验证**——Edit/Write 失败会报错
- 每个 Edit 的 `old_string` 必须精确匹配（含缩进），且唯一
- 删除了编号条目后，重新编后续序号

### 5. 自检 + 确认

变更完成后，过一遍这个 checklist：

- [ ] 每条新增/修改的事实有文件路径或行号引用？
- [ ] 没有把同一个事实写在两个不同的文件里？
- [ ] 没有超过 5 条变更？
- [ ] 修正的内容做了连带清理？
- [ ] 新内容和已有内容不重复？
- [ ] 没有任何"感觉"、"应该"、"好像"的模糊表述？

然后汇报：

> Poiesis-dev 已更新（3/5 条限额）：
> - **合并追加** `references/confusions.md` §三 — size.lg 实例
> - **修正** `references/design-system.md` §三 — 合并逻辑描述，补充代码路径
> - **删除** `references/file-map.md` — 已移除的 `style-resolver.ts`

## 示例

```
本次开发: 修复 Variant size.lg 缺 padding → 渲染比 sm 还小的 bug

筛选:
  留下:
    1. Variant 分支漏定义属性 → 浏览器默认值（反直觉，下次还会踩）
    2. VariantEditor 分支切换时 properties 引用替换的机制（不是重新计算）
  丢掉:
    1. 今天用 eval_js 读了三个按钮的 getBoundingClientRect → 这是常规调试，套路已在 workflow.md
    2. design.omni 中 size.lg 的具体 token 值 → 是数据不是知识

读文件:
  → Read confusions.md → §三 已覆盖"分支属性叠加规则"，但缺少漏定义→浏览器默认值的具体警告
  → Read design-system.md → §三有合并逻辑描述，但没写代码路径
  → Read workflow.md → 已有 eval_js 调试脚本，不需追加

变更:
  → 合并追加 confusions.md §三：加粗警告 + size.lg 作为实例
  → 修正 design-system.md §三：补充合并逻辑在 VariantEditor.tsx 的具体行号

自检:
  ✓ 2 条变更，没超限
  ✓ 每个变更都有代码引用
  ✓ 没有重复
  ✓ size.lg 的 token 值没写进来（是数据不是知识）

确认:
  Poiesis-dev 已更新（2/5 条限额）：
  - 合并追加 references/confusions.md §三 — 漏定义→默认值的警告 + size.lg 实例
  - 修正 references/design-system.md §三 — 合并逻辑代码路径
```

```
本次开发: Monaco CodeEditorModal 调试，eval_js 中 monaco 未定义

筛选:
  留下:
    1. 通过 window.__monacoEditor debug hook 暴露编辑器实例（Victauri 调试模式）
  丢掉:
    1. import * as monaco 是模块作用域——JS 常识
    2. monaco.editor.getModels() 是公开 API——不需记录

读文件:
  → Read workflow.md → §2.2 经典调试序列只覆盖 DOM/CSS，缺少 Monaco 编辑器调试
  → Read confusions.md → §十六 已有 Monaco 集成陷阱，不需追加

变更:
  → 合并追加 workflow.md §2.2：新增"调试 Monaco 编辑器"子节

自检:
  ✓ 有代码路径 (CodeEditorModal.tsx L136)
  ✓ 机制不是剧情
  ✓ 不与其他文件重复

确认:
  Poiesis-dev 已更新（1/5 条限额）：
  - 合并追加 references/workflow.md §2.2 — Monaco debug hook 模式
```
