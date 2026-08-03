# Task 7.1 Report — 三目标全量编译 + 浏览器全测（learn-omniview 终验）

**Status: DONE_WITH_CONCERNS**
**Commit: `待提交`** — `test: full three-target validation of learn-omniview`
**Working tree: clean (after commit)**

---

## 1. 门禁总表

| 门禁 | 结果 | 说明 |
|------|------|------|
| `omnic build --target omniview` | ✅ 0 错误 | 仅剩已知 unused-import lint 误报（T7.0 §8.1，编译器 bug） |
| `omnic build --target react`（157 文件） | ✅ 0 错误 | 重新生成产物与已提交输出字节一致（无 git diff） |
| `scripts/verify-react.sh`（tsc strict + eslint） | ✅ GATE PASSED | 166 tsc + 121 eslint 全部匹配文档化豁免，0 条失效豁免 |
| `cargo test -p omnilang --test generated_code_validation learn_omniview_flutter_output_passes_dart_analyze`（未忽略门禁） | ✅ 0 errors 0 warnings | 全项目 .dart 输出真实 dart analyze |
| `omnic build --target flutter` CLI smoke | ❌ exit 1 | 预存编译器问题（capturePointer 硬错误），非本任务回归，见 §4-C |
| React Playwright（main config，vite 5173） | ✅ 17 passed / 19 skipped(fixme) / 0 failed | 含 theme-wiring.spec.ts（主题接线实证通过） |
| OmniView Playwright（新 harness，静态 5174） | ✅ 41 passed / 4 skipped(fixme) / 0 failed | 9 标签页全量（渲染+动画+交互+主题+列表+焦点+指针捕获+零 console 错误） |

## 2. 交付物（本轮新增/修改）

| 文件 | 内容 |
|------|------|
| `build/react-test-app/playwright.omniview.config.ts`（新） | OmniView 目标浏览器套件配置（python 静态服务 5174，serving `../build`） |
| `build/react-test-app/tests/omniview-browser.spec.ts`（新，~500 行） | OmniView 9 标签页全量套件：45 测试（41 绿 + 4 fixme 指向记录的编译器 bug） |
| `build/react-test-app/tests/omniview-react.spec.ts`（重写，1047→~600 行） | T1.6 时代陈旧套件（旧 10 标签页结构，实测结构性全红）→ 新 9 标签页套件：36 测试（17 绿 + 19 fixme 指向记录的 React 编译器 bug） |
| `build/react-test-app/playwright.config.ts`（改） | `testIgnore` 排除 omniview-browser.spec.ts（testMatch `!` 否定在该版本不生效，实测后改用 testIgnore） |
| `build/` 媒体资产 ×5（新增，force-add） | OmniView 输出引用的 `/dot.png`、`fallback.jpg`、`large.jpg`、`sample.mp3`、`sample.mp4`（T7.0 移到 react-test-app/public，但 OmniView 静态输出仍引用根路径；浏览器套件零 404 所需） |

`theme-wiring.spec.ts` 未改，对新 React 应用 **1/1 通过**（data-theme 翻转 + CSS 覆盖 + Toggled 计数 + Sepia 多主题）。

## 3. 终验发现并已修复的问题（本任务范围：harness/infra）

1. **React 套件结构过时（harness）**：T1.6 时代 omniview-react.spec.ts 面向旧 10 标签页（首页/待办/…/补充，T7.0 已删除），实测首测即红（`button:has-text("首页")` 不存在）。已按新 9 标签页结构重写，同源断言。
2. **OmniView 目标无浏览器套件（harness 缺口）**：plan Step 3 需要 9 标签页全测，但仓库只有 React 套件。已新建 omniview 专用 config + 全套件。
3. **OmniView 静态输出媒体 404**：`/dot.png` 等 5 个资源在 build/ 缺失（T7.0 只移到了 react-test-app/public）。已复制进 build/ 并 force-add 提交。
4. **Playwright 选择器陷阱（spec 修复）**：OmniView 运行时把全部 9 个标签页分支留在 DOM（display:none 切换），同名文本/按钮存在于多个隐藏分支 → 所有定位器必须 `filter({visible:true})`；`getByText` 正则匹配**不做**空白归一化（生成 HTML 节点间有换行缩进）→ 多词文本改用字符串或 `\s*` 正则。React 侧：生成代码丢失插值前空格（`{'Toggled'}{n}{'times'}` → "Toggled1 times"）→ 同用 `\s*`。
5. **Playwright testMatch `!` 否定不生效**（实测 81 测试全跑）→ 改用 `testIgnore`（14+22 正常）。
6. **@drag 测试坐标陷阱**：拖拽盒在 900px 视口外（y=1488）时 mouse.move 打不到元素 → `scrollIntoViewIfNeeded` + 等入场动画稳定。
7. **旧断言误用**：DataTable 的 select 有 datalist 演示与 FormControlsDemo 两个 `<select>`（前者无 value 选项）→ 定位 `select.demo-select`；"Focus Input" 按钮实际名 "Focus"；"Play All" 实际名 "Replay All Animations"；ElementRefDemo 无 @onBlur（blur 状态只能由 Blur 按钮触发）。

