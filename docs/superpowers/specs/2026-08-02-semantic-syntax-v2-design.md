# OmniLang 语义级 DSL v2 设计文档（全量重设计）

**日期**: 2026-08-02
**状态**: Implemented（2026-08-03 终验完成——learn-omniview 三目标全量绿 + 9 标签页浏览器全测 0 runtime error + 16 篇文档与编译器事实一致 + 注入载荷语料回归闭环；验证协议逐项勾选见 §4，逐域验收对照见 §5 各域验证清单）。**注记（2026-08-05，v3 修复时代）**：本计划 Phase 6 起进入 v3 修复时代——终验复核发现 §7 两项承诺（#28 Map.set、#33 嵌套 #root）未落地，已按 `.architecture/deferrals.md` 台账如实标注（见 §7 deferrals 行）；v3 行为修复的权威追踪为 deferrals 台账 + `docs/superpowers/plans/2026-08-03-semantic-syntax-v3-fixes.md`，本设计文档不再宣称未实现承诺
**依据**: `.architecture/reviews/feature-omniview-semantic-syntax-compilability.md`（13 专家评审）+ ADR-052/053/054/055
**设计方法论**: 语义驱动分层（方案 A，用户确认）——每域：语义意图 → BNF v2 → 三目标手写对照样例 → 映射表 → 诊断表 → 验证清单；设计-验证循环（用户确认）

---

## 1. 背景与动因

13 专家评审结论：语义级 DSL 三层架构**必要且充分**（Pragmatic Ratio 0.63），语义层本身大体纯净；但「语法 → 地道 React/Flutter 代码」在 HEAD 未兑现——失败点集中在：语法层残留（文档虚构/三表面/平台形状泄漏/该删未删）、codegen 层静默降级、验证门禁盲区、文档三方漂移。

用户设计哲学（本设计的第一原则）：**一切前端语法都是让编译器知道「这是前端的什么功能、要达到什么效果」，编译器理解意图后，为每个编译目标生成该平台最地道的代码**。

## 2. 设计原则（v2 基线）

1. **语义纯度可检验**：每个语法构造必须能回答「编译器因此知道了什么前端功能」；凡是表达「实现机制」而非「功能意图」的语法（DOM 形状、JS 形状、平台术语）不得进入语言层
2. **地道映射有证据**：每域设计附三目标「手写对照样例」（React/Flutter 工程师会手写的代码）；语法映射目标 = 生成语义等价的地道形式；对照样例入 spec 作为验收基线
3. **floor 外硬诊断，绝无 parse-and-ignore**：删除的语法全部产生「编译错误/警告 + 迁移提示」；「解析但惰性」路径清零
4. **同一语义只有一个名字**：消除三表面（值绑定）、三名字（主题）、双机制（闭包）类破碎
5. **文档虚构归零**：spec 只写已实现或将实现的语法；「已实现范围 vs 设计记录」显式区分（实现状态逐项跟踪见 `docs/superpowers/specs/semantic-universe.md`——语义集 vs 已实现集分离，设计 P3，Task 7.2）；文档同步随实现同 commit
6. **词汇语义域封闭（设计 P1 复发机制）**：每域设计时强制回答「词汇的语义域是什么」——新增语义词汇先查登记册（`docs/superpowers/specs/vocabulary-registry.md`，5 个封闭词汇域）；Flutter 旗标名（`textField`/`image`…）/ React 合成事件名（`onClick`…）**不得直接成为语义词汇**（借词 → 语义纯度违反，a11y 域教训见 域 10）；每域模板含「词汇来源」字段，登记册与编译器白名单交叉断言由 `scripts/verify-vocabulary.py` 强制（漂移即门禁失败）
7. **语义集与已实现集分离（设计 P3 复发机制）**：白名单（BNF）定义「前端功能全集」——只随语义域变化（「这是不是前端功能」），**不随实现进度漂移**；每项实现状态（支持/部分/未支持 + 诊断行为）单独跟踪于 `docs/superpowers/specs/semantic-universe.md`（5 语义域：可动画属性/数组操作/事件/RefOp/design token 类型），`scripts/verify-semantic-universe.py` 强制（条目必有状态、白名单 ⊆ 语义集、支持/部分 ∈ 白名单、未支持 ∉ 白名单——漂移即门禁失败）。教训：19 属性白名单曾是「已实现集」——borderRadius/fontSize 上轮排除理由事实错误、v2 才补回；ArrayMethod 语义 pass 提升到所有后端但 floor 门缺失（v3 Task 4.8 补 E_CODEGEN）

## 3. 四项关键决策（用户已确认）

| # | 决策 | 决议 |
|---|------|------|
| D1 | 主题切换语义 | **`@themes { ... }` 声明 + `@theme` 信号绑定**（编译器自动接线三目标）；`theme = "dark"` 语句不发明（= 普通 store action 设置 @theme 信号）；spec 删除该虚构语法 |
| D2 | 值绑定收敛 | **`bind:value` 唯一双向绑定**；删除 `value={sig}` 语义绑定形式（单向用普通 prop `value={expr}`）；删除隐式 auto-bind。v4 D-V4-4 修订：`value={sig}` 仅当元素带 `@ref` 时属该已删形式（v1 双向形状）；无 `@ref` 的 `value={sig}` 即普通单向 prop——合法 |
| D3 | capturePointer | **无参数** `capturePointer()`/`releasePointer()`（语义：锁定后续指针事件到本元素）；多指针显式锁定 = floor 外硬诊断；Flutter = error 诊断 |
| D4 | Typed slots | **删除 `data_param` 与类型约束**；写 `@slot name: Type` → 编译错误 + 提示；Slots 语义纯净为「父传内容」（default + named + fallback） |

## 4. 验证协议（反复验证）

### 4.1 每域设计-验证循环

```
设计定稿（本节模板） → spike 实现（parser + semantic + ≥1 后端 + 测试）
→ 门禁（cargo test / tsc --strict / dart analyze / eslint react-hooks）
→ 浏览器行为验证（焦点、绑定方向、主题切换、key 行为、动画）
→ 发现矛盾 → 修订设计 → BNF spec 同步（doc/spec/parser 三方一致）→ 下一域
```

### 4.2 交叉与终验

- ✅ **每完成 3 域**：全量浏览器测试 + 文档/BNF 一致性审计（仿 deferrals #24/#25 模式）— 逐域浏览器实证（T2.2 指针捕获/T2.3 值绑定/T4.2 主题/T5.1 node 实证）+ BNF spec 逐域同步（§2/§3/§4/§5/§6/§7/§11）+ 全量 9 标签页浏览器与文档审计归 Phase 7 终验（T7.1/T7.2，见下）
- **终验**：learn-omniview 三目标全量编译（React: tsc --strict 0 错误 + eslint 0 告警；Flutter: dart analyze 0 错误；OmniView: 浏览器 9 标签页全测）+ 16 篇文档全量对齐 + 注入载荷语料回归
  - ✅ 2026-08-03（Phase 7 终验，Tasks 7.0-7.3）：React `verify-react.sh` GATE PASSED（tsc strict + eslint 全豁免且零失效条目）· Flutter `learn_omniview_flutter_output_passes_dart_analyze` un-ignore 后 0 错误 0 警告（T5.8a 达成，T7.1 复核）· OmniView 9 标签页浏览器套件 41 绿 + React 套件 17 绿 + theme-wiring 实证，0 failures（T7.1；P0 缺陷 BUG-1/BUG-3 于 T7.1a 修复后 fixme 自移除）· 232 块文档全量审计 0 失败 + docs-gate CI job（T7.2）· 注入载荷语料最终回归 5 后端 × 9 插入点 × 19 载荷 = 855 元组，含 a11y 新表面（T7.3，2 个 Flutter 注入缺口发现并修复；原「18 载荷/810」为安全方向漏计，2026-08-03 最终评审按文件实计数修正）

### 4.3 破坏性政策

- 已删除语法 → 硬诊断 + 迁移提示（`validate_ref_ops` 先例：`ref.style.*` → "use signal + style binding instead"）
- demo 迁移随域进行（92 文件分批）；文档同步随域同 commit
- 每域结束零「open-undispositioned」发现（fix-now / hard-diagnostic / delete / defer-with-trigger 四选一）

### 4.4 前置基础设施修复（第一优先级，任何域设计前完成）

评审发现的门禁盲区必须先闭合（否则后续修改无验证网）：

1. ✅ react-test-app 真实化：React 19 + Zustand v5 + `strict: true` + `*.module.css` ambient + `tsc --noEmit` 脚本 + eslint-plugin-react-hooks（Task 0.4，commits a53ccbce..381f9ad7——166 tsc + 121 eslint 全匹配豁免清单 18 条（路径级、含计数上限）各带修复任务引用，门禁诚实失败已验证；初始快照 284 tsc + 109 eslint / 20 条，随 T0.4 编译器修复收窄）
2. ✅ 展示项目 Flutter 门禁：`generated_code_validation.rs` 增加 learn-omniview → Flutter → 全输出 dart analyze（Task 0.5 全量枚举模式 + 诚实 `#[ignore]`；T5.8a un-ignore 后全量 0 错误 0 警告 5/5 绿）
3. ✅ `compile_never_panics` 加断言 + codegen 后端 fuzz 目标（Task 0.1——proptest 真断言 + codegen fuzz；flutter feature 运行验证 7/7）
4. ✅ 增量缓存恢复未变更文件诊断（`analyse_multi_cached`，poiesis 主路径）+ 回归测试（Task 0.2——缓存命中诊断合并 + 回归测试；解析期错误在缓存命中时丢失为已记录 known-gap，缓存格式设计遗留）
5. ✅ per-backend feature 门禁修复：`is_dangerous_url` 移入 `crates/omnilang-core/src/codegen/sanitize.rs` + feature 矩阵 CI job（Tasks 0.3/0.3a——vue/dioxus 跨后端解耦 + 编译级 per-backend feature-matrix job 全绿）
6. ✅ CLI/编辑器 warnings 网关收口（`OmniViewResult` 加 warnings 字段、CLI 三命令打印、lint 提升进共享管线）（Task 0.6——warnings 三网关可见：CLI/单文件/编辑器路径 + F1 warnings 恢复）
7. ✅ 治理同步：deferral #17/#22 关闭（附证据）、spec 状态转 Approved、删除 `lib.rs.orig`（Tasks 0.5/0.7——#17 全量枚举门禁 + #22 `cargo check --all-features` exit 0 实证 + spec Approved + .orig 删除）
8. ✅ 参数化 fixture 矩阵补 @if/@for（deferral #7 收口）（Task 0.8——control_flow fixture 含 @if/@for 参数化形状 + 契约测试，T5.5 un-ignore 全绿）

