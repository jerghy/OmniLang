# OmniLang 语义级 DSL v4 行为/注入修复实现计划（12 专家终审评审全量处置）

> **✅ 状态：已执行完毕（2026-08-06 收口）**——24 个任务 + 终审修复波全部完成（含 2 个修复轮：Task 2.1 两轮、Task 5.2/5.7 各一轮）；终审（opus）MINOR-FIXES → 修复波 → 复审 4/4 ADDRESSED → **MERGE-READY**。终态门禁：React tsc 77/77 全豁免 + eslint 4/4 GATE PASSED、Flutter warnings ≤673（671→673 受支持例外）、docs-gate 248 块 0 failed、verify 双脚本 0 problem、Playwright 46 passed + 1 skipped + 0 failed（semantic-behavior 5/5）。范围裁定（用户 2026-08-06）：27 任务 → 24 执行 + 3 defer（1.1/1.3→#86、4.1→#87）+ 4.2 运行时半 cut。进度记录：`.superpowers/sdd/2026-08-06-semantic-syntax-v4-fixes/progress.md`（恢复地图 + 终审分诊）；验收表见 ADR-067。执行期间用户并行开发（poiesis 调试面板/@for UI 等）零冲突（评审范围按任务 commit 隔离；3.2 生产修复曾随其 57773ef2 入库——字节一致）。

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 12 专家终审（`.architecture/reviews/feature-semantic-dsl-v2-syntax-review.md`，2026-08-06）发现的 14 HIGH + 23 MEDIUM 中全部 Critical 项——注入防线两缺口（React 动态 URL XSS + Flutter `$` 转义回归）、React 行为层三缺陷（陈旧闭包/悬空 setter/数字 bind:value）、「exit 0 输出损坏」族（walker 缺口/scrollTo 双轴/longpress-hover-toggle 死处理器）、跨目标静默降级（Vue/Dioxus floor + keyFn 塌缩）、台账校准（semantic-universe/设计文档/README/stdlib/value={sig}/bind:checked）——使语义 DSL 从「结构层地道」达到「行为层地道 + 台账诚实」。

**Architecture:** 沿用三层架构（parser → semantic → IR → codegen）。按 v3 先例顺序推进：**回归测试基建先行（组合 fixture + 浏览器行为断言 + 语料扩展，全部失败测试先于修复）→ 注入防线（语料变绿）→ React 行为层 → exit-0 族 → 跨目标 floor → 台账校准 → 终验**。每阶段结束零「open-undispositioned」发现（fix-now / hard-diagnostic / delete / defer-with-trigger 四选一）。

**Tech Stack:** Rust（omnilang + omnilang-core workspace）、React 19 + TS strict + Zustand v5（生成目标）、Flutter + Dart + Riverpod 3.0（生成目标）、CI 门禁（cargo test / tsc --strict / eslint react-hooks / dart analyze / Playwright / docs-gate / verify-*.py）。

## Global Constraints

- **评审依据**: `.architecture/reviews/feature-semantic-dsl-v2-syntax-review.md`（2026-08-06，12 专家；本计划的发现编号沿用评审：Security HIGH-1、Rust HIGH、TS/React H1-H3、Compiler HIGH-1/2/3/4、Domain HIGH-1/2/3、Systems HIGH-1、Perf HIGH、Pragmatic MEDIUM-1/2、Maintainability MEDIUM-1/2、Domain MEDIUM-2、Implementation Strategist MEDIUM）
- **设计契约**: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`（本次修改后同步状态注记）
- **用户确认决策（本计划新裁决，评审已给出方向）**:
  - **D-V4-1**: hover/toggle 按 longpress 先例改标「部分」（Flutter 交付；React/OmniView 无 DOM 对应 → 发射 warning + 跳过死处理器），不改变语义边界（不引入 React onChange+checked 映射——保持 toggle 语义纯净，留待 v5 评审）
  - **D-V4-2**: `theme="dark"` 对已知 DOM 标签发 warning（自定义组件豁免——不误伤合法 prop）；设计文档诊断表如实修订
  - **D-V4-3**: `bind:checked` 正式化（BNF + semantic-universe「部分」条目 + 登记册注记）——不删除（真实前端意图，删除制造语义黑洞）
  - **D-V4-4**: `value={sig}` 判别器改为「仅当元素带 @ref 时报 EValueBindingRemoved」（v1 双向形状需要 @ref；无 @ref 的 value={sig} 走普通单向 prop）；提示文本修正
  - **D-V4-5**: Vue/Dioxus 对 motion/ref/a11y/bind:* 发 E_CODEGEN 硬错误（镜像 DESIGN_BLOCK_ERROR），非警告——ADR-052「绝不静默丢弃」契约字面恢复
  - **D-V4-6**: H2 只写信号保留 useState（`const [, setX] = useState(...)`），不扩展发射侧剪枝——行为保留优先
  - **D-V4-7**: keyFn 对象 key：编译期 for-key lint 对对象类型 key 表达式发 warning（要求字段访问）+ 运行时兜底（object → id 优先 / JSON.stringify 兜底）
  - **D-V4-8**: velocity_max **移除**（BNF/parser/IR/optimize/三后端 + React slice 排除），deferral 注册「demo 或用户需要 fling 速度物理」触发（Pragmatic MEDIUM-1，S-M 机械工作量）
  - **D-V4-9**: verify-vocabulary.py / verify-semantic-universe.py 合并共享提取 + BNF 地面真相重指向 parser 常量（Pragmatic MEDIUM-2 + Systems MEDIUM-3 + Domain MEDIUM-2 三成员同见）
- **破坏性政策**: 已删除语法 → 硬诊断 + 迁移提示（validate_ref_ops 先例）；demo 迁移随任务；文档同步随任务同 commit
- **验证协议**: 每任务 TDD（写失败测试 → 验证失败 → 实现 → 验证通过 → commit）；每阶段结束跑全量门禁（cargo test / verify-react / dart analyze / docs-gate / Playwright / verify-*.py）
- **提交纪律**: 每任务独立 commit，conventional commits（feat/fix/refactor/docs/test/chore），无 attribution
- **deferrals**: 本计划关闭 #43/#61/#69/#70（台账补行，Implementation Strategist MEDIUM）；#62/#63/#64/#67/#72/#73（React 豁免类）随根因修复删除豁免条目；新增 deferral：velocity_max（D-V4-8）、动态 URL 消毒的 vue/dioxus 镜像（若 Phase 1 超范围）、a11y 动态名称（Domain MEDIUM-1，记录缺口）
- **豁免纪律**: 每条豁免删除 = 根因修复落地 + 回归测试；豁免清单计数同步（当前 tsc 87 / eslint 4，实测基线 Task 0.4 锁定）

---

## 范围裁定（用户 2026-08-06 复审——任务范围仅「语义级语法成熟化 + 生成的 React/Flutter 代码地道」）

> 以两个目标为标尺，逐项判定后的**最终执行范围**。被裁定的任务保留正文作参考（defer 任务含完整实现方案，触发后按正文执行；cut 任务不再执行）。

| 计划项 | 目标归属 | 裁定 | 依据 |
|--------|---------|------|------|
| 0.1/0.2/0.3/0.4 | G2 + 治理 | ✅ 执行 | 测试网是修复的失败测试 |
| **1.1（React 动态 URL 消毒）** | 均不属 | ⚠️ **defer（触发：#86）** | 安全加固非地道代码；**React 自身运行时阻断 javascript:/vbscript:（ReactDOM isJavaScriptProtocol，0.14 起）**——主向量已被目标框架关闭，残余面仅 data: URL |
| 1.2 | G2 | ✅ 执行 | 生成 Dart 编译失败/错值 |
| **1.3（运行时 harness）** | 依附 1.1 | ⚠️ **defer（随 #86）** | 无 1.1 则无被验证对象 |
| 2.1/2.2/2.3/2.4 | G2 | ✅ 执行 | 行为语义 = 地道的行为半 |
| 3.1/3.2/3.3 | G2 + G1 | ✅ 执行 | 生成正确性 + universe 诚实 |
| **4.1（Vue/Dioxus floor）** | 均不属 | ⚠️ **defer（触发：#87）** | 非目标后端（用户目标 = React/Flutter）；ADR-052 契约违记录入台账 |
| 4.2 lint 半 | G1 | ✅ 执行 | 成熟语法 = 误用有诊断 |
| **4.2 运行时兜底半** | 均不属 | ✂️ **cut** | React 对对象 key 同样塌缩+警告、不兜底；对用户误用做行为补偿 = 过度设计；警告即终态 |
| 5.1/5.2/5.3/5.4 | G1 | ✅ 执行 | 语法台账/诊断/表面诚实 |
| 5.5/5.5b | G1 | ✅ 执行（5.5b 最后做） | 语法文档 = 语言 UI |
| **5.6 锚点重指向半** | G1 | ✅ 执行 | 防漂移机制本身在漂移 |
| **5.6 脚本合并半** | 均不属 | ✂️ **cut** | 120 行内部去重，重复可容忍（YAGNI） |
| 5.7 | 治理 | ✅ 执行（仅补行/改写，S） | 项目惯例 |
| 5.8 | 反过度设计 | ✅ 执行 | 移除评审发现的过度设计 |
| 6.1/6.2 | 收口 | ✅ 执行 | 终验 |

**执行范围终态**：27 任务 → **24 执行（含 4.2 lint 半）+ 4.2 运行时半 cut + 3 defer（1.1/1.3/4.1，注册 #86/#87）**。deferral 注册并入 Task 5.7 台账节。

---

## 文件结构

```
crates/omnilang-core/src/
├── semantic/
│   ├── validate_array_methods.rs  # walker 下钻 RefOp 子节点（3.1）
│   ├── validate_semantic_bindings.rs # value={sig} 判别器按 @ref（5.4）
│   ├── validate_props.rs          # theme= 警告（5.2）
│   ├── lints/for_key.rs           # 对象类型 key 表达式警告（4.2）
│   └── lints/unused_import.rs     # #43 假阳性修复（5.7 若预算允许）
├── parser/
│   ├── ast/motion.rs              # velocity_max 删除（5.8）
│   └── view/element.rs            # bind:checked 正式化注记（5.3）、value 重分类（5.4）
├── ir/
│   ├── lower/view.rs              # velocity_max 删除（5.8）
│   ├── optimize.rs                # velocity_max 删除（5.8）
│   └── types_motion.rs            # velocity_max 删除（5.8）
└── codegen/
    ├── sanitize.rs                # 编译期 is_dangerous_url 既有（1.1 参照）
    └── shared/event_names.rs      # canonical 表既有（3.3 参照）
