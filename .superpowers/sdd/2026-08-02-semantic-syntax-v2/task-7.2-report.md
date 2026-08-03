# Task 7.2 Report — 文档全量对齐审计 + 文档编译 CI 门禁

**Status: DONE**
**Commit: `[filled after commit]`** — `docs: full docs-parser alignment audit + doc-compile CI gate`
**Scope**: docs/** + `scripts/verify-docs.py` + `scripts/doc-gate-exemptions.txt` + `.github/workflows/ci.yml`（零编译器改动）

---

## 1. 门禁总表

| 门禁 | 结果 | 说明 |
|------|------|------|
| `python scripts/verify-docs.py --check`（232 个 omni 块） | ✅ 0 failed / 147 semantic notes | 16 篇指南 + BNF spec + 设计文档全部可编译块对 parser 验收 |
| 门禁诚实性（反空洞测试） | ✅ 破坏一个块 → exit 1；还原 → exit 0 | 故意损坏 `@mount` 语法后门禁正确失败 |
| `omnic build -t omniview`（learn-omniview） | ✅ exit 0 | 93 源文件全量编译成功 |
| 栅栏平衡 / 文档结构 | ✅ 全部平衡 | 变更后无未闭合代码栅栏 |

## 2. 审计方法

- 新建 `scripts/verify-docs.py`：抽取全部 fenced 块 → 按上下文分类（完整文件 / 组件体 / view / fn 语句 / const 表达式多 wrapper 回退）→ `omnic analyse --format json` 逐块验收。
- **完整块**（顶层声明开头）：任何错误即失败（E_IMPORT 除外——文档教授假设性项目结构）；解析错误**不做 wrapper 回退**（view-text 宽松解析会吞掉坏语法，回退会掩盖——反空洞测试证实）。
- **片段块**：只以解析错误失败；语义错误（未声明名等）为 NOTE（wrapper 上下文所致）。
- 空语言块仅当内容像 .omni 代码才检查（BNF 产生式/纯文本/平台映射表/运算符表/关键字表/类型引用表跳过——它们不教授可编译语法）。
- 豁免机制：`scripts/doc-gate-exemptions.txt`（14-grammar-lang §7.1 错误恢复教学块——故意含语法错误，文档自身注释说明）。
- 门禁默认跳过设计文档（全部块为 BNF/tsx 生成代码示例）。

## 3. 修复的文档分歧（按文件）

| 文件 | 修复 |
|------|------|
| README / 01 | `omnic check` → `omnic analyse`；`omnic run` 描述修正（非"开发服务器+热更新"，补 `omnic watch`）；版本 0.1.0 → 0.2.0；GSAP 句删除；`@themes` 注释 v2 形式 |
| 02-types | 联合体声明去尾分号（`type X = A \| B`，parser 真值）+ 注记；构造值示例移入 fn；"类型注解位置"混合片段重构为可编译组件 |
| 03-expressions | 展开示例重构（let 入 fn、action 入 store、自包含数据） |
| 04-functions | 信号写入 fn 移入组件；async 示例补 `std/http` import + ApiResult 定义；json 示例补 `std/json` import |
| 05-components | 成员骨架 `...` 占位 → 真实声明（event 需类型）；usage `<Greeting/>`/`<Child/>` 顶层 → 父组件 view；fn 片段 → 组件；const+signal → 组件；`capturePointer(id)` → 无参（T2.2）；@drag 约束注记更新（T5.1 已恢复）；TodoApp 补 store 定义 |
| 06-templates | 模板字符串插值 `{name}` → `${name}`（反引号内 `{name}` 是字面量——编译实证） |
| 07-events | 事件数据 fn 片段 → 组件 |
| 08-stores | `std/http` import；TodoApp 补 store 定义（自包含） |
| 09-styling | 顶层 view/style → 组件包裹 |
| 10-design-system | `@themes light, dark;` → `@themes { ... }`（4 处）；class/variant `...` 占位 → 真实内容；类/变体块补 token 定义（E0005）；ThemeStore @theme 块补含 `@themes` 的 design 块（E0006） |
| 11-motion | GSAP 句删除 |
| 12-standard-library | `data = raw` → 返回；顶层 let → fn；顶层 action → store；`fn f() async -> T` → `fn f() -> T async` |
| 13-project-structure | `{ ... }` 占位 → 真实体；补 `LearnDesign` import |
| 14-grammar-lang | `import "omniview"` → `import omniview`（BNF + 示例）；`Some { value }` 裸字段 → 类型化；`{ ... }` 占位 → 真实体；错误恢复块 → 豁免 |
| 15-grammar-view | style 条目补 `;` 分隔（新行分隔不解析——编译实证）；`@themes` v2（2 处 + BNF）；类块补 token 定义；`@for` BNF 补 key_expr/scroll_hint/@empty；变体块 → design 内 + token 定义；附录 C 全面修复（import/样式 `;`/Todo 定义/ThemeStore） |
| spec (2026-07-31) | §2.3 view{} 包裹 + focus 绑定补 @ref；§3.3 motion `over:` 移到 from/to 后（+ 补 scaleUp/swipeCard 定义 + view 包裹）；§3.7 内联动画体重写为真实 map 形式 `@enter={{ ... }}`（原"语法未实现"注记双向失真——map 形式已实现）；§4.3 token 类型大写 → 小写 + 值加引号 + 主题覆盖续行修 + 动态 token 补 store + class `;` + `@default` 只收 dim=value 对 + variant 命名冲突；§4.4 view 包裹 + variant dot 形式 → 调用形式（dot 形式 parser 拒绝——实证）+ token() 内联样式 → var()；§5.3 view 包裹 + 消费侧组件化；§6.3 字段简写 `text,` → `text: text` + Todo/nextId 定义 + view 包裹 + value={sig} → bind:value；§9 lifecycle view 包裹 + saveToLocal 定义 |

## 4. 历史发现核验清单（13 专家评审）

- ✅ README/01 CLI 命令（`omnic check` → `analyse`）+ 版本 0.2.0 —— 已修
- ✅ 06-templates 插值：view 文本 `{name}` 合法（parser 实证）；模板字符串 `${name}` 已修
- ✅ 10-design-system "design 仅 OmniView" 陈旧句 —— 现文档无此句（未发现残留）
- ✅ 08-stores `.filter` —— 语言方法教学正确（T1.2 落地后文档已对齐）；`.filter()` 表格行同步检查通过
- ✅ 11-motion GSAP 句 —— 已删除
- ✅ 15-grammar-view 内部矛盾 —— event_decl 类型 ✓ / @for BNF key_expr+scroll_hint+@empty 已补 / `import "omniview"` 引号已改 / `$counter` 已注记为 doc-fiction
- ✅ spec 状态 Approved（T0.7 已做）
- ✅ spec §6.2 store BNF（T3.5 已同步）

## 5. 编译器分歧记录（不修——按任务约定记录供 follow-up）

1. **联合体类型尾分号不一致**：`type X = A | B;` 解析错误，`type X = i32;` 合法——同一构造的分号规则不统一（文档已按 parser 真值教学，加注记）。
2. **`@themes { ... }` 无尾分号 vs `@default-theme x;` 必须有**：同族不一致。
3. **style 块条目必须 `;` 分隔**：新行分隔静默误解析，报错信息 "expected property name" 令人困惑。
4. **class 值混合规则**：`"str" token(x)` 合法；`token(x) token(y)` 与 `token(x) " " token(y)` 均拒绝——字符串开头限定未文档化。
5. **fn 类型参数在 return 位置调用失败 E0010**：`fn f(cb: fn(i32)->bool) -> bool { return cb(1); }` 报 "expected Bool, found fn type"（let 绑定形式正常）。
6. **`import "omniview";` 诊断误导**："expected 'as' after path"——引号形式是常见错误，无迁移提示。
7. **view-text 宽松解析**：view 文本中坏语法回退为字面文本（T3.1 已知）——门禁因此无法全量覆盖片段语法（已用 wrapper 多上下文 + 完整块硬失败缓解）。

## 6. 验证

- 门禁绿：232 块 0 失败；反空洞测试通过（破坏 → exit 1）。
- learn-omniview omniview 目标全量编译 exit 0。
- 全量文档栅栏平衡。
- 本任务零编译器改动（`git diff` 无 crates/omnilang-core 变更）。

## 7. CI 门禁（新增 docs-gate job）

- `cargo build --release -p omnic` → `python3 scripts/verify-docs.py --omnic target/release/omnic --check` → `omnic build --target omniview`（learn-omniview，输出到 /tmp 不脏工作树）。
- 豁免清单 `scripts/doc-gate-exemptions.txt`（1 条：错误恢复教学块，带理由）。