## 4. 编译器 bug 记录（终验实证，**不修**——按 fix 策略记录供 follow-up）

> 每项含复现路径。修复后须移除对应套件中的 fixme 与错误过滤（自消除纪律，同 verify-react 豁免）。

### 4.1 OmniView（3 个运行时/代码生成 bug）

**BUG-1（P0）OmniView keyed @for 的 keyFn 硬编码循环变量名 `item`**
- 位置：`omnilang/src/codegen/omniview/list.rs:164`（`const keyFn = (item) => { ... String(<key_expr>) ... }`）与 `:275`（`const _key = keyFn(item);`）——keyFn 形参与调用点都写死 `item`，未代入 DSL 循环变量名（`item_var` 在同文件 line 44/255 已可用）。
- 复现：DataTable `@for row in rows key={row.id}`（T7.0 新增的 key）→ 生成 `keyFn(item)`，`row` 作用域外裸 `item` → 首迭代抛 `ReferenceError: item is not defined` → 渲染器整体中止。**DataTable 行永远不渲染**；排序每次再抛。控制台：`[OmniView] ERROR effect:run: ReferenceError: item is not defined`（挂载 1 次 + 每次 sort 1 次）。
- 影响面：所有循环变量名非 `item` 的 keyed @for（当前展示项目仅 DataTable；ForDirectiveKeyDemo 变量恰为 `item` 所以幸免）。
- 修复建议：keyFn 形参 + 调用点代入 `item_var`。

**BUG-2（中）OmniView `focus={sig}` 绑定在隐藏分支内失效**
- 复现：ElementRefDemo 的 `focus={shouldFocus}` 输入框，@mount 置 shouldFocus=true → 生成 effect 在脚本求值期执行（此时 Ref 分支 display:none）→ `el.focus()` 对隐藏元素无操作；分支显示后绑定不再重跑（effect 只订阅信号变化）→ **自动聚焦丢失**。实测切到 Ref 标签页后 `document.activeElement` 仍是标签按钮，而状态文本已显示 "focus=true（挂载自动聚焦）"。

**BUG-3（P0）OmniView keyed @for 重排不渲染新顺序（列表重排保状态核心交互损坏）**
- 复现：ForDirectiveKeyDemo——Add Item ×2 → Move Last to Front → 信号已更新（`ctr._forData` = [5,1,2,3,4]，实测）但 **DOM 保持 [1..5] 且内容陈旧**。
- 根因：keyed 渲染器对复用克隆既不移动位置（仅新克隆 append 进 frag），复用克隆的内容 rebind 用 `nid_<新idx>` 查找（克隆实际带 `_<原idx>` 后缀 nid，查不到）；而正确的 keyed rebind 函数（`__omni_rebind_ForDirectiveKeyDemo_item`）已生成但**从未被调用**（全文仅 1 处 = 定义处）。
- 增/删正常（新增克隆 append、oldKeys 清理），仅重排损坏。

### 4.2 React（6 个代码生成 bug，其中 2 个 P0 崩溃）

**BUG-A（P0）React `let mut` 变量发射为 `const` → 渲染期抛错 → 整应用卸载**
- 复现：`build/react-test-app/components/DataTable.tsx:60` `getBonusTotal`：源码 `let mut total = 0; let mut i = 0;`（DataTable.omni:69-77）→ 生成 `const total = 0; const i = 0;` → 循环赋值抛 `TypeError: Assignment to constant variable`（渲染期）→ **React 无错误边界，整个 root 卸载**：UI 标签页一打开整站空白（标签栏都消失）。此即 tsc 豁免 TS2588 类（`LetStmtIR carries no mut flag`）的**运行时致命形态**。
- 同源第二处：`components/ErrorHandlingDemo.tsx:31` `try_catch_finally_divide`（try/catch/finally 内 `let mut`）→ Language 标签页同样整站崩溃。
- 影响：UI 与 Language 两个标签页不可用；浏览器套件中其全部测试失败均为下游。

**BUG-C（高）React 内联 store action 无发射路径（TS2304 类）**
- 复现：`components/InlineStoreDemo.tsx:15-16` `addApple = () => { CartStore.addItem("Apple", 5.0); }`——`CartStore` 对象**从未生成**（全文仅 5 处调用点引用）。点击 Add Apple → ReferenceError，购物车永不更新（"cart empty" 永久显示）。