---

## 5. 逐域设计

每个域按模板：语义定义 / **词汇来源**（登记册条目——先查 `docs/superpowers/specs/vocabulary-registry.md`，写清语义词汇的来源域与排除词汇；Flutter 旗标/React 合成事件名借词即违规）/ BNF v2（完整定义变更与新增；未变部分标注「保留 v1」）/ 手写对照样例 / 映射表 / 诊断表 / 验证清单。白名单节另含「语义集 vs 实现状态」双列表（权威全集：`docs/superpowers/specs/semantic-universe.md`——域 1/2/3/7/8：语义集成员 → 实现状态（支持/部分/未支持 + 诊断行为））。

> 注：本设计文档中的 BNF 为设计意图。每域 spike 第一步做「BNF ↔ parser 事实对照」，以 parser 事实为准同步 BNF spec 文件（`docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md` 按域重写为 v2）。

---

### 域 1: Ref 系统

#### 语义定义

「元素引用」让编译器知道三件事：
1. **命令式操作**：对某个元素执行一次性动作（聚焦、滚动、测量、选择、指针捕获）
2. **声明式绑定**：元素状态与数据持续同步（焦点状态、滚动位置、值）——「双向值绑定」= 控件值与数据永远一致
3. 引用由 `@ref={name}` 声明建立（保留 v1 语法，ADR-042）

#### BNF v2

```
ref_op            ::= 'ref' '.' ref_method '(' args? ')'
ref_method        ::= 'focus' | 'blur'
                    | 'scrollTo' | 'scrollBy'
                    | 'measure' '(' ')'               (* 单形式；结果 { width, height, x, y }（DOMRect 对等）— 维度读用 ref.width / ref.height / ref.position *)
                    | 'select' | 'setSelection' '(' expr ',' expr ')'
                    | 'value'                        (* 读 *)
                    | 'capturePointer' | 'releasePointer'    (* 无参数 *)
ref_binding       ::= 'focus' '=' expr            (* 声明式焦点绑定，保留 *)
                    | 'scroll-x' '=' expr | 'scroll-y' '=' expr   (* 保留 *)
value_binding     ::= 'bind:value' '=' expr       (* 唯一双向绑定，保留 v1 *)
                    | 'bind:checked' '=' expr     (* 选中态双向绑定——checkbox/radio；v4 正式化：React/OmniView 双向、Flutter warning+跳过 *)
value_prop        ::= 'value' '=' expr            (* 普通单向 prop，无魔法 *)
```

**删除**：`RefOpIR::SetValue`（`ref.value = x` / `ref.setValue(x)` 命令式写）、`value={sig}` 语义绑定形式（v1 双向形状——带 `@ref` 的写法；无 `@ref` 时即 value_prop，合法，v4 D-V4-4）、隐式 auto-bind、`capturePointer(id)` 参数

#### 语义集 vs 实现状态（Task 7.2，设计 P3）

> 语义集 = 前端功能全集（与实现无关，权威全集：`semantic-universe.md` 域 4）；**语法表面（BNF 白名单）只随语义集变化，不随实现进度漂移**。下表：语义集成员 → 实现状态（支持/部分/未支持 + 诊断行为）。

| 语义集成员 | 实现状态 | 诊断行为 |
|-----------|---------|---------|
| focus | 支持 | 无（三目标；Flutter FocusNode） |
| blur | 支持 | 无（三目标） |
| scrollTo | 支持 | 无（三目标；平滑度由目标默认决定——React 瞬时/Flutter 300ms easeInOut，滚动契约） |
| scrollBy | 支持 | 无（三目标） |
| measure | 支持 | 无（三目标；{ width, height, x, y }，Flutter 安全 cast） |
| select | 支持 | 无（三目标） |
| setSelection | 支持 | 无（三目标；(start, end)） |
| value | 支持 | 无（三目标；命令式写已删除——`ref.value = x` → error「用 bind:value 双向绑定」） |
| capturePointer | 部分 | React/OmniView 活动指针捕获；**Flutter → error「Flutter 不支持指针捕获，用 GestureDetector」**（floor 外） |
| releasePointer | 部分 | 同上 |

#### 手写对照样例

```tsx
// React 工程师手写（双向值绑定）
<input value={query} onChange={(e) => setQuery(e.target.value)} />

// Flutter 工程师手写（双向值绑定）
TextField(
  controller: _textController,   // TextEditingController
  onChanged: (v) => setState(() => query = v),
)
// focus 声明式绑定
Focus( focusNode: _inputFocus, child: TextField(...) )
// scrollTo
_scrollController.animateTo(y, duration: 300ms, curve: Curves.easeInOut)
```

#### 映射表（语义 → 三目标地道实现）

| 语义 | React | Flutter | OmniView |
|------|-------|---------|----------|
| bind:value | `value` + `onChange` setter（双向） | `TextEditingController` + `onChanged` 回写 | runtime 双向同步 |
| `bind:checked` | 控件选中态与信号双向一致 | React `checked` + `onChange` 回写 `e.target.checked`；OmniView 双向（change 事件 + 守卫）；**Flutter warning「bind:checked → param 'value' not supported on 'TextField'; skipped」** | 部分 |
| value={expr}（单向） | 普通 `value={expr}` prop | 普通 `controller:`/`initialValue`（无 listener） | 普通属性 |
| focus={sig} | `useEffect([sig]) → ref.current?.focus()` | `Focus` + `didUpdateWidget` 快照比较 | 既有 |
| scroll-y={pos} | `useEffect([pos]) → scrollTo` | `ScrollController.animateTo` 快照比较 | 既有 |
| capturePointer() | 活动指针 `setPointerCapture` | **error 诊断**（floor 外） | runtime 活动 pointerId |
| scrollTo(x,y) | `scrollTo({left, top})`（瞬时，目标默认） | 垂直优先 + 双轴差异警告 | 瞬时 |

**关键契约（评审实测修复）**：`bind:value` 在三目标必须**双向且方向一致**——React 发射 `value` + `onChange`（不回写即 bug）；Flutter 发射 controller + `onChanged` 回写；禁止任何目标单向化。

**滚动契约**：`scrollTo` 平滑度由目标默认决定（React 瞬时 / Flutter 300ms easeInOut），语义层不承诺；诊断表记录该差异，防用户当 bug 报。

#### 诊断表

| 输入 | 诊断 |
|------|------|
| `ref.setValue(x)` / `ref.value = x` | error：「命令式写已删除，用 bind:value 双向绑定」 |
| `ref.capturePointer(id)` | error：「capturePointer 无参数；多指针锁定不支持」 |
| `ref.style.*` / DOM API（保留 v1） | error + 迁移提示（既有） |
| `bind:value` 用于不可绑定元素 | error（既有） |
| `value={sig}`（无 @ref） | 普通单向 prop——合法（D2 value_prop 产生式；v4 修复：此前被误报为已删形式）；无诊断 |
| `value={sig}`（带 @ref） | v1 双向形状——EValueBindingRemoved + 迁移提示（bind:value / getter 调用 / computed）；error |
| Flutter `capturePointer()` | error：「Flutter 不支持指针捕获，用 GestureDetector」 |

#### 验证清单

- [x] fixture：`bind:value` 三目标生成形状断言（React onChange 存在 / Flutter onChanged 存在——**方向一致性**）— Task 2.3：bind:value 唯一双向 + 三目标方向 fixture（含 React store 限定 setState、Flutter ref 文本框接线修复）+ 浏览器实证
- [x] 浏览器：输入回写信号、信号更新回写控件（三目标各一轮）— Task 2.3 浏览器实证（方向一致性实测）
- [x] focus 绑定：无关 setState 不抢焦点（Flutter 快照比较修复验证）— Task 5.6：语义绑定快照四类全覆（焦点稳定）
- [x] `ref.value = x` 编译错误 + 迁移提示 — Task 2.1：SetValue 删除 + 三写形式硬诊断（verbatim 提示）+ 全 workspace 清扫
- [x] capturePointer() 无参三目标行为 + Flutter error 诊断 — Task 2.2：capturePointer 无参（React 活动指针 tracker + OmniView runtime tracker + Flutter error+throw）+ demo 迁移 + 浏览器实证

---

### 域 2: 动画系统

#### 语义定义

「动画」让编译器知道：**当某个时机发生时，这些属性从 A 值平滑过渡到 B 值**（时机 = 8 触发器；范围 = 属性白名单；播放 = 关键帧 + 播放控制）。滚动/拖拽驱动动画 = 「属性值由滚动进度/拖拽位移实时映射」。

