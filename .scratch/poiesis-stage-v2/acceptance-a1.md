# Poiesis 阶段 v2 Task 1（A1 行为卡保存往返）验收记录

> 实机验证（Victauri MCP 驱动），demo2（D:\space\omni\demo2）。
> 日期：2026-08-13 · 环境：修复前画布编译错误基线 → 修复后全链路

## 结论

**✅ A1 通过**：行为卡"保存 → 重开 → 行为仍在"完整往返验证成功；**实机发现 1 个真实缺陷（空字符串默认值往返丢失）+ 1 个动作模板缺陷（记录）**，前者已修复并提交。

## 缺陷 1：空字符串默认值往返丢失（已修复）

**症状**：重开 demo2 后画布报 "expected '=' — found Semicolon"，`<p>{input_f1bd5d_value}</p>` 渲染破裂。

**根因链**（源码级定位）：
1. 磁盘 `signal input_f1bd5d_value: String = ""` 语法正确（M3 修复后的保存形状）
2. JS parser（parser.ts collectDefaultValue）剥引号存"内容值"：`""` → JS 空串 `""`（与"无默认值"undefined 不可区分）
3. emitter（source-emitter.ts 5 处发射点）守卫 `sig.defaultValue !== undefined && sig.defaultValue !== ""`——空串被当作"无默认值"丢弃 → 输出 `signal input_f1bd5d_value: String;` → WASM 编译报错
4. M3 验收只覆盖了内存创建路径（connectElementLink 直接给 `'""'` 字面量），"保存 → 重开"解析路径从未验证——A1 正是为此而生

**修复**（两轮，首轮 replace_all 因缩进/变量名差异漏掉组件信号发射 2 处）：
- `formatDefaultValue`：String 类型空串 → 返回 `'""'`（补引号，区别于无默认值）
- 全部 5 处发射守卫统一为 `!== undefined`
- 测试锁契约：`source-emitter-attrs.test.ts` 新增 6 用例（含 parse 端到端）
- 提交：`9b2ae5aa`（首轮）+ `…`（二轮残留守卫）

**验证证据**：修复后重开 demo2 → 画布渲染正常（Primary/Danger/Hello Card/Ghost 全渲染）；保存输出 `signal input_f1bd5d_value: String = "";` 往返保持。

## 缺陷 2（记录，未修）："设置"动作模板 text 参数不补引号

**症状**：行为卡 当[点击]→[设置]→[newSignal]→[demo] 提交 → 保存失败（`[save] WASM analyse errors`）。

**根因**：ACTIONS.set 模板 `${c.target} = ${c.value};`——text 参数原样插入，用户输入 `demo` → `newSignal = demo;`（demo 未定义变量）→ WASM analyse 拦截保存（保护机制工作正常）。

**判定**：动词集是 G3 后置项；"设置"的 value 参数语义（字符串字面量 vs 表达式）需设计决策。**记 backlog，不在 A1 修**。规避：验证改用"加"动作（amount 数值模板安全）。

## 行为卡往返剧本（完整闭环）

| 步骤 | 验证内容 | 结果 | 证据 |
|---|---|---|---|
| 1 | 选中元素 → [行为] 标签行为卡表单（事件/动作/参数） | ✅ | behavior-card 表单 7 事件 × 10 动作 |
| 2 | 已有事件绑定重开可见（M3 前内联 onPress） | ✅ | "点击 — ()=>{ newSignal=\"点击成功\"; }" |
| 3 | 添加行为卡 当[点击]→[加]→[newSignal]（同事件替换路径） | ✅ | 内存生成代码 `fn onPressAuto() -> void { newSignal = newSignal + (1); }` + ghost `@onPress={onPressAuto}`（pid -2 递增） |
| 4 | 保存落盘 | ✅ | main.omni mtime 更新；`fn onPressAuto` + `@onPress={onPressAuto}` + pid 注释三件套 |
| 5 | 重开项目 → 画布编译 | ✅ | 无编译错误，全元素渲染 |
| 6 | 重开后行为回显 | ✅ | [行为] 标签 EventsSection 显示 `ƒ onPressAuto` + 事件行 |
| 7 | 点击生效 | ⏸️ 边界 | ghost 按钮 `disabled={newSignal}`（demo2 元素特性，newSignal 非空）——点击被禁用；fn 存在 + 指向 + 编译通过 = 行为可用，运行时生效验证建议走调试窗口 |

## 提交清单

```
9b2ae5aa fix(poiesis-stage-v2): 空字符串默认值往返丢失——formatDefaultValue 补引号 + 发射守卫只判 undefined（A1 实机发现）
<二轮>    fix(poiesis-stage-v2): 空串默认值守卫残留——组件信号发射（comp.signals）两处 def/defStr 变体补修（A1 实机二轮）
```

## 记录的边界

1. **点击生效**：ghost disabled 特性挡住——运行时验证留调试窗口（非 A1 核心，fn/指向/编译已闭环）
2. **动作模板缺陷**（缺陷 2）："设置" text 参数不补引号——记 backlog 待 G3 词库设计
3. demo2 磁盘 `main.omni.bak` 备份存在（验证前创建，验证后磁盘为修复后正确形状，备份可删）
