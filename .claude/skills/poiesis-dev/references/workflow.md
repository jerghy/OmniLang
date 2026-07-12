# 开发工作流与调试

## 一、标准开发流程

### 1.1 启动环境

```
/start-poiesis  →  等脚本完成  →  Victauri MCP 连接  →  开始工作
```

启动脚本做的事（`start-poiesis.sh`）：
1. 杀旧 `poiesis.exe` + 僵死 `cargo.exe` 进程
2. `npx tauri dev`（Vite + Rust 编译 + 启动窗口）
3. 循环等 `poiesis.exe` 出现（最长 120s）
4. 日志写 `/tmp/poiesis.log`

### 1.2 修改代码的标准流程

```
1. 理解问题
   ├─ Victauri screenshot → 看 UI 现状
   ├─ dom_snapshot → 看 DOM 结构
   └─ eval_js → 读运行时数据 (computed styles, bounding boxes, className)

2. 定位代码
   ├─ Grep 搜索相关组件/逻辑
   ├─ 参考 references/file-map.md 找文件
   └─ Read 相关文件理解逻辑

3. 修改代码
   ├─ Edit / Write 修改源文件
   └─ 遵循往返闭环原则：改 parser 必检查 emitter

4. 验证
   ├─ Victauri: eval_js 验证数据 / screenshot 验证 UI
   ├─ 点编辑器内"保存项目"触发编译
   └─ 确认画布预览正确
```

### 1.3 修改 .omni 文件的特殊流程

```
直接改磁盘 .omni 文件:
  1. 在编辑器中关闭对应 Tab
  2. 用 Edit/Write 修改磁盘文件
  3. 在编辑器中重新打开 Tab（触发 parser 重解析）
  4. 验证画布渲染

通过编辑器 UI 修改:
  1. 用 VariantEditor/ClassEditor/ContextInspector 修改
  2. 点"保存项目"
  3. 自动触发重编译 + 画布刷新
```

## 二、Victauri 调试工具箱

### 2.1 核心工具速查

| 工具 | 一句话 | 典型用法 |
|------|--------|---------|
| `dom_snapshot` | 查看完整可访问 DOM 树 | 理解 UI 结构、找 ref_id |
| `screenshot` | 窗口截图 PNG | 视觉验证 |
| `eval_js` | 执行任意 JS | 读 computed styles/data/state |
| `find_elements` | 按 text/role/css/test_id 找元素 | 定位目标元素 |
| `interact` | click/hover/focus/scroll | 操作 UI |
| `input` | fill/type_text/press_key | 填表单 |
| `wait_for` | 等条件满足 | 等编译完成/等元素出现 |
| `inspect get_styles` | 读 computed CSS | 验证渲染结果 |
| `inspect get_bounding_boxes` | 读元素位置/尺寸 | 排查布局问题 |
| `invoke_command` | 调 Tauri 后端命令 | 读文件/编译/保存 |
| `introspect command_catalog` | 列出所有可用命令 | 发现后端能力 |
| `logs console` | 读 console.log | 看前端日志 |
| `explain last_action` | 最近事件因果链 | 理解"发生了什么" |

### 2.2 经典调试序列

**排查 UI 渲染问题：**
```
dom_snapshot → 确认元素存在
  → eval_js: document.querySelector('...').getBoundingClientRect()
  → eval_js: getComputedStyle(el).padding (看为什么 padding 不对)
  → inspect get_styles: 确认 computed CSS
  → 定位源码 → 修改 → screenshot 验证
```

**排查数据问题：**
```
eval_js: 读 Store 状态
  vs invoke_command read_file: 读磁盘 .omni
  → 对比差异 → 确定是内存问题还是磁盘问题
```

**排查编译问题：**
```
invoke_command compile_project → 看 errors
  或 WASM: OmniCompiler.diagnostics() → 看诊断
```

### 2.3 eval_js 常用脚本

```javascript
// 读元素渲染信息
(() => {
  const btns = document.querySelectorAll('button');
  return Array.from(btns).map(b => ({
    text: b.textContent.trim(),
    className: b.className,
    rect: b.getBoundingClientRect(),
    computed: {
      padding: getComputedStyle(b).padding,
      fontSize: getComputedStyle(b).fontSize,
      height: getComputedStyle(b).height
    }
  }));
})()

// 读所有 token() 引用
(() => {
  const spans = document.querySelectorAll('span');
  return Array.from(spans)
    .map(s => s.textContent.trim())
    .filter(t => t.startsWith('token('));
})()

// 读 Store 中某个 variant 的数据
// 读 Store 数据（store/index.ts L112-114 暴露了全局引用）
(() => {
  const s = window.__POIESIS_STORE__.getState();
  return {
    page: s.currentPage,
    codeFlowVisible: s.designPage?.codeFlowVisible,
    elements: s.elements?.length,
    componentDefs: Object.fromEntries(
      Object.entries(s.componentDefs || {}).map(([k,v]) => [k, {
        props: v.props?.length,
        signals: v.signals?.length,
        computeds: v.computeds?.length,
        hasElements: !!(v).elements
      }])
    ),
    activeTabType: s.designPage?.openTabs?.find(
      t => t.id === s.designPage?.activeTabId
    )?.tabType
  };
})()

// 检查特定组件的 props/signals/computeds
(() => {
  const s = window.__POIESIS_STORE__.getState();
  const def = s.componentDefs?.main;
  if (!def) return 'componentDefs.main undefined';
  return {
    props: def.props?.map(p => p.name + ':' + p.type),
    signals: def.signals?.map(s => s.name + ':' + s.type),
    computeds: def.computeds?.map(c => c.name + ':' + c.returnType + '=' + c.expression)
  };
})()

// 读 Store 中某个 variant 的数据
// (需要 app 暴露 store)
```