#### BNF v2

```
trigger_attr      ::= '@enter' | '@hover' | '@press' | '@focus' | '@in-view'
                    | '@scroll' scroll_params?
                    | '@drag-horizontal' drag_params? | '@drag-vertical' drag_params?
scroll_params     ::= 'start:' expr 'end:' expr 'scrub:' bool      (* 恢复 *)
drag_params       ::= 'constraints:' '[' expr ',' expr ']'
                      'elastic:' expr 'threshold:' expr ('velocity_max:' expr)?   (* 恢复 *)
keyframe_body     ::= '{' prop ':' expr (';' prop ':' expr)* '}'   (* map 形式，保留 v1 *)
                    | '{' 'over:' duration 'from' '{...}' 'to' '{...}' '}'  (* 完整体 = 设计记录，未实现 *)
playback          ::= 'loop:' | 'count:' | 'direction:' | 'delay:' | 'fill:' | 'duration:' | 'easing:' | 'steps:'   (* 保留 v1 *)
whitelist         ::= 19 项（17 保留 + borderRadius + fontSize）   (* 语义集定义——全集与实现状态见 semantic-universe.md 域 1；语法表面不随实现进度漂移 *)
```

**删除**：`paused`（属性与播放参数）→ 硬诊断「用条件渲染（@if 信号）替代」

**新增**：`borderRadius`/`fontSize` 入白名单；`@scroll`/`@drag-*` 控制维度恢复（start/end/scrub、constraints/elastic/threshold，velocity_max 可选）

#### 语义集 vs 实现状态（Task 7.2，设计 P3）

> 语义集 = CSS 可动画属性全集（与实现无关，权威全集：`semantic-universe.md` 域 1）；白名单 19 项 = 已实现子集。**语法表面（白名单）只随语义集变化**——实现状态单独列跟踪；borderRadius/fontSize 教训（上轮以「实现未验证」为由排除，理由事实错误）：语义集判断只看语义域，不看实现进度。

| 语义集成员 | 实现状态 | 诊断行为 |
|-----------|---------|---------|
| opacity | 支持 | 无（三目标） |
| scale | 支持 | 无（三目标；Flutter Transform.scale） |
| translateX | 支持 | 无（三目标） |
| translateY | 支持 | 无（三目标） |
| rotate | 支持 | 无（三目标；Flutter Transform.rotate） |
| backgroundColor | 支持 | 无（三目标；Flutter Tween\<Color\>） |
| color | 支持 | 无（三目标；Flutter Tween\<Color\>） |
| width | 支持 | Flutter 经 SizedBox 动画 + 性能提示（[flutter] prefer scale/transform）；关键字值（auto 等）→ error「keyword value cannot be animated」 |
| height | 支持 | 同上 |
| borderRadius | 支持 | 无（三目标；Flutter BorderRadiusTween，Task 5.2） |
| fontSize | 支持 | 无（三目标；Flutter TextStyle，Task 5.2） |
| translateZ | 部分 | React/OmniView 动画；**Flutter 跳过 + [flutter] warning**（无对应 widget 构造） |
| scaleX | 部分 | 同上 |
| scaleY | 部分 | 同上 |
| rotateX | 部分 | 同上 |
| rotateY | 部分 | 同上 |
| rotateZ | 部分 | 同上 |
| skewX | 部分 | React/OmniView 动画；**Flutter 跳过 + [flutter] warning**（SDK 无 Transform.skew） |
| skewY | 部分 | 同上 |
| 白名单外可动画属性（borderColor/boxShadow/fontWeight/filter/top/…） | 未支持 | 解析期 error「白名单外属性」——19 项封闭；语义集成员但未实现（全集见 semantic-universe.md 域 1） |

#### 手写对照样例

```tsx
// React 工程师手写（@scroll 驱动）
useEffect(() => {
  const onScroll = () => { el.animate(...).currentTime = progress(); };
  window.addEventListener('scroll', onScroll, { passive: true });
  return () => window.removeEventListener('scroll', onScroll);
}, []);

// Flutter 工程师手写（@hover 缩放）
MouseRegion(
  onEnter: (_) => _ctrl.forward(),
  onExit: (_) => _ctrl.reverse(),
  child: ScaleTransition(scale: _scale, child: child),
)
// Flutter 工程师手写（fontSize 动画）
TweenAnimationBuilder<double>(
  tween: Tween(begin: 12, end: 16), duration: 300ms,
  builder: (c, v, _) => Text(..., style: TextStyle(fontSize: v)),
)
```

#### 映射表

| 语义 | React | Flutter | OmniView |
|------|-------|---------|----------|
| 8 触发器 | WAAPI + observers（既有，保留） | AnimationController + `_Omni*` helper（重构：共享文件 + 类型化配置） | 既有 |
| borderRadius/fontSize 动画 | WAAPI 原生 | `BorderRadiusTween` / `TextStyle.lerp` | runtime transition |
| @scroll start/end/scrub | 滚动区间 → progress 映射 | 同左（`_OmniScrollDrivenWidget` 参数类型化） | 既有模式扩展 |
| @drag constraints/elastic/threshold | 手势映射 + extent 推导（既有 WARNING 升级为实现） | 同左（`_OmniDragWidget` 参数类型化 + 约束解算） | 既有模式扩展 |
| @if mode:"fade" | CSS 淡入（既有） | **AnimatedSwitcher 真实现**（已实现；分支带 ValueKey，默认过渡） | CSS transition |
| steps() | WAAPI 原生 | warning + `Curves.linear` 回退（保留，spec 同步为 warning） | 原生 |

**Flutter helper 重构**（评审 MEDIUM）：`_OmniInViewWidget`/`_OmniScrollDrivenWidget`/`_OmniDragWidget` 从「每文件重复发射」改为单一共享生成文件 `_omniview_runtime.dart`；start/end/constraints 等配置编译期类型化（当前字符串运行时解析）；scrub 用 Ticker 时间而非 60fps 硬编码。

#### 诊断表

| 输入 | 诊断 |
|------|------|
| `paused:` / `@paused` | error：「paused 已删除，用 @if 条件渲染替代」 |
| 白名单外属性（blur/brightness 等） | error（解析期，既有）——白名单文档化为「语义集 v1：可动画性已被三目标验证」 |
| Flutter steps() | warning + linear 回退注释（既有） |
| 完整体 `over:/from:/to:` 形式 | error：「未实现；用 map 形式 @enter={{ ... }}」 |

#### 验证清单

- [x] borderRadius/fontSize 三目标动画运行（浏览器 + 快照）— Task 5.2：白名单 19 + BorderRadiusTween/TextStyle 映射 + React Npx 单位 + 快照/tsc/dart-analyze 验证
- [x] @scroll start/end/scrub：滚动区间行为（浏览器实测进度 → 动画）— Task 5.1：@scroll/@drag 维度恢复 + node 实证（scrollY=500 → progress 0.5 + clamp；scrub 收敛实测）+ Flutter 数值分支执行级测试
- [x] @drag constraints/elastic：拖拽范围限制 + 弹性回弹（浏览器手势）— Task 5.1：node 实证（过冲 -150 → -125 → 0.625 弹性回弹；velocity_max Flutter/OmniView 消费、React 接受但忽略）+ 浏览器套件 @drag 300px 实证（T7.1）
- [x] @if mode:"fade" Flutter 视觉淡入（shape 断言 + 真实 dart analyze 替代——仓库无 widget-test 基建，T4.5 a11y 同先例）
- [x] paused → error + 迁移提示 — Task 5.3：paused 双形式硬诊断（E_PAUSED_REMOVED verbatim）+ PlaybackProp::Paused 死变体删除 + 文档清剿
- [x] Flutter 共享 runtime 文件生成（多组件不重复）— Task 5.6：`_omniview_runtime.dart` 共享（3/79 按需门控）+ 类型化配置（字符串解析清零）+ scrub Ticker 时间

---

### 域 3: Design Tokens + 主题

#### 语义定义

「设计系统」让编译器知道：
1. **设计值**：13 种类型化的设计值（颜色/尺寸/字距/…）
2. **主题集合**：`@themes { dark, light }` —— 这个应用有 N 套主题变体
3. **当前主题**：`@theme` 标记的信号 = 当前主题模式；切换它 = 切换主题
4. **类/变体**：把一组设计值打包为可复用样式单元

#### BNF v2

```
themes_decl        ::= '@themes' '{' ident (',' ident)* '}'        (* 新增；位于 design 块内，与 token/class 同级 *)
default_theme_decl ::= '@default-theme' ident ';'                (* 新增；默认主题名，须为 @themes 枚举之一（语义验证） *)
theme_signal       ::= 'signal' ident ':' 'String' '@theme'       (* 新增标记；值必须是 @themes 枚举之一（语义验证） *)
token_types        ::= Color | Dimension | Duration | Easing | FontFamily | FontSize
                     | FontWeight | LetterSpacing | LineHeight | Opacity | Radius | Shadow | ZIndex   (* 13 类型，保留 v1——语义集定义，实现状态见下表 *)
class_decl         ::= 'class' ident '{' style_entries '}'        (* 保留 v1 + base 块 + &:: 伪元素补 BNF *)
variant_decl       ::= 'variant' ident '{' ... '}'                (* 保留 v1 *)
```

**删除**：`theme = "dark"` 语句（spec 虚构，编译器无此语法）；`@native` 平台层 token（死 token）

**修改**：classes/variants 三目标**真正实现**（Flutter 当前静默 no-op）；不可映射 design 值 → warnings 通道；React 生成 runtime 命名冲突解决

#### 语义集 vs 实现状态（Task 7.2，设计 P3）