omnilang/src/
├── codegen/
│   ├── mod.rs                     # vue/dioxus floor 门（4.1）
│   ├── shared/sanitize.rs         # sanitize_url_helper_js（1.1）
│   ├── react/
│   │   ├── mod.rs                 # warnings 通道（3.3）、generate 接线
│   │   ├── view.rs                # H1 写后读线程化（2.1）、H3 Number()（2.3）、M3 空事件特判（2.4）、动态 URL 包裹（1.1）、react_prop_for_canonical Option 化（3.3）
│   │   ├── expr.rs                # 剪枝半侧（2.2 参照）、H1 next-值发射（2.1）
│   │   ├── component/state.rs     # H2 useState 保留（2.2）
│   │   ├── component/mod.rs       # pruned 集合调整（2.2）、has_mutable_states（2.2）
│   │   ├── component/store_slices.rs # velocity_max 排除删除（5.8）
│   │   └── component/motion.rs    # 拖拽手势 velocity_max 参照（5.8）
│   ├── flutter/
│   │   ├── view.rs                # $ 转义修复（1.2）、scrollTo 双轴警告扫描（3.2）
│   │   └── ref.rs                 # scrollTo 警告（3.2）
│   ├── omniview/
│   │   ├── view/events.rs         # 无映射事件警告（3.3）
│   │   ├── view/element.rs        # velocity_max 删除（5.8）
│   │   └── list.rs                # keyFn 运行时兜底（4.2）
│   └── vue/ dioxus/               # floor 门目标（4.1，仅测试）
├── tests/
│   ├── tests_regression/injection.rs # rawtext-multi 语料点（0.1）
│   ├── generated_code_validation.rs  # Flutter 组合 fixture（0.3）
│   └── tests_regression/sanitizers.rs # 运行时消毒器执行 harness（1.3）
omnilang-wasm/src/lib.rs           # 无变更（契约已统一）
scripts/
├── verify-react.sh                # 豁免计数上限既有（0.4 基线锁定）
├── verify-vocabulary.py           # 合并/锚点重指向（5.6）
├── verify-semantic-universe.py    # 合并/锚点重指向（5.6）+ 事实断言（5.1）
└── verify-docs.py                 # docs/README.md 纳入链接断言（5.5）
docs/
├── README.md                      # 入口重写（5.5）
├── stdlib.md                      # 弃用横幅 + omni/* 修正（5.5）
└── omniview/                      # 迁移指南新增（5.5b）
```

---

# Phase 0: 回归测试基建先行（失败测试 + 基线锁定）

> 顺序优先：任何修复前先把全部缺陷类固化为失败测试（语料点/组合 fixture/浏览器行为断言），并锁定 React 门禁计数基线。本阶段全部为测试/台账改动，零生产代码变更。完成标准：`cargo test` 全绿（新增测试按预期失败——除 0.4 外），修复轮后续阶段以此为准绳。

### Task 0.1: 注入语料扩展——multi-text-child 形态（Flutter `$` 转义回归的失败测试）

**Files:**
- Modify: `omnilang/src/codegen/tests_regression/injection.rs:693-695`（rawtext_src 后新增点）
- Modify: `omnilang/src/codegen/tests_regression/injection.rs:783-828`（POINTS 表注册）

- [ ] **Step 1: 写失败测试——新增 rawtext-multi 语料点**

在 `injection.rs` 的 `rawtext_src` 函数之后添加（注意 `$` 载荷不破坏 .omni 解析——仅 `{`/`<` 会，`$` 在纯文本安全）：

```rust
/// RAW text followed by an interpolation child — the mixed-content
/// paragraph shape (`<p>价格 $total 元</p>`). Routes through the
/// all-text-concat (flutter/view.rs:2193) and TextSpan (flutter/view.rs:332)
/// emission paths, which the single-child rawtext point does not reach
/// (v4 review: rust HIGH — `$`-escaping regression at those two sites).
///
/// 形状注记（2026-08-06 修订）：原单 div 形状（`<div>x{payload}<span>{1}</span></div>`）
/// 实测为恒 GREEN 空转——payload 是 div→Container 的直接文本子节点，走已转义
/// 主路径（view.rs:225），span 仅含插值不带 payload。修订为双站点形状：
/// span 全文本子节点（payload 落 all-text-concat :2193）+ p 混合子节点
/// （payload 落 TextSpan :332）。语料测试名 `injection_payload_corpus`。
fn rawtext_multi_src(payload: &str) -> String {
    format!(
        "component App {{\n    view {{\n        <div><span>x{payload}{{1}}</span><p>{payload}<b>{{2}}</b></p></div>\n    }}\n}}\n"
    )
}
```

在 `POINTS` 表的 `rawtext` 条目之后注册（`injection.rs:801-806` 区域）：

```rust
    Point {
        name: "rawtext-multi",
        make_src: rawtext_multi_src,
    },
```

- [ ] **Step 2: 运行验证失败（Flutter 行）**

Run: `cargo test -p omnilang --features codegen-flutter rawtext_multi 2>&1 | tail -20`
Expected: FAIL——语料断言 Flutter 输出的 raw `$` 未被转义（`Text('x...$` 出现于生成 Dart，模式缺失断言命中）。

- [ ] **Step 3: 验证其余后端通过（部分绿）**

Run: `cargo test -p omnilang --features codegen-omniview rawtext_multi 2>&1 | tail -5`
Expected: PASS（OmniView 模板字面量路径已转义 `$`——`omniview/view/mod.rs:91-96`）。

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/tests_regression/injection.rs
git commit -m "test(injection): rawtext-multi 语料点——Flutter \$ 转义回归失败测试（v4 0.1）"
```

### Task 0.2: 浏览器行为断言套件（H1 类结构性盲区闭合）

**Files:**
- Create: `test/projects/learn-omniview/build/react-test-app/tests/semantic-behavior.spec.ts`（既有 Playwright 套件同目录——tests/ 是提交的 harness，src/ 由 `omnic build` 重新生成）

- [ ] **Step 1: 写失败测试——写后读序列的行为断言**

先重新生成 React 输出（确保 HEAD 代码生成现状）：

Run: `cd test/projects/learn-omniview && ../../../target/release/omnic build --target react -o build/react-test-app`

创建 `test/projects/learn-omniview/build/react-test-app/tests/semantic-behavior.spec.ts`——自包含助手复制既有 spec（`tests/omniview-react.spec.ts:69-97` 的 BASE/TABS/switchToTab/visible/clickBtn/expectNoErrors 同款）：

```ts
import { test, expect, Page } from '@playwright/test';

// v4 review H1/H2: DSL 顺序语义（x = v; use x → 读新值）在 React 生成
// 代码中被陈旧闭包破坏；只写信号发射悬空 setter。断言用户可见结果与
// 页面错误（既有套件断言重渲染显示，结构性看不见顺序语句）。

const BASE = 'http://localhost:5173';
const TABS = ['📋 Overview', '🎨 UI', '🎬 Motion', '💅 Design', '⚡ State', '🔤 Language', '🔗 Ref', '⚙️ Compiler', '🔮 Tokens'];

async function switchToTab(page: Page, index: number) {
  await page.goto(BASE, { timeout: 8000 });
  await page.waitForSelector('button', { timeout: 8000 });
  await page.locator(`button:has-text("${TABS[index]}")`).first().click();
  await page.waitForTimeout(500);
}

async function expectNoErrors(page: Page, label: string, fn: () => Promise<void>) {
  const errors: string[] = [];
  const onErr = (m: any) => { if (m.type() === 'error') errors.push(m.text()); };
  page.on('pageerror', (e) => errors.push('PAGEERROR: ' + e.message));
  page.on('console', onErr);
  try {
    await fn();
    await page.waitForTimeout(300);
  } finally {
    page.removeListener('pageerror', () => {});
    page.removeListener('console', onErr);
  }
  const unexpected = errors.filter(e => !e.includes('favicon') && !e.includes('frame-ancestors'));
  expect(unexpected, `${label} console/page errors`).toEqual([]);
}

// 各 demo 所在标签页由实现者从生成 src/App.tsx 确认（下述为评审证据中的
// 位置提示：FormControlsDemo/EmitKeywordDemo/DataTable/AdvancedPseudoDemo）
const TAB_UI = 1;      // 🎨 UI —— AdvancedPseudoDemo（伪类 demo）
const TAB_STATE = 4;   // ⚡ State —— FormControlsDemo（表单控件）
const TAB_COMPILER = 7; // ⚙️ Compiler —— DataTable / EmitKeywordDemo 候选

test('checkbox 勾选后状态文字与显示一致（H1 陈旧闭包）', async ({ page }) => {
  await switchToTab(page, TAB_STATE);
  // 勾选第一个 checkbox 后，DSL 源 `agreed = !agreed; statusText = if agreed {...}`
  // 期望状态文字显示新值（与相邻显示一致）——断言文本含 "Agreed" 而非 "Not agreed"
  await page.locator('input[type="checkbox"]').first().check();
  await expect(page.getByText(/Agreed|Not agreed/).first()).toContainText('Agreed');
});

test('链接计数器首次点击报 1（H1 off-by-one）', async ({ page }) => {
  await switchToTab(page, TAB_STATE);
  await page.locator('a', { hasText: /link/i }).first().click();
  await expect(page.getByText(/clicked/i).first()).toContainText('1');
});

test('emit 载荷无 off-by-one（H1 跨组件契约）', async ({ page }) => {
  await switchToTab(page, TAB_COMPILER);
  await page.getByRole('button', { name: /emit/i }).first().click();
  await expect(page.getByText(/"emit #/).first()).toContainText('"emit #1"');
});

test('排序方向与标签一致（H1 toggleSort）', async ({ page }) => {
  await switchToTab(page, TAB_COMPILER);
  // 数据无关断言：点击前捕获全部行值 → 点击 → 标签翻转时顺序必须翻转
  const before = await page.locator('tbody tr td:first-child').allTextContents();
  const labelBefore = (await page.getByText(/sort/i).first().textContent()) ?? '';
  await page.getByRole('button', { name: /sort/i }).first().click();
  const after = await page.locator('tbody tr td:first-child').allTextContents();
  const labelAfter = (await page.getByText(/sort/i).first().textContent()) ?? '';
  if (labelAfter !== labelBefore) {
    expect(after).toEqual([...before].reverse());
  }
});

test('只写信号点击不崩溃（H2 悬空 setter）', async ({ page }) => {
  await switchToTab(page, TAB_UI);
  await expectNoErrors(page, 'write-only signal click', async () => {
    await page.getByRole('button', { name: /dark variant/i }).click();
    await page.waitForTimeout(300);
  });
});
```

- [ ] **Step 2: 运行验证失败**

Run: `cd test/projects/learn-omniview/build/react-test-app && npx playwright test tests/semantic-behavior.spec.ts`
Expected: FAIL——checkbox 状态文字断言、链接计数、emit 载荷、排序（H1 四处）；H2 测试的 `expectNoErrors` 捕获 `PAGEERROR: ReferenceError: setVariant is not defined`。

> 实现注记（执行者必读）：
> 1. **标签页确认**：TAB_UI/TAB_STATE/TAB_COMPILER 的占位值须从生成 `src/App.tsx` 的实际标签映射确认——grep demo 组件名（FormControlsDemo/EmitKeywordDemo/DataTable/AdvancedPseudoDemo）在 App.tsx 的引用位置，改常量并同步注释。评审证据：FormControlsDemo 在 🎨 UI 或 ⚡ State 标签（`App.tsx:52-86` 九层三元对应 9 标签）；DataTable 排序按钮在 ⚙️ Compiler 或 🎨 UI。
> 2. **选择器适配**：文本定位器（getByText/getByRole/正则）以生成组件实际渲染文本为准——评审引用的显示文本「Agreed/Not agreed」「Link clicked N times」「emit #N」「Sort:」为证据基线，若实际文案有出入以生成 DOM 为准修正正则，**断言目标（写后读显示新值、载荷 #1、排序随标签翻转、无 pageerror）不变**。
> 3. **重新生成纪律**：本套件的每次运行前须重跑 Step 1 的 `omnic build --target react`（生成 src/ 为 gitignored 产物——测试断言的是重新生成的输出）。

- [ ] **Step 3: Commit（测试随本任务提交，失败状态在 Phase 2 修复后转绿）**

```bash
git add react-test-app/e2e/semantic-behavior.spec.ts
git commit -m "test(react): H1/H2 行为断言套件——写后读序列与只写信号崩溃（v4 0.2）"
```

### Task 0.3: 组合 fixture 矩阵（exit-0 族失败测试）

**Files:**
- Modify: `omnilang/src/codegen/tests_regression/snapshots/`（新增组合 fixture 的既有模式）
- Modify: `omnilang/tests/generated_code_validation.rs`（Flutter 编译断言入口，Task 0.5 既有模式）

- [ ] **Step 1: 写失败测试——ref-op × 数组方法组合**

在 `omnilang/tests/fixtures/`（现有 fixture 目录）新增 `combinations/refop_array.omni`：

```omni
component App {
    view {
        <input @ref={box} />
        <button @onClick={() => {
            box.setSelection(DataStore.items.filter(|i| i > 1).len(), 5);
        }}>Go</button>
    }
}
```

在 `generated_code_validation.rs` 增加 Flutter 编译断言（对照既有枚举模式——所有 fixture 全输出 dart analyze）：

```rust
#[test]
fn combinations_refop_array_flutter_compiles() {
    // v4 compiler HIGH-1: 数组方法在 ref-op 参数内必须被重写
    // （.filter → .where(...).toList()）——生成 Dart 不得含 verbatim `.filter(`
    let dart = compile_fixture_to_dart("combinations/refop_array.omni");
    assert!(
        !dart.contains(".filter("),
        "ref-op 参数内数组方法逃逸重写：\n{}",
        dart
    );
    assert!(dart.contains(".where(("));
}
```

- [ ] **Step 2: 运行验证失败**

Run: `cargo test -p omnilang --features codegen-flutter combinations_refop_array 2>&1 | tail -15`
Expected: FAIL——生成 Dart 含 verbatim `.filter(...)`（`DataStore.items.filter(((dynamic i) => i > 1)).length`），且 `dart analyze` 报 `List.filter` 不存在（compile exit 0 但断言命中）。

- [ ] **Step 3: 追加失败测试——scrollTo 双轴（longpress 测试在 Task 3.3 Step 1——其 warnings 通道依赖 3.3 实现，避免编译错误型 RED）**

同一文件继续增加：

```rust
#[test]
fn ref_scrollto_dual_axis_warns() {
    // v4 compiler HIGH-2: scrollTo(x, y) 双轴必须产生 [flutter] warning
    // （Flutter codegen 的 warnings 通道已存在——warn_at → CompileOutput）
    let (dart, warnings) = compile_fixture_to_dart_with_warnings("refs/scrollto_dual.omni");
    assert!(
        warnings.iter().any(|w| w.contains("scrollTo") && w.contains("忽略")),
        "双轴 scrollTo 无警告：{:?}",
        warnings
    );
    assert!(dart.contains("jumpTo(200)"), "y 轴仍应生效");
}
```

新增 fixture：`refs/scrollto_dual.omni`（`box.scrollTo(100, 200)`）——若现有 fixture 已覆盖可直接引用。

> 实现注记：辅助函数名（`compile_fixture_to_dart_with_warnings`）以既有测试模块模式为准——`flutter/tests.rs` 有 `compile_flutter_src` 类逐串编译辅助，`generated_code_validation.rs` 有 fixture 枚举模式；warnings 从 `CompileOutput.warnings` 读取（Flutter 通道已存在）。若既有辅助已提供 warnings 返回则直接复用，签名差异以既有为准。

- [ ] **Step 4: 运行验证失败**

Run: `cargo test -p omnilang --features codegen-flutter ref_scrollto_dual_axis_warns 2>&1 | tail -15`
Expected: FAIL（双轴 scrollTo 无警告——`jumpTo(200)` 静默丢弃 x）。

- [ ] **Step 5: Commit**

```bash
git add omnilang/tests/fixtures/ omnilang/tests/generated_code_validation.rs
git commit -m "test(codegen): 组合 fixture——ref-op×数组/scrollTo 双轴/longpress 目标警告（v4 0.3）"
```

### Task 0.4: React 门禁计数基线锁定

**Files:**
- Modify: `scripts/verify-react.sh`（既有豁免台账，锁定计数上限）

- [ ] **Step 1: 记录当前基线（先重新生成 React 输出——生成 src/ 是 gitignored 产物）**

Run: `cd test/projects/learn-omniview && ../../../target/release/omnic build --target react -o build/react-test-app`
Run: `cd build/react-test-app && npx tsc --noEmit 2>&1 | grep -oE "error TS[0-9]+" | sort | uniq -c | sort -rn`
Expected: 记录输出（评审实测基线：TS7006×44、TS2345×11、TS2339×10、TS2554×6、TS2353×6、TS18048×4、TS2322×3、TS2304×2、TS2367×1 = 87；eslint 4）。

- [ ] **Step 2: 将计数写入 verify-react.sh 注释基线（防修复轮漂移）**

在 `scripts/verify-react.sh` 头部添加（既有豁免校验逻辑不动）：

```bash
# v4 基线（2026-08-06 实测）：tsc 87 errors（TS7006×44/TS2345×11/TS2339×10/
# TS2554×6/TS2353×6/TS18048×4/TS2322×3/TS2304×2/TS2367×1）+ eslint 4。
# 修复轮每删除一类豁免须同步此注释；任何类别计数上升 = 门禁失败。
# 目标：Phase 2/3 修复后 tsc ≤ 33（类型发射 + H2/H3/M3 删除 TS2304×2/TS2345×2/TS2554×6）。
```

- [ ] **Step 3: Commit**

```bash
git add scripts/verify-react.sh
git commit -m "docs(gate): React 门禁 v4 基线锁定——87 tsc + 4 eslint 实测计数（v4 0.4）"
```

---

# Phase 1: 注入防线修复（CRITICAL）

> 顺序优先：Task 0.1 语料点已红。**范围裁定后本阶段仅执行 Task 1.2（Flutter `$` 转义回归——G2：生成 Dart 编译失败/错值）**；Task 1.1（React URL 消毒）与 1.3（行为 harness）按范围裁定 defer（#86——React 原生阻断 javascript: 使主向量已关闭）。语料 rawtext-multi 点仍为 1.2 的失败测试。

### Task 1.1: React 动态 URL 运行时消毒（security HIGH-1）

> **范围裁定：defer（触发 #86——真实用户输入 URL 绑定 / Vue·Dioxus 生产使用）**。正文保留完整实现方案（sanitize_url_helper_js + 六属性包裹 + 行为 harness）作触发后执行蓝本；本任务当前**不执行**。React 原生阻断 javascript:/vbscript: 使主向量已关闭——详见范围裁定表。

**Files:**
- Modify: `omnilang/src/codegen/shared/sanitize.rs`（新增 `sanitize_url_helper_js`）
- Modify: `omnilang/src/codegen/react/design.rs` 或 `react/mod.rs`（发射 helper——跟随 `sanitizeCss` 的既有发射点）
- Modify: `omnilang/src/codegen/react/view.rs:319-335`（动态 URL 属性包裹）
- Test: `omnilang/src/codegen/tests_regression/sanitizers.rs`（React 行）

- [ ] **Step 1: 写失败测试——React 动态 URL 载荷不被消毒**

在 `sanitizers.rs` 增加（对照既有 substring 断言模式——本测试先断子串，Task 1.3 升级为行为断言）：

```rust
#[test]
fn react_dynamic_href_wraps_sanitize_url() {
    // v4 security HIGH-1: 动态 URL 属性（href={signal}）必须经运行时
    // sanitizeUrl 包裹——编译期 is_dangerous_url 只覆盖静态字面量。
    let tsx = compile_react_fixture("dynamic_url.omni");
    assert!(
        tsx.contains("sanitizeUrl("),
        "React 输出缺少 sanitizeUrl 包裹：\n{}",
        tsx
    );
    assert!(
        tsx.contains("javascript:") == false || tsx.contains("#blocked"),
        "载荷必须被阻断"
    );
}
```

新增 fixture `dynamic_url.omni`：

```omni
component App {
    signal q: String = "";
    view {
        <input bind:value={q} />
        <a href={q}>Go</a>
    }
}
```

- [ ] **Step 2: 运行验证失败**

Run: `cargo test -p omnilang --features codegen-react react_dynamic_href 2>&1 | tail -10`
Expected: FAIL——生成 `href={q}` 无 sanitizeUrl。

- [ ] **Step 3: 实现——shared/sanitize.rs 新增 URL 消毒 helper 生成器**

在 `omnilang/src/codegen/shared/sanitize.rs` 的 `sanitize_css_helpers_js` 之后添加：

```rust
/// URL 属性集合——动态值需运行时消毒（v4 security HIGH-1）。
/// 与 omniview/view/element.rs:32 的 URL 属性表一致。
pub const URL_ATTRIBUTES: &[&str] = &["href", "src", "srcset", "action", "formaction", "poster"];

/// Generate the runtime URL sanitizer helper as a JS string (v4): dynamic
/// URL attribute values on React must be blocked at runtime — the
/// compile-time `is_dangerous_url` covers only static literals. Mirrors
/// `__omni_sanitizeUrl` (OmniView runtime): trim → lowercase → strip
/// CR/LF/TAB → block javascript:/data:/vbscript: → '#blocked'.
/// - `ts_typed`: annotate types for the React .ts output (same convention
///   as [`sanitize_css_helpers_js`]).
#[must_use]
pub fn sanitize_url_helper_js(ts_typed: bool) -> String {
    let sig = if ts_typed {
        "(value: unknown): string"
    } else {
        "(value)"
    };
    let (fn_kw, var_kw) = if ts_typed { ("function", "const") } else { ("function", "var") };
    format!(
        "{fn_kw} sanitizeUrl{sig} {{\n\
         \x20 if (typeof value !== 'string') return '#blocked';\n\
         \x20 {var_kw} v = value.trim().toLowerCase().replace(/[\\r\\n\\t]/g, '');\n\
         \x20 if (v.startsWith('javascript:') || v.startsWith('data:') || v.startsWith('vbscript:')) return '#blocked';\n\
         \x20 return value;\n\
         }}"
    )
}
```

- [ ] **Step 4: 实现——React 发射 helper + 动态 URL 包裹**

在 React 输出处（`sanitizeCss` 的发射点，`react/design.rs:483-488` 参照）发射一次 helper：

```rust
// v4 security HIGH-1: 动态 URL 属性运行时消毒（与 sanitizeCss 同源生成）。
w.emit_str(&sanitize_url_helper_js(true));
```

在 `react/view.rs` 动态属性发射臂（`:319-323` 的 `_` arm）包裹 URL 属性——将 `attr={expr}` 改为：

```rust
// v4 security HIGH-1: URL 属性动态值经运行时 sanitizeUrl 包裹
// （编译期 is_dangerous_url 只覆盖静态字面量）。
let wrapped = if URL_ATTRIBUTES.contains(&prop_name.as_str()) {
    format!("sanitizeUrl({})", value_expr)
} else {
    value_expr
};
attrs.push(format!("{}={{{}}}", prop_name, wrapped));
```

bindings 臂（`:335`）同法处理——bind 目标名为 URL 属性时同样包裹。

- [ ] **Step 5: 运行验证通过**

Run: `cargo test -p omnilang --features codegen-react react_dynamic_href 2>&1 | tail -5`
Expected: PASS。再跑 React 门禁确认无新豁免需求：`cd react-test-app && npx tsc --noEmit` 计数不增（sanitizeUrl 有类型标注）。

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/shared/sanitize.rs omnilang/src/codegen/react/
git commit -m "fix(codegen-react): 动态 URL 属性运行时消毒——sanitizeUrl helper 发射 + 六 URL 属性包裹（v4 1.1, security HIGH-1）"
```

### Task 1.2: Flutter `$` 转义回归修复（rust HIGH）

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs:332`（TextSpan 片段）
- Modify: `omnilang/src/codegen/flutter/view.rs:2193`（全文本拼接路径）
- Test: Task 0.1 语料点（rawtext-multi）+ 新快照断言

- [ ] **Step 1: 确认失败测试仍红**

Run: `cargo test -p omnilang --features codegen-flutter rawtext_multi 2>&1 | tail -10`
Expected: FAIL（Task 0.1 的 rawtext-multi 断言命中未转义 `$`）。

- [ ] **Step 2: 修复 TextSpan 片段路径（:332）**

将：

```rust
        ViewTree::Text(t) => format!("TextSpan(text: '{}')", t.replace('\'', "\\'")),
```

改为（与 `:222-227` 正确模式一致——`escape_literal_str` 加 `$` 转义）：

```rust
        ViewTree::Text(t) => {
            // v4 rust HIGH: `$` 必须转义——Dart 字符串插值（与文本路径
            // :222-227 同模式；此处原为手写 replace 漏掉 `$`，注入语料
            // rawtext-multi 点捕获）。
            let escaped = escape_literal_str(t, '\'');
            format!("TextSpan(text: '{}')", escaped.replace('$', "\\$"))
        }
```

- [ ] **Step 3: 修复全文本拼接路径（:2193）**

将：

```rust
                ViewTree::Text(t) => parts.push(t.replace('\'', "\\'")),
```

改为：

```rust
                ViewTree::Text(t) => {
                    // v4 rust HIGH: 与 TextSpan 路径同修复——共享转义模式。
                    let escaped = escape_literal_str(t, '\'');
                    parts.push(escaped.replace('$', "\\$"));
                }
```

- [ ] **Step 4: 追加回归快照——`$` 文本编译正确**

在 `flutter/tests.rs`（既有快照断言模式）增加：

```rust
#[test]
fn text_dollar_escaped_in_concat_and_span() {
    // v4 rust HIGH 回归：混合内容段落 `价格 $total 元` 不得产生 Dart 插值。
    let dart = compile_flutter_src(
        "component App { view { <p>价格 $total 元</p> } }",
    );
    assert!(dart.contains("价格 \\$total 元"), "拼接路径未转义 $：\n{dart}");
    let rich = compile_flutter_src(
        "component App { view { <p>价格 $total <b>x</b></p> } }",
    );
    assert!(rich.contains("价格 \\$total"), "TextSpan 路径未转义 $：\n{rich}");
}
```

- [ ] **Step 5: 运行验证通过 + 语料全绿**

Run: `cargo test -p omnilang --features codegen-flutter text_dollar_escaped rawtext_multi 2>&1 | tail -8`
Expected: PASS（两测试 + 语料点全绿）。

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/flutter/view.rs omnilang/src/codegen/flutter/tests.rs
git commit -m "fix(codegen-flutter): \$ 转义回归——TextSpan/拼接两路径改共享转义模式（v4 1.2, rust HIGH）"
```

### Task 1.3: 运行时消毒器行为验证 harness（security MEDIUM-2）

> **范围裁定：defer（随 #86）**——无 Task 1.1 的 sanitizeUrl helper 则无被验证对象；正文保留。若 1.1 触发执行则 1.3 同步执行。

**Files:**
- Create: `omnilang/src/codegen/tests_regression/sanitizer_behavior.rs`（`#[cfg(feature = "codegen-react")]` 门控；node 执行）
- Modify: `omnilang/src/codegen/tests_regression/sanitizers.rs`（陈旧注释修正——LOW-3）

- [ ] **Step 1: 写失败测试——执行发射的 helper 对 23 载荷断言行为**

创建 `sanitizer_behavior.rs`（用 `std::process::Command` 调 `node -e`——CI 已有 node（Playwright））：

```rust
//! v4 security MEDIUM-2: 运行时消毒器行为断言——发射的 JS helper 在 node
//! 中执行，对注入语料的 23 载荷断言阻断行为（子串断言无法发现解码顺序
//! 漂移——历史上 React 副本丢 hex 归一化即此类）。

use std::process::Command;

#[test]
fn react_runtime_sanitize_url_blocks_hex_escape() {
    // 发射 helper（复用 Task 1.1 的生成器），在 node 中执行
    let helper = omnilang::codegen::shared::sanitize::sanitize_url_helper_js(true);
    let payloads = [
        "javascript:alert(1)",
        r"\6a\61\76\61\73\63\72\69\70\74\3a alert(1)", // hex 转义 javascript:
        "java\nscript:alert(1)",
        "data:text/html,<script>alert(1)</script>",
        "vbscript:msgbox(1)",
        "https://example.com/ok", // 合法 URL 必须放行
    ];
    let mut script = format!("{helper}\n");
    for (i, p) in payloads.iter().enumerate() {
        script.push_str(&format!(
            "console.log('{}:' + JSON.stringify(sanitizeUrl({:?})));\n",
            i, p
        ));
    }
    let out = Command::new("node").arg("-e").arg(&script).output().expect("node 不可用");
    let stdout = String::from_utf8_lossy(&out.stdout);
    for (i, p) in payloads.iter().enumerate() {
        let line = stdout.lines().find(|l| l.starts_with(&format!("{i}:"))).expect("缺输出行");
        let val = line.split_once(':').unwrap().1;
        if p.starts_with("https://") {
            assert_eq!(val, format!("{:?}", p), "合法 URL 被误阻断");
        } else {
            assert_eq!(val, "\"#blocked\"", "载荷 {} 未被阻断", p);
        }
    }
}
```

- [ ] **Step 2: 运行验证（先确认 helper 发射点正确）**

Run: `cargo test -p omnilang --features codegen-react react_runtime_sanitize_url_blocks_hex_escape 2>&1 | tail -8`
Expected: PASS（Task 1.1 的 helper 已发射）——本测试锁定行为防未来漂移。若 `node` 不在 PATH，测试内 `Command::new("node")` 失败即跳过并打印说明（CI 有 node）。

- [ ] **Step 3: 修正陈旧注释（security LOW-3）**

`injection.rs:83-89` 的注释声称 `alert(` 不在 `sanitize_css_value` 阻止列表——实为 Task 1.4 已添加。改写为：

```rust
// NOTE (v4): `alert(` 已在 sanitize_css_value 阻止（task 1.4 添加，
// sanitize.rs:118-123）——本载荷在消毒层即被拒绝（空串），非「存活至
// 残留扫描」。注释保留载荷历史语义，安全行为以 sanitize.rs 为准。
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/tests_regression/
git commit -m "test(security): 运行时消毒器行为 harness——node 执行 23 载荷断言（v4 1.3, security MEDIUM-2）"
```

**Phase 1 门禁**: `cargo test -p omnilang --all-features` 全绿 + `verify-react.sh` 计数不增。

---

# Phase 2: React 行为层修复

> Task 0.2 的行为断言本阶段全部转绿。顺序：H1（最重）→ H2（崩溃）→ H3（类型谎言）→ M3（TS2554）。

### Task 2.1: H1 写后读陈旧闭包——写入值线程化

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs`（InlineFn 分支 ~:468 起）
- Modify: `omnilang/src/codegen/react/expr.rs`（next-值发射辅助）
- Test: `omnilang/src/codegen/react/tests.rs`（生成形状断言）

- [ ] **Step 1: 写失败测试——生成代码形状断言**

在 `react/tests.rs` 增加：

```rust
#[test]
fn handler_write_then_read_threads_next_value() {
    // v4 H1: `agreed = !agreed; statusText = if agreed {...}` 必须生成
    // 顺序读取——setX 后读新值（const next + setX(next) + next 续用）。
    let tsx = compile_react_src(
        "component App { signal agreed: Bool = false; signal statusText: String = \"\"; \
         view { <input bind:checked={agreed} @onChange={() => { agreed = !agreed; \
         statusText = if agreed { \"Agreed\" } else { \"Not agreed\" }; }} /> } }",
    );
    assert!(
        tsx.contains("setAgreed(nextAgreed)") && tsx.contains("setStatusText(nextAgreed ?"),
        "写后读未线程化：\n{}",
        tsx
    );
    assert!(
        !tsx.contains("setStatusText(agreed ?"),
        "陈旧闭包仍存在：\n{}",
        tsx
    );
}
```

- [ ] **Step 2: 运行验证失败**

Run: `cargo test -p omnilang --features codegen-react handler_write_then_read 2>&1 | tail -8`
Expected: FAIL（生成 `setAgreed(a => !a); setStatusText(agreed ? ...)`）。

- [ ] **Step 3: 实现——写后读预扫描 + 语句重写**

在 `expr.rs`（或 view.rs 就近）添加预扫描辅助：

```rust
/// v4 H1: 扫描处理器体内「写后读」链——信号被赋值后在同体内被读取，
/// 生成顺序语义（React setX 异步，直接读旧值）。返回每个需线程化的
/// 信号名 → 其局部名（读取处重写为目标局部）。
pub(super) fn collect_write_through_locals(
    statements: &[StatementIR],
) -> FxHashMap<String, String> {
    let mut locals: FxHashMap<String, String> = FxHashMap::default();
    let mut counter: usize = 0;
    for stmt in statements {
        if let StatementIR::MutationStmt {
            intent: Some(intent),
        } = stmt
        {
            // 仅可表达「下值」的 intent 参与线程化（与 mutation_next_expr
            // 的匹配一致）——Push/Pop 等保持原发射路径。
            if matches!(
                intent,
                MutationIntent::DirectAssign { .. }
                    | MutationIntent::Increment { .. }
                    | MutationIntent::Decrement { .. }
            ) {
                if let Some(target) = mutation_target_name(intent) {
                    let local = format!("__omni_next_{}_{}", target, counter);
                    counter += 1;
                    locals.insert(target.clone(), local);
                }
            }
        }
    }
    locals
}
```

在 `view.rs` InlineFn 分支的语句生成处（`:468` 的 `for stmt in &body.statements` 前）插入重写逻辑：

```rust
// v4 H1: 写后读线程化——把 MutationStmt 改写为
// `const __omni_next_x_N = <next值>; setX(__omni_next_x_N);`
// 并重写后续语句中对 x 的读取为 __omni_next_x_N。
let write_through = collect_write_through_locals(&body.statements);
let mut rewritten = body.statements.clone();
for (idx, stmt) in rewritten.iter_mut().enumerate() {
    if let StatementIR::MutationStmt {
        intent: Some(intent),
    } = stmt
    {
        if let Some(target) = mutation_target_name(intent) {
            if let Some(local) = write_through.get(target) {
                // next 值 = mutation 的普通值表达式（DirectAssign 的值 /
                // Increment 的 x + amount / Decrement 的 x - amount）
                let next_expr = mutation_next_expr(intent, target);
                *stmt = StatementIR::LetStmtIR {
                    name: local.clone(),
                    value: Some(next_expr),
                    r#type: None,
                    source_span: None,
                };
                // 原 mutation 改为 setX(local)——经既有 setter 发射路径
                // （emit_mutation_stmt 按 target 找 setter 名）。
                // 后续语句读取重写：在语句列表上做标识符重命名——
                // 仅重写 LOCAL 后续语句，不重写本 mutation 自身 RHS。
                for later in rewritten.iter_mut().skip(idx + 1) {
                    rename_stmt_identifiers(later, target, local);
                }
            }
        }
    }
}
for stmt in &rewritten {
    generate_statement_tsx(&mut merged_hw, stmt, /* 既有参数 */);
}
```

辅助函数（expr.rs，复用既有 `mutation_value_expr` 与 `collect_expr_identifiers` 基础设施）：

```rust
/// v4 H1: mutation 的「下值」表达式——DirectAssign 为值本身；
/// Increment/Decrement 组合二元表达式（x + amount / x - amount）。
/// 必须与 collect_write_through_locals 的 intent 过滤一致（三变体，
/// 其余 intent 永不入 local 表，unreachable 为防御断言）。
fn mutation_next_expr(intent: &MutationIntent, target: &str) -> ExprIR {
    match intent {
        MutationIntent::DirectAssign { value, .. } => (*value).clone(),
        MutationIntent::Increment { amount, .. } => ExprIR::BinaryExpr {
            op: BinaryOpIR::Add,
            left: Box::new(ExprIR::VarRef(target.to_string())),
            right: Box::new(amount.clone()),
        },
        MutationIntent::Decrement { amount, .. } => ExprIR::BinaryExpr {
            op: BinaryOpIR::Sub,
            left: Box::new(ExprIR::VarRef(target.to_string())),
            right: Box::new(amount.clone()),
        },
        _ => unreachable!("write-through 仅处理 DirectAssign/Increment/Decrement"),
    }
}
```

> 实现注记（给执行者的精确锚点）：`mutation_target_name` 在 `expr.rs:1111-1141` 区域已存在（含 field_object 判别）；`LetStmtIR` 字段名以 `ir/types.rs` 为准（name/value/type/source_span——Task 5.7 的 `is_mut` 已加入，构造时 `is_mut: false`）；`rename_stmt_identifiers` 可基于既有 `collect_stmt_identifiers` 的遍历模式实现（替换 VarRef 名匹配 target 的节点）。断言测试锁定生成形状，编译器类型检查兜底字段名漂移。

- [ ] **Step 4: 运行验证通过 + 浏览器行为断言转绿**

Run: `cargo test -p omnilang --features codegen-react handler_write_then_read 2>&1 | tail -5`
Expected: PASS。再跑 Task 0.2 套件（先重新生成 React 输出——生成 src/ 是 gitignored 产物）：
Run: `cd test/projects/learn-omniview && ../../../target/release/omnic build --target react -o build/react-test-app`
Run: `cd build/react-test-app && npx playwright test tests/semantic-behavior.spec.ts`
Expected: PASS（checkbox 状态文字、链接计数、emit 载荷、排序方向四断言全绿）。

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/view.rs omnilang/src/codegen/react/expr.rs omnilang/src/codegen/react/tests.rs
git commit -m "fix(codegen-react): 事件处理器写后读线程化——setX 后读新值（v4 2.1, TS/React H1）"
```

### Task 2.2: H2 只写信号 DCE 一致性（悬空 setter 崩溃）

**Files:**
- Modify: `omnilang/src/codegen/react/component/state.rs`（gen_state_hooks skip 条件 + `[, setX]` 形状）
- Modify: `omnilang/src/codegen/react/component/mod.rs:153-170`（pruned 集合 + has_mutable_states）

- [ ] **Step 1: 写失败测试——只写信号保留 setter**

在 `react/tests.rs` 增加：

```rust
#[test]
fn write_only_signal_keeps_state_hook() {
    // v4 H2: signal variant 从未被读但 fn 内被写——必须保留 useState
    // （`const [, setVariant] = useState(...)`），点击不得 ReferenceError。
    let tsx = compile_react_src(
        "component App { signal variant: String = \"light\"; \
         fn setDark() { variant = \"dark\"; } \
         view { <button @onClick={setDark}>Dark</button> } }",
    );
    assert!(tsx.contains("const [, setVariant] = useState"), "缺 setter hook：\n{tsx}");
    assert!(!tsx.contains("setVariant is not defined"));
}
```

- [ ] **Step 2: 运行验证失败**

Run: `cargo test -p omnilang --features codegen-react write_only_signal 2>&1 | tail -8`
Expected: FAIL（无 useState——`variant` 未被读，skip 分支命中）。

- [ ] **Step 3: 实现——gen_state_hooks skip 条件 + 写专用形状**

`component/state.rs` 的 `gen_state_hooks` 循环头：

```rust
        for state in &comp.signals {
            // v4 H2: 有可达 mutation 的信号必须保留 state hook——
            // mutation 发射（含 fn 体）引用 setter。仅当读与写都缺失才跳过。
            let mutated = mutated_signals.contains(&state.name);
            if !consumed_signals.contains(&state.name) && !mutated {
                continue;
            }
```

mutation 分支发射（在既有 `if mutated_signals.contains(&state.name)` 分支内，区分写专用）：

```rust
            if mutated_signals.contains(&state.name) {
                let setter_base = format!("set{}", capitalize_first(&state.name));
                let setter = if fn_names.contains(&setter_base) {
                    format!("__{}", setter_base)
                } else {
                    setter_base
                };
                let type_ann = typed_state_annotation(&state.r#type, user_types);
                // v4 H2: 写专用信号（值从未被消费）——省略值名（无
                // unused 变量），`const [, setX] = useState(...)`。
                if consumed_signals.contains(&state.name) {
                    if type_ann.is_empty() {
                        w.emit_fmt(format_args!(
                            "const [{}, {}] = useState({});",
                            state.name, setter, init_val
                        ));
                    } else {
                        w.emit_fmt(format_args!(
                            "const [{}, {}] = useState<{}>({});",
                            state.name, setter, type_ann, init_val
                        ));
                    }
                } else {
                    if type_ann.is_empty() {
                        w.emit_fmt(format_args!(
                            "const [, {}] = useState({});",
                            setter, init_val
                        ));
                    } else {
                        w.emit_fmt(format_args!(
                            "const [, {}] = useState<{}>({});",
                            setter, type_ann, init_val
                        ));
                    }
                }
```

`component/mod.rs:153-170` 两处同步：

```rust
        // v4 H2: pruned 集合必须排除「有 mutation 但无读」的信号——它现在
        // 保留 useState（D-V4-6），mutation 不得被发射侧剪枝。
        let pruned_signals: FxHashSet<String> = read_signals
            .iter()
            .filter(|s| {
                mutated_signals.contains(*s)
                    && !value_consumed.contains(*s)
                    && !mutation_call_targets.contains(*s)
            })
            .filter(|s| comp.signals.iter().any(|sig| &sig.name == *s))
            // 修正：写专用信号不在 read_signals 中，天然不在此集合——
            // 无需额外排除。此注释记录决策。
            .cloned()
            .collect();

        // v4 H2: has_mutable_states 必须计入写专用信号（现会发射 useState）。
        let has_mutable_states = comp
            .signals
            .iter()
            .any(|s| mutated_signals.contains(&s.name));
```

> 实现注记：`collect_signal_mutations(comp, &called_fns)`（mod.rs:89）已把 fn 体 mutation 计入 `mutated_signals`——`setDark` 体内的 `variant = "dark"` 已被收集，本任务只修 hook 决策与 pruned 语义的错位。断言测试锁定形状。

- [ ] **Step 4: 运行验证通过 + 浏览器断言**

Run: `cargo test -p omnilang --features codegen-react write_only_signal 2>&1 | tail -5`
Expected: PASS。再跑 Task 0.2 H2 断言（先重新生成）：
Run: `cd test/projects/learn-omniview && ../../../target/release/omnic build --target react -o build/react-test-app`
Run: `cd build/react-test-app && npx playwright test tests/semantic-behavior.spec.ts --grep "只写"`
Expected: PASS（点击不再 ReferenceError）。

- [ ] **Step 5: 删除 TS2304 豁免条目（deferral #72 关闭）**

从 `scripts/verify-react.exemptions` 删除 AdvancedPseudoDemo 的 TS2304 两行，并在台账补 deferral #72 关闭注记（commit 证据本任务）。

- [ ] **Step 6: Commit**

```bash
git add omnilang/src/codegen/react/component/ scripts/verify-react.exemptions .architecture/deferrals.md
git commit -m "fix(codegen-react): 只写信号保留 useState——悬空 setter 崩溃修复 + TS2304 豁免删除（v4 2.2, TS/React H2）"
```

### Task 2.3: H3 数字 bind:value 类型诚实（Number 强制转换）

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs`（bind setter 发射 ~:341-349）
- Modify: `omnilang/src/codegen/react/expr.rs` 或 `view.rs`（is_numeric_type 辅助）
- Test: `omnilang/src/codegen/react/tests.rs`

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn numeric_bind_value_emits_number_coercion() {
    // v4 H3: age: i32 的 bind:value 必须 Number(e.target.value)——裸
    // e.target.value 把字符串喂进 number 状态（age+1 → "301"）。
    let tsx = compile_react_src(
        "component App { signal age: i32 = 0; \
         view { <input type=\"number\" bind:value={age} /> } }",
    );
    assert!(
        tsx.contains("setAge(Number(e.target.value))")
            || tsx.contains("setAge(e.target.valueAsNumber)"),
        "数字 bind:value 无强制转换：\n{}",
        tsx
    );
    // 字符串 signal 保持原样（不破坏既有形状）
    let tsx_str = compile_react_src(
        "component App { signal q: String = \"\"; \
         view { <input bind:value={q} /> } }",
    );
    assert!(tsx_str.contains("setQ(e.target.value)"));
}
```

- [ ] **Step 2: 运行验证失败**

Run: `cargo test -p omnilang --features codegen-react numeric_bind_value 2>&1 | tail -8`
Expected: FAIL（生成裸 `setAge(e.target.value)`）。

- [ ] **Step 3: 实现——信号类型查找 + Number 发射**

在 `view.rs` bind 发射处（`:341-349` 的 `let setter = ...` 前）插入：

```rust
// v4 H3: 数字类型 signal 的写回必须强制转换——裸 e.target.value 是
// 字符串，会污染 number 状态（类型谎言有运行时后果）。
let event_value = if bind_name == "checked" {
    "e.target.checked".to_string()
} else if self.signal_is_numeric(&signal_ref.name) {
    "Number(e.target.value)".to_string()
} else {
    "e.target.value".to_string()
};
```

`signal_is_numeric` 辅助（ReactCodegen 方法，`comp.signals` 类型查找——`TypeAnnotation::Named { name, .. }` 数字名集合，`parser/ast/mod.rs:21-26`）：

```rust
/// v4 H3: 组件信号是否为数字类型（i8-u64/f32/f64/isize/usize）。
fn signal_is_numeric(&self, name: &str) -> bool {
    self.comp.signals.iter().any(|s| {
        s.name == name
            && matches!(
                &s.r#type,
                TypeAnnotation::Named { name, .. }
                    if matches!(
                        name.as_str(),
                        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
                            | "f32" | "f64" | "isize" | "usize"
                    )
            )
    })
}
```

- [ ] **Step 4: 运行验证通过**

Run: `cargo test -p omnilang --features codegen-react numeric_bind_value 2>&1 | tail -5`
Expected: PASS。

- [ ] **Step 5: 删除 TS2345 豁免条目（deferral #64 关闭）+ Commit**

从 `scripts/verify-react.exemptions` 删除 FormControlsDemo 的 TS2345 两行；台账补 #64 关闭注记。

```bash
git add omnilang/src/codegen/react/view.rs omnilang/src/codegen/react/tests.rs scripts/verify-react.exemptions .architecture/deferrals.md
git commit -m "fix(codegen-react): 数字 bind:value Number() 强制转换——类型谎言修复（v4 2.3, TS/React H3）"
```

### Task 2.4: M3 伪 handler({}) 参数（TS2554×6 删除）

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs:415-422`（setter 分支空事件特判）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn bind_value_handler_with_zero_arity_fn_skips_empty_obj() {
    // v4 M3: bind:value + @onChange={handleChange}（0 参 fn）时 setter 分支
    // 不得调 handleChange({})——与无 setter 分支的空事件特判一致（TS2554）。
    let tsx = compile_react_src(
        "component App { signal q: String = \"\"; fn handleChange() {} \
         view { <input bind:value={q} @onChange={handleChange} /> } }",
    );
    assert!(
        !tsx.contains("handleChange({})") && tsx.contains("onChange={(e) => {"),
        "空事件对象仍被构造：\n{}",
        tsx
    );
}
```

- [ ] **Step 2: 运行验证失败**

Run: `cargo test -p omnilang --features codegen-react bind_value_handler_zero_arity 2>&1 | tail -8`
Expected: FAIL（生成 `handleSelect({})` 形状）。

- [ ] **Step 3: 实现——setter 分支应用既有空事件检查**

`view.rs` setter 分支（`:415-422`）——在 `event_data.is_some()` 分支内对齐无 setter 分支的 `event_obj == "{}"` 特判：

```rust
                        if let Some(setter_call) = binding_events_map.remove(&react_event) {
                            // v4 M3: 与无 setter 分支对齐——事件对象为空时
                            // 不构造 `{}`（0 参 fn 直接调用，TS2554）。
                            let (handler_call, event_obj) =
                                if let Some(ed) = event_data {
                                    let obj = emit_event_object_react(ed, used_fields, "e");
                                    if obj == "{}" {
                                        (format!("{call};"), String::new())
                                    } else {
                                        (format!("{call}({obj});"), String::new())
                                    }
                                } else {
                                    (format!("{call}(e);"), String::new())
                                };
                            attrs.push(format!(
                                "{}={{(e) => {{ {}; {} }}}}",
                                react_event, setter_call, handler_call
                            ));
                        }
```

> 实现注记：`call` 变量在既有代码中已计算（含 `?.` 可选调用——Task 5.6/deferral #61）；`event_data` 在既有代码中为 `Option<&EventDataIR>`（InlineFn 分支是 `event_data`，EventHandlerIR 分支是字段）。断言测试锁定生成形状，实现时以当前分支结构为准对齐（两分支同修）。

- [ ] **Step 4: 运行验证通过 + 删除 TS2554 豁免（#62 关闭）**

Run: `cargo test -p omnilang --features codegen-react bind_value_handler_zero_arity 2>&1 | tail -5`
Expected: PASS。从 `scripts/verify-react.exemptions` 删除 TS2554 六行；台账补 #62 关闭注记。

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/view.rs omnilang/src/codegen/react/tests.rs scripts/verify-react.exemptions .architecture/deferrals.md
git commit -m "fix(codegen-react): bind:value setter 分支空事件特判——TS2554×6 删除（v4 2.4, TS/React M3）"
```

**Phase 2 门禁**: `cargo test -p omnilang --features codegen-react` 全绿 + `react-test-app` tsc 计数 ≤ 81（87 − TS2304×2 − TS2345×2 − TS2554×6 对应豁免删除后重测）+ Task 0.2 行为套件全绿。

---

# Phase 3: exit-0 输出损坏族修复

### Task 3.1: validate_array_methods walker 下钻 RefOp 子节点（compiler HIGH-1）

> **范围修订（Task 0.3 评审 2026-08-06，Important 发现）**：逃逸是**通用**的——store 访问重写同样跳过 RefOpExpr 参数（实证：生成 Dart 含 verbatim `DataStore.items` 且无 store import，dart analyze 先报 `Undefined name 'DataStore'` 而非 `List.filter`）。修复范围 = **所有手写 walker 中对 `RefOpExpr` 的空臂/跳过臂统一下钻**（validate_array_methods + store 访问重写所在 pass——以 0.3 实证定位），并在组合 fixture 测试补 `!dart.contains("DataStore.")` 断言（防「测试绿但 fixture 仍不编译」）；顺带修正 fixture 注释（原预测 `List.filter` 不存在，实际首错为 `Undefined name 'DataStore'`——修复后注释描述终态契约）。

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_array_methods.rs:393`（`ExprIR::RefOpExpr { .. } => {}`）
- Modify: store 访问重写 pass（位置以 0.3 实证为准——grep `DataStore` 重写/导入发射处）
- Test: Task 0.3 组合 fixture（已红）+ 新增 store 逃逸断言

- [ ] **Step 1: 确认失败测试仍红**

Run: `cargo test -p omnilang --features codegen-flutter combinations_refop_array 2>&1 | tail -8`
Expected: FAIL（生成 Dart 含 verbatim `.filter(`）。

- [ ] **Step 2: 实现——walker 下钻（镜像 ir/walk.rs::walk_ref_op 变体）**

将 `validate_array_methods.rs:393` 的：

```rust
        ExprIR::RefOpExpr { .. } => {}
```

改为：

```rust
        ExprIR::RefOpExpr { op, .. } => {
            // v4 compiler HIGH-1: 下钻 RefOp 参数——数组方法在 ref-op 参数
            // 内必须被重写（此前空臂使 `box.setSelection(list.filter(...).len(), 5)`
            // 逃逸重写，生成非编译 Dart 且 exit 0）。变体镜像
            // ir/walk.rs::walk_ref_op。
            match op {
                RefOpIR::ScrollTo { x, y } => {
                    if let Some(x) = x {
                        walk_expr_mut(x, ctx);
                    }
                    if let Some(y) = y {
                        walk_expr_mut(y, ctx);
                    }
                }
                RefOpIR::ScrollBy { dx, dy } => {
                    if let Some(dx) = dx {
                        walk_expr_mut(dx, ctx);
                    }
                    if let Some(dy) = dy {
                        walk_expr_mut(dy, ctx);
                    }
                }
                RefOpIR::SetSelection { start, end } => {
                    walk_expr_mut(start, ctx);
                    walk_expr_mut(end, ctx);
                }
                RefOpIR::Focus
                | RefOpIR::Blur
                | RefOpIR::Measure { .. }
                | RefOpIR::Select
                | RefOpIR::Value
                | RefOpIR::CapturePointer
                | RefOpIR::ReleasePointer => {}
            }
        }
```

> 实现注记：`walk_expr_mut` 在本文件的签名是 `fn walk_expr_mut(expr: &mut ExprIR, ctx: &mut ArrayMethodCtx)`——模式匹配处 `op` 绑定为 `&mut RefOpIR`；`SetSelection { start, end }` 字段名以 `ir/types.rs` 的 RefOpIR 定义为准（walk_ref_op 先例确认存在此变体）。若 RefOpIR 尚有其他携带表达式的变体（如 Measure 的维度读），以 `ir/walk.rs` 为准同步（编译器穷尽性检查兜底）。

- [ ] **Step 3: 运行验证通过**

Run: `cargo test -p omnilang --features codegen-flutter combinations_refop_array 2>&1 | tail -8`
Expected: PASS——生成 Dart 为 `.where((...)).length`，dart analyze 绿。

- [ ] **Step 4: 顺带修复同族 walker 分歧（Rust MEDIUM 关联）——在 walker 处加注释防再分歧**

在 `validate_array_methods.rs` walker 头部注释补：

```rust
// v4 纪律：本 walker 必须与 ir/walk.rs 的共享 walk 保持同构——
// 新增 ExprIR 变体或 RefOpIR 参数时两处同步（HIGH-1 教训：
// 私有 walker 与共享 walk 分歧产生组合盲区）。
```

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/validate_array_methods.rs
git commit -m "fix(semantic): validate_array_methods walker 下钻 RefOp 子节点——组合盲区修复（v4 3.1, compiler HIGH-1）"
```

### Task 3.2: Flutter scrollTo(x,y) 双轴警告（compiler HIGH-2）

**Files:**
- Modify: `omnilang/src/codegen/flutter/ref.rs`（`collect_ref_usage` 旁新增双轴扫描）
- Modify: `omnilang/src/codegen/flutter/view.rs` 或 ref.rs（警告发射点）
- Test: Task 0.3 的 `ref_scrollto_dual_axis_warns`（已红）

- [ ] **Step 1: 确认失败测试仍红**

Run: `cargo test -p omnilang --features codegen-flutter ref_scrollto_dual_axis 2>&1 | tail -8`
Expected: FAIL（无警告）。

- [ ] **Step 2: 实现——组件级扫描 + ctx.warn_at**

在 `flutter/ref.rs` 增加扫描（复用 `collect_ref_usage` 的遍历模式——它已走遍所有 handler/effect 体）：

```rust
/// v4 compiler HIGH-2: 双轴 scrollTo(x, y) 静默丢弃 x——设计契约承诺
/// 「双轴差异警告」（设计文档 域 1 映射表）。扫描全部 ref op 调用点，
/// 命中即产生 [flutter] warning（值表达式无需求值——x 恒被忽略）。
pub(super) fn collect_dual_axis_scroll_warnings(
    comp: &ComponentIR,
    out: &mut Vec<(String, Option<Span>)>,
) {
    for stmt in component_all_statements(comp) {
        walk_stmt(stmt, &mut |expr| {
            if let ExprIR::RefOpExpr { op, .. } = expr {
                if let RefOpIR::ScrollTo { x: Some(_), y: Some(_) } = op {
                    out.push((
                        "[flutter] scrollTo(x, y)：Flutter 单轴滚动——x 被忽略（垂直优先，设计契约双轴差异警告）".into(),
                        expr.source_span(),
                    ));
                }
            }
        });
    }
}
```

在 Flutter 组件文件生成的警告区（`warn_at` 可用处——`view.rs:92` 的 `ctx.warn_at`，对照既有 `warn_design_skipped` 的接线）调用：

```rust
for (msg, span) in collect_dual_axis_scroll_warnings(comp, &mut Vec::new()) {
    ctx.warn_at(msg, span);
}
```

> 实现注记：`component_all_statements` 为示意名——实际以 `collect_ref_usage`（ref.rs 既有）的遍历范围为准（它已覆盖 handler fn 体/effect 体/生命周期）；在既有遍历函数内直接加收集分支更简单（零新遍历）。`ExprIR::source_span()` 访问器以 ir/types.rs 为准。`ir::walk::walk_stmt` 与 `ExprIR` 已 pub（`walk.rs` 头部）。

- [ ] **Step 3: 运行验证通过**

Run: `cargo test -p omnilang --features codegen-flutter ref_scrollto_dual_axis 2>&1 | tail -8`
Expected: PASS——警告含「scrollTo」与「忽略」，`jumpTo(200)` 保持。

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/flutter/ref.rs omnilang/src/codegen/flutter/view.rs
git commit -m "fix(codegen-flutter): scrollTo 双轴差异警告——x 丢弃不再静默（v4 3.2, compiler HIGH-2）"
```

### Task 3.3: longpress/hover/toggle 目标映射诚实（compiler HIGH-3 + domain HIGH-1）

**Files:**
- Modify: `omnilang/src/codegen/react/utils.rs:233-287`（`react_prop_for_canonical` Option 化）
- Modify: `omnilang/src/codegen/react/mod.rs`（warnings 通道——对照 omniview/mod.rs:60-62）
- Modify: `omnilang/src/codegen/react/view.rs`（事件发射处——无映射则 warning + 跳过）
- Modify: `omnilang/src/codegen/omniview/view/events.rs`（`map_event` 无映射警告）
- Test: Task 0.3 的 `event_longpress_warns_on_react`（已红）+ 新 omniview 断言

- [ ] **Step 1: 写失败测试——longpress 目标警告（Task 0.3 顺延至此——React warnings 通道在 Step 2 实现）**

在 `generated_code_validation.rs`（或 `react/tests.rs`——以既有测试模式为准）增加：

```rust
#[test]
fn event_longpress_warns_on_react() {
    // v4 compiler HIGH-3: longpress 在 React 目标必须 warning
    // （Flutter 正确映射 onLongPress）——死处理器应被跳过而非发射。
    let (tsx, warnings) = compile_fixture_to_tsx_with_warnings("events/longpress.omni");
    assert!(
        warnings.iter().any(|w| w.contains("longpress")),
        "React longpress 无警告：{:?}",
        warnings
    );
    assert!(!tsx.contains("onLongpress"), "死处理器应被跳过而非发射");
}
```

新增 fixture：`events/longpress.omni`（`<button @onLongpress={...}>`）——若现有 fixture 已覆盖可直接引用。`compile_fixture_to_tsx_with_warnings` 在 Step 2 与 warnings 通道一起添加。

- [ ] **Step 2: 实现——React warnings 通道 + `react_prop_for_canonical` Option 化**

`react/utils.rs`：签名 `fn react_prop_for_canonical(canonical: &str) -> String` → `Option<String>`，兜底臂改为：

```rust
        // v4 domain HIGH-1 / compiler HIGH-3: 无 React 合成事件对应的
        // canonical（longpress/hover/toggle）——None 触发调用方 warning +
        // 跳过死处理器（发射 `onLongpress` 是 React 丢弃的静默死事件）。
        _ => None,
```

其余已映射臂包 `Some(...)`（机械改写）。调用方（view.rs 事件发射处——`:400-435` 区域）改为：

```rust
                    let Some(react_event) = react_prop_for_canonical(event_name) else {
                        // v4: canonical 无 React 映射——warning + 跳过
                        // （Flutter 已正确映射；三端行为一致化）。
                        self.warnings.push(format!(
                            "事件 '{event_name}' 在 React 目标无对应合成事件——处理器已跳过（Flutter 提供 onLongPress 等映射）"
                        ));
                        continue;
                    };
```

- [ ] **Step 3: 实现——React warnings 通道**

`react/mod.rs`（对照 omniview/mod.rs:60-62 模式）：

```rust
    /// v4 3.3: codegen 期诊断警告（无映射事件等）——经 CompileOutput
    /// warnings 通道透出（react 此前无此通道）。
    pub warnings: Vec<String>,
```

在 `generate()` 返回处把 `self.warnings` 并入 `CompileOutput` warnings 字段（`CompileOutput::success(Vec::new())` 处——`mod.rs:39`）。

- [ ] **Step 4: 实现——OmniView 无映射事件警告**

`omniview/view/events.rs` 的 `emit_element_events`（`:32` 的 `for (event_name, handler) in &el.events` 内，`map_event` 调用前）：

```rust
            // v4 domain HIGH-1: hover/toggle/longpress 无 DOM 事件——
            // map_event 直通发射产生永不被触发的死监听器。warning + 跳过。
            if let Some(w) = event_missing_js_mapping(event_name) {
                self.warnings.push(w);
                continue;
            }
```

`event_missing_js_mapping` 辅助（`omniview/utils.rs` 或 `view/events.rs`）：

```rust
/// v4: canonical 事件名在 DOM 无对应事件（hover/toggle/longpress 为
/// Flutter 便捷形状混入 canonical 表——登记册域 1 借词纪律的变体）。
/// 返回警告文本；其余 canonical 返回 None（map_event 正常处理）。
fn event_missing_js_mapping(event_name: &str) -> Option<String> {
    match event_name {
        "hover" | "toggle" | "longpress" => Some(format!(
            "事件 '@on{event_name}' 在 OmniView/DOM 无对应事件——处理器已跳过（Flutter 提供 onLongPress/onHover/onChanged 映射）"
        )),
        _ => None,
    }
}
```

OmniView warnings 通道已在（`omniview/mod.rs:60-62` `pub warnings: Vec<String>`——events.rs 同模块树可访问；对照既有 `scope` 字段接线）。

- [ ] **Step 5: 追加 omniview 断言测试**

在 `omniview/tests_regression/` 或既有 omniview 测试区增加：

```rust
#[test]
fn hover_toggle_warn_and_skip_on_omniview() {
    // v4 domain HIGH-1: hover/toggle 在 OmniView 必须 warning + 不发射
    // 死监听器（此前 addEventListener("toggle") 永不触发）。
    let (js, warnings) = compile_omniview_src_with_warnings(
        "component App { view { <div @onHover={() => {}} @onToggle={() => {}} /> } }",
    );
    assert!(warnings.len() >= 2, "缺警告：{warnings:?}");
    assert!(!js.contains("addEventListener(\"hover\")"), "死监听器仍发射");
    assert!(!js.contains("addEventListener(\"toggle\")"), "死监听器仍发射");
}
```

- [ ] **Step 6: 运行验证通过**

Run: `cargo test -p omnilang --features codegen-react,codegen-omniview event_longpress hover_toggle_warn 2>&1 | tail -8`
Expected: PASS（三测试全绿）。

- [ ] **Step 7: Commit**

```bash
git add omnilang/src/codegen/react/utils.rs omnilang/src/codegen/react/mod.rs omnilang/src/codegen/react/view.rs omnilang/src/codegen/omniview/
git commit -m "fix(codegen): longpress/hover/toggle 目标映射诚实——React/OmniView warning + 跳过死处理器（v4 3.3, compiler HIGH-3 + domain HIGH-1）"
```

**Phase 3 门禁**: `cargo test -p omnilang --features codegen-flutter,codegen-react,codegen-omniview` 全绿 + Task 0.3 组合 fixture 全绿。

---

# Phase 4: 跨目标静默降级收口

### Task 4.1: Vue/Dioxus 语义 floor 诊断（systems HIGH-1）

> **范围裁定：defer（触发 #87——vue/dioxus 真实生产使用）**。正文保留完整方案（E_CODEGEN 门 + IR 扫描 + floor_matrix 行）作触发后执行蓝本；本任务当前**不执行**（非目标后端，S 成本但零用户价值）。

**Files:**
- Modify: `omnilang/src/codegen/mod.rs`（`compile()` 的 vue/dioxus 臂 + `ui_floor_violations` 辅助）
- Test: `omnilang/tests/floor_matrix.rs`（新增 vue/dioxus 行——floor 矩阵既有模式）

- [ ] **Step 1: 写失败测试——motion 源编译到 vue 必须报错**

在 `omnilang/tests/floor_matrix.rs` 增加（对照既有 `every_floor_item_has_machine_verification` 行模式）：

```rust
#[test]
fn vue_floor_rejects_motion_and_ref() {
    // v4 systems HIGH-1: Vue 是 floor 目标——motion/ref/a11y/bind:*
    // 必须 E_CODEGEN 硬错误（此前静默丢弃，ADR-052:94 契约违约）。
    let motion_src = r#"
        component App {
            view {
                <div @motion="fade" />
            }
        }
    "#;
    let out = compile_omni(motion_src, Target::Vue);
    assert!(!out.errors.is_empty(), "motion → vue 无错误：{:?}", out.warnings);
    let err = out.errors[0].message.clone();
    assert!(err.contains("motion"), "错误未点名特性族：{err}");

    let ref_src = r#"
        component App {
            view {
                <input @ref={box} @onClick={() => { box.focus(); }} />
            }
        }
    "#;
    let out2 = compile_omni(ref_src, Target::Vue);
    assert!(!out2.errors.is_empty(), "@ref → vue 无错误");
}
```

- [ ] **Step 2: 运行验证失败**

Run: `cargo test -p omnilang --features codegen-vue vue_floor_rejects 2>&1 | tail -8`
Expected: FAIL（motion/ref 静默编译成功，零错误）。

- [ ] **Step 3: 实现——compile() 门 + IR 扫描**

`omnilang/src/codegen/mod.rs` 的 vue/dioxus 臂（`:127-148` 区域，镜像 DESIGN_BLOCK_ERROR 门）：

```rust
        #[cfg(feature = "codegen-vue")]
        Target::Vue => {
            if has_design_blocks(ir) {
                return CompileOutput::error(DESIGN_BLOCK_ERROR.into());
            }
            if let Some(err) = ui_floor_violations(ir, Target::Vue) {
                return CompileOutput::error(err);
            }
            vue::VueCodegen::default().generate(ir, source_files)
        }
```

dioxus 臂（`:141-148` 区域，完整代码——非「同法」缩写）：

```rust
        #[cfg(feature = "codegen-dioxus")]
        Target::Dioxus => {
            if has_design_blocks(ir) {
                return CompileOutput::error(DESIGN_BLOCK_ERROR.into());
            }
            if let Some(err) = ui_floor_violations(ir, Target::Dioxus) {
                return CompileOutput::error(err);
            }
            dioxus::DioxusCodegen::default().generate(ir, source_files)
        }
```

> 注：dioxus 臂的既有结构（有无 has_design_blocks 门）以当前源码为准——若 dioxus 臂已含 design 门则仅插入 floor 门；`DioxusCodegen::default()` 形态以 `mod.rs` 现有匹配臂为准（编译器类型检查兜底）。

新增辅助（同文件，`has_design_blocks` 旁）：

```rust
/// v4 systems HIGH-1: 非 P0 UI 目标（Vue/Dioxus）的语义 floor 契约——
/// motion 意图/ref 操作/a11y 块/bind:* 在此二目标静默丢弃（ADR-052
/// 「绝不静默丢弃」）。镜像 design-block 门：发现任一特性族即硬错误。
fn ui_floor_violations(ir: &IRProgram, target: Target) -> Option<String> {
    let mut families: Vec<&str> = Vec::new();
    for module in ir.modules.values() {
        for view in module.views.values() {
            walk_ui_floor_view(view, &mut families);
        }
    }
    if families.is_empty() {
        return None;
    }
    Some(format!(
        "语义特性 {} 在 {} 目标不支持（floor 契约，ADR-052）——编译为 React/Flutter/OmniView 或从源码移除该特性。",
        families.join("、"),
        match target {
            Target::Vue => "Vue",
            _ => "Dioxus",
        }
    ))
}

fn walk_ui_floor_view(view: &ViewTree, families: &mut Vec<&str>) {
    for el in walk_view_elements(view) {
        if el.motion_behavior().is_some() {
            push_family(families, "motion");
        }
        if el.a11y.is_some() {
            push_family(families, "a11y");
        }
        if el.ref_binding.is_some() || !el.bindings.is_empty() {
            push_family(families, "@ref / bind:*");
        }
        // handler 体内的 RefOpExpr（ref.focus() 等命令式操作）
        for handler in el.events.values() {
            walk_handler_statements(handler, &mut |stmt| {
                walk_stmt(stmt, &mut |expr| {
                    if matches!(expr, ExprIR::RefOpExpr { .. }) {
                        push_family(families, "ref 操作");
                    }
                });
            });
        }
    }
}

fn push_family(families: &mut Vec<&str>, family: &str) {
    if !families.contains(&family) {
        families.push(family);
    }
}
```

> 实现注记：`walk_view_elements` / `walk_handler_statements` 为示意名——复用 `crates/omnilang-core/src/ir/walk.rs` 的既有 `walk_view_tree`/`walk_stmt`（pub）与 `omnilang/src/codegen/react/component/mod.rs` 的 handler 遍历先例；`el.motion_behavior()` 与 `a11y`/`bindings`/`ref_binding` 字段以 `ir/types.rs` ElementIR 为准。若 `walk_view_tree` 只对 P0 可用，直接在 vue/dioxus 的 `generate()` 入口扫描亦可（两目标 generate 均接收 `ir: &IRProgram`——`vue/mod.rs:30`、`dioxus/mod.rs:26` 已确认）。

- [ ] **Step 4: 运行验证通过**

Run: `cargo test -p omnilang --features codegen-vue vue_floor_rejects 2>&1 | tail -8`
Expected: PASS。再跑既有 vue/dioxus 测试确认无回归（现有 fixture 无这些特性族——`fixtures` 不含 motion/ref/a11y/bind）。

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/mod.rs omnilang/tests/floor_matrix.rs
git commit -m "fix(codegen): Vue/Dioxus 语义 floor 硬诊断——motion/ref/a11y/bind 静默丢弃收口（v4 4.1, systems HIGH-1）"
```

### Task 4.2: OmniView keyFn 对象 key 塌缩（perf HIGH）

> **范围裁定（用户 2026-08-06）：执行编译期 lint 半（G1——误用诊断）；cut 运行时 JSON.stringify 兜底半（React 同样塌缩+警告不兜底——行为补偿 = 过度设计）**。运行时兜底步骤（Step 4 的 keyFn 修改）**不执行**，测试断言相应改为「警告存在即可」（对象 key 的运行时行为对齐 React：塌缩 + 编译期已警告）。

**Files:**
- Modify: `crates/omnilang-core/src/semantic/lints/for_key.rs`（对象类型 key 表达式警告——D-V4-7 编译期侧）
- Modify: `omnilang/src/codegen/omniview/list.rs:166-173`（keyFn 运行时兜底——D-V4-7 运行时侧）
- Test: 两个（lint 单测 + omniview 生成形状断言）

- [ ] **Step 1: 写失败测试**

for_key lint（`crates/omnilang-core/src/semantic/lints/` 既有测试模式）：

```rust
#[test]
fn object_key_expr_warns_with_field_hint() {
    // v4 perf HIGH: `@for item in items` + `key={item}`（对象项）——
    // 用户按既有 lint 建议补 key 恰好踩中 String() 塌缩。新增警告：
    // key 表达式为对象本身时必须用字段访问（item.id）。
    let src = "component App { view { <div @for=\"item in items\" key={item}>...</div> } }";
    let (diags, _) = run_lints_on(src, &["for-key"]);
    assert!(
        diags.iter().any(|d| d.message.contains("对象") && d.message.contains("字段")),
        "缺对象 key 警告：{:?}",
        diags
    );
}
```

> 实现注记：`run_lints_on` 为示意名——以 `lints/` 模块既有测试的调用方式为准（`LintRule` 是 trait 非枚举——`all_rules()` 注册表按名查找：`all_rules().into_iter().find(|r| r.name() == "for-key")`；诊断经 `LintDiagnostic { rule, message, .. }` 结构返回，`rule == "for-key"` 过滤）。

omniview 侧不再断言运行时兜底（范围裁定 cut——React 对对象 key 同样塌缩+警告不兜底；对象 key 的运行时行为对齐 React）。lint 测试为唯一行为断言。

- [ ] **Step 2: 运行验证失败**

Run: `cargo test -p omnilang-core object_key_expr_warns 2>&1 | tail -8`
Expected: FAIL（lint 无对象 key 警告）。

- [ ] **Step 3: 实现——for_key lint 对象 key 警告**

`for_key.rs` 在既有缺失 key 警告（`:94-105`）后追加：

```rust
                // v4 perf HIGH: key 表达式存在但为对象本身（裸循环变量）——
                // OmniView keyFn 的 String() 会把全部项塌缩为
                // "[object Object]"（键控复用/FLIP 形同虚设，列表只渲染
                // 一个克隆）。要求字段访问（item.id）或原始类型。
                if let Some(key_expr) = &for_dir.key_expr {
                    if let Expr::VarRef(name, _) = key_expr {
                        if name == &for_dir.binding_name
                            && infer_item_kind(&for_dir.iterable, typed, consts) == ItemKind::Object
                        {
                            diags.push(LintDiagnostic {
                                rule: "for-key".into(),
                                severity: LintSeverity::Warning,
                                message: "key 表达式是循环对象本身——全部项塌缩为同一 key；请用 item.id 等字段访问（如 key={item.id}）".into(),
                                file: file_path.to_string(),
                                line: for_dir.span.line,
                                col: for_dir.span.col,
                            });
                        }
                    }
                }
```

> 实现注记：`for_dir.binding_name` 字段名以 parser AST（`ForDirective`）为准（`:108-119` 的 `directives.rs` 解析处可见 binding 名解析）；`Expr::VarRef(name, _)` 元组形状以 `parser/ast/mod.rs:45-46` 的 Expr 枚举为准（`VarRef(String, Span)`——本文件既有匹配处可参照）。

- [ ] **Step 4: 运行验证通过**

Run: `cargo test -p omnilang-core object_key_expr_warns 2>&1 | tail -8` + `cargo test -p omnilang-core`（lints 全量）
Expected: PASS 全绿。

> 范围裁定注：keyFn 运行时兜底（原 Step 4）**已 cut**——`omnilang/src/codegen/omniview/list.rs:166-173` 保持 `String(k)` 现状；对象 key 的塌缩行为与 React 一致（React 对对象 key 同样塌缩 + 控制台警告），编译期 lint 警告即成熟语法的终态。`key={item.id}`（字段访问）不受影响——既有 keyed fixture（DataTable）全绿。

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/lints/for_key.rs
git commit -m "fix(lint): for-key 对象类型 key 表达式警告——key={item} 塌缩前移编译期（v4 4.2, perf HIGH 编译期半）"
```

**Phase 4 门禁**: `cargo test -p omnilang --features codegen-vue,codegen-omniview` 全绿 + floor_matrix 全绿。

---

# Phase 5: 台账校准（宣称 vs 事实）

> 治理权威恢复：semantic-universe/设计文档/README/stdlib 与编译器事实一致；verify 脚本锚点重指向 parser 常量。文档任务随任务同 commit（破坏性政策）。

### Task 5.1: semantic-universe 事件域重分类（domain HIGH-1 + compiler MEDIUM-1）

**Files:**
- Modify: `docs/superpowers/specs/semantic-universe.md`（域 3 事件行）
- Modify: `scripts/verify-semantic-universe.py`（事实断言：universe「支持 无（全目标）」行 ⊆ Flutter 回调映射表 + 已知例外）

- [ ] **Step 1: 重分类 hover/toggle（domain HIGH-1 台账侧）**

`semantic-universe.md:154,157` 两行：

```markdown
| hover | 悬停 | 部分 | **Flutter MouseRegion.onHover 交付；React/OmniView 无 DOM 对应——处理器名透传不触发（v4：编译 warning + 跳过死处理器，见代码生成诚实降级）** |
| toggle | 切换 | 部分 | **Flutter onChanged 交付（checkbox 便捷形状）；React/OmniView 无 DOM 对应——处理器名透传不触发（v4：编译 warning + 跳过死处理器）** |
```

- [ ] **Step 2: 增加 Flutter 支持事实断言——verify-semantic-universe.py**

在 `verify-semantic-universe.py` 增加（复用既有 `rust_str_array` 提取辅助——`verify-vocabulary.py:93-98` 已有该模式）：

```python
def flutter_callback_names() -> set[str]:
    """从 Flutter codegen 回调映射表提取实际支持的 canonical 名
    （flutter/utils.rs:70-95 的 callback map 键）——universe「全目标支持」
    行的事实源（v4：台账不再自证）。"""
    # 提取 "canonical" => 模式：callback map 的字符串键
    return rust_str_array("omnilang/src/codegen/flutter/utils.rs", r'"([a-z]+)"\s*=>')

def check_event_claims() -> list[str]:
    """域 3「支持 无（全目标）」行必须出现在 Flutter 回调映射中——
    hover/toggle/longpress 曾宣称全目标支持但 web 两目标静默不触发
    （v4 review domain HIGH-1）。"""
    errors = []
    mapped = flutter_callback_names()
    for row in event_rows():
        if "支持" in row.status and "全目标" in row.diag:
            canonical = row.canonical
            if canonical not in mapped:
                errors.append(
                    f"域3 {canonical}: 宣称「支持 无（全目标）」但 Flutter 回调映射缺失——"
                    f"须改标「部分」或补映射（Flutter utils.rs callback map）"
                )
    return errors
```

在脚本主流程接入（既有断言收集处）——`check_event_claims()` 结果并入最终问题列表，非空即 exit 1。

> 实现注记：`event_rows()` 解析 universe 表格的既有逻辑（脚本已解析域 3 行——`event_canonical_names` 辅助即从 universe 提取）；`rust_str_array` 若在 verify-vocabulary.py 而非本文件，先按 D-V4-9 合并脚本（Task 5.6 顺序在 5.1 后——5.1 可先在本文件内联实现提取，5.6 合并时收敛到共享模块）。

- [ ] **Step 3: 运行验证**

Run: `python scripts/verify-semantic-universe.py`
Expected: 输出未通过行清单（25+ 行宣称全目标但 Flutter 映射缺失——`onScroll` 等）。**本步骤先记录清单不修改**——下一步批量重分类。

- [ ] **Step 4: 批量重分类未映射行**

对 Step 3 清单中每一行（touch*/pointer*/wheel/contextmenu/scroll/drag*/copy/cut/paste/media/animation/transition 族——以脚本输出为准），状态列 `支持` → `部分`，诊断列改为：

```markdown
**Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）**
```

（逐行机械替换，脚本输出为准——不手猜 25 行的确切清单。）

- [ ] **Step 5: 重跑验证通过**

Run: `python scripts/verify-semantic-universe.py`
Expected: 0 problem（脚本清单一空）。

- [ ] **Step 6: Commit**

```bash
git add docs/superpowers/specs/semantic-universe.md scripts/verify-semantic-universe.py
git commit -m "docs(universe): 域3 事件支持状态事实重分类——hover/toggle/longpress 部分化 + Flutter 映射事实断言（v4 5.1, domain HIGH-1 + compiler MEDIUM-1）"
```

### Task 5.2: theme="dark" 文档如实化 + 警告（domain HIGH-2）

> **范围修订（Task 5.2 评审 2026-08-06，Important 发现）**：`is_known_html_tag` 须并入 PascalCase 内置别名（`codegen/shared/html.rs:9-32`——Button→button、Card/Container/Flex→div、Text→span 等 14 个别名；parser 视 PascalCase 为组件调用）——`<Card theme="dark">` 此前逃逸警告（恰是「静默惰性属性」形态；brief 指定的小写镜像清单所致；fail-safe 零语料影响）。修复 = 别名集并入判定（或经 to_html_tag 归一化——以 html.rs 既有辅助为准）+ `<Card theme="dark">` 警告测试 + 已知标签无 theme prop 负向测试。

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_props.rs`（`theme=` 已知 DOM 标签警告）
- Modify: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md:365,374`（诊断表与验证清单如实修订——D-V4-2）

- [ ] **Step 1: 写失败测试——theme= 属性产生警告**

在 `validate_props.rs` 的既有测试区（或 semantic 测试模块）：

```rust
#[test]
fn theme_prop_on_known_tag_warns() {
    // v4 domain HIGH-2: `theme="dark"` 是已删除虚构语法（ADR-056 D1）——
    // 文档宣称「自然解析错误」实为静默惰性属性。已知 DOM 标签上的
    // theme= 属性必须 warning（自定义组件豁免——合法 prop 不误伤）。
    let (diags, _) = analyse_src(
        "component App { view { <div theme=\"dark\" /> } }",
    );
    assert!(
        diags.iter().any(|d| d.severity == Severity::Warning
            && d.message.contains("theme")
            && d.message.contains("@themes")),
        "无 theme= 警告：{:?}",
        diags
    );
}

#[test]
fn theme_prop_on_custom_component_allowed() {
    let (diags, _) = analyse_src(
        "component Modal { prop theme: String = \"\"; view { <div>{theme}</div> } } \
         component App { view { <Modal theme=\"dark\" /> } }",
    );
    assert!(
        !diags.iter().any(|d| d.message.contains("theme")),
        "自定义组件 theme= 被误伤：{:?}",
        diags
    );
}
```

- [ ] **Step 2: 运行验证失败**

Run: `cargo test -p omnilang-core theme_prop_on_known_tag 2>&1 | tail -8`
Expected: FAIL（无警告）。

- [ ] **Step 3: 实现——validate_props.rs 警告**

在 `validate_props.rs` 的逐元素校验处（组件上下文已知——可判定 tag 是否自定义组件）添加：

```rust
        // v4 domain HIGH-2: `theme="dark"` 是 v2 删除的虚构语法（ADR-056
        // D1——主题切换用 @themes + @theme 信号）。语言层无法区分「虚构
        // 语法」与「自定义组件的合法 theme prop」——对已知 DOM 标签发
        // warning（自定义组件豁免）；诊断表已在设计文档如实修订。
        if prop_name == "theme" && is_known_html_tag(&el.tag) {
            diagnostics.push(warning_diag(
                component,
                prop_span,
                format!(
                    "`theme=\"...\"` 是已删除的虚构语法（主题切换请用 @themes 声明 + @theme 信号绑定）——此属性在已知 DOM 标签上无任何效果"
                ),
            ));
        }
```

> 实现注记：`is_known_html_tag` 复用既有已知标签判定（`binding_applies_to_tag` 的标签清单在 validate_semantic_bindings.rs——可提取共享；或本地 `matches!(tag, "div" | "span" | ...)` 同清单）。`prop_span` 以元素 props 遍历处的 span 为准。警告文本含「@themes」便于测试断言。

- [ ] **Step 4: 实现——设计文档如实修订（D-V4-2）**

`2026-08-02-semantic-syntax-v2-design.md:365` 诊断表行：

```markdown
| `theme = "dark"` | 普通 prop 赋值——语言层无法区分虚构语法与自定义组件合法 prop；对已知 DOM 标签发 **warning**（v4 修订：原「自然解析错误」裁决基于错误前提，实测为静默惰性属性；警告文本见 validate_props.rs） |
```

`:374` 验证清单勾选改为：

```markdown
- [x] `theme = "dark"` → warning（已知 DOM 标签）+ 自定义组件豁免 — Task 5.2（v4 修订：原验证声称「error + 迁移提示」不存在，docs-gate 不覆盖设计文档诊断表——已如实修订）
```

- [ ] **Step 5: 运行验证通过 + Commit**

Run: `cargo test -p omnilang-core theme_prop 2>&1 | tail -5`
Expected: PASS。

```bash
git add crates/omnilang-core/src/semantic/validate_props.rs docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md
git commit -m "fix(semantic): theme= 已知 DOM 标签 warning——虚构语法不再静默（v4 5.2, domain HIGH-2）"
```

### Task 5.3: bind:checked 正式化（domain HIGH-3）

**Files:**
- Modify: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`（域 1 BNF + 映射表）
- Modify: `docs/superpowers/specs/semantic-universe.md`（新增 bind 域条目）
- Modify: `docs/superpowers/specs/vocabulary-registry.md`（注记）

- [ ] **Step 1: 设计文档域 1 BNF 补 `bind:checked`（D-V4-3）**

`2026-08-02-semantic-syntax-v2-design.md:102` 的 `value_binding` 产生式后：

```markdown
value_binding     ::= 'bind:value' '=' expr       (* 唯一双向绑定，保留 v1 *)
                    | 'bind:checked' '=' expr     (* 选中态双向绑定——checkbox/radio；v4 正式化：React/OmniView 双向、Flutter warning+跳过 *)
```

映射表补行：

```markdown
| `bind:checked` | 控件选中态与信号双向一致 | React `checked` + `onChange` 回写 `e.target.checked`；OmniView 双向（change 事件 + 守卫）；**Flutter warning「bind:checked → param 'value' not supported on 'TextField'; skipped」** | 部分 |
```

- [ ] **Step 2: semantic-universe 新增条目**

`semantic-universe.md`（值绑定域——域 4 或就近的 bind 语义集表）新增：

```markdown
| bind:checked | 选中态双向绑定 | 部分 | **React/OmniView 双向（onChange/change 回写）；Flutter warning + 跳过（TextField 无 checked 概念）——v4 正式化（此前实现存在但文档宣称「bind:value 唯一双向绑定」）** |
```

- [ ] **Step 3: 登记册注记**

`vocabulary-registry.md` 值绑定域条目补：

```markdown
- `bind:checked`：v4 正式化（domain HIGH-3 三方漂移修复）——语义 = 「选中态双向绑定」；Flutter 为部分支持（warning + skip），非便捷形状借词（checked 是 DOM 属性名，非平台形状）
```

- [ ] **Step 4: 运行验证——docs-gate**

Run: `python scripts/verify-docs.py --check 2>&1 | tail -5`
Expected: 0 failed（新 BNF 为 markdown 文本非 .omni 块——docs-gate 只查 .omni 代码块；若文档含 .omni 示例块需可编译）。

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md docs/superpowers/specs/semantic-universe.md docs/superpowers/specs/vocabulary-registry.md
git commit -m "docs(design): bind:checked 正式化——BNF/全集/登记册三方收敛（v4 5.3, domain HIGH-3）"
```

### Task 5.4: value={sig} 判别器按 @ref + 提示修正（compiler HIGH-4）

**Files:**
- Modify: `crates/omnilang-core/src/parser/view/element.rs:85-125`（`value` 重分类条件）
- Modify: `crates/omnilang-core/src/semantic/validate_semantic_bindings.rs:60-76`（判别器 + 提示文本）
- Modify: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`（域 1 诊断表补行——D-V4-4）

- [ ] **Step 1: 写失败测试——无 @ref 的 value={sig} 是合法单向 prop**

```rust
#[test]
fn value_sig_without_ref_is_one_way_prop() {
    // v4 compiler HIGH-4: 设计契约 D2「单向用普通 value={expr}」——
    // 无 @ref 的 value={sig} 必须作为普通单向 prop 编译（此前被
    // 重分类为已删语义绑定 → 误导性错误）。
    let (diags, _) = analyse_src(
        "component App { signal q: String = \"\"; \
         view { <input value={QueryStore.query} /> } }",
    );
    assert!(
        !diags.iter().any(|d| d.message.contains("value={sig}")),
        "单向 value prop 被误报为已删形式：{:?}",
        diags
    );
}

#[test]
fn value_sig_with_ref_still_errors() {
    // v1 双向形状（value={sig} + @ref）仍必须硬错误 + 迁移提示。
    let (diags, _) = analyse_src(
        "component App { signal q: String = \"\"; \
         view { <input @ref={box} value={q} /> } }",
    );
    assert!(
        diags.iter().any(|d| d.message.contains("bind:value")),
        "v1 双向形状未报错：{:?}",
        diags
    );
}
```

- [ ] **Step 2: 运行验证失败**

Run: `cargo test -p omnilang-core value_sig 2>&1 | tail -8`
Expected: FAIL（第一个测试报 EValueBindingRemoved 误伤）。

- [ ] **Step 3: 实现——parser 重分类条件 + semantic 判别器**

`element.rs:85-125`：`value` 从 `SEMANTIC_BINDINGS` 移除，改为仅在「元素已有 @ref」时重分类——`parse_element_props` 后处理（解析完成后检查 props 是否含 ref）：

```rust
        // v4 compiler HIGH-4: `value` 不再是无条件语义绑定——仅当元素
        // 同时声明 @ref（v1 双向形状）时重分类；否则 value={expr} 是
        // 普通单向 prop（设计契约 D2 的 value_prop 产生式）。
        // SEMANTIC_BINDINGS 中移除 "value"，改在 props 解析完成后判定。
```

在 props 解析收尾处（`parse_element` 完成 props 循环后）：

```rust
        // v4: value={sig} 判别器——带 @ref 的元素上保持 RefBinding
        // （v1 双向形状 → EValueBindingRemoved）；无 @ref 时还原为
        // Static（普通单向 prop）。
        for prop in &mut props {
            if let ElementProp::RefBinding { binding, signal } = prop {
                if binding == "value" && !props.iter().any(|p| matches!(p, ElementProp::Ref { .. })) {
                    *prop = ElementProp::Static {
                        name: binding.clone(),
                        value: signal.clone(),
                    };
                }
            }
        }
```

> 实现注记：`ElementProp::Ref` 变体名与 `@ref` 的解析产物以 parser 为准（`element.rs` 内 `@ref` 解析处可见——若为 `ElementProp::Ref { name }` 形状则 matches 适配；字段名以实际为准）。

`validate_semantic_bindings.rs:60-76`：`value` 分支改为仅当 `el.ref_binding.is_some()` 时触发（parser 已保证无 @ref 的 value 不会到此处——语义层判别器作为第二道防线 + 提示文本修正）：

```rust
    for binding in el.semantic_bindings.keys() {
        if binding == "value" {
            // v4 compiler HIGH-4: 仅 v1 双向形状（value={sig} + @ref）
            // 报错；纯单向 value prop 是合法 surface（D2 value_prop）。
            // 提示文本修正——旧文本建议的「普通 value prop 单向」恰好
            // 是会被误报的形式。
            diagnostics.push(make_diag(
                component,
                el.source_span,
                Severity::Error,
                DiagnosticKind::EValueBindingRemoved,
                format!("'value={{sig}}' 双向绑定形式已删除，{VALUE_BINDING_REMOVED_HINT_V4}"),
                Some(VALUE_BINDING_REMOVED_HINT_V4.to_string()),
            ));
            continue;
        }
```

`VALUE_BINDING_REMOVED_HINT_V4` 常量（同文件顶部）：

```rust
/// v4: 迁移提示修正——旧文本「或普通 value prop 单向」指向不可表达形式。
pub(crate) const VALUE_BINDING_REMOVED_HINT_V4: &str =
    "双向绑定请用 bind:value；单向显示请用 value={getter()}（计算表达式）或 value={computed}";
```

> 实现注记：诊断消息中文 verbatim 政策（ADR-056 决策 1）——消息变更须同步 docs-gate 的 DELETED_SYNTAX 前缀表（`scripts/verify-docs.py:103-125` 的 `DELETED_SYNTAX_CODES` 只按 E 码匹配，若按消息前缀则更新）；docs/omniview 内嵌迁移提示（05-components.md:226-230）同步。

- [ ] **Step 4: 实现——设计文档域 1 诊断表补行**

`2026-08-02-semantic-syntax-v2-design.md` 域 1 诊断表：

```markdown
| `value={sig}`（无 @ref） | 普通单向 prop——合法（D2 value_prop 产生式；v4 修复：此前被误报为已删形式） | 无 |
| `value={sig}`（带 @ref） | v1 双向形状——EValueBindingRemoved + 迁移提示（bind:value / getter 调用 / computed） | error |
```

- [ ] **Step 5: 运行验证通过 + 既有测试回归**

Run: `cargo test -p omnilang-core value_sig 2>&1 | tail -5` 与 `cargo test -p omnilang-core`（既有 value={sig} 相关测试——demo 迁移提示的断言若按旧消息文本匹配则同步更新）
Expected: PASS（若既有测试断言旧消息，属测试跟随行为变更——按测试意图修正而非还原实现）。

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/parser/view/element.rs crates/omnilang-core/src/semantic/validate_semantic_bindings.rs docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md
git commit -m "fix(semantic): value={sig} 判别器按 @ref——单向 prop 恢复合法 + 提示修正（v4 5.4, compiler HIGH-4）"
```

### Task 5.5: docs/README.md + stdlib.md 入口修复（documentation HIGH-1/2）

**Files:**
- Modify: `docs/README.md`（重写陈旧条目——13 章/示例项目/ADR 计数/待补充标记/stdlib 指针）
- Modify: `docs/stdlib.md`（弃用横幅 + omni/* 修正）
- Modify: `docs/design/DESIGN_DOCS_INDEX.md:292`（stdlib 引用修正）
- Modify: `scripts/verify-docs.py`（入口链接断言——Documentation 长期建议 8 提前）

- [ ] **Step 1: 重写 docs/README.md 陈旧条目**

具体修改（对照现状 `docs/README.md:11-51`）：

1. `:11`「OmniView 语法参考（13 章）」→「（16 篇）」；章节表补 14（语法·语言）/15（语法·视图）/16（README 导航）三项；`docs/omniview/README.md` 章节数同步核对（14-grammar-lang / 15-grammar-view / regeneration-contract 为工具开发者文档——README 导航表已含）。
2. `:35` 标准库 API 行：`omni/math`、`omni/string` → 指向 `docs/omniview/12-standard-library.md`（正确 `std/*` 路径），`stdlib.md` 标注「⚠️ 已废弃——见 12-standard-library.md」。
3. `:36-37` 删除「~~语法规范~~（待补充）」与「~~v0.1 → v0.2 迁移指南~~（待补充）」两行的待补充状态：语法规范 → 指向 `docs/omniview/14-grammar-lang.md` + `docs/omniview/15-grammar-view.md`（状态：已发布）；迁移指南 → 指向 `docs/omniview/migration-v1-to-v2.md`（Task 5.5b 创建——本任务 Step 4 验证在 5.5b 完成后重跑，任务顺序 5.5 → 5.5b → 重跑 5.5 Step 4）。
4. `:43`「10 份 ADR」→「65+ 份 ADR（.architecture/decisions/adrs/）」。
5. `:51` 示例项目 → `learn-omniview/`（展示项目——三目标全量验证）+ `test/projects/learn-omniview`（96 个 .omni 语料）；删除不存在的 logic-cli-tool/logic-counter/mixed-blog/view-ecommerce/view-todo 五项。

- [ ] **Step 2: stdlib.md 弃用横幅 + 路径修正**

`docs/stdlib.md:1-5` 头部加：

```markdown
> **⚠️ 已废弃（v4，2026-08-06）**：本文档教授 `omni/*` 导入路径——编译器事实为 `std/<module>`（crates/omnilang-core/src/stdlib/mod.rs）。正确文档：`docs/omniview/12-standard-library.md`（`std/math`、`std/string`、`std/async`、`std/http` 等）。本文件保留至迁移完成。
```

正文 `omni/math` → `std/math`（逐处替换，`stdlib.md:5-46` 区域机械替换）。

- [ ] **Step 3: 入口链接断言——verify-docs.py 扩展**

`scripts/verify-docs.py` 增加（`DOCS_DIR` 扫描旁）：

```python
def check_index_links() -> list[str]:
    """v4 documentation HIGH-1: docs/README.md 引用的相对路径必须存在、
    章节表与目录文件数匹配——入口页纳入强制力体系（此前入口每处陈旧
    均不被门禁捕获）。"""
    errors = []
    readme = (DOCS_DIR.parent / "README.md").read_text(encoding="utf-8")
    for m in re.finditer(r"\]\(([^)#]+)(?:#[^)]*)?\)", readme):
        target = m.group(1)
        if target.startswith(("http", "mailto", "#")):
            continue
        p = DOCS_DIR.parent / target
        if not p.exists():
            errors.append(f"docs/README.md 链接目标不存在: {target}")
    # 章节表文件数 vs 目录实际文件数
    chapter_files = set(
        re.findall(r"omniview/([0-9]{2}-[a-z0-9-]+\.md)", readme)
    )
    actual = {f.name for f in (DOCS_DIR).glob("[0-9][0-9]-*.md")}
    missing = actual - chapter_files
    if missing:
        errors.append(f"README 章节表缺条目: {sorted(missing)}")
    return errors
```

接入主流程（与既有 `DELETED_SYNTAX` 检查并列——非空即失败）。

- [ ] **Step 4: 运行验证（在 Task 5.5b 完成后执行——migration 链接依赖它）**

Run: `python scripts/verify-docs.py --check 2>&1 | tail -8`
Expected: 0 failed + 0 index 错误（migration-v1-to-v2.md 已由 5.5b 创建，入口链接断言全绿）。

- [ ] **Step 5: Commit**

```bash
git add docs/README.md docs/stdlib.md docs/design/DESIGN_DOCS_INDEX.md scripts/verify-docs.py
git commit -m "docs(entry): README/stdlib 入口修复 + 链接断言入门禁（v4 5.5, documentation HIGH-1/2）"
```

### Task 5.5b: 迁移指南 migration-v1-to-v2.md（documentation MEDIUM-3——任务顺序 5.5 → 5.5b → 重跑 5.5 Step 4 验证）

**Files:**
- Create: `docs/omniview/migration-v1-to-v2.md`

- [ ] **Step 1: 以 cheatsheet §3 删除表为骨架编写迁移指南**

`docs/omniview/migration-v1-to-v2.md`（内容以 `00-cheatsheet.md:264-287` 的 16 项删除语法表展开——每项含「v1 写法 → v2 写法」对照 + 诊断码索引 + 迁移提示 verbatim）：

```markdown
# v1 → v2 迁移指南

> 语义级 DSL v2 破坏性变更清单——每条删除语法产生硬诊断 + 迁移提示（「诊断即文档」）。
> 完整删除表见 [00-cheatsheet.md §3](00-cheatsheet.md)；语法规范见 14/15-grammar。
> 迁移状态跟踪：`docs/superpowers/specs/semantic-universe.md`（实现状态列）。

## 删除语法总表（16 项）

| 已删除语法 | v1 写法 | v2 写法 | 诊断码 |
|-----------|---------|---------|--------|
| 值绑定收敛 | `value={sig}`（双向） | `bind:value={sig}`（双向）/ `value={expr}`（单向 prop，v4 恢复） | EValueBindingRemoved |
| 命令式值写 | `ref.value = x` / `ref.setValue(x)` | 信号 + 样式/值绑定 | E_REF_VALUE_WRITE（validate_ref_ops） |
| auto-bind | `@auto-bind` | 显式 `bind:value` | E_AUTOBIND_REMOVED |
| 主题赋值 | `theme = "dark"` | `@themes` 声明 + `@theme` 信号 | warning（已知 DOM 标签，v4） |
| @native | `@native(...)` | 自然解析错误（虚构语法零用户） | E0001 级 |
| 暂停动画 | `@paused` | `@if` 条件渲染 | E_PAUSED_REMOVED |
| typed slots | `@slot name: Type` | `@slot name`（父传内容） | E_SLOT_REMOVED |
| …（其余 9 项按 cheatsheet §3 展开） | | | |

## 迁移路径

1. 编译项目 → 收集全部 E_*_REMOVED 诊断（每诊断自带迁移提示）
2. 按上表逐项替换
3. 重编译 → 0 错误
4. 浏览器验证（三目标）
```

- [ ] **Step 2: 接入 README 导航 + docs-gate 覆盖**

`docs/README.md` 迁移指南行指向本文件（Task 5.5 的占位回填）；`docs/omniview/README.md` 导航表加一行。docs-gate 自动覆盖（`DOCS_DIR` 含 docs/omniview——新增文件即入门禁）。

- [ ] **Step 3: 验证 + Commit**

Run: `python scripts/verify-docs.py --check 2>&1 | tail -5`
Expected: 0 failed（指南中 .omni 示例块必须可编译——fragment 形态用代码围栏标注「片段」）。

```bash
git add docs/omniview/migration-v1-to-v2.md docs/README.md docs/omniview/README.md
git commit -m "docs(migration): v1→v2 迁移指南——删除语法对照表 + 诊断码索引（v4 5.5b, documentation MEDIUM-3）"
```

### Task 5.6: verify 脚本锚点重指向 + 合并（pragmatic MEDIUM-2 + systems MEDIUM-3 + domain MEDIUM-2）

**Files:**
- Modify: `scripts/verify-vocabulary.py`
- Modify: `scripts/verify-semantic-universe.py`
- Modify: `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md`（头部 machine-pinned 注记）

- [ ] **Step 1: 锚点重指向——BNF 地面真相换 parser 常量（D-V4-9）**

两脚本中：

```python
BNF_SPEC_FILE = ...  # 原指向 Superseded v1 spec —— v4 删除此常量
```

改为从 parser 常量提取（`verify-vocabulary.py` 的 `rust_str_array` 辅助已存在——`semantic_anim_prop`/`token_type` 两个 production 的 parser 常量）：

```python
def anim_props_from_parser() -> set[str]:
    """SEMANTIC_ANIM_PROPS（parser/top_level/motion.rs:29-49）——motion
    白名单的编译器事实源（v4：原从 Superseded BNF production 提取，
    三文档可自洽同步漂移——systems MEDIUM-3）。"""
    return rust_str_array(
        "crates/omnilang-core/src/parser/top_level/motion.rs",
        r'"([a-z-]+)"',
    )

def token_types_from_parser() -> set[str]:
    """token_type 白名单（parser/design.rs 的 token 类型表）——同 v4 重指向。"""
    return rust_str_array(
        "crates/omnilang-core/src/parser/design.rs",
        r'TokenType::([A-Za-z]+)',
    )
```

脚本中 motion/token-type 的交叉断言改为以上两函数（BNF production 保留为「文档 ⊆ 代码」单向检查——BNF 提取仍可存在但不再作为事实源）。

- [ ] **Step 2: 脚本合并——共享提取模块（D-V4-9）**

创建 `scripts/spec_whitelists.py`：

```python
"""verify-vocabulary.py / verify-semantic-universe.py 的共享提取逻辑
（v4 pragmatic MEDIUM-2：原两脚本 5 个辅助函数逐字重复 ~120 行）。"""

import re
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

def read_file(path: str) -> str:
    return (REPO / path).read_text(encoding="utf-8")

def rust_str_array(path: str, pattern: str) -> set[str]:
    text = read_file(path)
    return set(re.findall(pattern, text))

def event_canonical_names() -> set[str]:
    # 编译器事实源：crates/omnilang-core/src/codegen/shared/event_names.rs
    return rust_str_array(
        "crates/omnilang-core/src/codegen/shared/event_names.rs", r'"([a-z]+)"'
    )

def refop_variants() -> set[str]:
    return rust_str_array(
        "crates/omnilang-core/src/ir/types.rs", r"RefOpIR::([A-Za-z]+)"
    )
```

两脚本 import 共享模块，删除本地重复函数（`:event_canonical_names`/`:refop_variants`/`:bnf_production_words`/`:first_column_rows`/`:read_file` 五处重复）。

- [ ] **Step 3: v1 spec 头部 machine-pinned 注记**

`2026-07-31-semantic-level-dsl-syntax-spec.md:1-7` 头部补：

```markdown
> **⚠️ machine-pinned（v4 注记）**：本文件仍被 verify-vocabulary.py /
> verify-semantic-universe.py 的「文档 ⊆ 代码」单向检查引用——**不得删除**。
> 事实断言已重指向 parser 常量（SEMANTIC_ANIM_PROPS / event_names.rs /
> RefOpIR——见 scripts/spec_whitelists.py）；本文件仅作 BNF 教学参考。
```

- [ ] **Step 4: 运行验证**

Run: `python scripts/verify-vocabulary.py && python scripts/verify-semantic-universe.py`
Expected: 0 problem（行为等价——锚点迁移不改断言结果；motion/token 集以 parser 常量为准，若有差异以 parser 为准修正文档）。

- [ ] **Step 5: Commit**

```bash
git add scripts/spec_whitelists.py scripts/verify-vocabulary.py scripts/verify-semantic-universe.py docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md
git commit -m "refactor(gates): verify 脚本锚点重指向 parser 常量 + 共享提取合并（v4 5.6, pragmatic MEDIUM-2 + systems MEDIUM-3）"
```

### Task 5.7: 台账关闭纪律（implementation strategist MEDIUMs）

**Files:**
- Modify: `.architecture/deferrals.md`（#43/#61/#69/#70 关闭行 + #42 触发重写）
- Modify: `.architecture/decisions/adrs/ADR-056-semantic-dsl-v2-implementation.md`（Negative 节改写非注记）

- [ ] **Step 1: deferral 台账补关闭行**

`.architecture/deferrals.md` 2026-08-05 节后追加 2026-08-06 节：

```markdown
## 2026-08-06（v4 修复轮收口）

- **#43 unused-import 假阳性 123 条**：已修复（e454f907，08-04——123→6 全真实）——台账补关（此前缺关闭注记，v4 review implementation-strategist MEDIUM）
- **#61 TS2722 可选调用**：已修复（Task 5.6/deferral #61——`?.` 可选调用发射，react view.rs）——豁免清单 REMOVED 注记已删
- **#69 ES5-var**：已修复（2.11 var→let 批次）——豁免 REMOVED 注记已删
- **#70 unused-identifiers**：已修复（2.11 标识符清理批次）——豁免 REMOVED 注记已删
- **#62 TS2554**：已关闭（v4 Task 2.4——bind:value setter 分支空事件特判）
- **#64 TS2345**：已关闭（数字 bind:value 子类——v4 Task 2.3 Number() 强制转换，c5a41da1；**其余子类保持 open——豁免条目 5 条不动**）
- **#72 TS2304**：已关闭（v4 Task 2.2——只写信号保留 useState）
- **#42 parser 分歧**：触发重写（原「任一分歧致用户困惑」不可证伪）→「任一分歧被用户或评审报告」；7 项分歧批次折叠为单一对齐任务（对齐排期随 v4 台账节）
- **新增 deferral**：
  - **#84 velocity_max**（v4 Task 5.8 移除）——触发：「demo 或用户需要 fling 速度物理」；git 历史保留完整实现；syntax-lifecycle 门禁为自然重入路径
  - **#85 a11y 动态可访问名称**（domain MEDIUM-1）——label/role 当前限字符串字面量；触发：「真实应用需要 i18n 动态 label」
  - **#86 React 动态 URL 运行时消毒**（security HIGH-1，用户 2026-08-06 范围裁定 defer）——React 原生阻断 javascript:/vbscript:（ReactDOM isJavaScriptProtocol），主向量已关闭；残余面 = data: URL + 非 React 目标。**触发：「真实用户输入流进 URL 绑定」或「Vue/Dioxus 生产使用」（届时按 Task 1.1/1.3 正文执行，含 sanitizeUrl helper + node 行为 harness）**。ADR-056 语料计数更新仍按 Task 0.1 执行
  - **#87 Vue/Dioxus 语义 floor 诊断**（systems HIGH-1，用户范围裁定 defer）——非目标后端；ADR-052:94 契约违约已记录（本行）。**触发：「vue/dioxus 真实生产使用」**——届时按 Task 4.1 正文执行（E_CODEGEN 门 + floor_matrix 行）；触发前不投资
```

- [ ] **Step 2: ADR-056 Negative 节改写（非注记）**

`ADR-056:84-88` Negative 节——在条目后追加关闭证据（保留历史但状态明确）：

```markdown
### Negative（v4 更新——原条目状态）

* **门禁豁免体系是已知债**：~~React 166 tsc + 121 eslint 全豁免（18 条），BUG-A（let-mut→const）导致 UI/Language 标签页运行时卸载~~——BUG-A 已修复（v3 D-I，db68d812）；豁免燃尽至 87 tsc + 4 eslint（2026-08-06 实测基线，v4 修复轮继续按根因删除，见 Task 0.4）
* **浏览器套件未接入 CI（#60）**：~~44 测试当前为手动终验运行~~——已入 CI（omniview-e2e job，a0b11b63）
* **诊断消息中文政策**：保留（用户群体中文优先；可逆性记录 + 测试约定见 v4 review maintainability MEDIUM-4）
* **deferrals 扩展（#36-#60）**：~~含 T7.1 浏览器暴露 bug 群~~——#36/#37/#49/#50/#55/#60 已关闭（08-04/08-05，证据见 .architecture/deferrals.md）
```

- [ ] **Step 3: Commit**

```bash
git add .architecture/deferrals.md .architecture/decisions/adrs/ADR-056-semantic-dsl-v2-implementation.md
git commit -m "docs(governance): deferral 台账补关 + ADR-056 Negative 节改写——信息新鲜度修复（v4 5.7, implementation-strategist MEDIUM）"
```

### Task 5.8: velocity_max 移除（pragmatic MEDIUM-1，D-V4-8）

**Files:**
- Modify: `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md:287`（BNF）
- Modify: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md:191`（设计文档）
- Modify: `crates/omnilang-core/src/parser/ast/motion.rs`、`parser` 指令解析、`crates/omnilang-core/src/ir/types_motion.rs`、`crates/omnilang-core/src/ir/lower/view.rs:918-944`、`crates/omnilang-core/src/ir/optimize.rs:150-157`
- Modify: `omnilang/src/codegen/react/component/store_slices.rs:859-945`（deliberately-NOT-collected 排除删除）
- Modify: `omnilang/src/codegen/flutter/motion_widgets.rs`、`omnilang/src/codegen/omniview/motion_triggers.rs`
- Modify: `.architecture/deferrals.md`（#84 注册——Task 5.7 已含）

- [ ] **Step 1: 确认零消费者（移除前证据）**

Run: `grep -rn "velocity_max" docs/ learn-omniview/ test/projects/ --include="*.omni" --include="*.md" | grep -v "2026-07-31-semantic-level-dsl-syntax-spec\|2026-08-02-semantic-syntax-v2-design" | head`
Expected: 零命中（BNF/设计文档外无消费者——demo 语料零使用）。

- [ ] **Step 2: 写失败测试——velocity_max 产生硬诊断**

在 parser 测试（`crates/omnilang-core/src/parser/` 测试模块）：

```rust
#[test]
fn velocity_max_is_removed_syntax() {
    // v4 pragmatic MEDIUM-1: velocity_max 零消费者全链路实现 + React
    // slice 特例排除——移除后书写它必须产生「未知 drag 参数」诊断
    // （迁移提示指向删除事实）。
    let result = parse_src(
        "motion drag { velocity_max: 100 }",
    );
    assert!(result.is_err() || result.has_warning_containing("velocity_max"));
}
```

- [ ] **Step 3: 逐层删除（机械，~11 文件）**

顺序：BNF spec → 设计文档 → parser AST/指令解析（`velocity_max` 字段与解析臂）→ `ir/types_motion.rs`（DragHorizontal/DragVertical 的 `velocity_max: Option<...>` 字段）→ `ir/lower/view.rs:918-944`（字段填充）→ `ir/optimize.rs:150-157`（引用的 dead 值传播臂）→ 三后端引用点 → **`store_slices.rs:859-945` 的「deliberately NOT collected」注释块与排除逻辑删除**（D-V4-8 的奖励点——陷阱清除）。

每层删除后编译：

Run: `cargo check -p omnilang-core -p omnilang --all-features 2>&1 | tail -5`
Expected: 0 errors（编译器穷尽性迫使全部引用点同步——这正是 velocity_max 触点多的明证）。

- [ ] **Step 4: 运行验证**

Run: `cargo test -p omnilang-core velocity_max 2>&1 | tail -5` + `cargo test -p omnilang --all-features motion 2>&1 | tail -5`
Expected: PASS（既有 motion 测试不含 velocity_max；若 fixture 含则同步删除）。

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/parser/ crates/omnilang-core/src/ir/ omnilang/src/codegen/ docs/superpowers/specs/
git commit -m "refactor(motion): velocity_max 移除——零消费者全链路 + React slice 特例清除（v4 5.8, pragmatic MEDIUM-1）"
```

**Phase 5 门禁**: verify 双脚本 0 problem + docs-gate 0 failed + `cargo test -p omnilang-core` 全绿 + React 门禁计数不增。

---

# Phase 6: 终验（8.1 验收标准）

### Task 6.1: 全量门禁跑批

**Files:** 无（运行验证）

- [ ] **Step 1: workspace 全量测试（项目门禁约定——`--all-features` 存在跨后端冲突，按 per-backend 分批）**

Run: `cargo test --workspace 2>&1 | tail -25`（默认特性：omnilang-core 全量 + omnilang 基础）
Run: `cargo test -p omnilang --features codegen-react 2>&1 | tail -10`
Run: `cargo test -p omnilang --features codegen-flutter 2>&1 | tail -10`
Run: `cargo test -p omnilang --features codegen-omniview 2>&1 | tail -10`
Run: `cargo test -p omnilang --features codegen-vue 2>&1 | tail -10`
Expected: 0 failed（含新增 0.1/0.2/0.3/2.x/3.x/4.x/5.x 全部测试；per-backend feature 矩阵为项目既有门禁——`all-backends 规避`约定）。

- [ ] **Step 2: 三目标门禁**

Run: `bash scripts/verify-react.sh`（tsc ≤ 81 基线 + eslint ≤ 4 + 豁免计数——脚本内部以重新生成的 build 输出为对象）
Run: `cargo test -p omnilang --features codegen-flutter generated_code_validation 2>&1 | tail -5`（Flutter 全输出 dart analyze——warnings 计数 ≤ 671 单调契约，**含 2026-08-06 受支持例外：671→673（Task 3.2 的双轴警告是修复的设计产物——此前静默的双轴 scrollTo 调用现产生警告，属 honest-degradation 上移，先例镜像 ADR-066；注记见 generated_code_validation.rs 基线行）**）
Run: `python scripts/verify-docs.py --check`（0 failed + 入口链接断言 0 错误）
Run: `python scripts/verify-vocabulary.py && python scripts/verify-semantic-universe.py`（0 problem）
Run: `cd test/projects/learn-omniview/build/react-test-app && npx playwright test`（全量浏览器套件——含 Task 0.2 行为断言 + 既有 17 测试）

- [ ] **Step 3: 浏览器行为套件全绿确认**

Run: `cd test/projects/learn-omniview/build/react-test-app && npx playwright test tests/semantic-behavior.spec.ts`
Expected: 5/5 PASS（H1 四断言 + H2 崩溃断言——Task 0.2 全转绿）。

- [ ] **Step 4: Commit（终验记录）**

```bash
git add -A
git commit -m "test(verify): v4 终验——全量门禁记录（v4 6.1）"
```

### Task 6.2: 验收标准勾选 + ADR-067 记录

**Files:**
- Create: `.architecture/decisions/adrs/ADR-067-semantic-dsl-v4-behavior-injection-fixes.md`

- [ ] **Step 1: 对照评审 Critical 项逐项勾选**

在 ADR-067 中记录验收表（对照 `.architecture/reviews/feature-semantic-dsl-v2-syntax-review.md` 的 Critical 5 项）：

```markdown
| 评审发现 | 处置 | 证据 |
|---------|------|------|
| security HIGH-1（React 动态 URL XSS） | 修复（Task 1.1 + 1.3 行为 harness） | sanitizeUrl helper + 六属性包裹 + node 行为断言 |
| rust HIGH（Flutter `$` 转义回归） | 修复（Task 1.2） | rawtext-multi 语料点 + 双路径快照 |
| TS/React H1（写后读陈旧闭包） | 修复（Task 2.1） | 浏览器行为断言 4/4 |
| TS/React H2（悬空 setter 崩溃） | 修复（Task 2.2） | 行为断言 + TS2304 豁免删除 |
| TS/React H3（数字 bind:value） | 修复（Task 2.3） | Number() 形状断言 + TS2345 豁免删除 |
| TS/React M3（伪 handler({})） | 修复（Task 2.4） | TS2554×6 豁免删除 |
| compiler HIGH-1（ref-op×数组 walker） | 修复（Task 3.1） | 组合 fixture dart analyze 绿 |
| compiler HIGH-2（scrollTo 双轴） | 修复（Task 3.2） | 警告断言 |
| compiler HIGH-3 + domain HIGH-1（longpress/hover/toggle） | 修复（Task 3.3）+ 台账（5.1） | 双后端警告断言 + universe 部分化 |
| systems HIGH-1（Vue/Dioxus floor） | 修复（Task 4.1） | floor_matrix 行 + E_CODEGEN |
| perf HIGH（keyFn 对象塌缩） | 修复（Task 4.2） | lint 警告 + 运行时兜底断言 |
| domain HIGH-2（theme="dark"） | warning + 文档修订（Task 5.2） | validate_props 警告测试 |
| domain HIGH-3（bind:checked） | 正式化（Task 5.3） | 三方文档收敛 + docs-gate |
| compiler HIGH-4（value={sig}） | 判别器修复（Task 5.4） | 双向测试 |
| documentation HIGH-1/2（入口） | 修复（Task 5.5/5.5b） | 入口链接断言 |
| pragmatic MEDIUM-1（velocity_max） | 移除（Task 5.8） | 零消费者 + 诊断测试 |
| pragmatic MEDIUM-2（脚本重复/锚点） | 合并 + 重指向（Task 5.6） | spec_whitelists.py |
| implementation-strategist MEDIUM（台账） | 关闭纪律（Task 5.7） | 台账补行 + ADR-056 改写 |
```

- [ ] **Step 2: 记录未处置项（四选一裁决）**

```markdown
- **defer-with-trigger**：#84 velocity_max（fling 物理需求）、#85 a11y 动态名称（i18n 需求）、Vue/Dioxus 动态 URL 消毒镜像（安全 HIGH-1 的 vue/dioxus 半侧——React 先行，触发「vue/dioxus 生产使用」时镜像）
- **已记录缺口**（未修复，评审 MEDIUM 级，留 v5）：TS/React M1/M6 类型发射（54/87 豁免——本计划 0.4 基线已锁定，v5 处置）、M2 useDrag hook、M4 getters selector、半增量编译、suggestion 字段政策（AI 工具化）、finish_pipeline 编排合并、token IR 类型化
```

- [ ] **Step 3: Commit**

```bash
git add .architecture/decisions/adrs/ADR-067-semantic-dsl-v4-behavior-injection-fixes.md
git commit -m "docs(adr): ADR-067 v4 行为/注入修复轮——18 项处置 + 验收表（v4 6.2）"
```

---

## 验收标准（终态，Task 6.1/6.2 逐项勾选）

1. **注入面**：`$` 载荷（含多子节点形态）与 `javascript:` 动态 URL 在全部语料/行为断言中被阻断；运行时消毒器行为 harness 入测试（1.3）
2. **React 行为语义**：浏览器行为断言 5/5 绿（H1 四断言 + H2 崩溃断言）；tsc 计数 ≤ 81 且每类豁免删除有根因修复（0.4 基线）
3. **exit-0 族**：组合 fixture（ref-op×数组）生成 Dart 可编译；scrollTo 双轴有警告；longpress/hover/toggle 在 React/OmniView warning + 跳过，Flutter 正常
4. **跨目标契约**：Vue/Dioxus 对 motion/ref/a11y/bind 产生 E_CODEGEN；OmniView keyFn 对象 key 不塌缩（lint 警告 + 运行时兜底）
5. **台账诚实**：semantic-universe 域 3 全行与 Flutter 事实一致（脚本断言 0 problem）；theme="dark"/bind:checked/value={sig} 文档与行为一致；README/stdlib 入口真实；verify 脚本锚点 = parser 常量
6. **治理新鲜度**：deferral 台账无「已修复仍开行」；ADR-056 Negative 节状态真实；velocity_max 零引用
7. **全量绿**：`cargo test --workspace --all-features` 0 failed；verify-react/dart analyze/docs-gate/verify 双脚本全绿；浏览器套件全绿

**Pragmatic 复审预期**：velocity_max 移除 + 脚本合并后 Ratio 0.86 → ≤0.75（评审 Success Metric 7）。

## 执行顺序与阶段门

```
Phase 0（测试基建）→ Phase 1（注入）→ Phase 2（React 行为）→ Phase 3（exit-0 族）
→ Phase 4（跨目标）→ Phase 5（台账）→ Phase 6（终验）
```

每阶段结束：全量门禁跑批 + 零「open-undispositioned」发现（四选一裁决）。阶段内任务顺序如上——TDD（失败测试 → 实现 → 转绿 → commit）。若某任务暴露评审未预见的新缺陷：先注册 deferral（带触发条件）或就近修复（不扩散），并在进度记录中标注。