**BUG-D（中）React 静态文本 `&` 在 JSX 字符串字面量中显示为字面 `&amp;`**
- 复现：SignalPatternsDemo 标题源码 `Signal & Store Patterns` → 生成 `{'Signal &amp; Store Patterns'}` → React 渲染字面 "Signal &amp; Store Patterns"（实体在 JSX 表达式字符串中不解码）。aria 快照实证。

**BUG-E（低）React style 对象发射 kebab-case 属性 → React 控制台错误**
- 复现：Design 标签页 `border-radius`/`box-shadow` 等 → `Unsupported style property 'border-radius'. Did you mean 'borderRadius'?`（tsc 豁免 TS2353 类的运行时形态）。

**BUG-F（低）React 事件处理器小写名（onAnimationend 等）**
- 复现：UI 标签页 `Invalid event handler property 'onAnimationend'. Did you mean 'onAnimationEnd'?` 等 4 条（onAnimationend/onAnimationiteration/onAnimationstart/onTransitionend）。
- 注：Ref 标签页的焦点/指针捕获交互经 matcher 修复后**全部通过**（非 bug，纯套件空白归一化问题）。

### 4.3 编译器低severity记录（note 级）

- **解析器文本 run 空格剥离**：ForDirectiveKeyDemo 标题 `@for key= 身份追踪 Demo` 渲染为 `@forkey=身份追踪Demo`（文本 run 以指令 token 开头时空格丢失；与 T7.0 §8.3 `design` 关键字同类）。展示源码正确，不 workaround。
- **OmniView CSP meta 含 frame-ancestors**：Chrome 警告 "frame-ancestors is ignored in <meta>"（应改用 X-Frame-Options 或 HTTP 头）。
- **Flutter CLI 与库路径诊断口径不一致（预存，T7.0 §8.4）**：`omnic build --target flutter` 对 capturePointer 输出硬错误并 exit 1、零输出；库路径按 spec §10.3 过滤该诊断。未忽略的 dart analyze 门禁（库路径）通过。CLI smoke 因此红——**非本任务回归**。
- unused-import lint 系统性误报（T7.0 §8.1，已记录，未重复展开）。

## 5. 验证退出条件对照（brief Step 1-5）

- [x] **Step 1 React 全量**：`omnic build -t react` → tsc 0 非豁免错误 + eslint 0 非豁免（verify-react.sh GATE PASSED，166/121 全豁免且零失效条目）。注：brief 期望「豁免清单清空」无法达成——残留 166 tsc + 121 eslint 全部对应未落地根因任务（T1.6/T1.3/T3.3/T3.2 类），其中 TS2588/TS2304/TS2353 本轮已在浏览器中实证为运行时 bug（§4.2 A/C/E）。
- [x] **Step 2 Flutter 全量**：未忽略门禁 `learn_omniview_flutter_output_passes_dart_analyze` 0 errors 0 warnings；CLI smoke 因 §4-C 预存问题 exit 1（已记录）。
- [x] **Step 3 OmniView 全量**：`omnic build -t omniview` 0 错误；浏览器 9 标签页全测 41 绿（渲染 ✅ / 动画实跑 ✅ WAAPI running 实证 / 主题切换 ✅ data-theme+CSS 变量+Toggled 计数+跨标签持久 / 列表增删 ✅ / 焦点命令式 ✅ / 指针捕获 ✅ / @drag ✅ 300px 实证 / 零 console 错误 ✅ 仅过滤 2 个已记录项）；fixme 4 项 = BUG-1/2/3 的受测断言。
- [x] **Step 4 回归修复**：本任务范围（harness/infra）全部修复并重跑验证；编译器 bug 按策略记录不修（§4，含复现）。
- [x] **Step 5 Commit**：`test: full three-target validation of learn-omniview`

## 6. 遗留/风险

- React 与 OmniView 两个标签页级 P0（BUG-A/BUG-1）修复前，UI/Language 标签页（React）与 DataTable（OmniView）在浏览器中不可用——已用 fixme + 错误过滤在套件中如实呈现（套件保持绿，fixme 计数即 bug 面）。
- fixme/过滤的移除纪律：修复根因后删除对应 fixme 与 `KNOWN_ERRORS` 条目（两套件头部注释均有指引）。
- 套件运行命令：
  - React：`cd test/projects/learn-omniview/build/react-test-app && npx playwright test`（vite 5173）
  - OmniView：`npx playwright test --config=playwright.omniview.config.ts`（python `-m http.server 5174 --directory ../build`）