> 语义集 = 设计值类型全集（与实现无关，权威全集：`semantic-universe.md` 域 5）；语法表面（13 类型）只随语义集变化，实现状态单独列跟踪。

| 语义集成员 | 实现状态 | 诊断行为 |
|-----------|---------|---------|
| Color | 支持 | 无（三目标） |
| Dimension | 支持 | 无（三目标） |
| Radius | 支持 | 无（三目标） |
| FontFamily | 支持 | 无（三目标） |
| FontSize | 支持 | 无（三目标） |
| FontWeight | 支持 | 无（三目标） |
| LineHeight | 支持 | 无（三目标） |
| LetterSpacing | 支持 | 无（三目标） |
| Shadow | 支持 | 无（三目标） |
| Duration | 支持 | 无（三目标） |
| Easing | 支持 | 无（三目标；Flutter 不可映射 easing → warning + 降级） |
| Opacity | 支持 | 无（三目标） |
| ZIndex | 部分 | React/OmniView CSS z-index；**Flutter → warning「Flutter z 序由 widget 顺序决定」**（降级） |

#### 手写对照样例

```tsx
// React 工程师手写（主题切换 —— 生成目标）
const [mode, setMode] = useState('light');
useEffect(() => { document.documentElement.setAttribute('data-theme', mode); }, [mode]);
// 设计 token 消费
<div className={styles.card} style={{ background: 'var(--card-bg)' }} />

// Flutter 工程师手写（主题切换 —— 生成目标）
class ThemeModeNotifier extends Notifier<ThemeMode> { ... }
MaterialApp(themeMode: ref.watch(themeModeProvider), theme: _light, darkTheme: _dark)
// class 消费
Container(decoration: AppDesign.cardDecoration(), child: ...)
```

#### 映射表

| 语义 | React | Flutter | OmniView |
|------|-------|---------|----------|
| @themes + @theme 信号 | 生成 `useThemeStore` 订阅用户 store → `data-theme` 属性（**接线**，当前无人写） | 生成 `ThemeModeNotifier` + `MaterialApp(themeMode:)` **自动接线**（当前只注释） | 识别 @theme 信号（消除 `mode` 魔法名） |
| 切换主题 | 普通 action 写信号 | 同左 | 同左 |
| class:Design.X | `styles['X']` + CSS modules | `AppDesign.X()` 装饰（**实现**，当前空桩） | CSS 类 |
| variant:Design.Y | 同 class 机制 | **编译期合并**（base ∪ 选中分支，默认值应用；ThemeExtension 查找被否决——生成的 ThemeData 从不注册 extensions，`Theme.of(...).extension` 运行时为 null——见 T4.3 实施注记） | CSS 变体 |
| class:active={cond} | 条件拼接（既有） | `decoration: cond ? X() : null`（**实现**，当前空 BoxDecoration） | 既有 |
| 动态 token | CSS 变量 + `setProperty`（既有，消毒修复入域 9） | 运行时 Values（既有） | 既有 |
| ZIndex | CSS z-index | **warning**：「Flutter z 序由 widget 顺序决定」 | CSS |

**React 命名冲突修复**：design runtime 生成模块（`useThemeStore`/`useDesignTokens`）移入专用目录（如 `__design/`），杜绝与用户 `stores/useThemeStore.ts` basename 冲突。

#### 诊断表

| 输入 | 诊断 |
|------|------|
| `theme = "dark"` | 普通 prop 赋值——语言层无法区分虚构语法与自定义组件合法 prop；对已知 DOM 标签发 **warning**（v4 修订：原「自然解析错误」裁决基于错误前提，实测为静默惰性属性；警告文本见 validate_props.rs） |
| `@native(...)` | 自然解析错误（token 已删除；T4.4 裁决同上） |
| 不可映射 token 值（box-shadow 组合等） | warning + 生成代码降级注释（warnings 通道落地） |
| display:grid / float | error（spec §10.3 实现） |
| cursor / zIndex | warning（spec §10.3 实现） |

#### 验证清单

- [x] 浏览器：点主题卡片 → data-theme 属性实际变化 + `[data-theme="dark"]` CSS 覆盖生效（React + OmniView）；Flutter themeMode 切换（widget-test）— Task 4.2：三目标接线 + 浏览器实证（React/OmniView 实测 data-theme 翻转 + CSS 覆盖生效；Flutter ThemeModeNotifier + main.dart shell——widget-test 基建缺位由 shape 断言 + dart analyze 替代，同 T4.5 先例）+ T7.1 theme-wiring.spec 1/1 通过
- [x] `theme = "dark"` → warning（已知 DOM 标签）+ 自定义组件豁免 — Task 5.2（v4 修订：原验证声称「error + 迁移提示」不存在，docs-gate 不覆盖设计文档诊断表——已如实修订）
- [x] class:/variant: 三目标视觉效果对照（Flutter 不再空桩）— Task 4.3：classes/variants 真实装饰（0 桩 0 TODO）+ 编译期变体合并（ThemeExtension 否决理由成立）+ warnings 通道
- [x] 不可映射值 → warning（快照断言）— Tasks 4.3/4.4：warnings 通道实证（learn-omniview 554 诊断）+ §10.3 四行诊断实现（ZIndex/cursor warning + grid/float error）
- [x] useThemeStore 无命名冲突 — Tasks 4.2/4.4：`__design/` 命名隔离（守卫移除 YAGNI 正确）+ 碰撞场景实证

---

### 域 4: Slots

#### 语义定义

「插槽」让编译器知道：**这里渲染父组件传入的内容**（命名位置 + 未传时的默认回退）。v2 纯净化为「父传内容」，不含数据参数。

#### BNF v2

```
slot_decl       ::= '@slot' ident ('=' string)? '{' view_tree '}'   (* body 级声明，保留 v1 *)
slot_consume    ::= ident '=' '{' view_tree '}'                      (* 命名 props 消费，保留 v1 *)
```

**删除**：`@slot name: Type`（类型约束 + data_param）→ error「v2 不支持 slot 数据参数」；遗留 `@slot(name)` 指令形式；`slot="name"` 属性消费形式

#### 手写对照样例

```tsx
// React 工程师手写
function Card({ header, children }: CardProps) {
  return <div className="card">{header}{children}</div>;
}
// Flutter 工程师手写
class Card extends StatelessWidget {
  const Card({super.key, this.header, this.child});
  final Widget? header; final Widget? child;
  Widget build(BuildContext c) => Column(children: [if (header != null) header!, child ?? SizedBox()]);
}
```

#### 映射表

| 语义 | React | Flutter | OmniView |
|------|-------|---------|----------|
| named slot | prop + 类型化 `ReactNode?`（既有） | `Widget?` 参数 + 非空守卫（既有） | named slot（既有） |
| fallback | children 默认值（既有） | `?? fallback`（既有） | fallback（既有） |

#### 诊断表

| 输入 | 诊断 |
|------|------|
| `@slot name: Type` | error：「类型约束与数据参数已删除；slot = 父传内容」 |
| `slot="name"` 消费 | error：「改用命名 props：name={...}」 |
| `@slot(name)` 指令 | error：「改用 body 级声明 @slot name=... 」 |

#### 验证清单

- [x] default + named + fallback 三目标行为对等 fixture（含未传 fallback 场景）— Task 6.1：slot 三形态对等 fixture（default+named+fallback 含裸实例）
- [x] 三种已删除形式 → error + 迁移提示 — Task 6.1：三删除形式 × 三 P0 后端错误测试（hint 文本逐字断言）+ IR/三后端清理
- [x] AdvancedTypesDemo 迁移（移除声明侧文档类型）— Task 6.1：demo 迁移零残留（grep 验证）

---

### 域 5: Stores

#### 语义定义

「状态容器」让编译器知道：**这个容器持有响应式状态（signal），从状态派生值（getter），修改状态的唯一途径（action），以及副作用（effect）**。主题模式（@theme）是带特殊接线的 signal。

#### BNF v2

```
store_decl     ::= 'store' ident '{' store_member* '}'       (* 顶层与内联统一 *)
store_member   ::= signal_decl | getter_decl | action_decl   (* 删除 function_def 分支差异 *)
signal_decl    ::= 'signal' ident ':' type ('@theme')?       (* @theme 新增标记 *)
getter_decl    ::= 'getter' ident ':' type '=' expr          (* 保留 v1 *)
action_decl    ::= 'action' ident '(' params ')' (':' type)? ('async')? block   (* 返回类型/async 入 BNF spec *)
effect_decl    ::= 'effect' '(' deps ')' block               (* 保留；deps 语义见下 *)
```

**修改**：effect deps 语义化——**声明 deps 必须覆盖 body 读取的 store 字段**，否则编译错误（修复 NotificationsPanel 类「deps 漏字段 → 静默不触发」）；顶层/内联 store BNF 统一；`$counter`/`$store.counter` 存取语法删除（统一 `Store.field`）

#### 手写对照样例

```tsx
// React 工程师手写（Zustand v5）
export const useTodoStore = create<TodoStore>()((set, get) => ({
  items: [], count: 0,
  setFilter: (f) => set({ filter: f }),
}));
// 订阅 —— 切片选择器（per-field）
const items = useTodoStore((s) => s.items);

// Flutter 工程师手写（Riverpod 3.0）
class TodoStore extends Notifier<TodoState> {
  @override TodoState build() => const TodoState();
  void setFilter(String f) => state = state.copyWith(filter: f);
}
final todoStoreProvider = NotifierProvider<TodoStore, TodoState>(TodoStore.new);
```

