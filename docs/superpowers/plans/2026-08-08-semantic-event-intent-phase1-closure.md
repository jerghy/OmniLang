# 阶段 1 收口：红门修复 + 编辑器事件意图化同步

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 阶段 1（事件意图化，0cfd6acf..762be2b1，ADR-075）收口——两红门归零（docs-gate 3 FAIL / verify-vocabulary 1 problem，2026-08-08 实测）、vocabulary-registry 域 1 与新语义事件模型对齐、poiesis 编辑器事件意图化四链同步 + TS→Rust 门禁（Systems Architect 评审 Task 10 性质）。

**Architecture:** 阶段 1 代码层已收口（workspace 2000 passed / clippy EXIT=0 / verify-semantic-universe 0 problems / learn-omniview 零旧名）。红的是：① docs-gate（`verify-docs.py` 3 个 complete 块教 @onClick → E0006）；② `verify-vocabulary.py:98` 全文件正则误捕 `legacy_spelling` 臂。编辑器侧四链漂移（parser-view 读侧归一化 / source-emitter 发侧原样 / EventsSection 分组旧名 / componentProtocol 事件清单）会让编辑器往返旧项目发射硬错误源码；且无 TS→Rust 门禁（ADR-005 双解析器漂移永久漏洞）。**本计划零 Rust 语义改动**——只动 python 脚本 / docs / 编辑器 TS / wasm 产物追踪。

**Tech Stack:** Python（verify 脚本）、Markdown（docs/registry）、TypeScript（poiesis 编辑器 + vitest）。

## Global Constraints

