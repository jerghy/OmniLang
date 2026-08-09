# 06 — 批次 1 域裁决：反馈与模态

Type: grilling
Status: resolved
Blocked by: 04, 05

## Question

**反馈与模态**能力的边界裁决与设计方向——按 05 定稿的裁决模板执行。

覆盖能力项：toast/通知、modal/dialog、tooltip、骨架屏
起点：`docs/superpowers/specs/2026-08-08-semantic-design-coverage-design.md` §4.2 契约草案（showToast 内建语义函数 / `<dialog @modal={openSig}>` 元素指令 + @slot 投影）

## 决议内容

1. 逐项边界裁决（进语言/留生态/不支持 + 三判据证据）
2. 入语言项的 1-2 句设计方向（语义意图 → 编译器选路 → 地道目标 codegen；React 原生 `<dialog>`/Flutter showDialog 先例方向）
3. 本批次阶段 spec 的范围边界（哪些进、哪些明确排除）
4. 与本批次相关的半实现信号/登记册扩展（semantic-universe 新域 10 反馈模态）

## 解析即

决议记录 `## Answer` → `Status: resolved` → 地图 Decisions-so-far 追加一行 → 本批次即成为路线图一个阶段（后续独立 spec→plan→SDD）。

## Answer

**批次 1（反馈+模态）域裁决定案**（2026-08-09）。附带完成「真人写法」标准审计（用户新拍板测试标准）——见 `research/04-design-direction-audit.md`。

### 裁决表

| 能力项 | 结论 | 依据 |
|---|---|---|
| modal/dialog | 🔜 **进语言**（`@modal` 元素指令 + @slot 投影） | 双端平台级标准齐全（`<dialog>.showModal` vs `showDialog`）；焦点/Esc/遮罩三要素两端原生内建——编译器零手写焦点逻辑（研究 02 关键事实） |
| toast/通知 | 🔜 **进语言**（`showToast(msg, {kind, duration})` 内建函数） | 命令式意图三端可对齐（React 自生成轻量/Flutter SnackBar/OmniView 自绘）；语料 = learn-omniview 手写 NotificationStore/Panel |
| 焦点陷阱 | 🔜 随 @modal 进语言（附随行为，不单独立法） | 附随性；双端内建（研究 02 实证） |
| 骨架屏 | ⏳ @load 的 loading 形态扩展（不独立成项） | 三端无独立标准；与加载三态同一语义的不同呈现 |
| 错误边界 | 🏞 **留生态 + 语言边界注记**（用户拍板，按「真人写法」测试） | Flutter 无组件级惯用法（真人走 @load error 数据级错误/全局兜底）→ 跨目标判据不成立；数据级错误已由 @load error 信号覆盖。触发条件 = Flutter 生态出现组件级事实标准 或 真实语料需求 |
| tooltip | 🏞 留生态候选 | React 无标准形态；触发模型跨目标差异大（悬停 vs 长按）；属性级修饰语义窄 |

### 设计方向（进语言两项，方向级定稿）

1. **@modal**：`<dialog @modal={openSig}>…</dialog>` + @slot 投影 → React 原生 `<dialog>`（**受控 `open` prop 形态，阶段 spec 细化**——audit 细化点 1）/ Flutter `showDialog`+FocusTraversalGroup / OmniView 自绘 + 焦点管理
2. **showToast**：内建语义函数 → React 自生成轻量 toast（零依赖，audit 细化点 3）/ Flutter `ScaffoldMessenger.showSnackBar` / OmniView 自绘

### 真人写法审计结论（research/04-design-direction-audit.md）

- 已实现域（@form/@load/@drag/@ref/事件）**全部达标**——生成代码与真人惯用写法同构
- 契约草案（§4.2/4.3/4.4）**方向全部达标**；3 细化点（@modal React 受控形态 / @route 库绑定与依赖注入 / showToast 自生成细节）均属阶段 spec 决策，不阻塞方向级
- 审计原则沉淀：引库路线不构成否决（React Router/TanStack Query 是真人主流）；一端无地道形态 → 降级或留生态；已实现域是「地道代码」事实基准

### 批次范围

阶段 spec（批次 1）范围：toast/modal/焦点陷阱 + 骨架屏 @load 形态；tooltip/错误边界记语言边界。semantic-universe 新域 10 反馈模态登记随批次 spec。**详细语法（showToast 签名/@modal 受控形态）留阶段 spec**。

> **2026-08-09 翻案注记**（全面重审，07 车票标准校准）：错误边界与 tooltip 按新标准（前端功能跨框架通用）**翻案为进语言**——错误边界 → React ErrorBoundary / Flutter 数据级错误+全局兜底注记（批次 1 含）；tooltip → 属性级指令，双端映射存在（低优先，批次 5 交互归组评估）。骨架屏不变（@load 形态）；本车票原「留生态」裁决以翻案后为准。