**React 订阅粒度（评审 HIGH）**：组件内每个被引用的 signal/getter/action 发射**切片 selector 调用**（`useTodoStore((s) => s.items)` 或生成的 per-field hooks），禁止整 store 订阅 + `useMemo([Store])` 恒新依赖。Flutter 侧已有精确 `select()`，保持。

#### 诊断表

| 输入 | 诊断 |
|------|------|
| effect deps 未覆盖 body 读取 | error：「effect 依赖漏掉：NotificationStore.unreadCount」 |
| `$counter` 存取 | error：「用 Store.field」 |
| store 内 `fn` 成员 | error：「用 getter 或 action」 |
| @theme 标记于非 String signal | error：「@theme 信号必须是 String」 |

#### 验证清单

- [x] effect deps 覆盖验证（漏字段 → 编译错误，fixture）— Task 3.4：ClosureExpr 臂 + `validate_effect_deps` pass（E_EFFECT_DEPS_MISSING 字段级匹配）+ FilterBar [searchText, demoFrameworks] 实证
- [x] 三目标 store 生成形状快照（切片选择器 / select() / Notifier）— Task 3.3：切片订阅全量（FilterBar 5 slices/0 whole-store + 全部读点改写 + 5 类 fallback）+ Task 3.5 store BNF 统一 + React mutation 三修（M2/M3/M4）+ verify-react 162 tsc 全豁免
- [x] @theme 信号识别 + 接线（域 3 联动）— Tasks 4.1/4.2：@themes/@theme 语法 + IR + 跨文件语义 pass + 三目标接线（React data-theme effect / Flutter ThemeModeNotifier / OmniView IR 驱动消除 mode 魔法名）
- [x] 多实例组件 store 隔离回归（既有测试保持）— 既有 prop-signal per-instance 回归全绿（多实例组件 prop 信号 per-instance 修复保持）

---

### 域 6: Control Flow

#### 语义定义

「控制流」让编译器知道：**条件渲染（这个元素在条件为真时存在）与列表渲染（为列表中每个 item 重复渲染，且 item 身份可追踪）**。身份追踪（key）是 v2 的核心契约——「列表项状态随身份保持」。

#### BNF v2

```
if_directive   ::= '@if' expr ('mode:' '"fade"' | '"wait"')? '{' view_tree '}'   (* 保留 v1 *)
for_directive  ::= '@for' ident 'in' expr ('key=' '{' expr '}')?
                   ('empty=' '{' view_tree '}')? ('[scroll]')? '{' view_tree '}'  (* 保留 v1 语法 *)
```

**修改（契约统一）**：
- `key_expr`：**对象项（record/map item）未提供 key → 语义警告**「对象项列表建议提供 key= 以保持状态」；提供了则三目标全部发射（React `key={expr}` / Flutter `key: ValueKey(expr)` / OmniView keyed 克隆复用——ADR-055）
- `[scroll]`：三目标一致——滚动列表语义；非滚动上下文（Column/Row/嵌套）默认文档流布局（Flutter: `Column(children:)`，**当前无条件 ListView.builder 必须修复**）
- `mode:"fade"`/`"wait"` 三目标对等（域 2 联动）
- `empty_body` 保留；iterable 单次求值（修复双求值）

#### 手写对照样例

```tsx
// React 工程师手写（列表 + key）
<ul>{items.map((item) => <li key={item.id}>{item.name}</li>)}</ul>

// Flutter 工程师手写
Column(children: [for (final item in items) KeyedSubtree(key: ValueKey(item.id), child: ListTile(...))])
// 滚动列表
ListView.builder(itemCount: items.length, itemBuilder: (c, i) => ListTile(key: ValueKey(items[i].id), ...))
```

#### 映射表

| 语义 | React | Flutter | OmniView |
|------|-------|---------|----------|
| @for 默认（非滚动） | map + key（修复 [object Object]） | `Column(children:)`（修复 ListView.builder） | 既有 DOM 渲染 |
| @for + [scroll] | 滚动容器 + map | `ListView.builder` | 既有 |
| key_expr | `key={expr}`（既有） | `key: ValueKey(expr)`（**实现**） | keyed 克隆复用（ADR-055） |
| empty_body | 三元（既有） | `if (isEmpty) emptyBody else ...` | 既有 |
| @if mode:"wait" | 降级注释（既有） | AnimatedSwitcher + 显式 FadeTransition（**实现**；等待旧内容过渡完成后插入新内容） | 既有 |
| @if mode:"fade" | 真实淡入（既有） | AnimatedSwitcher（**实现**） | 既有 |

#### 诊断表

| 输入 | 诊断 |
|------|------|
| 对象项 @for 无 key | warning：「对象项列表建议提供 key=；未提供时使用索引，重排将丢失状态」 |
| @for 内 @for 且外层 [scroll] | warning：「嵌套滚动列表」 |

#### 验证清单

- [x] keyed 列表重排/过滤保状态（浏览器三目标：输入框状态、滚动位置）— Task 5.5：@for 契约（Flutter Column/ValueKey 分派 + React 索引 key/单次求值 + node 行为实证含 key 碰撞证明）；T7.1 浏览器实证暴露 BUG-1/BUG-3（keyFn 硬编码/重排不渲染）→ T7.1a 修复（item_var 参数化 + rebind 接线）→ 44 passed/0 failed，fixme 自移除
- [x] 非滚动 @for 布局无崩溃（Flutter 嵌套 @for widget-test）— Task 5.5：Flutter Column/ValueKey 分派 + 契约测试 un-ignore 全绿（嵌套 @for 经 control_flow fixture 三目标编译 + dart analyze）
- [x] 对象项无 key → warning（快照断言）— Task 5.5：for-key lint（对象项无 key 语义警告）
- [x] @if/@for 参数化 fixture 加入跨后端矩阵 — Task 0.8：control_flow fixture（@if/@for 参数化）+ T5.5 契约测试 un-ignore

---

### 域 7: Events

#### 语义定义

「事件」让编译器知道：**用户与元素交互时触发处理器，并携带该事件的类型化数据**（54 canonical 事件 schema，保留 v1）。

#### BNF v2

```
event_attr    ::= '@click' | '@input' | '@change' | ...   (* 54 canonical，保留 v1——语义集定义，全集与实现状态见 semantic-universe.md 域 3（登记册 域 1 同载） *)
event_handler ::= '@on' ident '=' '{' (params)? '=>' block '}'   (* 事件内联 fn：唯一允许箭头处 *)
                  | ident '=' '{' expr '}'                (* 裸 onPress/onClick 形式（无 @ 前缀）亦接受 *)
```

**修改**：`on:` 块形式（旧 `event_handler ::= 'on' ':' '{' params '=>' block '}'`）**已删除**——parser 对 `on: { ... }` 发射硬错误「on: 已删除，用 @onXxx={...}」（遗留分支曾静默吞 body + `__states_removed` 哨兵零诊断，v3 修复任务 4.1 恢复「绝无 parse-and-ignore」）。事件内联 fn 唯一表面为 `@onXxx={...}`（或裸 `onXxx={...}`）。event 字段提取**桩全部诊断化**——每个被提取的字段必须有真实映射（删除 `debugName` 当数据的做法，key 事件用稳定 `keyCode`/`keyLabel` 映射）；`map_event` 表单表复用（删除 react/utils.rs 与 omniview/utils.rs 双写，dblclick 类 bug 源头）

#### 语义集 vs 实现状态（Task 7.2，设计 P3）

> 语义集 = 54 canonical 事件全集（封闭域，与实现无关，权威全集：`semantic-universe.md` 域 3）；语法表面只随语义集变化，实现状态单独列跟踪。54 为 `event_names.rs` 共享表实计数（旧「20 类型」为文档漂移，Task 7.2 修正）。

| 语义集成员 | 实现状态 | 诊断行为 |
|-----------|---------|---------|
| click | 支持 | 无（全目标） |
| dblclick | 支持 | 无（全目标；`map_event` 单表复用） |
| longpress | 部分 | **Flutter 便捷事件（onLongPress）；React/OmniView 无 DOM 对应——处理器名透传不触发** |
| hover | 支持 | 无（全目标） |
| change | 支持 | 无（全目标） |
| input | 支持 | 无（全目标；Flutter onChanged） |
| toggle | 支持 | 无（全目标） |
| submit | 支持 | 无（全目标） |
| focus | 支持 | 无（全目标） |
| blur | 支持 | 无（全目标） |
| select | 支持 | 无（全目标） |
| resize | 支持 | 无（全目标） |
| keydown | 支持 | 无（全目标；KeyEvent 稳定映射 keyCode/keyLabel） |
| keyup | 支持 | 无（全目标） |
| keypress | 支持 | 无（全目标） |
| mouseenter | 支持 | 无（全目标） |
| mouseleave | 支持 | 无（全目标） |
| mousedown | 支持 | 无（全目标） |
| mouseup | 支持 | 无（全目标） |
| mousemove | 支持 | 无（全目标） |
| mouseover | 支持 | 无（全目标） |
| mouseout | 支持 | 无（全目标） |
| pointerdown | 支持 | 无（全目标） |
| pointerup | 支持 | 无（全目标） |
| pointermove | 支持 | 无（全目标） |
| pointercancel | 支持 | 无（全目标） |
| scroll | 支持 | 无（全目标） |
| wheel | 支持 | 无（全目标；mousewheel 非标准拼写 → wheel） |
| touchstart | 支持 | 无（全目标） |
| touchend | 支持 | 无（全目标） |
| touchmove | 支持 | 无（全目标） |
| touchcancel | 支持 | 无（全目标） |
| contextmenu | 支持 | 无（全目标） |
| drag | 支持 | 无（全目标） |
| dragstart | 支持 | 无（全目标） |
| dragend | 支持 | 无（全目标） |
| dragover | 支持 | 无（全目标） |
| dragleave | 支持 | 无（全目标） |
| drop | 支持 | 无（全目标） |
| copy | 支持 | 无（全目标；未映射字段 → warning + 降级注释） |
| cut | 支持 | 同上 |
| paste | 支持 | 同上 |
| play | 支持 | 无（全目标） |
| pause | 支持 | 无（全目标） |
| ended | 支持 | 无（全目标） |
| load | 支持 | 无（全目标） |
| error | 支持 | 无（全目标） |
| animationstart | 支持 | 无（全目标；React 合成 onAnimationStart 等） |
| animationend | 支持 | 无（全目标） |
| animationiteration | 支持 | 无（全目标） |
| transitionend | 支持 | 无（全目标；React 合成 onTransitionEnd 等） |
| transitionrun | 支持 | 无（全目标） |
| transitionstart | 支持 | 无（全目标） |
| transitioncancel | 支持 | 无（全目标） |