- **compiler is ground truth**：修复只动脚本/文档/编辑器 TS，绝不动 Rust 语义（E0006 消息、canonical 表、SEMANTIC_EVENT_NAMES 均为锚点）
- 诊断消息中文 verbatim 不动
- 门禁目标：`verify-docs.py --check` 0 failed（用**新鲜** omnic——跑前比对 mtime，见 Task 2）、`verify-vocabulary.py --check` 0 problems、`verify-semantic-universe.py --check` 维持 0 problems、poiesis `tsc --noEmit` + vitest 全绿、`npm run build` 通过
- 每任务独立 commit（conventional commits 中文描述，沿仓库习惯）
- **明确 defer（本计划不做）**：EVENT_NAME_SUGGESTIONS 含 4 个自身硬错误名（发现 A，留阶段 4）；@empty 运行时 bug（阶段 2 候选）；事件改名双 undo（EventsSection.tsx:186-194 两次 setProp——store 无 batch API 则注释登记不进 scope）；历史设计文档（docs/design、docs/superpowers/specs 2026-06/07 系列、docs/2026-08-06-view-syntax-usage-guide.md）是**时点记录不改写**——docs-gate 只检查 docs/omniview/*.md + 3 个 spec 文件

---

### Task 1: verify-vocabulary.py event_canonical_names fn-scoped 修复

**Files:**
- Modify: `scripts/verify-vocabulary.py`（`event_canonical_names` :95-98）

**Interfaces:**
- Consumes: 无（纯脚本修复）
- Produces: 提取集 = canonical 表 54 名，不含 legacy 臂（onClick/onMouseEnter/onMouseLeave）

- [ ] **Step 1: 确认缺陷**
Run: `python scripts/verify-vocabulary.py --check` → 1 problem：`whitelist-only: ['onClick', 'onMouseEnter', 'onMouseLeave']`。根因：`event_canonical_names` 全文件正则 `=>\s*Some\("([^"]+)"\)` 同时命中 `canonical_event_name` 的 match 臂（54 canonical）与 `legacy_spelling` 的臂（`"onPress" => Some("onClick")` 等 3 个）。

- [ ] **Step 2: 最小实现（fn-scoped）**
按仓库既有先例 `scripts/verify-semantic-universe.py:120-129`（fn-scoped 正则限定函数体）改写提取：先定位 `pub fn canonical_event_name` 的函数体（`fn canonical_event_name` 到下一个 `pub fn` 或文件内边界），再在其内部跑既有正则。`legacy_spelling` 臂自然排除。加注释说明为什么必须 fn-scoped（防回归教训）。

- [ ] **Step 3: 验证**
Run: `python scripts/verify-vocabulary.py --check` → 0 problems。反例验证（防回归）：临时在 `legacy_spelling` 加一条假臂 `"onTest" => Some("onBogus")` 重跑提取逻辑（可写临时脚本或直接跑 `--inspect` 观察 extracted 集）→ 确认提取集不含 onBogus；然后还原。

- [ ] **Step 4: Commit**
```bash
git add scripts/verify-vocabulary.py
git commit -m "fix(gate): verify-vocabulary 事件提取 fn-scoped——排除 legacy_spelling 臂误捕（Task 1）"
```

---

### Task 2: docs-gate 修复（3 红块 + 文本残留 + DELETED_SYNTAX_PREFIXES + binary 新鲜度检查）

**Files:**
- Modify: `docs/omniview/00-cheatsheet.md`、`docs/omniview/migration-v1-to-v2.md`、`docs/omniview/05-components.md`、`docs/omniview/06-templates.md`、`docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md`
- Modify: `scripts/verify-docs.py`（DELETED_SYNTAX_PREFIXES :118-126 + 新增 binary 新鲜度检查）
- Modify: `crates/omnilang-core/src/semantic/validate_event_names.rs`（:293-295 过期注释，1 行）

**Interfaces:**
- Consumes: 无
- Produces: docs-gate 0 failed；fragment 教旧名从此 FAIL；本地旧 binary 假绿陷阱关闭

- [ ] **Step 1: 修 3 个 FAIL 块（complete 块，任意 error 即 fail）**
- `docs/omniview/00-cheatsheet.md:175` 起完整块：`@onClick={onClick}` → `@onPress={onPress}`（块内 fn 名同步改名，注意块内 `clicks` 信号/计数器语义不变）
- `docs/omniview/migration-v1-to-v2.md:185` 起完整块（:229 `@onClick={() => { clicks = clicks + 1; }}` → `@onPress={...}`）
- `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md:829` 起完整块（:881 `@onClick={handleAdd}`、:886 `@onClick={() => { TodoStore.toggle(item.id) }}` 两处 → `@onPress`）

- [ ] **Step 2: 文本残留（不触发 gate 但教错误知识）**
- `00-cheatsheet.md:255`：解析规则叙述更新——「未知事件名 → warning（E0006）+ 最近合法建议」之后补「语义事件分级（spec 2026-08-08）：首批 14 个语义事件名是唯一合法拼写，旧 DOM 名（onClick/onMouseEnter/onMouseLeave 等）→ 硬错误 + 迁移提示；其余 40 个透传名仍走未知名 warning」；:265 鼠标表行 `@onMouseEnter/@onMouseLeave` → `@onEnter/@onLeave`（保留 onMouseDown/onMouseUp/onMouseMove 等透传名）
- `migration-v1-to-v2.md:121`：`=> 仅用于事件内联 @onClick={...}` → `@onPress={...}`
- `05-components.md:202`、`06-templates.md:226`：注释「与 @onClick 一致性高」→ 「与 @onPress 等语义事件一致性高」（引用 07-events-and-bindings 语义事件表）

- [ ] **Step 3: verify-docs.py DELETED_SYNTAX_PREFIXES 加「事件名 "」前缀**
`scripts/verify-docs.py:118-126`：追加 `"事件名 \""`（E0006 旧名消息前缀）——fragment 块教 @onClick 从 NOTE 放行变为 FAIL。这是 3 个 complete 块修复之外的必要防线（否则 fragment 里的旧名永久漏网）。

- [ ] **Step 4: verify-docs.py binary 新鲜度检查（关闭本地假绿陷阱）**
新增检查：omnic 二进制 mtime < `crates/omnilang-core/src` 下最新 .rs mtime → 打印醒目错误并 gate 模式 FAIL（`--inspect` 豁免），消息说明「阶段 1 教训：旧 binary 跑 docs-gate 报假绿（2026-08-08 实证）」。CI 每次重建 omnic 不受影响。

- [ ] **Step 5: 过期注释修正**
`validate_event_names.rs:293-295`：测试注释「onEnter canonical 目前为 None」→「canonical 已入表（Task 3，mouseenter 族）——本测试只断言不产生 Error」。

- [ ] **Step 6: 验证**
Run: 比对 omnic mtime vs 核心源最新 mtime（当前 debug binary 2026-08-08 14:13 > 源文件 13:04，新鲜）→ `python scripts/verify-docs.py --check` → 0 failed；`python scripts/verify-vocabulary.py --check` 不受影响仍 0；`cargo test -p omnilang-core legacy_event`（注释修正后无回归，可只跑该模块）

- [ ] **Step 7: Commit**
```bash
git add docs/omniview scripts/verify-docs.py crates/omnilang-core/src/semantic/validate_event_names.rs
git commit -m "fix(docs): docs-gate 3 红块 @onClick→@onPress + 文本残留 + 事件名前缀 FAIL + binary 新鲜度检查（Task 2）"
```

---

### Task 3: vocabulary-registry 域 1 语义事件入册 + verify-vocabulary.py 新断言

**Files:**
- Modify: `docs/superpowers/specs/vocabulary-registry.md`（域 1）
- Modify: `scripts/verify-vocabulary.py`（新断言，复用 `rust_str_array` :101）

**Interfaces:**
- Consumes: `crates/omnilang-core/src/codegen/shared/event_names.rs` 的 `SEMANTIC_EVENT_NAMES`（常量，Task 1 已 fn-scoped 提取同文件）
- Produces: 域 1 语义事件名表（14 名）↔ `SEMANTIC_EVENT_NAMES` 双向断言

- [ ] **Step 1: 域 1 叙事对齐（当前矛盾：仍教「click=正确语义词汇」——实际 @onClick/@click 都是硬错误）**
- 「来源域」段落更新：canonical 名（54）= 编译器**内部解析目标**（`canonical_event_name` 单表）；DSL 表面 = 14 个语义事件名；平台形状（onClick/onKeyDown React 合成名等）= 排除词汇不变
- 现有「语义词汇」表（54 canonical）保留——它是内部解析面，断言锚点不变

- [ ] **Step 2: 新增小节「语义事件名（DSL 表面）」**
表格第一列 = 14 个语义事件名（onPress/onInput/onChange/onSubmit/onKeyDown/onKeyUp/onPointerDown/onPointerMove/onPointerUp/onScroll/onFocus/onBlur/onEnter/onLeave），语义列简明（如「点按（click/tap 别名收编）」），注明与 canonical 的解析关系（onPress→click 等）。格式必须符合脚本 `first_column_rows` 解析（header + separator + 每行第一列非空）。

- [ ] **Step 3: verify-vocabulary.py 新断言**
仿既有 `assert_set` 模式：提取新小节第一列 vs `rust_str_array(event_names.rs 全文, "SEMANTIC_EVENT_NAMES")`，双向断言。同时加一条防御断言：`EVENT_NAME_SUGGESTIONS` 不含 legacy 名（onClick/onTap/onMouseEnter/onMouseLeave）——**注意**：该防御断言当前会 FAIL（发现 A 已知未修，defer 阶段 4）→ 不要加这条，只在代码注释记录发现 A defer。**只加 SEMANTIC_EVENT_NAMES 断言。**

- [ ] **Step 4: 验证**
Run: `python scripts/verify-vocabulary.py --check` → 0 problems；`python scripts/verify-semantic-universe.py --check` 维持 0

- [ ] **Step 5: Commit**
```bash
git add docs/superpowers/specs/vocabulary-registry.md scripts/verify-vocabulary.py
git commit -m "docs(registry): 域 1 语义事件名入册 + SEMANTIC_EVENT_NAMES 双向断言（Task 3）"
```

---

### Task 4: poiesis 编辑器事件意图化四链同步 + TS→Rust 门禁

**Files:**
- Modify: `poiesis/src/services/codegen/parser-view.ts`（:333-337 归一化）
- Modify: `poiesis/src/services/codegen/source-emitter.ts`（:669-679 发射）
- Modify: `poiesis/src/data/componentProtocol.ts`（UNIVERSAL_EVENTS :49-103 + 特殊事件表）
- Modify: `poiesis/src/components/panels/inspector/EventsSection.tsx`（EVENT_GROUPS :20-35）
- Modify: `poiesis/src/wasm/.gitignore`（产物追踪）+ `scripts/build-wasm.sh`（可能需要）
- Test: `poiesis/src/services/codegen/__tests__/`（新增事件语义化测试，先例 `store-action-wasm-validity.test.ts`）

**Interfaces:**
- Consumes: Rust `legacy_spelling` + `canonical_event_name` 语义（事件_names.rs 注释为锚，不 import Rust）
- Produces: 编辑器往返合法源码；发射事件名 ↔ 真实 WASM 编译器（`compile_multi_to_omniview`，app 保存门禁同管线）零 E0006 断言

- [ ] **Step 1: 读侧语义归一化（parser-view.ts:333-337）**
on 前缀规则后追加语义映射（大小写不敏感，镜像 Rust canonical + legacy_spelling）：
`click/press/tap` 族 → `onPress`；`mouseenter/enter` 族 → `onEnter`；`mouseleave/leave` 族 → `onLeave`；其余保留。单一映射表常量 + 注释指向 `event_names.rs`（三处抄录中的第四处，注释必须声明「与 Rust 同步，靠 Task 4 门禁防漂移」）。`@press` 运动触发器在编辑器 AT 分支本就当事件绑定处理（既有行为，归一化结果 onPress 不变——不加新逻辑）。

- [ ] **Step 2: 发侧防御映射（source-emitter.ts:669-679）**
发射 `@${evt}` 前对事件名做同表映射（props 直接注入旧名路径的兜底）。两处发射点（style.eventBindings 与 elNode.props 分支）统一。

- [ ] **Step 3: componentProtocol.ts 事件清单对齐**
UNIVERSAL_EVENTS：删除 `onClick`（:52）/`onTap`（:53）/`onMouseEnter`（:70）/`onMouseLeave`（:71）；增补 `onEnter`/`onLeave`（payload 按 Rust PointerEvent schema，React 合成面填 MouseEvent）+ `onPointerDown`/`onPointerMove`/`onPointerUp`（payload PointerEvent）。检查特殊事件表（button 等，:195 附近）是否含旧名，同步清理。`getComponentEvents` 逻辑不动。

- [ ] **Step 4: EventsSection.tsx EVENT_GROUPS 对齐（:20-35）**
「点击」组：去 onClick/onTap → `["onPress", "onDoubleClick", "onContextMenu"]`；「鼠标」组：onMouseEnter/onMouseLeave → `onEnter`/`onLeave`（保留 onMouseDown/onMouseUp/onMouseMove 透传）；新增「指针」组 `["onPointerDown", "onPointerMove", "onPointerUp"]`；「输入」组 onInput/onChange 不变。核对全部组无其他硬错误名。

- [ ] **Step 5: TS→Rust 门禁（扩展 store-action-wasm-validity 先例）**
新增测试（如 `event-emission-wasm-validity.test.ts`）：用 SourceEmitter 发射一个含**全部 EVENT_GROUPS 事件名**的视图（每个事件绑一个组件 fn），经 `compile_multi_to_omniview` 断言**零 E0006**（有 E0006 即列表输出失败事件名）。事件名清单直接从 `componentProtocol.ts` 导出（单一来源，不硬编码）。
**前置（必须解决）**：`poiesis/src/wasm/.gitignore` 为 `*`——wasm 二进制未入 git，CI checkout 无二进制 → wasm-validity 测试（含既有 store-action 版）在 CI 无法运行、app 构建缺运行时。方案：改 .gitignore 例外规则（`*` + `!omnilang_wasm.js` + `!omnilang_wasm_bg.wasm` + `!*.d.ts`）提交 4 产物，并先本地跑 `bash scripts/build-wasm.sh` 重建（当前本地二进制 2026-08-07 17:03 早于阶段 1——旧编译器仍接受 @onClick，必须重建到阶段 1 语义后提交）。若重建产物过大或方案受阻，回退方案：CI 前增加 wasm 构建步骤——实施者选择并说明理由。

- [ ] **Step 6: 验证**
Run（cd poiesis）：`npx tsc --noEmit` 全绿 + `npx vitest run --config vitest.config.js src/services/codegen/__tests__/` 全绿（注意：CI 命令用 `vitest.config.js` 但文件实为 `poiesis/vitest.config.ts`、根目录另有 `vitest.config.js`——若命令报配置路径错误，是 CI 配置 bug，修正 CI 命令为正确路径并注释）+ `npm run build` 通过 + 新测试逐条绿。

- [ ] **Step 7: Commit**（拆 2-3 个：读发侧 + UI/协议 + 门禁产物）
```bash
git commit -m "fix(editor): 事件意图化四链同步——parser-view 归一化/source-emitter 映射/EVENT_GROUPS/UNIVERSAL_EVENTS（Task 4）"
git commit -m "test(editor): 事件发射物过真实 WASM 编译器零 E0006 断言 + wasm 产物入 git（Task 4）"
```

---

## 收口验证（全部任务完成后）

Run（本地，全用新鲜 binary）：
1. `python scripts/verify-docs.py --check` → 0 failed
2. `python scripts/verify-vocabulary.py --check` → 0 problems
3. `python scripts/verify-semantic-universe.py --check` → 0 problems
4. `cd poiesis && npx tsc --noEmit && npx vitest run src/services/codegen/__tests__/ && npm run build` → 全绿
5. `cargo test --workspace`（无 Rust 语义改动，仅注释/无代码改动时可选——Task 2 改过注释，跑 `cargo test -p omnilang-core legacy_event` 模块足够）
6. git log 汇总 + 每任务独立 commit 可追溯

**明确 defer（登记）**：发现 A（EVENT_NAME_SUGGESTIONS 过滤 legacy 名）→ 阶段 4；@empty 运行时 bug → 阶段 2；事件改名双 undo → 编辑器 UX backlog；历史设计文档不改写。
