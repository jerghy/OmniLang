# ADR-072: 调试窗口直载编译产物——P38 沙箱 iframe 决策反转

## Status

Accepted

## Context

用户拍板（2026-08-07）：

> 「调试窗口用iframe的意义好像不大，也为后面更多功能设阻碍，它一个调试自己写的应用应该不用那么多安全限制，直接就是 WASM 多文件编译为 HTML，加载到这个网页里面就行了，而且 wasm 里面还有很多原子化功能，可以加很多调试功能的代码」

### 沙箱 iframe 架构的由来

ADR-010（P38b）为防 XSS 升级链（`postMessage('*')` → eval(comp.js) → innerHTML 注入 → 全 Tauri API 访问），在调试窗口内嵌 `sandbox="allow-scripts"`（无 allow-same-origin）iframe 承载编译产物，`__TAURI_INTERNALS__` 留在父窗口，应用代码与调试壳通过 postMessage 桥接（compile-result / hot-replace / element-click / highlight / window-ready 五类消息）。

### 沙箱 iframe 的实害（真机暴露）

用户报「调试窗口点击按钮没有效果」——根因调查发现**捕获阶段 click 拦截器吞事件**：旧桥在 document 捕获阶段无条件 `stopPropagation()` 转发 element-click，把 iframe 内应用的**全部点击**吞掉（按钮监听器永远收不到）。这是桥接层的结构性缺陷：拦截转发逻辑与应用事件共享同一事件流，无隔离界面。

### 沙箱安全价值与信任边界矛盾

- 编辑器本体是「可信开发环境」（ADR-028 已接受 XSS 边界：编辑器内任何 XSS 即可执行任意代码）
- 调试窗口只有 `core:event:allow-emit` + `core:event:allow-listen` 两个能力——编译产物即使逃逸沙箱也**触碰不到**文件系统/命令等敏感面，事件面是调试功能本身的传输层
- 沙箱防的是「用户代码访问 Tauri API」，但调试窗口的唯一 Tauri 面就是事件收发——沙箱护住的正是调试功能要用的能力，隔离价值趋近于零

### 直载的额外收益

- 调试壳与编译产物同文档：inspector 直读 `window.__omni_subs`（runtime v2 信号 Map），highlight 直接 DOM 操作，无需消息往返
- 桥接层消除 → 事件流干净 → 应用按钮正常响应（真机验证）
- WASM 编译器侧原子化功能（nidMap、信号注册表等）可直接在窗口内被调试功能消费，未来调试原语（信号跟踪、DOM 检查）放编译器侧
- 消除 srcdoc 流程的双重执行缺陷（旧方案尾部再拼 payload.js，app JS 跑两遍）

## Decision Drivers

* **用户拍板**：调试自己写的应用不需要沙箱级安全限制
* **真机根因**：桥接层捕获阶段 stopPropagation 吞掉应用全部点击（用户问题的直接元凶之一）
* **信任边界一致**：编辑器=可信开发环境（ADR-028），调试窗口事件能力面小，沙箱隔离价值≈0
* **功能可扩展**：直载后同文档，调试功能（inspector/highlight/hot-replace）免协议维护；WASM 侧调试原语可加入
* **权限面事实**：调试窗口 capabilities 只有事件两权限，编译产物无命令面可触碰

## Decision

**1. 移除调试窗口的沙箱 iframe 与 postMessage 桥接层**：WASM 多文件编译的 HTML 输出直载调试窗口页面本体（`#root` 容器）。编译产物与调试壳同文档、同信任边界（=编辑器可信开发环境）。

**2. 直载契约（applyCompile）**：
| 产物段 | 处理 |
|---|---|
| CSS | 写入 `<style id="omni-styles">`（幂等替换） |
| HTML 正文 | 按文档序剥离 `<script>` 后插入 `#root`；若整体恰为单个 `<div id="root">` 占位则解包（避免重复 id） |
| JS 脚本 | 按文档序 `(0, eval)` 执行，逐个 try/catch → compile-info 报错 |
| payload.js | 仅当 html 段无脚本时回退（消除双重执行） |

**3. 安全姿态调整**：编译产物获得调试窗口页面同等事件能力（emit/listen）——这是调试功能本身需要的面。命令面不变（capabilities 不扩张）。生产 CSP `script-src 'self'` 阻止 inline eval 为 pre-existing 限制（dev 为唯一工作流，Vite 响应无 CSP）。

**4. 调试功能同文档化**：
- inspector 直读 `window.__omni_subs`（runtime v2 Map，含 peek）；`__omni_signals` 恒空仅作旧产物回退
- element-select：仅在 inspector 激活时注册捕获阶段拦截（stopPropagation 不 preventDefault，保留输入聚焦），非激活态零介入
- highlight：直接 `[data-nid]` DOM 操作
- hot-replace：直接 eval 组件 JS + 保存/恢复信号（旧桥同款逻辑）

**5. 暴露调试产物**：`window.__omni_loadedScripts = scripts`（最近一次编译的脚本数组，排查编译产物用）。