#### 手写对照样例

```tsx
// React 工程师手写
<button onClick={(e) => { e.preventDefault(); emit('action', e.detail); }}>
// Flutter 工程师手写
IconButton(onPressed: () => ...)
// 键盘事件（稳定 key 映射）
TextField(onKeyDown: (e) => { if (e.logicalKey == LogicalKeyboardKey.enter) ... })
```

#### 诊断表

| 输入 | 诊断 |
|------|------|
| 未映射事件字段（如 ClipboardEvent.text 在 Flutter） | warning + 降级默认值注释（不再静默空串） |
| KeyEvent.code | 稳定映射（keyCode 整数/KeyLabel），不发射 debugName |

#### 验证清单

- [x] 事件字段三目标一致性 fixture（含键盘/剪贴板/拖放）— Task 2.4：字段桩全诊断化（warnings 通道线程化 + 降级注释）+ physicalKey.keyId 稳定映射 + 三目标 fixture
- [x] 别名表（dblclick 等）单表复用测试 — Task 2.4：`map_event` 五后端单表（删除 react/utils.rs 与 omniview/utils.rs 双写，dblclick 类 bug 源头）
- [x] FnRef stopPropagation 回归测试补齐（评审：零测试）— Task 2.4：FnRef 4 回归测试（含 stopPropagation 扫描；后续 7330e783 修复 FnRef stopPropagation 扫描）

---

### 域 8: Expressions & Statements

#### 语义定义

「计算层」让编译器知道：**表达式如何求值**（闭包、数组操作、匹配、异步、字段访问）——v2 核心：**一种意图只允许一种写法**。

#### BNF v2

```
closure_expr   ::= '|' params '|' expr        (* 唯一通用闭包形式；箭头仅事件内联 fn *)
array_method   ::= expr '.' 'filter' '(' closure ')'      (* 新增语言方法——语义集定义，实现状态见下表（全集：semantic-universe.md 域 2） *)
                 | expr '.' 'map' '(' closure ')'
                 | expr '.' 'count' '(' closure ')'
match_expr     ::= 'match' expr '{' arm+ '}'  (* 保留 v1；Flutter 变体臂 + 本地类型修复 *)
block_expr     ::= '{' stmt* expr '}'         (* 保留；含语句的 BlockExpr = P0 支持，非 P0 硬诊断 *)
style_expr     ::= '$signal' | '$field'       (* 通道修复：$signal 仅真信号；字段访问用 $field *)
```

**修改**：
- 闭包：管道 `|x|` 唯一通用闭包（文档同步——spec 教学箭头闭包是评审 HIGH）；箭头 `=>` 仅事件内联 fn
- 数组方法语言化：`items.filter(|x| ...)`/`items.map(...)`/`items.count(...)` 成为语言方法（语义：过滤/映射/计数）→ 映射 JS `filter/map/filter.length`、Dart `where/map/where.length`、OmniView 同 JS；`array_filter` 等函数保留为底层 stdlib（不删除）

#### 语义集 vs 实现状态（Task 7.2，设计 P3）

> 语义集 = 数组操作语言方法全集（与实现无关，权威全集：`semantic-universe.md` 域 2）；语法表面只随语义集变化，实现状态单独列跟踪。教训：语义 pass 曾提升到所有后端但 3 后端 verbatim 零诊断——floor 门（E_CODEGEN）v3 Task 4.8 补齐。

| 语义集成员 | 实现状态 | 诊断行为 |
|-----------|---------|---------|
| filter | 支持 | JS 族 `filter` / Dart 族 `where(...).toList()`；Python/Rust/Dioxus → **E_CODEGEN**（floor 门，v3 Task 4.8） |
| map | 支持 | JS 族 `map` / Dart 族 `map(...).toList()`；同上 |
| count | 支持 | JS 族 `filter(...).length` / Dart 族 `where(...).length`；同上；`count()` 无谓词 → error「count 需要一个谓词闭包」 |
| sort / find / reduce / some / every / unique / reverse / flat | 未支持 | 当前经普通方法调用/stdlib 表面（`array_sort`/`array_find` 等）；语言化需先登记语义全集（状态 未支持 → 支持）+ 三目标验证 |

#### 手写对照样例
- match：Flutter 变体臂 + 本地类型发射修复（deferral #30，当前生成不可编译 Dart）
- 样式表达式通道（与域 9 联动）：`lower_expr_to_json_from_ir` 仅对真实信号/store 引用用 `$signal` 下划线形式；`obj.field` 用 `$field: ["obj","field"]` 新形式——消灭 `row_score` 类未定义标识符

#### 手写对照样例

```tsx
// React 工程师手写（过滤 + 计数）
const done = items.filter((i) => i.done);
const total = items.filter((i) => i.done).length;

// Dart 工程师手写
final done = items.where((i) => i.done).toList();
final total = items.where((i) => i.done).length;
```

#### 诊断表

| 输入 | 诊断 |
|------|------|
| 通用表达式中的 `x => ...` 箭头 | error：「通用闭包用管道 |x|；箭头仅用于事件内联」 |
| `.count()` 无闭包参数 | error：「count 需要一个谓词闭包」 |
| 含语句 BlockExpr → 非 P0 后端 | error（E_CODEGEN）：「此目标不支持含语句的块表达式」 |
| match 变体臂 Flutter | 修复（#30），纳入门禁 |

#### 验证清单

- [x] filter/map/count 三目标编译 + 运行（含 Flutter where 映射）— Task 1.2：数组方法语言化（filter/map/count 三目标映射 + node 运行断言）
- [x] 闭包形式 parser 一致性（管道唯一性测试）— Task 1.1：管道闭包唯一（E_CLOSURE_ARROW 提示）+ spec §6.3/§9.3 同步
- [x] match 全目标编译（Flutter #30 修复后进门禁）— Task 1.4：match Flutter 变体臂 + 本地类型发射（#30 关闭）+ 门禁 un-ignore（T5.8a 全量 0 错误 0 警告）
- [x] style 通道 `$field` 回归（`style: { color: fn(item.field) }` 三目标）— Task 1.3：$field 通道（store→$signal / signal-like→$field / 局部→$ident 三路判别 + 7 签名线程化）+ OmniView $call 参数递归 + 真实 DataTable 验证

---

### 域 9: Styling

#### 语义定义

「样式」让编译器知道：**元素的类名归属与内联样式值**。v2 重点：**注入安全是样式通道的硬性约束**（类名/样式值任何插入点不可逃逸）。

#### BNF v2

```
class_attr     ::= 'class:' ident+                    (* 保留 v1 *)
class_map      ::= 'class:' '{' (string ':' expr)+ '}' (* 保留 v1 *)
style_inline   ::= 'style:' '{' (css_prop ':' expr)+ '}' (* 保留 v1；值通道修复 *)
```

**修改**：
- 类名字符集校验（lowering 层，与 `expect_ident` 一致 `[a-zA-Z0-9_-]`）——`class: "x',alert(1),'"` → 编译错误（注入修复双保险之一）
- codegen 转义：`format_class_ref` 用 `escape_literal_str(name, '\'')`（注入修复之二）
- 样式值消毒**单一事实来源**：Rust 生成 JS helper 字符串（`runtime.rs` 模式），React 运行时与 OmniView 运行时共用（修复三份拷贝漂移/hex-escape 绕过）
- Dioxus `style_classes` 死代码清理

#### 手写对照样例

```tsx
// React 工程师手写
<div className={styles.card + (active ? " active" : "")} style={{ color: "var(--text)" }} />
```

#### 诊断表

| 输入 | 诊断 |
|------|------|
| 类名含非法字符（引号/括号等） | error：「类名仅允许字母数字下划线连字符」 |
| 样式值含危险模式 | 编译期 error（url: data:/javascript:）+ 运行期消毒（既有 + 统一） |

#### 验证清单