### 2.4 等待异步操作

```javascript
// ❌ 不要用固定 sleep
// ✅ 用 wait_for

// 等编译完成（状态栏显示"就绪"）
wait_for { condition: "text", value: "就绪", timeout_ms: 10000 }

// 等元素出现
wait_for { condition: "selector", value: ".Button.danger.lg" }

// 等异步命令完成
wait_for { condition: "expression", value: "document.querySelector('.status')?.textContent === 'done'" }

// 等 IPC 空闲
wait_for { condition: "ipc_idle", timeout_ms: 5000 }
```

## 三、文件修改的影响范围

### 3.1 类型定义变更

修改 `core/types/design.ts` → 影响：
- `store/types.ts`（PoiesisState 接口）
- `store/slices/design-system.ts`（action 实现）
- `services/codegen/design-emitter.ts`（生成端）
- `services/codegen/parser.ts`（解析端）
- 所有消费这些类型的组件

### 3.2 Variant 分支修改

修改 `VariantEditor.tsx` 中分支编辑逻辑 → 影响：
- `design-emitter.ts` 发射时对应分支的 emit 顺序
- `parser.ts` 解析时分支的归属
- 画布预览的 CSS class 生成

### 3.3 属性卡片修改

修改 `cards/*.tsx` → 影响：
- `ClassEditor.tsx`（复用同一套卡片）
- `VariantEditor.tsx`（复用同一套卡片）
- `ContextInspector.tsx` 中的内联样式编辑（部分复用）

## 四、构建与测试

### 4.0 Debug 窗口架构要点

Debug 窗口（`label: "debug"`）是预创建的隐藏 WebView，通过按钮 show/hide 而非动态创建/销毁：

- **预创建**：`lib.rs` L898-913 在 `setup()` 中用 `WebviewWindowBuilder::new(app, "debug", ...).visible(false)` 预创建
- **打开**：`open_debug_window` 命令调用 `window.show()` + `set_focus()`
- **关闭**：`close_debug_window` 命令调用 `window.hide()`（不销毁——重建会阻塞 IPC）
- **防自动恢复**：`lib.rs` L894 — `tauri_plugin_window_state::Builder::default().with_denylist(&["debug"])` 防止上次会话的可见状态在新启动时恢复
- **已保存的旧状态**：手动清理 `%APPDATA%/com.omnilang.poiesis/.window-state.json` 中的 `"debug"` 条目

通信走 Tauri 事件总线（不是 `postMessage`）：
- Main → Debug：`debug:compile-result`（全量）、`debug:hot-replace`（增量）、`debug:highlight`（高亮）
- Debug → Main：`debug:window-ready`（就绪）、`debug:element-click`（元素选择）、`debug:bridge-ready`（bridge 就绪）

### 4.1 构建命令

```bash
# 仅前端检查
cd poiesis && npx tsc --noEmit

# 前端构建
cd poiesis && npm run build

# Rust 检查
cd poiesis/src-tauri && cargo check

# 完整构建（通过 Tauri CLI）
cd poiesis && npx tauri build

# 开发模式（你通常用这个）
# 由 start-poiesis.sh 自动执行
```

### 4.2 测试

```bash
# 前端单元测试
cd poiesis && npx vitest run

# 特定测试文件
cd poiesis && npx vitest run src/store/__tests__/editor-helpers.test.ts

# Rust 测试
cd poiesis/src-tauri && cargo test
```

### 4.3 构建失败常见原因

| 症状 | 常见原因 |
|------|---------|
| TS 类型错误 | 修改了类型定义但没更新所有使用处 |
| WASM 编译失败 | .omni 语法有误 → 用 `diagnostics()` 检查 |
| Rust 编译失败 | omnilang crate API 变更 → 更新 `lib.rs` 调用 |
| Vite 构建失败 | import 路径错误 / 循环依赖 |

## 五、往返闭环验证清单

修改了解析或生成代码后，检查：

- [ ] 新建一个 variant，保存 → 重新打开 → variant 还在，属性完整
- [ ] 修改 variant 属性，保存 → `.omni` 文件内容正确
- [ ] 手动在 `.omni` 中加属性 → 重开 Tab → Store 中属性存在
- [ ] token 引用 `token(xxx)` 往返后保持不变
- [ ] PID 注释 `// @variant id=xxx` 往返后保持相同 ID

## 六、常见 Bug 模式

### 6.1 "改了但没生效"

```
可能原因:
1. 改的是磁盘文件，编辑器用的是内存 → 需要重载 Tab
2. 改了 emitter 没改 parser（或反过来）→ 往返不同步
3. 改了 Store action 但组件没重渲染 → 检查 zustand selector
4. WASM 缓存没失效 → 关掉 Tab 重开触发重新编译
```

### 6.2 "值正确但渲染不对"

```
可能原因:
1. CSS 属性名用错了格式（kebab vs camelCase）
2. Variant 合并逻辑有 bug（分支覆盖顺序不对）
3. CSS specificity 问题——被更高优先级的规则覆盖
4. StyleValueSegment 解析/发射有 bug → 检查 compileDesignSegments
```

### 6.3 "保存后数据丢失"

```
可能原因:
1. emitter 生成不完整 → 检查 pipeline 各阶段
2. PID 分配冲突 → 检查 pidIndex 去重逻辑
3. project.json 被破损 → check JSON.parse
```