**Architectural Components Affected:**
* 调试窗口 shell：`poiesis/public/debug-window/index.html`（dev 加载路径）+ `poiesis/src-tauri/debug-window/index.html`（打包路径）——两份同步副本，重写为直载 shell
* 桥接协议（debug:compile-result / hot-replace / element-click / highlight / window-ready）：消息面保留（Tauri 事件协议不变，主窗口侧 useDebugWindow/useDebugCompile 零改动），仅承载端从「iframe+postMessage」变为「同文档 listener」
* 沙箱层：`sandbox="allow-scripts"` iframe 移除（ADR-010 P38b 的结构性载体）

**Interface Changes:**
* Tauri 事件协议：**不变**（五类消息、payload 形状、window-ready counter 语义全部保持）
* 语言/编译器：**无**（.omni 源码与 WASM 编译产物零改动）
* 能力面：调试窗口 capabilities 不变（事件两权限）

## Consequences

### Positive

* **应用按钮正常响应**（真机端到端验证：input→信号→@if 联动、真实点击→信号赋值→effect 全链路通过）
* **调试功能免桥接**：inspector 直读信号、highlight 直操作 DOM、hot-replace 直接 eval——协议维护面消除
* **未来调试功能无障碍**：WASM 侧原子化调试原语可加（信号跟踪、DOM 检查、nidMap 消费），同文档直取
* **无双重执行**：srcdoc 尾部拼 payload.js 的双执行缺陷随桥接层一并消除
* **单文档心智模型**：无需跨上下文（iframe+parent）推理，调试壳代码量下降

### Negative

* **编译产物与调试壳同文档**：产物 JS 可触达壳的 DOM/事件面——信任边界从「产物隔离」变为「编辑器信任」（ADR-028 同款判断：编辑器本身即可执行任意代码，调试窗口不新增敏感能力）
* **生产 CSP 限制依旧**：`script-src 'self'` 阻止 inline eval，生产包调试窗口无法加载应用 JS（pre-existing，非本决策引入；dev 是唯一工作流）
* **编译产物 debug bridge 与壳并存**：产物自带 `__omni_debug` 桥（emit bridge-ready/nidMap，listen highlight 因 args 形状失败）——无害但存在噪音（后续可让编译器侧 debug 原语统一接管）

### Neutral

* Tauri 事件协议不变，主窗口侧（useDebugWindow/useDebugCompile）零改动
* 两份 shell 副本同步纪律沿用（cp + diff 校验）

## Implementation Strategy

### Blast Radius

**Impact Scope**: 小——单窗口 shell 重写（两份副本），主窗口侧零改动，编译器零改动。真机验证发现 1 个 pre-existing 编译器 bug（见 Validation）与本决策无关。

**Affected Components**:
- `poiesis/public/debug-window/index.html`（dev 路径）
- `poiesis/src-tauri/debug-window/index.html`（打包路径，与前者同内容）
- 移除：旧 srcdoc 组装、iframe 承载、postMessage 桥

**Risk Mitigation**:
- 两份副本以 cp + diff 保证一致（防走查漂移）
- 真机端到端验证（编译推送 → 直载 → 交互 → 信号联动 → inspector 读值 → element-select 事件）
- 事件协议不变 → 主窗口回归面为零

### Reversibility

**Reversibility Level**: 高——shell 单文件可整体回退到 iframe 版本；事件协议未变，主窗口侧无需同步回退。

### Sequencing & Timing

已完成（本 ADR 即实施记录）：
1. shell 重写（applyCompile 直载 + inspector 直读 + 拦截守卫化）
2. 真机验证（见 Validation）
3. 本文档固化决策

## Implementation

**已实施（2026-08-07）**：
- `public/debug-window/index.html` 重写为直载 shell（约 400 行）：applyCompile、refreshInspector 直读 `__omni_subs`、element-select 激活态拦截、highlight 直操作、hot-replace 直接 eval、window-ready 保留
- `src-tauri/debug-window/index.html` 同步（cp + diff IDENTICAL）
- 旧桥移除点：iframe 元素、inspector-overlay、postMessage 发送/接收、捕获阶段无条件 stopPropagation

**真机验证结果（Victauri，demo2 项目）**：
- 直载路径：3 组件注册、信号初始 "signalcs/"、scripts:2 无双份执行、primary disabled 正确、@if 正确隐藏
- 端到端交互：input 事件 → `newSignal="demo"` → @if 显示（display:block）+ p 文本联动；清空 → primary 解锁；真实 OS 级点击 primary → `newSignal="点击成功"` → p 更新 + @if 隐藏
- inspector：点 ball → 面板显示 `Main_newSignal: 点击成功`（直读 __omni_subs）
- element-select：激活态点击 p → `debug:element-click {nid:"el-p-003"}` 正确发出
- **发现的 pre-existing 编译器 bug（与 shell 无关）**：`MyCard_id_el_MyCard_015` 信号从未注册 → 每次加载 `effect:run TypeError: Cannot read properties of undefined (reading 'set')`——id 绑定引用未注册信号，待编译器侧处置（记录在案，另行修复）