- [x] 注入载荷语料回归（`'`/`"`/`\`/反引号/`${`/`</` 在 class/style/text/attr/event 每插入点，per-backend 安全快照）— Task 3.1：5 后端 × 8 插入点 × 17 载荷语料（含实体载荷 + fallback 形状 + 控制组防空过）；**T7.3 最终回归扩展**：新增 a11y 插入点 + `x*/ alert(1) //` 注释终结载荷 → 5 后端 × 9 插入点 × 19 载荷（6 字符 + 4 实体 + 9 利用）= 855 元组全绿（发现并修复 2 个 Flutter 注入缺口：a11y label `$` 插值 + role 降级注释 `*/` 逃逸；原「18 载荷/810」为安全方向漏计，2026-08-03 最终评审按文件实计数修正）
- [x] `class: "x',alert(1),'"` 编译错误（不产出可编译载荷）— Task 3.1：lowering 字符集校验（E0006 类名仅允许字母数字下划线连字符）+ codegen 双保险转义 + 全后端编译错误断言
- [x] 消毒器行为一致性测试（Rust/React 运行时/OmniView 运行时三份同源）— Task 3.2：单一来源 `shared/sanitize.rs::sanitize_css_helpers_js`（双调用点）+ 22 用例双拷贝断言 + `\a` 洞修复 + RED 实证

---

### 域 10: a11y（新增域）

#### 语义定义

「可访问性」让编译器知道：**这个控件的可访问名称/角色/可见性/实时区域**——辅助技术如何理解这个界面。这是前端意图的核心组成部分（评审 MEDIUM-2：当前只能写 aria-*（DOM 词汇），Flutter 目标静默丢失）。

**词汇来源（登记册 域 2——「词汇来源」字段示范）**：语义原语（label/role/hidden/live/alt）= WAI-ARIA 概念（可访问名称/角色/隐藏/实时区域/图像替代）；role 词汇 = WAI-ARIA 规范名子集（button/textbox/link/checkbox/radio/img）；**排除词汇 = Flutter Semantics 旗标名**（textField/image——借词黑名单，Task 4.3 已 ARIA 化；本登记册机制防复发）。

#### BNF v2

```
a11y_block    ::= 'a11y:' '{' a11y_entry* '}'      (* 元素属性，与 style: {...} 同形态 *)
a11y_entry    ::= 'label' ':' string
                | 'role' ':' string
                | 'hidden' ':' expr                (* 动态隐藏 *)
                | 'live' ':' '"polite"' | '"assertive"'
                | 'alt' ':' expr                   (* 图像替代文本 *)
```

语义原语集（v1 范围）：label（可访问名称）/ role（角色）/ hidden（辅助技术隐藏）/ live（实时区域）/ alt（图像替代）。`required` 保持表单语义属性（不进本域）。

#### 手写对照样例

```tsx
// React 工程师手写
<button aria-label="Close" aria-hidden={!visible}>×</button>
<input aria-label="Search" role="searchbox" />

// Flutter 工程师手写
Semantics(label: "Close", container: true, child: IconButton(...))
// 隐藏
ExcludeSemantics(excluding: !visible, child: ...)
// 实时区域
Semantics(liveRegion: true, child: Text(status))
```

#### 映射表

| 语义 | React | Flutter | OmniView |
|------|-------|---------|----------|
| label | `aria-label` | `Semantics(label:)`/`semanticLabel` | aria-label 透传 |
| role | `role` | ARIA 规范角色子集（button/textbox/link/checkbox/radio/img → 对应 flags；其余 → warning） | role 透传 |
| hidden | `aria-hidden` | `ExcludeSemantics`（动态：条件包裹） | aria-hidden 透传 |
| live | `aria-live` | `Semantics(liveRegion:)` | aria-live 透传 |
| alt | `alt`（img） | `Image.alt`/`semanticLabel` | alt 透传 |

**Flutter role 子集**（Task 4.3 词汇 ARIA 化）：可映射角色为 WAI-ARIA 规范名（button/textbox/link/checkbox/radio/img）→ Semantics flags（内部映射：`textbox` → `textField: true`、`img` → `image: true`）；不可映射 → warning + 生成代码注释（诚实降级）。遗留 Flutter 旗标名（textField/image）→ warning + 建议规范名（textbox/img），codegen 仍内部映射保证既有源码不破（web 输出不再出现非法 `role="textField"`）。诊断表记录子集。

#### 验证清单

- [x] 三目标语义等价（Flutter 语义树断言：Semantics 节点 label/flag 存在）— Task 4.5：fixture `a11y.omni` 三目标生成形状断言 + `Semantics(label:)`/`ExcludeSemantics`/`liveRegion`/`semanticLabel` 存在性（`tests_regression/parameterized.rs` + `generated_code_validation.rs`）
- [x] 现有 demo a11y 属性（FormPage 8 处 aria-label 等）迁移到 a11y 块 + 三目标生效 — Task 4.5：`learn_omniview_formpage_a11y_migration_across_targets`（OmniView 属性不变 / React 无占位符重复 / Flutter 新出 Semantics）
- [x] 不可映射 role → warning + 注释 — Task 4.5：语义 pass warning + Flutter T2.4 warnings 通道 + 生成代码 `/* a11y: … */` 注释（`test_a11y_unmappable_role_warns_and_comments`）
- [x] a11y 加入 floor 表与文档 — Task 4.5：floor 表（设计文档 §10.1）+ 语法规范 §11

---

## 6. 设计顺序与实施节奏

| 阶段 | 内容 | 前置 | 验证出口 |
|------|------|------|---------|
| 0 | 前置基础设施（§4.4 八项） | 无 | react-test-app strict tsc 全绿；展示项目 Flutter 门禁绿；proptest 真断言 |
| 1 | 域 8 Expressions（地基） | 0 | 闭包/数组方法/match/style 通道 fixture 全绿 |
| 2 | 域 1 Ref + 域 7 Events | 1 | 双向绑定方向一致性；事件字段诊断 |
| 3 | 域 9 Styling + 域 5 Stores | 1 | 注入语料回归；effect deps 验证 |
| 4 | 域 3 Design+主题 + 域 10 a11y | 2,3 | 主题切换浏览器实证；语义树断言 |
| 5 | 域 2 动画 + 域 6 控制流 | 1,2 | @for 三目标契约；拖动/滚动驱动动画 |
| 6 | 域 4 Slots | 2 | slot 三目标对等 |
| 7 | 终验 | 全部 | learn-omniview 三目标全量 + 9 标签页 + 文档全对齐 |

每阶段结束：`cargo test --workspace` 全绿 + 该域门禁绿 + 文档/BNF/demo 三方一致（该域范围）。

## 7. 影响面（破坏性分析）

| 面 | 影响 |
|----|------|
| parser | 9 域 BNF 变更（删除/新增子句）；类名字符集校验 |
| semantic | validate_ref_ops（10 ops）、bind:value 语义、@theme/@themes 验证、effect deps 验证、对象项 key 警告、a11y 验证 |
| IR | RefOpIR 删 SetValue；ElementIR 增 a11y 字段；ListRenderingIntent key/scroll 三目标消费；design IR 增 themes/theme-signal；motion 触发器参数（scroll/drag 维度）；`$field` 表达式形式 |
| codegen | 三后端 × 各域（最大：Flutter @for/classes/主题接线；React 函数发射/切片订阅/注入转义） |
| 文档 | 语法规范 v2（逐域重写）、16 篇用户文档、设计文档 |
| demo | 92 文件迁移（bind 遗留/a11y 属性/主题 demo 接线/AdvancedTypesDemo 去类型） |
| deferrals | #17/#22 关闭；#30（match Flutter）随域 8 修复并关闭；#28（Map.set）与 #33（嵌套 #root）**未落地**——2026-08-03 终验复核（T7.3）确认「随域 8 处理」「随域 6/终验处理」承诺未兑现（`Map.set` 三后端仍 verbatim 发射 → 运行期 TypeError；OmniView innerHTML 仍重包 `#root`，`[data-component]` 2 命中）。**2026-08-05（v3 Task 6.7）处置**：移出本设计实现范围，保持 defer-with-trigger——状态与触发条件以 `.architecture/deferrals.md` #28/#33 行为准 |
| CI | 门禁扩展（§4.4）+ @if/@for fixture + 注入载荷回归 |

## 8. 伴随性修复清单（非语法，但随实现落地）

评审发现的 codegen/运行时修复不改变语法、但属于同一实现批次（逐项入实施计划）：
- React：C1 函数发射遍历补全、C2 样式通道、array_sort comparator、setX 函数式更新、Object.assign 清理、set() 合并、effect deps 从 body 分析、computed deps 含 props、fn 参数类型、typed useState
- Flutter：语义绑定快照比较、prop 类型转换层、_Omni* helper 共享化、store ==/hashCode、ref.measure 安全 cast、event 字段桩诊断
- 通用：WASM FFI 契约统一、Python/Vue BlockExpr 诊断、is_dangerous_url 移位、增量缓存诊断、motion IR `#[non_exhaustive]`、--all-features 修复
- 性能：React 切片订阅、Flutter 按元素 motion controller、const 传播、滚动驱动相等守卫 — Task 2.10：@scroll/@drag-* 进度相等守卫（`lastProgress`，进度未变不重 seek）+ seek duration 提升出 handler（单次 `getTiming()`）；React 切片订阅扩展至 motion 通道（motion 值中的 `Store.field` 经 `store_field_renames` 解析为 slice 局部，整 store 订阅回退消除——重渲染粒度与视图表达式一致；含 store 引用的 motion 保持函数内 `_M` 表，模块级表不可见组件作用域 slice 局部）

---

## 9. Feature Floor 机器化矩阵（设计 P5——「floor 内行为对等」从声明变断言）

> **制度（v3 修复计划 Phase 7，Task 7.3）**：ADR-052/054 的 24-item feature floor 曾是手写声明表——两轮评审各发现「声称 ✅ 而行为未接通」的条目（上轮主题切换、本轮 multi-theme Flutter 塌缩），「对等」无法证伪。本矩阵把每项 floor 条目硬编码进 `omnilang/tests/floor_matrix.rs` 的 `floor_table()`，覆盖检查 `every_floor_item_has_machine_verification` 强制每项映射 ≥1 编译级测试（参数化 fixture/单元测试）或行为级标注（widget test/Playwright + future 队列）——**新增/删除 floor 条目必须先挂机器验证**。
> **层级**：编译级（本版本第一版已全量标注——每行 ≥1 个既有/新增测试名）；行为级（widget test/Playwright——3.2 基建扩展队列；已存在的浏览器实证列在「行为级测试」列，未机器化的行为欠账列在「行为级 future」列，不阻塞编译级）。
> **权威清单**：`omnilang/tests/floor_matrix.rs` `floor_table()`（硬编码，25 行 = 24 规范项 + ADR-052 表列扩展原语 compound assignment——v1 §10.1 floor 未列，本矩阵收录保证零静默丢项）；下表与其逐行一致。运行：`cargo test -p omnilang --test floor_matrix`（Flutter 行需 `--features codegen-flutter`）。

| # | 类别 | floor 条目（ADR-052） | 矩阵 ID（floor_matrix.rs） | 编译级映射（测试名，文件::fn 或 fixture） | 行为级 |
|---|---|---|---|---|---|
| 1 | Ref | focus/blur | `ref_focus_blur` | `refs.omni`（parameterized::refs ×3 后端）+ `react::tests::test_react_ref_ops_format_native_dom_calls` | — |
| 2 | Ref | scrollTo/scrollBy | `ref_scroll_to_by` | `react::tests::test_react_ref_ops_format_native_dom_calls` + `react::tests::test_react_semantic_binding_scroll_x` + `flutter::tests::test_format_ref_op_flutter_variants` | — |
| 3 | Ref | width/height/position（measure） | `ref_measure` | `react::tests::test_react_ref_ops_format_native_dom_calls` + `flutter::tests::test_task58_measure_emits_wrapper` | — |
| 4 | Ref | select/setSelection | `ref_select_selection` | `react::tests::test_react_ref_ops_format_native_dom_calls` + `flutter::tests::test_format_ref_op_flutter_variants` | — |
| 5 | Ref | value/setValue | `ref_value` | `refs.omni`（bind:value 双向）+ `react::tests::test_react_bind_value_emits_onchange_setter` + `flutter::tests::test_bind_value_ref_text_field_two_way` | 已存在：Playwright 输入回写实证（T2.3）；future：widget-test 队列（3.2）双向行为断言 |
| 6 | Animation | 8 triggers | `anim_triggers` | `react::motion::tests::test_react_motion_enter/hover/inview/scroll/drag` + `focus_trigger_emits_focusin_focusout_listeners` + `flutter::tests::scroll_trigger_passes_typed_start_end_to_widget` 等 8 项 | 已存在：node @scroll 实证 + Playwright @drag 300px（T5.1/T7.1）；future：widget-test 队列触发器执行断言 |
| 7 | Animation | multi-stop multi-property keyframes | `anim_multi_stop_keyframes` | `flutter::tests::test_build_prop_tween_expr_three_stops_sequence` + `motion_full_pipeline_per_property_and_playback` + `react::motion::tests::semantic_props_convert_to_waapi_keyframes` | — |
| 8 | Animation | playback（loop/count/direction/delay/fill/duration/easing） | `anim_playback` | `flutter::tests::test_emit_playback_start_loop/direction/fill/delay` + `test_motion_pipeline_warnings_and_playback` + `react::motion::tests::motion_durations_emit_numeric_ms` + `steps_easing_emitted_as_waapi_string` | — |
| 9 | Animation | inline body lowering | `anim_inline_lowering` | `animation.omni`（内联 tween → __anon_m_N 提升）+ `flutter::tests::test_motion_inline_body_handling` | — |
| 10 | Design | 13 token types | `design_token_types` | `design_parse_test::parse_all_token_types` + `design_tokens.omni` | — |
| 11 | Design | static tokens + themes | `design_static_themes` | `design_tokens.omni`（dark 覆盖）+ `tests_regression::main::themes_fixture_wires_theme_signal_through_pipeline` + `react::tests::react_design_theme_signal_wires_subscription` + `theme_initial_consistency::default_theme_mismatch_errors` | 已存在：Playwright theme-wiring spec（T4.2/T7.1）；future：widget-test 队列 ThemeMode 切换断言 |
| 12 | Design | classes | `design_classes` | `design_tokens.omni`（Card）+ `react::tests::react_design_classes_emit_literal_class_names` + `flutter::tests::test_design_class_resolves_to_factory_decoration` | — |
| 13 | Design | variants | `design_variants` | `design_tokens.omni`（Button.secondary）+ `flutter::tests::test_design_variant_resolves_to_real_decoration` | — |
| 14 | Design | dynamic tokens | `design_dynamic_tokens` | `react::tests::react_design_dynamic_tokens_emit_hook` + `design_codegen_test::e2e_dynamic_token_full_pipeline` + `design_codegen_test::emits_js_effect_for_dynamic_token` | — |
| 15 | Slots | default + named + fallback | `slots` | `slots.omni` + `slots_body.omni` + `slots_three_forms.omni`（parameterized ×3 后端） | — |
| 16 | Control Flow | @if ternary + transition_mode | `control_if` | `control_flow.omni`（fade）+ `react::tests::conditional_fade_mode_emits_css_fade_transition` + `conditional_wait_mode_degrades_with_stated_comment` + `flutter::tests::page_switcher_uses_indexed_stack` + `omniview::tests::test_html_conditional_wait_mode_is_honest_instant_swap` | 已存在：Playwright 9 标签页条件渲染实测（T7.1） |
| 17 | Control Flow | @for + key_expr + empty_body + [scroll] | `control_for` | `control_flow.omni` + `parameterized::flutter_control_flow_contract` + `react::tests::react_for_key_expr_emits_expr_key` + `react_for_empty_body_single_eval` + `flutter::tests::flutter_for_scroll_emits_listview_builder_with_value_key` + `flutter_for_non_scroll_emits_column` | 已存在：Playwright keyed 保状态（T7.1，BUG-1/BUG-3 回归闭环）；future：widget-test 队列重排保状态断言 |
| 18 | Stores | signals, getters, actions, effects | `stores` | `store.omni` + `react::tests::test_zustand_store_with_signals_and_getters` + `flutter::tests::test_store_dart_with_getters` + `test_store_dart_with_action` + `effect_deps.omni`（effect deps 验证） | — |
| 19 | Lifecycle | mount, unmount, effect with deps | `lifecycle` | `react::tests::test_tsx_component_with_lifecycle_mount` + `effect_cleanup_reads_are_deps` + `flutter::tests::test_component_with_mount/unmount_lifecycle` + `test_effect_emits_real_method_and_lifecycle_wiring` + `omniview::tests::test_component_with_lifecycle_mount` | — |
| 20 | Expressions | OptionalChain | `expr_optional_chain` | `omnilang-core::codegen::shared::expr::optional_chain_default_preserves_question_dot` + `dart_optional_chain_keeps_question_dot` + `react::tests::react_optional_chain_method_len_translates_to_length` | — |
| 21 | Expressions | BlockExpr IIFE | `expr_block_expr` | `omnilang-core::codegen::shared::expr::dart_block_expr_emits_iife` + `rust_block_expr_emits_block_expression` + `tests_regression::main::react_block_expr_statements_compile_clean` + `python/vue_block_expr_statements_diagnosed_not_dropped`（floor 外硬诊断） | — |
| 22 | Expressions | MatchExpr | `expr_match` | `js_runtime_tests::js_runtime_match_literal_arms_pass`（执行级）+ `react::tests::fn_tail_match_emits_return` + `generated_code_validation::match_variant_arms_flutter_compiles`（#30 变体臂） | — |
| 23 | Expressions | Await | `expr_await` | **新增 fixture** `floor_matrix::await_compile_tier_fixture`（async action + await 三 P0 后端）+ `js_runtime_tests::js_runtime_stdlib_gap_modules_pass`（执行级）+ showcase 门禁（DataFetching/DataSubmit/async_demo 三目标） | future：Playwright DataFetching 成功路径（2.3 un-fixme 候选）+ widget-test 队列（3.2） |
| 24 | Expressions | Spread | `expr_spread` | `tests_regression::main::react_store_action_consecutive_sets_coalesce_into_one`（`[...items, text]`）+ `flutter::tests::store_action_struct_literal_emits_constructor`（`[...todos]`）+ `omnilang-core::codegen::shared::expr::dart_concat_spreads_typed_array_variable` | — |
| 25 | Expressions | compound assignment（ADR-052 表列；v1 §10.1 未列——扩展收录） | `expr_compound_assign` | **新增 fixture** `floor_matrix::compound_assign_compile_tier_fixture`（`count += 1` 三 P0 后端）+ `omnilang-core::parser::mod::stmt_compound_assign_add/sub` + `flutter::tests::test_stmt_mutation_increment` + `omniview::tests::test_stmt_mutation_increment` + `react::tests::test_tsx_stmt_mutation_increment_signal` | — |

**行为级 future 队列汇总（3.2 基建 widget test/Playwright 扩展）**：`ref_value`（bind:value 双向行为断言）、`anim_triggers`（触发器执行断言）、`design_static_themes`（ThemeMode 切换）、`control_for`（重排保状态）、`expr_await`（DataFetching 成功路径）。其余 20 行编译级全量覆盖，行为级无欠账。

---

## References

- `.architecture/reviews/feature-omniview-semantic-syntax-compilability.md`（本设计的评审依据）
- ADR-052/053/054/055、ADR-021（floor + 硬诊断）
- `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md`（v1 BNF，逐域重写为 v2）
- `docs/superpowers/plans/2026-07-31-semantic-level-dsl-design.md`（v1 设计，@scroll/@drag 语义定义沿用）
- `.architecture/deferrals.md`（#17-#33）