**用户原始问题「点击没效果」的完整根因链**：
1. 页面自身 `disabled={newSignal}` 且初始值 "signalcs/" 为 truthy → 主按钮加载即禁用（设计如此）
2. 旧 postMessage 桥捕获阶段 `stopPropagation()` 吞掉 iframe 内全部点击（结构性缺陷，本 ADR 根治）
3. danger 按钮当前源码无 @onClick（btnFn 定义了但未被引用）→ 点击无效果是源码如此（非 bug）

## Alternatives Considered

### 保留沙箱 iframe，仅修拦截器

**Pros:**
* 保住 P38b 沙箱边界

**Cons:**
* 用户已拍板移除 iframe；拦截器修复治标不治本（桥接层的结构缺陷仍在：任何捕获阶段逻辑都会与 app 事件流纠缠）；未来调试功能继续受限（跨文档消息协议每功能都要扩）

### 直载 + 保留 postMessage 桥（仅换承载端）

**Pros:**
* 改动最小

**Cons:**
* 同文档后桥已无存在意义（直读/直操作皆可）；保留桥 = 保留事件流纠缠风险与协议维护面

### 提权调试窗口 capabilities（放开命令面）

**Pros:**
* 编译产物可直接调 Tauri 命令

**Cons:**
* 超出需求（用户只要求直载 HTML）；事件面已够调试功能用；能力面扩张违背最小权限

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 本决策是**移除一个已证实阻碍功能的抽象层**（沙箱 iframe + postMessage 桥）——减法决策。真机根因（拦截器吞点击）与用户拍板双重驱动，零新增抽象、零新依赖、零编译器改动。

**Decision Challenge**:

**Proposed Decision**: "移除调试窗口沙箱 iframe 与 postMessage 桥，WASM 编译产物直载页面本体"

**Necessity Assessment**: 10/10
- **Current need**: 用户拍板 + 真机 bug 根因（吞点击）结构性修复——两重压力并存
- **Future need**: WASM 侧调试原子功能同文档可消费；inspector/highlight 免协议维护
- **Cost of waiting**: 每个新调试功能都要设计桥协议消息；吞点击类事件流纠缠风险持续存在
- **Evidence of need**: 用户明确指令 + 端到端根因链实证（点击到达 document 但无 listener 触发）

**Complexity Assessment**: 1/10
- **Added complexity**: 负——移除抽象层（iframe、桥、双文档状态同步），shell 单文件
- **Maintenance burden**: 低——同文档直读取代协议维护
- **Learning curve**: 低——删除为主，保留的事件协议语义不变
- **Dependencies introduced**: 零

**Alternative Analysis**: 三项替代均不优：修拦截器治标不治本；同文档保留桥自相矛盾；提权能力面违反最小权限。减法是唯一路径。

**Simpler Alternative Proposal**: 无更简路径——「移除抽象层」即最小决策；能力面与事件协议均不动。

**Recommendation**: ✅ Approve decision

**Justification**: 用户拍板 + 根因实证 + 纯减法零新增。事件协议保持使主窗口回归面为零，直载契约显式文档化防未来误用。

**Pragmatic Score**:
- **Necessity**: 10/10
- **Complexity**: 1/10
- **Ratio**: 0.10 *(Target: <1.5)*

**Overall Assessment**: 适当工程——删除阻碍性抽象，架构变简，功能变通。无过度设计。

## Validation

**Acceptance Criteria:**
- [x] 编译产物直载 #root，CSS/HTML/脚本按契约落位，无双份执行
- [x] 应用交互全链路：input → 信号 → @if 联动；真实点击 → 信号赋值 → effect 更新
- [x] inspector 面板直读 `__omni_subs` 显示信号实时值
- [x] element-select 激活态正确发事件；非激活态零介入（按钮可点）
- [x] 主窗口侧（useDebugWindow/useDebugCompile）零改动且编译推送循环正常
- [ ] （pre-existing 编译器 bug，另立项）`MyCard_id_el_MyCard_015` 未注册信号 effect 错误

**Testing Approach:**
* Victauri 真机端到端（eval_js 驱动 + 真实 OS 级点击 + IPC 日志核对事件）
* 两份 shell 副本 diff 校验
* 主窗口编译推送循环回归（window-ready → 150ms debounce → full push）

## References

* [ADR-010 P38b + P39 Strategy](ADR-010-p38b-p39-security-and-optimization-strategy.md) — 沙箱 iframe + postMessage 桥的决策源（本 ADR 反转其承载层，保留其「剥离 Tauri 命令面」精神）
* [ADR-028 OmniView Debug Runtime Design Tradeoffs](ADR-028-omniview-debug-runtime-design-tradeoffs.md) — 编辑器=可信开发环境的信任边界判断
* [ADR-071 ref 语义级语法的编译器责任模式](ADR-071-ref-semantic-compiler-responsibility.md) — 编译器侧原子功能/调试原语定位
* [memory: session-2026-08-07-ref-flexible-ops](memory://session-2026-08-07-ref-flexible-ops) — 会话实况
* [memory: poiesis-tauri-security-model](memory://poiesis-tauri-security-model) — 编辑器安全模型（本决策后需更新）
