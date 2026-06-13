# OmniLang 可视化编辑器 — 执行计划

> **状态：** 基于 [visual-editor-design.md](visual-editor-design.md) v1.4 + 五轮 Council 决策 + 两轮代码评审修正
> **日期：** 2026-06-12
> **形态：** Web 应用（`omnic serve` + 浏览器 UI）
> **总工期估算：** 24-30 周（M1 拆分为 M1a+M1b）
> **修正记录：** R1(F1-F6)—事实修正；R2—M1 拆分为集成 spike(M1a)+编辑器基础(M1b)；ElementData 升级 nodeType 判别器+预留 M3+ 槽位；补充 G1-G5 规格(编译 API 位置/错误协议/推送协议/依赖声明/项目根配置)

---

## §1 架构总览

### 1.1 运行时拓扑

```
用户桌面
├── omnic serve (Python 进程, 用户终端启动)
│   ├── Python 运行时 (用户已安装)
│   ├── OmniLang 编译器 (omnilang + omniview)
│   ├── FastAPI HTTP 服务
│   │   ├── GET  /              → 编辑器 SPA (React, 静态文件)
│   │   ├── POST /api/compile   → 编译 .omni → 目标代码
│   │   ├── GET  /api/fs/*      → 文件浏览/读取
│   │   ├── POST /api/fs/*      → 文件写入 (原子写入)
│   │   ├── WS   /api/watch     → 实时编译推送 + heartbeat
│   │   └── GET  /preview/*     → 反向代理到 Vite dev server
│   └── Vite dev server (管理预览用的 npm 项目)
│
└── 浏览器 (localhost:5173)
    └── React 18 SPA
        ├── 画布 (布局 + 连线)
        ├── 属性面板
        ├── 组件面板
        ├── 代码面板 + 代码片段
        ├── 组件树面板
        ├── 设计令牌面板
        └── IndexedDB (撤销栈 + 编辑器状态)
```

### 1.2 通信协议

```
浏览器 ←→ omnic serve 之间仅两条通道:

1. HTTP REST (请求-响应):
   /api/compile        POST { source, target } → { files: { path: content } }
   /api/fs/list        POST { dirPath }       → { entries: [...] }
   /api/fs/read        POST { filePath }      → { content }
   /api/fs/write       POST { filePath, content } → { ok: true }
   /api/init-project   POST { name, target }  → { projectDir }
   /api/deploy         POST { projectDir, platform } → { url }

2. WebSocket (推送 + heartbeat):
   /api/watch          启动文件监听
   → { type: "compiled", files: {...}, diff: {...} }  ← 编译结果推送
   → { type: "error", message: "..." }                ← 编译错误推送
   ← { type: "ping" }                                  ← 浏览器 heartbeat (每 5s)
   → { type: "shutdown" }                              ← 3 次 missed → 自我退出

心跳协议（解决孤儿进程 CRT-1）:
- 浏览器每 5s 发送 WebSocket ping
- 服务端连续 3 次未收到 ping（15s）→ 开始优雅退出倒计时（30s）
- 30s 内如果恢复 → 取消退出
- 30s 超时 → sys.exit(0)
```

### 1.3 文件写入协议（解决竞态 CRT-3）

```
所有 .omni 文件写入使用原子写入:

1. 浏览器 POST /api/fs/write { path: "src/App.omni", content: "..." }
2. 服务端:
   a. 写入临时文件: src/App.omni.tmp.{uuid}
   b. fsync 强制刷盘
   c. os.rename(tmp, src/App.omni)  ← 原子操作
   d. 返回 { ok: true }
3. 浏览器收到响应后，发送 /api/compile { file: "src/App.omni" }
4. 服务端此时读取的 .omni 保证是完整文件
```

### 1.4 跨包整合规格（G1-G5，第二轮评审补充）

**G1 — 编译 API 位置。** `build_command()` 重构后的共享编译入口放在 **`omniview/api.py`**（新文件）。`omniview/cli.py` 和 `editor/server/api/compile.py` 共同导入，避免 editor→omniview 反向依赖或 omniview→editor 方向依赖。

```python
# omniview/api.py (新文件，编译管道共享入口)
def compile_omni(source: str, filepath: str, target: str = "react") -> dict:
    """编译 .omni 源码 → { filename: content } 字典。纯函数，无文件 I/O。"""
    # ... 重构后的 build_command() 管道 ...
    return {"App.tsx": "...", "App.module.css": "..."}
```

**G2 — 统一错误协议。** 所有编译阶段（parser/lowering/codegen）返回统一的错误格式，前端据此在画布上标记错误。

```python
# omniview/api.py
from dataclasses import dataclass

@dataclass
class CompileError:
    file: str         # 源文件名
    line: int         # 行号 (1-based)
    col: int          # 列号 (1-based)
    severity: str     # "error" | "warning"
    code: str         # 错误码，如 "E0101"
    message: str      # 人类可读描述
    suggestion: str | None = None  # 修复建议

@dataclass
class CompileResult:
    success: bool
    files: dict[str, str]  # { filename: content }
    diff: dict[str, str] | None = None
    errors: list[CompileError] = field(default_factory=list)
```

**G3 — 文件变更推送协议。** WebSocket 消息格式精确定义。

```typescript
// 浏览器收到的 WS 消息类型
type WSMessage =
  | { type: "compiled"; files: Record<string, string>; diff: Record<string, string> | null; errors: CompileError[] }
  | { type: "error"; message: string }
  | { type: "shutdown" }
```

服务端轮询 tick 后 debounce 200ms 推送一次编译结果。如果编译失败 → files 为空，errors 非空。

**G4 — 依赖声明。** Python 端用 optional-dependencies；前端独立 `package.json`。

```
# pyproject.toml 追加:
[project.optional-dependencies]
editor = ["fastapi>=0.110", "uvicorn[standard]>=0.29"]

# editor/client/package.json — 前端独立依赖（见 §6.2）
```

`editor/server/` 通过 `[tool.setuptools.packages.find]` 的 `include` 加 `"editor*"` 注册为可导入包。

**G5 — 项目根配置。** 严格通过 CLI 参数指定。

```bash
omnic serve --dir ./my-project    # 必须指定 --dir
omnic serve                       # 报错: Error: --dir is required
```

服务端 `app.state.project_dir = Path(args.dir).resolve()`。前端通过 `GET /api/project` 获取：`{ "dir": "/abs/path/to/project", "target": "react" }`。

### 1.5 MCP 辅助开发与测试

本项目的可视化编辑器是浏览器应用——**仅靠 HTTP 状态码和 JSON 返回值无法验证 UI 是否正确渲染。** 每个里程碑的每项功能，在代码实现后必须通过浏览器实际操控来验证。项目已配置两组 MCP 工具：

| MCP 工具组 | 用途 | 触发时机 |
|-----------|------|---------|
| **Playwright MCP** (`mcp__playwright__*`) | 浏览器操控：导航到编辑器 URL、填写表单、点击按钮、截取页面截图、获取页面快照（accessibility tree） | 每次实现新 UI 功能后、每次修改画布/面板代码后 |
| **Agent Vision MCP** (`mcp__agent-vision__analyze_image`) | 图片识别：对 Playwright 截取的截图进行 OCR、UI 元素识别、布局验证、错误检测 | 截图后立即分析——确认"画布上确实渲染了组件"而不是凭 HTTP 返回值猜测 |

#### 1.5.1 每个里程碑的 MCP 测试流程

**M1a 测试流程（服务端 + 最小 HTML）：**

```
1. 启动 omnic serve --dir ./project
2. Playwright MCP: browser_navigate → http://localhost:5173
3. Playwright MCP: browser_snapshot → 检查 DOM 中是否加载了编辑器 textarea + output div
4. Playwright MCP: browser_take_screenshot → 保存 /tmp/m1a-editor.png
5. Agent Vision MCP: analyze_image(/tmp/m1a-editor.png, "这个页面是否显示了编辑器界面？有哪些UI元素？")
6. Playwright MCP: browser_type → 在 textarea 中输入 .omni 源码
7. Playwright MCP: browser_click → 点击 Compile 按钮
8. Playwright MCP: browser_snapshot → 检查 output div 中是否出现了编译结果
9. Playwright MCP: browser_take_screenshot → 保存 /tmp/m1a-compiled.png
10. Agent Vision MCP: analyze_image(/tmp/m1a-compiled.png, "编译输出中是否显示了生成的代码？有没有报错信息？")
```

**M1b 测试流程（React UI 编辑器 MVP）：**

```
1. Playwright MCP: browser_navigate → http://localhost:5173
2. Playwright MCP: browser_snapshot → 确认左右分栏布局存在（文件列表 + 预览 iframe + 属性面板占位）
3. Playwright MCP: browser_click → 点击文件列表中的 .omni 文件
4. Playwright MCP: browser_take_screenshot → 保存 /tmp/m1b-editor.png
5. Agent Vision MCP: analyze_image(/tmp/m1b-editor.png, "编辑器是否显示左右分栏？左侧是否有文件列表？右侧是否有预览区域？")
6. Playwright MCP: browser_snapshot → 确认撤销按钮/快捷键存在
7. 修改 .omni 文件内容（通过 API 写入磁盘）
8. 等待 WebSocket 推送 + 预览刷新
9. Playwright MCP: browser_take_screenshot → 保存 /tmp/m1b-preview.png
10. Agent Vision MCP: analyze_image(/tmp/m1b-preview.png, "预览区域是否更新了？内容是否与.omni源码对应？")
```

**M2 测试流程（属性面板 + 事件绑定）：**

```
1. Playwright MCP: browser_navigate → 编辑器
2. Playwright MCP: browser_snapshot → 获取画布上所有可点击元素
3. Playwright MCP: browser_click → 点击画布上的 Button 元素
4. Playwright MCP: browser_take_screenshot → 保存 /tmp/m2-selected.png
5. Agent Vision MCP: analyze_image(/tmp/m2-selected.png, "被选中的元素是否有高亮边框？右侧属性面板是否显示了该元素的属性？")
6. Playwright MCP: browser_snapshot → 检查属性面板中是否列出 label/variant/disabled/onPress 等属性
7. Playwright MCP: browser_type → 修改 label 属性值为 "测试按钮"
8. Playwright MCP: browser_take_screenshot → 保存 /tmp/m2-prop-changed.png
9. Agent Vision MCP: analyze_image(/tmp/m2-prop-changed.png, "画布上的按钮文本是否已变为'测试按钮'？属性面板的值是否同步更新？")
```

**M3 测试流程（连线画布 + 代码块）：**

```
1. Playwright MCP: browser_navigate → 编辑器
2. Playwright MCP: browser_snapshot → 获取组件面板 + 画布元素
3. Playwright MCP: browser_drag → 从组件面板拖 Button 到画布
4. Playwright MCP: browser_take_screenshot → 保存 /tmp/m3-drag.png
5. Agent Vision MCP: analyze_image(/tmp/m3-drag.png, "画布上是否出现了新拖入的按钮？")
6. Playwright MCP: browser_snapshot → 检查连线模式按钮
7. Playwright MCP: browser_click → 开启连线模式
8. Playwright MCP: browser_take_screenshot → 保存 /tmp/m3-wires.png
9. Agent Vision MCP: analyze_image(/tmp/m3-wires.png, "画布上是否显示了连线端口？连线是否可见？是否有贝塞尔曲线？")
10. Agent Vision MCP: analyze_image(/tmp/m3-wires.png, "signal/computed/Store 块是否显示了正确的端口？块标签是否正确？")
```

**M4 测试流程（代码面板 + 代码片段 + 数据连接器）：**

```
1. Playwright MCP: browser_snapshot → 定位代码面板区域
2. Playwright MCP: browser_take_screenshot → 保存 /tmp/m4-code.png
3. Agent Vision MCP: analyze_image(/tmp/m4-code.png, "代码面板是否显示了 Monaco 编辑器？是否有语法高亮？")
4. Agent Vision MCP: analyze_image(/tmp/m4-code.png, "右侧是否有代码片段面板？片段列表是否按上下文过滤？")
5. Playwright MCP: browser_click → 双击代码片段
6. Playwright MCP: browser_snapshot → 确认代码面板中插入了片段内容
7. Playwright MCP: browser_take_screenshot → 保存 /tmp/m4-connector.png
8. Agent Vision MCP: analyze_image(/tmp/m4-connector.png, "数据连接器面板是否显示了 REST/GraphQL/Firebase 选项？")
```

**M5 测试流程（a11y + 状态 HUD + 代码所有权 + 一键发布）：**

```
1. Playwright MCP: browser_snapshot → 检查对比度评分徽章
2. Playwright MCP: browser_take_screenshot → 保存 /tmp/m5-a11y.png
3. Agent Vision MCP: analyze_image(/tmp/m5-a11y.png, "颜色选择器中是否显示了 WCAG 对比度评分？焦点顺序可视化叠加层是否可见？")
4. Playwright MCP: browser_click → 选中元素
5. Playwright MCP: browser_take_screenshot → 保存 /tmp/m5-hud.png
6. Agent Vision MCP: analyze_image(/tmp/m5-hud.png, "状态检查 HUD 是否显示了绑定 signal 的名称和当前值？")
7. Playwright MCP: browser_drop → 拖入 Lottie .json 文件
8. Playwright MCP: browser_snapshot → 确认生成了 Lottie 组件
```

#### 1.5.2 强制规则

| 规则 | 说明 |
|------|------|
| **UI 功能必须截图** | 每实现一个 UI 功能（新面板、新画布元素、新交互），必须先 Playwright 截图，再用 Agent Vision 分析截图确认正确渲染——**不能只靠 HTTP/console 输出断定** |
| **失败必须复现** | 如果 Agent Vision 返回"未找到"或"渲染异常"，必须用 Playwright 重新截取并分析，确认是代码问题而非截图时机问题 |
| **每次 M 交付前跑完整流程** | 每个里程碑的验收标准中，至少包含 3 张 Playwright 截图 + Agent Vision 分析记录 |
| **性能测试用 browser_evaluate** | 性能门禁（M2 50 组件 >30fps）通过 `browser_evaluate` 注入 JS 测量 `requestAnimationFrame` 帧率，不能用目测 |
| **截图文件命名规范** | `{milestone}-{feature}-{timestamp}.png`，如 `m3-wire-canvas-20260612.png`，存放于 `build_output/screenshots/` |

#### 1.5.3 现有 MCP 工具速查

**Playwright MCP — 浏览器操控：**

| 工具 | 用途 |
|------|------|
| `mcp__playwright__browser_navigate` | 导航到编辑器 URL |
| `mcp__playwright__browser_snapshot` | 获取页面无障碍快照（元素列表、状态） |
| `mcp__playwright__browser_click` | 点击元素（需 target ref 来自 snapshot） |
| `mcp__playwright__browser_type` | 在可编辑元素中输入文本 |
| `mcp__playwright__browser_drag` | 拖拽元素（组件面板→画布） |
| `mcp__playwright__browser_drop` | 拖放文件（Lottie/Rive 导入） |
| `mcp__playwright__browser_fill_form` | 批量填写多个表单字段 |
| `mcp__playwright__browser_take_screenshot` | 截图（PNG/JPEG，支持全页/元素级） |
| `mcp__playwright__browser_evaluate` | 执行 JS（性能测量、运行时状态读取） |
| `mcp__playwright__browser_press_key` | 按键（Ctrl+Z 撤销、Ctrl+F 搜索） |
| `mcp__playwright__browser_wait_for` | 等待文本出现/消失或指定时间 |
| `mcp__playwright__browser_console_messages` | 读取浏览器控制台日志 |
| `mcp__playwright__browser_network_requests` | 读取网络请求（验证 API 调用） |

**Agent Vision MCP — 截图分析：**

| 工具 | 用途 |
|------|------|
| `mcp__agent-vision__analyze_image` | 分析截图——OCR 文字、识别 UI 元素、检查布局、检测错误状态 |

**典型 analyze_image prompt 模板：**
```
"这个截图显示了 OmniLang 可视化编辑器的 [功能名称] 界面。
请检查：
1. 是否能看到 [预期 UI 元素]？
2. 元素位置是否正确（左右分栏、面板位置）？
3. 是否有任何渲染异常（乱码、重叠、溢出、空白区域）？
4. 文字内容是否与 [预期内容] 一致？"
```

### 1.6 编辑器自身 UX 设计要求

本编辑器是**给开发者用的生产工具**——不是 demo，不是内部工具。每个界面必须达到"用了就不想回命令行"的标准。

#### 1.6.1 核心原则

| 原则 | 含义 | 违反示例 |
|------|------|---------|
| **零文档上手** | 新用户不读任何文档，5 分钟内完成"打开项目→改代码→看到预览" | 需要查 Wiki 才知道 `--dir` 必填 |
| **操作即时反馈** | 每一次点击/按键/拖拽在 100ms 内有视觉响应 | 点 Compile 按钮 2 秒无反应 |
| **错误友好呈现** | 编译错误用红色标记+行号高亮+修复建议，不用裸堆栈 | `Error: NoneType has no attribute` |
| **键盘可达** | 所有常用操作有键盘快捷键，`?` 键查看快捷键列表 | 必须用鼠标才能编译 |
| **视觉层次分明** | 主操作区（画布）> 辅助面板 > 工具栏 > 状态栏，色彩/间距区分 | 所有区域同一灰度 |

#### 1.6.2 交互细节清单（每个 M 交付前必须满足）

**M1 — 基础交互：**
- [ ] Compile 按钮点击后 < 100ms 变为 "Compiling..." 禁用态
- [ ] 编译成功 → 状态栏绿色 "✅ Compiled — HH:MM:SS"
- [ ] 编译失败 → 状态栏红色 "❌ Compile failed" + 错误行高亮
- [ ] Ctrl+Enter 触发编译（焦点在任意位置）
- [ ] textarea 中 Tab 键 = 4 空格缩进（不跳转焦点）
- [ ] 文件加载中 → 浅灰骨架占位，非空白 textarea
- [ ] 连接断开 → 状态栏黄色 "🔌 Reconnecting..." + 3s 自动重连
- [ ] 窗口宽度 < 800px → 左右分栏自动变为上下分栏

**M2 — 画布交互：**
- [ ] 鼠标悬停元素 → 半透明蓝色边框
- [ ] 点击选中 → 2px 实线蓝色边框 + 属性面板 200ms 滑入
- [ ] 属性修改 → 画布元素闪烁黄色 100ms（确认已感知修改）
- [ ] 属性面板空状态 → "选择画布元素以编辑属性" + 图标
- [ ] 组件树（Outline）选中节点 → 画布自动滚动到可见位置

**M3 — 拖拽与连线：**
- [ ] 拖出组件 → 跟随鼠标的 ghost 半透明预览（60% opacity）
- [ ] 拖到有效区域 → ghost 变绿；无效区域 → 变红
- [ ] 连线端口 hover → 从 4px 放大到 8px（暗示可拖拽）
- [ ] 拖拽连线中 → 虚线跟随鼠标，松手弹菜单
- [ ] 连线模式切换 → 300ms 平滑过渡，连线淡入/淡出
- [ ] 画布缩放：Ctrl+滚轮 → 0.25x~2x，角落显示比例

**M4 — 代码编辑：**
- [ ] Monaco 加载中 → 骨架屏（灰色矩形模拟代码行），非白屏
- [ ] 代码片段双击插入 → 新代码闪烁高亮 500ms 黄色背景
- [ ] 数据连接器表单 → 必填字段标红色 `*`，测试按钮带 spinner
- [ ] 长文件（>500 行）→ Monaco minimap 自动启用

**M5 — 调试体验：**
- [ ] 状态 HUD → 选中元素旁弹出，200ms 淡入
- [ ] 连线断裂 → 红色虚线 + ⚠ 图标，hover 显示"目标已删除"
- [ ] 对比度检查 → 颜色选择器内 AA/AAA 徽章动态变色（绿/黄/红）
- [ ] Tab 焦点顺序 → 叠加层数字标签 300ms 淡入

#### 1.6.3 键盘快捷键总表

| 快捷键 | 功能 | M |
|--------|------|----|
| `Ctrl+Enter` | 编译当前文件 | M1 |
| `Ctrl+Z` / `Ctrl+Shift+Z` | 撤销 / 重做 | M1 |
| `Ctrl+S` | 保存文件 | M1 |
| `?` | 快捷键帮助面板 | M1 |
| `Delete` | 删除选中元素 | M2 |
| `Esc` | 取消选中/关闭面板/退出连线模式 | M2 |
| `Ctrl+P` | 切换右侧面板 | M2 |
| `Ctrl+B` | 切换左侧面板 | M3 |
| `Ctrl+D` | 复制选中元素 | M3 |
| `F2` | 重命名选中元素 | M3 |
| `Ctrl+Scroll` | 画布缩放 | M3 |
| `Ctrl+Shift+F` | 统一搜索 | M5 |

#### 1.6.4 状态指示器规范

| 状态 | 颜色 | 文字 | 位置 |
|------|------|------|------|
| 就绪 | 灰色 `#6B7280` | "Ready" | 底部状态栏 |
| 编译中 | 蓝色 `#3B82F6` | spinner + "Compiling..." | 底部状态栏 |
| 编译成功 | 绿色 `#16A34A` | "✅ Compiled — HH:MM:SS" | 底部状态栏 |
| 编译警告 | 黄色 `#D97706` | "⚠ Compiled with N warning(s)" | 底部状态栏 |
| 编译失败 | 红色 `#DC2626` | "❌ Compile failed" | 底部状态栏 |
| 连接断开 | 黄色 `#D97706` | "🔌 Reconnecting..." + 覆盖层 | 底部状态栏 |
| 文件未保存 | 橙色 dot | 文件名旁标签 | 文件名标签 |

#### 1.6.5 强制 UI 规则

| 规则 | 说明 |
|------|------|
| **空状态必有提示** | 面板无数据时显示引导文字+图标，不显示空白 |
| **加载必有骨架** | 异步加载在等待时显示 skeleton/placeholder |
| **错误必有建议** | 编译错误/警告附带修复建议，不裸报错 |
| **色彩必有语义** | 红=错误 黄=警告 绿=成功 蓝=进行中 灰=非活跃——全编辑器统一 |
| **动画 ≤ 300ms** | 所有过渡动画不超过 300ms |
| **响应式断点** | < 800px 上下分栏；< 500px 面板折叠为图标按钮 |
| **每次 M 交付前必查** | 用 Playwright MCP 截取编辑器各状态图 + Agent Vision 分别检查空状态/加载态/错误态/正常态 |

```
omnilang-project/
├── omnilang/                    # 已有：语言核心
├── omniview/                    # 已有：UI SDK
│   └── api.py                   #   ★ G1: 编译管道共享入口 (compile_omni + CompileError)
├── editor/                      # ★ 新增：可视化编辑器
│   ├── server/                  #   Python 服务端
│   │   ├── __init__.py
│   │   ├── main.py              #     FastAPI 入口，omnic serve 命令
│   │   ├── api/
│   │   │   ├── __init__.py
│   │   │   ├── compile.py       #     POST /api/compile
│   │   │   ├── filesystem.py    #     /api/fs/*
│   │   │   ├── project.py       #     /api/init-project
│   │   │   ├── deploy.py        #     /api/deploy (M5)
│   │   │   └── watch.py         #     WS /api/watch
│   │   ├── preview.py           #     Vite dev server 管理
│   │   └── heartbeat.py         #     WebSocket heartbeat
│   │
│   ├── client/                  #   React 前端
│   │   ├── package.json
│   │   ├── vite.config.ts
│   │   ├── tsconfig.json
│   │   ├── index.html
│   │   └── src/
│   │       ├── main.tsx
│   │       ├── App.tsx
│   │       ├── api/             #     与服务端通信
│   │       │   ├── client.ts    #       HTTP + WebSocket 封装
│   │       │   └── types.ts     #       API 类型定义
│   │       ├── store/           #     Zustand stores
│   │       │   ├── editor.ts    #       编辑器核心状态
│   │       │   ├── undo.ts      #       撤销栈 (IndexedDB 持久化)
│   │       │   ├── project.ts   #       项目文件状态
│   │       │   └── tokens.ts    #       设计令牌状态 (M3)
│   │       ├── canvas/          #     画布
│   │       │   ├── Canvas.tsx         # 画布主组件
│   │       │   ├── LayoutView.tsx     # DOM 布局视图
│   │       │   ├── WireOverlay.tsx    # Canvas2D 连线叠加层 (M3)
│   │       │   ├── CodeBlock.tsx      # 画布代码块 (M3)
│   │       │   ├── WireLabel.tsx      # fn 调用连线标签 (M3)
│   │       │   ├── SelectionOverlay.tsx
│   │       │   └── viewport.ts        # 虚拟滚动/视口管理
│   │       ├── panels/          #     面板
│   │       │   ├── PropertyPanel.tsx   # 属性面板 (M2)
│   │       │   ├── ComponentPalette.tsx # 组件面板 (M3)
│   │       │   ├── CodePanel.tsx       # 代码面板 (M4)
│   │       │   ├── SnippetPanel.tsx    # 代码片段面板 (M4)
│   │       │   ├── OutlinePanel.tsx    # 组件树面板 (M3)
│   │       │   ├── TokenPanel.tsx      # 设计令牌面板 (M3)
│   │       │   └── ConnectorPanel.tsx  # 数据连接器面板 (M4)
│   │       ├── commands/        #     撤销栈 Command
│   │       │   ├── types.ts
│   │       │   ├── AddElement.ts
│   │       │   ├── RemoveElement.ts
│   │       │   ├── SetProp.ts
│   │       │   ├── AddWire.ts
│   │       │   ├── RemoveWire.ts
│   │       │   └── registry.ts
│   │       ├── preview/         #     实时预览
│   │       │   └── PreviewFrame.tsx
│   │       ├── search/          #     统一搜索 (M5)
│   │       │   └── SearchIndex.ts
│   │       ├── a11y/            #     无障碍检查 (M5)
│   │       │   ├── ContrastChecker.ts
│   │       │   └── FocusOrder.ts
│   │       └── utils/
│   │           ├── diff.ts      #     .omni diff 渲染
│   │           └── id.ts        #     UUID 生成
│
├── tests/
│   └── editor/                  #   ★ 编辑器测试
│       ├── conftest.py
│       ├── test_server.py       #     服务端 API 测试
│       ├── test_heartbeat.py    #     心跳协议测试
│       ├── test_atomic_write.py #     原子写入测试
│       └── e2e/                 #     Playwright E2E
│           ├── test_m1_preview.spec.ts
│           ├── test_m2_property.spec.ts
│           ├── test_m3_canvas.spec.ts
│           └── test_m4_code.spec.ts
│
└── pyproject.toml               # 已有（需新增 fastapi/uvicorn 依赖）
```

---

## §3 里程碑详解

---

### M1a: 集成 Spike — 证明编译管道端到端（2 周）

> **目标：** 3 个任务证明架构可行性。**不建 UI 基础设施。** 只回答一个问题：`.omni 源码 → HTTP API → WebSocket 推送 → 浏览器预览` 这条链路能不能跑通？

**成功标准：** 浏览器中改 .omni 文件文本 → 服务端编译 → WebSocket 推送 HTML → 预览 `<div>` 自动刷新。

| # | 任务 | 产出 | 工时 |
|----|------|------|------|
| 1a | **`omniview/api.py`** — 重构 `build_command()` 为 `compile_omni(source, target) → CompileResult`。提取 200 行单体中的 preprocess→parse→lower→codegen 管道为纯函数。定义 `CompileError` + `CompileResult` 数据类（G2）。 | 编译 API | 3d |
| 1b | **`editor/server/main.py`** — FastAPI 最小服务（~80 行）。`GET /` 返回最小 HTML。`POST /api/compile` 调用 `compile_omni`。`WS /api/watch` 启动 `OmniFilePoller`。`--dir` 必填（G5）。`@lru_cache` 缓存 parser（F3）。 | HTTP+WS 服务 | 2d |
| 1c | **最小 HTML 预览页** — 50 行 HTML+JS：一个 `<textarea>` 编辑 .omni 源码，一个 `<div>` 显示编译结果。WebSocket 连接 → 收到 `compiled` → `innerHTML` 替换。无 React、无 Zustand、无撤销。 | 端到端验证 | 2d |

**M1a 验收（必须全部通过才能进 M1b）：**
- [ ] `omnic serve --dir ./project` 启动 → HTTP 200
- [ ] `POST /api/compile { source: "fn main() { println(\"hi\"); }" }` → `{ files: {"main.py": "print(\"hi\")"} }`
- [ ] 修改 .omni 文件（磁盘写入）→ WebSocket 推送 `compiled` 消息 → 页面 `<div>` 更新
- [ ] parser 缓存生效：第 2 次编译延迟 < 第 1 次的 50%
- [ ] `CompileError` 格式统一：parser/lowering/codegen 错误都用 `{file, line, col, severity, code, message}`

---

### M1b: 编辑器 MVP（3 周）

> **目标：** 在 M1a 验证过的管道上，建可操作的编辑器 MVP。**推迟 Zustand → M2**（M1b 用 React useState 足够），**推迟脏标记 → M2**（M1b 预览全量刷新），**保留撤销栈**（Command 模式 + IndexedDB——画布操作需要回退）。

**成功标准：** 浏览器中左右分栏布局，左侧文件列表，右侧实时预览 + 基础属性面板。

---

#### M1b.1 — omnic serve 补全（第 1 周）

##### 任务 1: `editor/server/main.py` — FastAPI 服务入口

```python
# editor/server/main.py
"""OmniLang Visual Editor — HTTP + WebSocket 服务."""

import sys
import webbrowser
from pathlib import Path
from fastapi import FastAPI
from fastapi.staticfiles import StaticFiles
from fastapi.middleware.cors import CORSMiddleware
import uvicorn

from editor.server.api.compile import router as compile_router
from editor.server.api.filesystem import router as fs_router
from editor.server.api.project import router as project_router
from editor.server.api.watch import router as watch_router
from editor.server.preview import PreviewManager
from editor.server.heartbeat import HeartbeatManager

app = FastAPI(title="OmniLang Editor")
app.add_middleware(CORSMiddleware, allow_origins=["*"], allow_methods=["*"], allow_headers=["*"])

# API 路由
app.include_router(compile_router, prefix="/api")
app.include_router(fs_router, prefix="/api")
app.include_router(project_router, prefix="/api")
app.include_router(watch_router, prefix="/api")

# 静态文件: 编辑器 SPA (生产构建产物)
STATIC_DIR = Path(__file__).parent.parent / "client" / "dist"
if STATIC_DIR.exists():
    app.mount("/", StaticFiles(directory=str(STATIC_DIR), html=True), name="static")

# 全局状态
preview_manager = PreviewManager()
heartbeat_manager = HeartbeatManager()


def cmd_serve(args):
    """CLI 入口: omnic serve"""
    port = args.port or 5173
    project_dir = args.dir or Path.cwd()

    # 如果有 npm 项目，启动 Vite dev server
    preview_port = None
    if (Path(project_dir) / "package.json").exists():
        preview_port = preview_manager.start(project_dir)

    print(f"  Editor:   http://localhost:{port}")
    if preview_port:
        print(f"  Preview:  http://localhost:{preview_port}")
    print(f"  Project:  {project_dir}")
    print(f"  Press Ctrl+C to stop.")

    # 将 project_dir 和 preview_port 注入 app state
    app.state.project_dir = str(project_dir)
    app.state.preview_port = preview_port

    # 打开浏览器
    webbrowser.open(f"http://localhost:{port}")

    uvicorn.run(app, host="127.0.0.1", port=port, log_level="info")
```

##### 任务 2: `editor/server/api/compile.py` — 编译 API

```python
# editor/server/api/compile.py
"""POST /api/compile — 编译 .omni → 目标代码."""

from fastapi import APIRouter, Request
from pydantic import BaseModel
from pathlib import Path

router = APIRouter()


class CompileRequest(BaseModel):
    file: str          # .omni 文件路径 (相对 project_dir)
    target: str = "react"


class CompileResponse(BaseModel):
    success: bool
    files: dict[str, str] | None = None   # { "App.tsx": "...", "App.module.css": "..." }
    diff: dict[str, str] | None = None     # { "App.tsx": "- old\n+ new" }
    error: str | None = None               # 编译错误信息


@router.post("/compile", response_model=CompileResponse)
async def compile_omni(req: CompileRequest, request: Request):
    """编译单个 .omni 文件，返回生成的目标代码和 diff。"""
    project_dir = Path(request.app.state.project_dir)
    omni_path = project_dir / req.file

    if not omni_path.exists():
        return CompileResponse(success=False, error=f"File not found: {req.file}")

    try:
        # 读取源码
        source = omni_path.read_text(encoding="utf-8")

        # 检测 import omniview（UI 模式 vs 纯逻辑模式）
        has_omniview = "import omniview" in source

        if has_omniview:
            # UI 模式: 使用 OmniView pipeline
            from omniview.preprocessor import preprocess
            from omnilang.lexer import create_omniview_parser
            from omniview.parser import OmniViewTransformer
            from omniview.lowering import IRFixture
            from omniview.codegen.react import ReactCodegen
            # ... (完整 pipeline，复用 omniview/cli.py 的 build_command 逻辑)
            files = _compile_ui_mode(source, str(omni_path), req.target)
        else:
            # 纯逻辑模式: 使用 OmniLang pipeline
            from omnilang.parser import parse
            from omnilang.semantic import analyze
            from omnilang.codegen import generate
            prog = parse(source, filename=str(omni_path))
            source_lines = {str(omni_path): source.split("\n")}
            type_map, rename_map = analyze(prog, source_lines)
            code = generate(prog, req.target, rename_map, type_map)
            ext_map = {"js": ".js", "python": ".py", "rust": ".rs", "dart": ".dart"}
            base = omni_path.stem
            files = {f"{base}{ext_map[req.target]}": code}

        # 计算 diff（与上次编译结果对比）
        diff = _compute_diff(req.file, files, project_dir)

        return CompileResponse(success=True, files=files, diff=diff)

    except Exception as e:
        return CompileResponse(success=False, error=str(e))
```

> **⚠ F3 — Parser 缓存（M1 必须项）。** `create_parser()` 和 `create_omniview_parser()` 每次调用构造 Lark Earley parser（~100ms/个）。在 `omnilang/lexer.py` 中加 `@lru_cache` 模块级单例，热路径延迟 < 1ms。否则每编译请求 200ms+ 税。需同时检查 `parser_helpers.py` 的 `parse_inline_expr()` / `parse_inline_block()` 内部调用是否也受益于缓存。

> **⚠ F3b — build_command() 重构。** `omniview/cli.py` 的 `build_command()`（753 行文件中的 200+ 行单体函数）每个错误路径都是 `print(..., file=sys.stderr) + return 1`，无异常层级。必须重构为可调用 API：提取 preprocess→parse→module_resolve→semantic_analyze→lower→codegen 管道为 `compile_omni(source, target) → {filename: content}` 纯函数，异常替代 print+return。

##### 任务 3: `editor/server/api/filesystem.py` — 文件系统 API（含原子写入）

```python
# editor/server/api/filesystem.py
"""文件浏览/读取/原子写入 API."""

import os
import uuid
from pathlib import Path
from fastapi import APIRouter, Request
from pydantic import BaseModel

router = APIRouter()


class FsListRequest(BaseModel):
    dirPath: str = "."     # 相对 project_dir 的路径


class FsReadRequest(BaseModel):
    filePath: str


class FsWriteRequest(BaseModel):
    filePath: str
    content: str


@router.post("/fs/list")
async def fs_list(req: FsListRequest, request: Request):
    """列出目录内容。仅返回 .omni / .tsx / .json / .css / .ts 等源码文件。"""
    project_dir = Path(request.app.state.project_dir)
    target_dir = (project_dir / req.dirPath).resolve()

    # 安全检查：不允许访问 project_dir 之外的路径
    if not str(target_dir).startswith(str(project_dir.resolve())):
        return {"entries": [], "error": "Access denied: path outside project"}

    SOURCE_EXTS = {".omni", ".tsx", ".ts", ".jsx", ".js", ".json", ".css", ".vue"}

    entries = []
    try:
        for entry in sorted(target_dir.iterdir()):
            if entry.name.startswith(".") or entry.name == "node_modules":
                continue
            entries.append({
                "name": entry.name,
                "path": str(entry.relative_to(project_dir)).replace("\\", "/"),
                "type": "directory" if entry.is_dir() else "file",
                "ext": entry.suffix if entry.is_file() else "",
            })
    except PermissionError:
        return {"entries": [], "error": "Permission denied"}

    return {"entries": entries}


@router.post("/fs/read")
async def fs_read(req: FsReadRequest, request: Request):
    """读取文件内容。"""
    project_dir = Path(request.app.state.project_dir)
    file_path = (project_dir / req.filePath).resolve()

    if not str(file_path).startswith(str(project_dir.resolve())):
        return {"content": "", "error": "Access denied"}

    if not file_path.exists():
        return {"content": "", "error": "File not found"}

    try:
        content = file_path.read_text(encoding="utf-8")
        return {"content": content}
    except UnicodeDecodeError:
        return {"content": "", "error": "Binary file not supported"}


@router.post("/fs/write")
async def fs_write(req: FsWriteRequest, request: Request):
    """原子写入文件内容。"""
    project_dir = Path(request.app.state.project_dir)
    file_path = (project_dir / req.filePath).resolve()

    if not str(file_path).startswith(str(project_dir.resolve())):
        return {"ok": False, "error": "Access denied"}

    # 原子写入：写临时文件 → fsync → rename
    file_path.parent.mkdir(parents=True, exist_ok=True)
    tmp_path = file_path.parent / f"{file_path.name}.tmp.{uuid.uuid4().hex[:8]}"

    try:
        tmp_path.write_text(req.content, encoding="utf-8")
        # Windows 上 os.fsync 需要文件描述符
        with open(tmp_path, "rb") as f:
            os.fsync(f.fileno())
        os.replace(tmp_path, file_path)  # 原子操作
        return {"ok": True}
    except Exception as e:
        if tmp_path.exists():
            tmp_path.unlink()
        return {"ok": False, "error": str(e)}
```

##### 任务 4: `editor/server/preview.py` — Vite dev server 管理

```python
# editor/server/preview.py
"""管理 Vite dev server 生命周期，用于实时预览编译输出。"""

import subprocess
import time
import socket
import requests
from pathlib import Path


def _find_free_port(start: int = 5174) -> int:
    """找到一个可用的 TCP 端口。"""
    for port in range(start, start + 100):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            if s.connect_ex(("127.0.0.1", port)) != 0:
                return port
    raise RuntimeError("No free port found")


class PreviewManager:
    """管理一个 npm 项目的 Vite dev server。"""

    def __init__(self):
        self._process: subprocess.Popen | None = None
        self._port: int | None = None

    def start(self, project_dir: str | Path) -> int:
        """启动 Vite dev server，返回端口号。"""
        project_dir = Path(project_dir)

        # 检查 node_modules 是否存在，不存在则自动安装
        if not (project_dir / "node_modules").exists():
            subprocess.run(
                ["npm", "install"],
                cwd=str(project_dir),
                capture_output=True,
                timeout=120,
            )

        self._port = _find_free_port()
        self._process = subprocess.Popen(
            ["npx", "vite", "--port", str(self._port), "--strictPort"],
            cwd=str(project_dir),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

        # 等待 server 就绪（最多 30s）
        for _ in range(60):
            try:
                requests.get(f"http://localhost:{self._port}", timeout=0.5)
                return self._port
            except requests.ConnectionError:
                time.sleep(0.5)

        raise RuntimeError("Vite dev server failed to start")

    def stop(self):
        """停止 Vite dev server。"""
        if self._process:
            self._process.terminate()
            try:
                self._process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self._process.kill()
            self._process = None
            self._port = None

    @property
    def port(self) -> int | None:
        return self._port
```

##### 任务 5: `editor/server/heartbeat.py` — WebSocket heartbeat + 自动退出

```python
# editor/server/heartbeat.py
"""WebSocket heartbeat 管理器。解决孤儿进程问题 (CRT-1)。"""

import asyncio
import sys
import threading
import time


class HeartbeatManager:
    """监听浏览器 WebSocket heartbeat。连续 3 次 missed → 退出。"""

    def __init__(self, max_missed: int = 3, ping_interval: int = 5, shutdown_delay: int = 30):
        self._last_ping = time.time()
        self._lock = threading.Lock()
        self._max_missed = max_missed
        self._ping_interval = ping_interval
        self._shutdown_delay = shutdown_delay
        self._monitor_thread: threading.Thread | None = None
        self._running = False

    def record_ping(self):
        """收到浏览器 WebSocket ping 时调用。"""
        with self._lock:
            self._last_ping = time.time()

    def start(self):
        """启动后台监控线程。"""
        self._running = True
        self._monitor_thread = threading.Thread(target=self._monitor_loop, daemon=True)
        self._monitor_thread.start()

    def stop(self):
        """停止监控。"""
        self._running = False

    def _monitor_loop(self):
        """后台循环：检查 heartbeat，超时则退出。"""
        missed_count = 0
        while self._running:
            time.sleep(self._ping_interval)
            with self._lock:
                elapsed = time.time() - self._last_ping
            if elapsed > self._ping_interval * 2:
                missed_count += 1
                if missed_count >= self._max_missed:
                    print(f"[omnic serve] No heartbeat for {self._shutdown_delay}s. Shutting down...")
                    time.sleep(self._shutdown_delay)
                    # 最后一次检查是否恢复
                    with self._lock:
                        final_elapsed = time.time() - self._last_ping
                    if final_elapsed > self._ping_interval * self._max_missed + self._shutdown_delay:
                        sys.exit(0)
                    else:
                        missed_count = 0
            else:
                missed_count = 0
```

---

#### M1b.2 — 编辑器 UI 骨架（第 2 周）

##### 任务 1: `editor/client/` 初始化

```bash
cd editor/client
npm create vite@latest . -- --template react-ts
npm install zustand idb pako
npm install -D @types/pako
```

```
# vite.config.ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      "/api": "http://127.0.0.1:8000",    # 代理到 Python 服务
      "/preview": "http://127.0.0.1:5174", # 代理到 Vite 预览
    },
  },
});
```

##### 任务 2: `editor/client/src/api/client.ts` — HTTP + WebSocket 通信层

```typescript
// editor/client/src/api/client.ts
/** 与服务端通信的统一客户端。 */

import { CompileRequest, CompileResponse, FsEntry } from "./types";

const BASE = "";  // 同源，由 Vite proxy 转发

class ApiClient {
  private ws: WebSocket | null = null;
  private wsHandlers: Map<string, (data: any) => void> = new Map();
  private heartbeatInterval: number | null = null;

  // ── HTTP ─────────────────────────────────

  async compile(file: string, target: string = "react"): Promise<CompileResponse> {
    const res = await fetch(`${BASE}/api/compile`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ file, target } as CompileRequest),
    });
    return res.json();
  }

  async listDir(dirPath: string = "."): Promise<{ entries: FsEntry[]; error?: string }> {
    const res = await fetch(`${BASE}/api/fs/list`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ dirPath }),
    });
    return res.json();
  }

  async readFile(filePath: string): Promise<{ content: string; error?: string }> {
    const res = await fetch(`${BASE}/api/fs/read`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ filePath }),
    });
    return res.json();
  }

  async writeFile(filePath: string, content: string): Promise<{ ok: boolean; error?: string }> {
    const res = await fetch(`${BASE}/api/fs/write`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ filePath, content }),
    });
    return res.json();
  }

  async initProject(name: string, target: string): Promise<{ projectDir: string }> {
    const res = await fetch(`${BASE}/api/init-project`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name, target }),
    });
    return res.json();
  }

  // ── WebSocket ────────────────────────────

  connectWatch(onCompiled: (data: CompileResponse) => void, onError: (msg: string) => void) {
    const protocol = location.protocol === "https:" ? "wss:" : "ws:";
    this.ws = new WebSocket(`${protocol}//${location.host}/api/watch`);

    this.ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      if (data.type === "compiled") onCompiled(data);
      else if (data.type === "error") onError(data.message);
    };

    this.ws.onclose = () => {
      this.stopHeartbeat();
      // 5s 后重连
      setTimeout(() => this.connectWatch(onCompiled, onError), 5000);
    };

    // 启动 heartbeat (CRT-1 缓解)
    this.startHeartbeat();
  }

  private startHeartbeat() {
    this.heartbeatInterval = window.setInterval(() => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.ws.send(JSON.stringify({ type: "ping" }));
      }
    }, 5000);
  }

  private stopHeartbeat() {
    if (this.heartbeatInterval !== null) {
      clearInterval(this.heartbeatInterval);
      this.heartbeatInterval = null;
    }
  }

  disconnect() {
    this.stopHeartbeat();
    this.ws?.close();
  }
}

export const api = new ApiClient();
```

##### 任务 3: `editor/client/src/App.tsx` — 基础布局

```tsx
// editor/client/src/App.tsx
import { useState, useEffect } from "react";
import { useEditorStore } from "./store/editor";
import { FileExplorer } from "./panels/FileExplorer";
import { PreviewFrame } from "./preview/PreviewFrame";

export default function App() {
  const { projectDir, setProjectDir, files, loadFiles } = useEditorStore();
  const [previewPort, setPreviewPort] = useState<number | null>(null);

  // 布局: 左侧文件列表 | 中间画布/预览 | 右侧属性面板(空占位)
  return (
    <div style={{ display: "flex", height: "100vh", fontFamily: "system-ui" }}>
      {/* 左侧: 文件列表 (200px) */}
      <aside style={{ width: 200, borderRight: "1px solid #e5e7eb", overflow: "auto", padding: 8 }}>
        <FileExplorer />
      </aside>

      {/* 中间: 画布 + 预览 (flex: 1) */}
      <main style={{ flex: 1, display: "flex", flexDirection: "column" }}>
        {/* 工具栏 */}
        <div style={{ height: 36, borderBottom: "1px solid #e5e7eb", padding: "4px 12px", display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ fontWeight: 600 }}>OmniLang Editor</span>
          {projectDir && <span style={{ color: "#6b7280", fontSize: 13 }}>{projectDir}</span>}
        </div>
        {/* 预览区 */}
        <div style={{ flex: 1 }}>
          <PreviewFrame />
        </div>
      </main>

      {/* 右侧: 属性面板 (占位, M2 实现) */}
      <aside style={{ width: 280, borderLeft: "1px solid #e5e7eb", padding: 8 }}>
        <p style={{ color: "#9ca3af", fontSize: 13 }}>属性面板 — M2 实现</p>
      </aside>
    </div>
  );
}
```

##### 任务 4: `editor/client/src/preview/PreviewFrame.tsx` — 实时预览 iframe

```tsx
// editor/client/src/preview/PreviewFrame.tsx
import { useEditorStore } from "../store/editor";

export function PreviewFrame() {
  const previewUrl = useEditorStore((s) => s.previewUrl);

  if (!previewUrl) {
    return (
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", color: "#9ca3af" }}>
        打开项目以查看预览
      </div>
    );
  }

  return (
    <iframe
      src={previewUrl}
      style={{ width: "100%", height: "100%", border: "none" }}
      title="Preview"
    />
  );
}
```

---

#### M1b.3 — 撤销/重做 + Git diff + heartbeat（第 3 周）

##### 任务 1: `store/undo.ts` — Command 模式撤销栈 + IndexedDB 持久化

```typescript
// editor/client/src/store/undo.ts
/**
 * 撤销栈实现。
 *
 * 架构:
 *   Command 接口 → execute() / undo()
 *   每次操作 push 到栈 → Ctrl+Z 调用 undo()
 *   持久化: IndexedDB (idb 库) + zlib 压缩 (pako)
 *
 *   Tab 冻结恢复 (CRT-2):
 *   document.visibilitychange → 立即序列化到 IndexedDB
 *   visibilityState === "visible" → 从 IndexedDB 重建
 */

import { create } from "zustand";
import { openDB, IDBPDatabase } from "idb";
import { deflate, inflate } from "pako";

// ── Command 接口 ───────────────────────────

export interface Command {
  readonly type: string;
  /** 正向执行（首次执行时调用）。返回 undo 所需的数据。 */
  execute(): void;
  /** 反向撤销。 */
  undo(): void;
  /** 命令的简短描述，用于 UI 显示（"修改 Button label"）。 */
  readonly description: string;
}

// ── IndexedDB Schema ───────────────────────

const DB_NAME = "omnilang-editor";
const DB_VERSION = 1;
const STORE_NAME = "undo-stack";

let dbPromise: Promise<IDBPDatabase> | null = null;

function getDB(): Promise<IDBPDatabase> {
  if (!dbPromise) {
    dbPromise = openDB(DB_NAME, DB_VERSION, {
      upgrade(db) {
        db.createObjectStore(STORE_NAME);
      },
    });
  }
  return dbPromise;
}

// ── Store ──────────────────────────────────

interface UndoState {
  /** 已执行的命令栈（栈顶 = 最近的操作）。 */
  stack: Command[];
  /** 最大栈深度（超过则从底部移除旧条目。默认 500）。 */
  maxDepth: number;
  /** 待持久化的快照计数器。 */
  _dirtyCounter: number;
  /** 每 N 次操作触发一次 IndexedDB 持久化。 */
  _persistInterval: number;

  push: (cmd: Command) => void;
  undo: () => void;
  redo: () => void;  // 预留，v1.2 后实现
  clear: () => void;
  persist: () => Promise<void>;
  restore: () => Promise<void>;
}

export const useUndoStore = create<UndoState>((set, get) => ({
  stack: [],
  maxDepth: 500,
  _dirtyCounter: 0,
  _persistInterval: 20,

  push: (cmd: Command) => {
    set((state) => {
      cmd.execute();
      const newStack = [...state.stack, cmd];
      // 超深度截断
      if (newStack.length > state.maxDepth) {
        newStack.splice(0, newStack.length - state.maxDepth);
      }
      // 定期异步持久化
      const dirty = state._dirtyCounter + 1;
      if (dirty % state._persistInterval === 0) {
        setTimeout(() => get().persist(), 0);
      }
      return { stack: newStack, _dirtyCounter: dirty };
    });
  },

  undo: () => {
    set((state) => {
      if (state.stack.length === 0) return state;
      const cmd = state.stack[state.stack.length - 1];
      cmd.undo();
      return { stack: state.stack.slice(0, -1) };
    });
  },

  redo: () => {
    // v1.2 后实现：维护一个 redo stack
  },

  clear: () => set({ stack: [], _dirtyCounter: 0 }),

  persist: async () => {
    const { stack } = get();
    const db = await getDB();
    // 序列化：只存储元数据（类型 + 描述），命令本身不可序列化
    const meta = stack.map((cmd) => ({
      type: cmd.type,
      description: cmd.description,
    }));
    const json = JSON.stringify(meta);
    // zlib 压缩
    const compressed = deflate(new TextEncoder().encode(json));
    await db.put(STORE_NAME, compressed, "stack");
  },

  restore: async () => {
    try {
      const db = await getDB();
      const compressed = await db.get(STORE_NAME);
      if (!compressed) return;
      const json = new TextDecoder().decode(inflate(compressed));
      const meta = JSON.parse(json);
      // 元数据恢复仅用于调试/显示，不可执行（命令对象不可逆序列化）
      console.log(`[Undo] Restored ${meta.length} entries from IndexedDB.`);
    } catch (e) {
      console.warn("[Undo] Failed to restore from IndexedDB:", e);
    }
  },
}));

// ── Tab 冻结恢复 (CRT-2) ────────────────────

document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "hidden") {
    // 标签即将被冻结 → 立即持久化
    useUndoStore.getState().persist();
    useEditorStore.getState().persistState();
  } else if (document.visibilityState === "visible") {
    // 标签恢复 → 从 IndexedDB 重建
    useEditorStore.getState().restoreState();
  }
});
```

##### 任务 2: `commands/` — 基础 Command 实现

```typescript
// editor/client/src/commands/types.ts
import type { Command } from "../store/undo";

/**
 * 画布节点数据（R2 升级：加 nodeType 判别器 + M3+ 槽位）。
 *
 * M1b 只用 nodeType="view"。M3 引入 "codeblock"。M3+ 槽位在 M1b 为 null/undefined，
 * 但结构已占位——M3 不需要 break schema。
 */
export interface ElementData {
  id: string;
  nodeType: "view" | "codeblock" | "group";  // ★ R2: 判别器，为 M3 预留
  tag: string;
  props: Record<string, any>;
  children: string[];
  parentId: string | null;
  // ── M3+ 槽位（M1b 不使用，但字段已占位）──
  ports?: { name: string; direction: "in" | "out"; type: string }[];
  renderMode?: "dom" | "canvas";      // CodeBlock 用 Canvas2D 渲染
  isFolded?: boolean;                 // CodeBlock 折叠状态
  typeAnnotation?: string;            // signal/computed 类型标注
  storeName?: string;                 // Store signal/action 归属
}

/**
 * 连线数据（R2 升级：加折叠状态）。
 */
export interface WireData {
  id: string;
  fromId: string;
  fromPort: string;
  toId: string;
  toPort: string;
  // ── M5 槽位 ──
  isCollapsed?: boolean;              // 折叠组中的连线
  collapseGroupId?: string;           // 属于哪个折叠组
}

export interface CommandContext {
  getElement: (id: string) => ElementData | undefined;
  addElement: (el: ElementData) => void;
  removeElement: (id: string) => void;
  setProp: (id: string, prop: string, value: any) => void;
  getProp: (id: string, prop: string) => any;
}
```

```typescript
// editor/client/src/commands/AddElement.ts
import type { Command } from "../store/undo";
import type { ElementData, CommandContext } from "./types";

export class AddElement implements Command {
  readonly type = "AddElement";
  readonly description: string;

  constructor(
    private el: ElementData,
    private ctx: CommandContext,
  ) {
    this.description = `添加 <${el.tag}>`;
  }

  execute(): void {
    this.ctx.addElement(this.el);
  }

  undo(): void {
    this.ctx.removeElement(this.el.id);
  }
}
```

```typescript
// editor/client/src/commands/SetProp.ts
import type { Command } from "../store/undo";
import type { CommandContext } from "./types";

export class SetProp implements Command {
  readonly type = "SetProp";
  readonly description: string;

  private oldValue: any;

  constructor(
    private elementId: string,
    private prop: string,
    private newValue: any,
    private ctx: CommandContext,
  ) {
    this.description = `修改 ${prop}`;
    this.oldValue = undefined;
  }

  execute(): void {
    this.oldValue = this.ctx.getProp(this.elementId, this.prop);
    this.ctx.setProp(this.elementId, this.prop, this.newValue);
  }

  undo(): void {
    this.ctx.setProp(this.elementId, this.prop, this.oldValue);
  }
}
```

```typescript
// editor/client/src/commands/registry.ts
import { AddElement } from "./AddElement";
import { RemoveElement } from "./RemoveElement";
import { SetProp } from "./SetProp";
import { AddWire } from "./AddWire";
import { RemoveWire } from "./RemoveWire";

/** 所有 Command 类型注册表。新 Command 类型在此注册后，撤销栈可以识别。 */
export const COMMAND_REGISTRY = {
  AddElement,
  RemoveElement,
  SetProp,
  AddWire,
  RemoveWire,
} as const;

export type CommandType = keyof typeof COMMAND_REGISTRY;
```

##### 任务 3: `store/editor.ts` — Zustand 编辑器核心状态

```typescript
// editor/client/src/store/editor.ts
/**
 * 编辑器核心 Zustand store。
 *
 * 维护:
 *   - 项目文件树
 *   - 当前打开的文件及其内容
 *   - 画布元素树 (ElementData[])
 *   - 连线拓扑 (WireData[])
 *   - 预览 URL
 *   - 选中元素 ID
 */

import { create } from "zustand";
import { openDB } from "idb";
import { deflate, inflate } from "pako";
import type { ElementData } from "../commands/types";

interface WireData {
  id: string;
  fromId: string;       // 源元素/端口 ID
  fromPort: string;     // 源端口名 (e.g. "onPress", "output")
  toId: string;         // 目标元素/端口 ID
  toPort: string;       // 目标端口名 (e.g. "set_count", "label")
}

interface EditorState {
  projectDir: string | null;
  previewUrl: string | null;
  files: { name: string; path: string; type: "file" | "directory" }[];
  activeFilePath: string | null;
  activeFileContent: string | null;

  elements: ElementData[];       // 画布上的组件树
  wires: WireData[];             // 连线拓扑
  selectedElementId: string | null;

  // Actions
  setProjectDir: (dir: string) => void;
  loadFiles: (dirPath?: string) => Promise<void>;
  openFile: (path: string) => Promise<void>;
  saveFile: () => Promise<void>;
  setPreviewUrl: (url: string) => void;

  addElement: (el: ElementData) => void;
  removeElement: (id: string) => void;
  setProp: (id: string, prop: string, value: any) => void;
  getProp: (id: string, prop: string) => any;
  selectElement: (id: string | null) => void;

  addWire: (wire: WireData) => void;
  removeWire: (id: string) => void;

  persistState: () => Promise<void>;
  restoreState: () => Promise<void>;
}

export const useEditorStore = create<EditorState>((set, get) => ({
  projectDir: null,
  previewUrl: null,
  files: [],
  activeFilePath: null,
  activeFileContent: null,
  elements: [],
  wires: [],
  selectedElementId: null,

  setProjectDir: (dir: string) => set({ projectDir: dir }),

  loadFiles: async (dirPath = ".") => {
    const { listDir } = await import("../api/client");
    const result = await listDir(dirPath);
    if (result.entries) set({ files: result.entries });
  },

  openFile: async (path: string) => {
    const { readFile } = await import("../api/client");
    const result = await readFile(path);
    set({ activeFilePath: path, activeFileContent: result.content });
  },

  saveFile: async () => {
    const { activeFilePath, activeFileContent } = get();
    if (!activeFilePath) return;
    const { writeFile, compile } = await import("../api/client");
    await writeFile(activeFilePath, activeFileContent!);
    // 触发编译
    await compile(activeFilePath);
  },

  setPreviewUrl: (url: string) => set({ previewUrl: url }),

  addElement: (el: ElementData) => set((s) => ({ elements: [...s.elements, el] })),
  removeElement: (id: string) => set((s) => ({ elements: s.elements.filter((e) => e.id !== id) })),
  setProp: (id: string, prop: string, value: any) =>
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === id ? { ...e, props: { ...e.props, [prop]: value } } : e
      ),
    })),
  getProp: (id: string, prop: string) => {
    return get().elements.find((e) => e.id === id)?.props[prop];
  },
  selectElement: (id: string | null) => set({ selectedElementId: id }),

  addWire: (wire: WireData) => set((s) => ({ wires: [...s.wires, wire] })),
  removeWire: (id: string) => set((s) => ({ wires: s.wires.filter((w) => w.id !== id) })),

  persistState: async () => {
    const { elements, wires, activeFilePath, activeFileContent } = get();
    const db = await openDB("omnilang-editor", 1);
    const state = { elements, wires, activeFilePath, activeFileContent };
    const compressed = deflate(new TextEncoder().encode(JSON.stringify(state)));
    await db.put("editor-state", compressed, "current");
  },

  restoreState: async () => {
    try {
      const db = await openDB("omnilang-editor", 1);
      const compressed = await db.get("editor-state", "current");
      if (!compressed) return;
      const json = new TextDecoder().decode(inflate(compressed));
      const state = JSON.parse(json);
      set({
        elements: state.elements || [],
        wires: state.wires || [],
        activeFilePath: state.activeFilePath || null,
        activeFileContent: state.activeFileContent || null,
      });
    } catch (e) {
      console.warn("[Editor] Failed to restore state:", e);
    }
  },
}));
```

##### 任务 4: `utils/diff.ts` — .omni diff 渲染

```typescript
// editor/client/src/utils/diff.ts
/**
 * 简单的行级 diff，类似 unified diff 格式。
 * 用于在预览区展示 omnic watch --diff 的输出。
 */

export interface DiffHunk {
  oldStart: number;
  newStart: number;
  lines: { type: "add" | "remove" | "context"; content: string }[];
}

> **⚠ F4 — diff 行号规范化（M1 必须项）。** ReactCodegen 每行生成 `// .omni:{sourceLine} — component {name}` 注释。修改 .omni 第 N 行 → 所有 N 行之后的生成代码行号漂移 → 每次都产生虚假 diff。**diff 前必须 strip 行号注释再比较**：`const canonical = (s: string) => s.replace(/\/\/\s*\.omni:\d+.*/g, "").replace(/\/\*\s*\.omni:\d+\s*\*\//g, "");`

export function computeDiff(oldText: string, newText: string): DiffHunk[] {
  // canonicalize: strip source line number comments before diffing
  oldText = canonicalizeSource(oldText);
  newText = canonicalizeSource(newText);
  const oldLines = oldText.split("\n");
  const newLines = newText.split("\n");

  // 简化实现：逐行比较，标记增删
  // 生产环境应使用 Myers diff 算法 (npm: diff)
  const hunks: DiffHunk[] = [];
  const maxLen = Math.max(oldLines.length, newLines.length);

  for (let i = 0; i < maxLen; i++) {
    const oldLine = oldLines[i];
    const newLine = newLines[i];
    if (oldLine === newLine) continue;
    hunks.push({
      oldStart: i + 1,
      newStart: i + 1,
      lines: [
        ...(oldLine !== undefined ? [{ type: "remove" as const, content: oldLine }] : []),
        ...(newLine !== undefined ? [{ type: "add" as const, content: newLine }] : []),
      ],
    });
  }
  return hunks;
}

export function renderDiffToHTML(hunks: DiffHunk[]): string {
  return hunks
    .map((h) =>
      h.lines
        .map(
          (l) =>
            `<span style="color:${l.type === "add" ? "#16a34a" : l.type === "remove" ? "#dc2626" : "inherit"}">${
              l.type === "add" ? "+" : l.type === "remove" ? "-" : " "
            } ${escapeHTML(l.content)}</span>`
        )
        .join("<br>")
    )
    .join("<br>");
}

function escapeHTML(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function canonicalizeSource(code: string): string {
  // strip source line number comments that would produce spurious diffs
  return code
    .replace(/\/\/\s*\.omni:\d+.*$/gm, "// .omni:-")
    .replace(/\/\*\s*\.omni:\d+\s*\*\//g, "/* .omni:- */");
}
```

##### 任务 5: `editor/server/api/watch.py` — WebSocket 实时推送

```python
# editor/server/api/watch.py
"""WebSocket /api/watch — 文件监听 + 实时编译推送 + heartbeat。"""

import asyncio
from pathlib import Path
from fastapi import APIRouter, WebSocket, WebSocketDisconnect, Request
# F5: 复用现有 omnic watch (os.path.getmtime 轮询，omniview/cli.py 第 596 行)
# 不使用 watchdog — 避免双文件监听竞态
import threading
import time

router = APIRouter()


class OmniFilePoller:
    """F5: 轮询式文件监听，复用 omnic watch 的 os.path.getmtime 策略。

    不用 watchdog — 避免与现有 omnic watch 的双文件监听竞态。
    omnic watch (omniview/cli.py:596) 每 500ms 轮询 os.path.getmtime。
    """

    def __init__(self, websocket: WebSocket, project_dir: Path, interval: float = 0.5):
        self._ws = websocket
        self._project_dir = project_dir
        self._interval = interval
        self._mtimes: dict[str, float] = {}  # 文件路径 → 上次 mtime
        self._running = False
        self._thread: threading.Thread | None = None

    def start(self):
        self._running = True
        self._thread = threading.Thread(target=self._poll_loop, daemon=True)
        self._thread.start()

    def _poll_loop(self):
        while self._running:
            time.sleep(self._interval)
            for omni_file in self._project_dir.rglob("*.omni"):
                if omni_file.name.startswith("."):
                    continue
                try:
                    mtime = omni_file.stat().st_mtime
                except OSError:
                    continue
                key = str(omni_file)
                if self._mtimes.get(key) != mtime:
                    self._mtimes[key] = mtime
                    # debounce: 等 200ms 无后续修改再触发
                    threading.Timer(0.2, self._trigger_compile, args=[omni_file]).start()

    def _trigger_compile(self, omni_file: Path):
        rel_path = str(omni_file.relative_to(self._project_dir)).replace("\\", "/")
        # ... 调用 compile pipeline (同 compile.py) ...
        # await self._ws.send_json({ "type": "compiled", ... })


@router.websocket("/watch")
async def watch_endpoint(ws: WebSocket, request: Request):
    await ws.accept()
    project_dir = Path(request.app.state.project_dir)
    poller = OmniFilePoller(ws, project_dir)
    poller.start()

    try:
        while True:
            data = await ws.receive_json()
            if data.get("type") == "ping":
                from editor.server.main import heartbeat_manager
                heartbeat_manager.record_ping()
    except WebSocketDisconnect:
        pass
    finally:
        poller._running = False
```

---

**M1b 验收标准：**
- [ ] `omnic serve --dir ./project` 启动，浏览器自动打开
- [ ] `omniview/cli.py` 中有 `cmd_serve()` 入口；`omnilang/cli.py` 有 thin alias（F6）
- [ ] `omniview/api.py` 中有 `compile_omni(source, target) → CompileResult`（G1）
- [ ] `CompileError` 统一格式：parser/lowering/codegen 都用 `{file, line, col, severity, code, message}`（G2）
- [ ] WebSocket 推送 `{ type: "compiled", files, diff, errors }` — debounce 200ms 后推送（G3）
- [ ] `pyproject.toml` 加 `editor` optional-deps + `editor*` 包注册（G4）
- [ ] `--dir` 必填，`GET /api/project` 返回 `{ dir, target }`（G5）
- [ ] `create_parser()` + `create_omniview_parser()` 有 `@lru_cache`，热路径 < 1ms（F3）
- [ ] diff 前 strip 行号注释（F4）+ 文件监听用轮询（F5）
- [ ] 修改 .omni 文件 → 编译器自动运行 → 预览区 < 2s 刷新
- [ ] Ctrl+Z 撤销最近 50 步操作，IndexedDB 持久化
- [ ] 关闭浏览器标签 → 15s 后 Python 进程自动退出
- [ ] 浏览器 Tab 后台 30min 恢复 → 编辑器状态从 IndexedDB 重建
- [ ] ElementData 含 `nodeType` 判别器 + M3+ 槽位（ports/renderMode/isFolded/typeAnnotation/storeName）

---

### M2: 属性面板 + 事件绑定 + 画布性能基础（3-4 周）

> **R2 调整：** Zustand 从 M1b 推迟到 M2；脏标记从 M1b 推迟到 M2。M1b 用 React useState 管理简单状态，预览为 iframe 全量刷新。

**目标：** 引入 Zustand 编辑器状态管理 → DOM 画布布局渲染 → 属性面板编辑 → 预览实时响应

---

#### M2.1 — 画布布局视图（第 1-2 周）

##### `canvas/LayoutView.tsx` — DOM 布局渲染器

```typescript
// editor/client/src/canvas/LayoutView.tsx
/**
 * 用 HTML DOM 渲染 .omni view 元素树。
 *
 * 性能策略 (T0-3):
 *   - 虚拟滚动: IntersectionObserver 只渲染视口内 + 100px 缓冲区的元素
 *   - 脏标记: 每个元素标记 dirty flag，只有受影响的元素 re-render
 *   - 批量更新: requestAnimationFrame 中批量提交 DOM 修改
 */

import { useRef, useEffect, useCallback } from "react";
import { useEditorStore } from "../store/editor";

// ── OmniView tag → HTML tag 映射 ──────────

const TAG_MAP: Record<string, keyof HTMLElementTagNameMap> = {
  Container: "div", Flex: "div", Grid: "div",
  Text: "span", Heading: "h2", Image: "img",
  Button: "button", Input: "input", TextArea: "textarea",
  Select: "select", List: "ul", ListItem: "li",
  Card: "div", Form: "form", Divider: "hr",
  Spinner: "div", Progress: "progress", Alert: "div",
};

// ── 虚拟视口配置 ─────────────────────────

const OVERSCAN = 100; // px, 视口外缓冲区

interface ViewportBounds {
  top: number;
  bottom: number;
}

function useVirtualViewport(containerRef: React.RefObject<HTMLDivElement>): ViewportBounds {
  // 简化实现：监听滚动，计算可见区域
  const bounds = useRef<ViewportBounds>({ top: 0, bottom: window.innerHeight });
  // 生产实现: IntersectionObserver per element
  return bounds.current;
}

// ── 脏标记系统 ───────────────────────────

interface DirtyTracker {
  isDirty: (id: string) => boolean;
  markDirty: (id: string) => void;
  markClean: (id: string) => void;
  /** 传播到所有子元素 */
  markSubtreeDirty: (id: string) => void;
}

function useDirtyTracker(): DirtyTracker {
  const dirtySet = useRef(new Set<string>());
  const childrenMap = useRef(new Map<string, string[]>());

  return {
    isDirty: (id: string) => dirtySet.current.has(id),
    markDirty: (id: string) => { dirtySet.current.add(id); },
    markClean: (id: string) => { dirtySet.current.delete(id); },
    markSubtreeDirty: (id: string) => {
      dirtySet.current.add(id);
      const children = childrenMap.current.get(id) || [];
      for (const childId of children) {
        dirtySet.current.add(childId);
      }
    },
  };
}

// ── 主组件 ────────────────────────────────

export function LayoutView() {
  const containerRef = useRef<HTMLDivElement>(null);
  const elements = useEditorStore((s) => s.elements);
  const selectedId = useEditorStore((s) => s.selectedElementId);
  const selectElement = useEditorStore((s) => s.selectElement);
  const viewport = useVirtualViewport(containerRef);

  const renderElement = useCallback((el: typeof elements[0]) => {
    const htmlTag = TAG_MAP[el.tag] || "div";
    const isSelected = el.id === selectedId;

    // 事件 prop → HTML 属性映射 (React 目标)
    const htmlProps: Record<string, any> = {};
    for (const [key, value] of Object.entries(el.props)) {
      if (key === "label") htmlProps["data-label"] = value;
      else if (key === "test-id") htmlProps["data-testid"] = value;
    }

    return (
      <div
        key={el.id}
        data-omni-id={el.id}
        data-omni-tag={el.tag}
        onClick={(e) => { e.stopPropagation(); selectElement(el.id); }}
        style={{
          outline: isSelected ? "2px solid #4F46E5" : "none",
          outlineOffset: 2,
          cursor: "pointer",
        }}
        {...htmlProps}
      >
        {/* 递归渲染子元素 */}
        {el.children.map((childId) => {
          const child = elements.find((e) => e.id === childId);
          return child ? renderElement(child) : null;
        })}
      </div>
    );
  }, [elements, selectedId, selectElement]);

  // 找到根元素（parentId === null 的元素）
  const rootElements = elements.filter((e) => !e.parentId);

  return (
    <div
      ref={containerRef}
      style={{ width: "100%", height: "100%", overflow: "auto", position: "relative" }}
    >
      {rootElements.map(renderElement)}
    </div>
  );
}
```

---

#### M2.2 — 属性面板（第 2-3 周）

##### `panels/PropertyPanel.tsx` — 属性编辑器

```typescript
// editor/client/src/panels/PropertyPanel.tsx
/**
 * 右侧属性面板。
 *
 * 根据选中元素的 tag 和元数据协议，动态渲染属性编辑器:
 *   - string prop   → 文本输入
 *   - enum prop     → 下拉选择
 *   - bool prop     → 复选框
 *   - number prop   → 数值滑块
 *   - event prop    → 事件绑定选择器
 *   - style prop    → 颜色选择器 / 间距选择器 / 字体选择器
 */

import { useEditorStore } from "../store/editor";
import { useUndoStore } from "../store/undo";
import { SetProp } from "../commands/SetProp";
import { COMPONENT_METADATA } from "../metadata/registry";

export function PropertyPanel() {
  const elements = useEditorStore((s) => s.elements);
  const selectedId = useEditorStore((s) => s.selectedElementId);
  const pushUndo = useUndoStore((s) => s.push);

  const el = elements.find((e) => e.id === selectedId);
  if (!el) {
    return <div style={{ padding: 16, color: "#9ca3af" }}>选择元素以编辑属性</div>;
  }

  const meta = COMPONENT_METADATA[el.tag];
  if (!meta) return <div style={{ padding: 16, color: "#9ca3af" }}>无元数据: {el.tag}</div>;

  const handlePropChange = (prop: string, value: any) => {
    const oldValue = el.props[prop];
    if (oldValue === value) return;

    pushUndo(new SetProp(
      el.id, prop, value,
      {
        getElement: (id) => elements.find((e) => e.id === id),
        addElement: () => {},
        removeElement: () => {},
        setProp: (id, p, v) => useEditorStore.getState().setProp(id, p, v),
        getProp: (id, p) => useEditorStore.getState().getProp(id, p),
      }
    ));
  };

  return (
    <div style={{ padding: 12, overflow: "auto", height: "100%" }}>
      <h3 style={{ margin: 0, fontSize: 14 }}>{`<${el.tag}>`} 属性</h3>

      {/* 基础属性 */}
      {Object.entries(meta.props).map(([name, config]) => (
        <PropEditor
          key={name}
          name={name}
          config={config}
          value={el.props[name] ?? config.default}
          onChange={(v) => handlePropChange(name, v)}
        />
      ))}

      {/* 事件属性 */}
      <h4 style={{ margin: "12px 0 4px", fontSize: 12, color: "#6b7280" }}>事件</h4>
      {Object.entries(meta.events).map(([name, config]) => (
        <EventBindingEditor
          key={name}
          eventName={name}
          config={config}
          currentHandler={el.props[name]}
          onChange={(v) => handlePropChange(name, v)}
        />
      ))}

      {/* 样式属性 */}
      <h4 style={{ margin: "12px 0 4px", fontSize: 12, color: "#6b7280" }}>样式</h4>
      {meta.styleProperties.map((prop) => (
        <StylePropertyEditor
          key={prop}
          name={prop}
          value={el.props[prop]}
          onChange={(v) => handlePropChange(prop, v)}
        />
      ))}
    </div>
  );
}

// ── 子组件 ────────────────────────────────

function PropEditor({ name, config, value, onChange }: {
  name: string; config: any; value: any; onChange: (v: any) => void;
}) {
  switch (config.editor) {
    case "text":
      return (
        <label style={{ display: "block", marginTop: 8 }}>
          <span style={{ fontSize: 11, color: "#6b7280" }}>{name}</span>
          <input
            type="text"
            value={value ?? ""}
            onChange={(e) => onChange(e.target.value)}
            style={{ width: "100%", padding: 4, marginTop: 2, boxSizing: "border-box" }}
          />
        </label>
      );
    case "select":
      return (
        <label style={{ display: "block", marginTop: 8 }}>
          <span style={{ fontSize: 11, color: "#6b7280" }}>{name}</span>
          <select
            value={value ?? config.default}
            onChange={(e) => onChange(e.target.value)}
            style={{ width: "100%", padding: 4, marginTop: 2 }}
          >
            {config.values.map((v: string) => (
              <option key={v} value={v}>{v}</option>
            ))}
          </select>
        </label>
      );
    case "checkbox":
      return (
        <label style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 8 }}>
          <input
            type="checkbox"
            checked={value ?? false}
            onChange={(e) => onChange(e.target.checked)}
          />
          <span style={{ fontSize: 12 }}>{name}</span>
        </label>
      );
    default:
      return null;
  }
}

function EventBindingEditor({ eventName, config, currentHandler, onChange }: {
  eventName: string; config: any; currentHandler: any; onChange: (v: any) => void;
}) {
  // 从 editor store 获取所有 fn/signal/store action
  // 生产实现: 遍历 AST 收集所有可绑定目标
  return (
    <label style={{ display: "block", marginTop: 8 }}>
      <span style={{ fontSize: 11, color: "#6b7280" }}>{eventName}</span>
      <select
        value={currentHandler ?? ""}
        onChange={(e) => onChange(e.target.value || null)}
        style={{ width: "100%", padding: 4, marginTop: 2 }}
      >
        <option value="">无</option>
        <option value="__new_fn__">＋ 新建函数...</option>
        <option value="__inline__">＋ 内联函数...</option>
        {/* 动态生成: 遍历 fn / signal / store action */}
      </select>
    </label>
  );
}

function StylePropertyEditor({ name, value, onChange }: {
  name: string; value: any; onChange: (v: any) => void;
}) {
  // 颜色属性 → 颜色选择器
  if (name.includes("color")) {
    return (
      <label style={{ display: "block", marginTop: 8 }}>
        <span style={{ fontSize: 11, color: "#6b7280" }}>{name}</span>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <input type="color" value={value ?? "#000000"} onChange={(e) => onChange(e.target.value)} />
          <input
            type="text"
            value={value ?? ""}
            onChange={(e) => onChange(e.target.value)}
            style={{ flex: 1, padding: 4 }}
          />
        </div>
      </label>
    );
  }
  // 间距/尺寸 → 带单位的数值输入
  if (["width", "height", "padding", "margin", "gap"].includes(name)) {
    return (
      <label style={{ display: "block", marginTop: 8 }}>
        <span style={{ fontSize: 11, color: "#6b7280" }}>{name}</span>
        <input
          type="text"
          value={value ?? ""}
          onChange={(e) => onChange(e.target.value)}
          placeholder="e.g. 16px"
          style={{ width: "100%", padding: 4, marginTop: 2 }}
        />
      </label>
    );
  }
  return null;
}
```

---

#### M2.3 — 组件元数据协议（第 4 周）

```json
// omniview/components/Button.json
{
  "component": "Button",
  "category": "Actions",
  "description": "可点击的按钮组件",
  "props": {
    "label": { "type": "string", "default": "Button", "editor": "text" },
    "variant": { "type": "enum", "values": ["primary", "secondary", "danger", "ghost"], "default": "primary", "editor": "select" },
    "disabled": { "type": "bool", "default": false, "editor": "checkbox" },
    "loading": { "type": "bool", "default": false, "editor": "checkbox" }
  },
  "events": {
    "onPress": { "description": "点击事件", "payload": null },
    "onHover": { "description": "鼠标悬停", "payload": null },
    "onFocus": { "description": "获得焦点", "payload": null }
  },
  "styleProperties": ["width", "height", "padding", "margin", "background_color", "border_radius", "font_size", "font_weight"],
  "children": true,
  "targets": ["react", "vue", "dioxus", "flutter"],
  "targetPropMapping": {
    "react": { "onPress": "onClick", "className": "className" },
    "vue": { "onPress": "@click", "className": "class" },
    "flutter": { "onPress": "onPressed", "className": null }
  }
}
```

```typescript
// editor/client/src/metadata/registry.ts
/** 组件元数据注册表。M2 期间逐个为内置组件编写 JSON。 */

import ButtonMeta from "../../../omniview/components/Button.json";
import TextMeta from "../../../omniview/components/Text.json";
import InputMeta from "../../../omniview/components/Input.json";
// ... 其余组件

export interface ComponentMeta {
  component: string;
  category: string;
  props: Record<string, { type: string; default?: any; values?: string[]; editor: string }>;
  events: Record<string, { description: string; payload: string | null }>;
  styleProperties: string[];
  children: boolean;
  targets: string[];
  targetPropMapping: Record<string, Record<string, string | null>>;
}

export const COMPONENT_METADATA: Record<string, ComponentMeta> = {
  Button: ButtonMeta as ComponentMeta,
  Text: TextMeta as ComponentMeta,
  Input: InputMeta as ComponentMeta,
  // ...
};
```

---

**M2 验收标准：**
- [ ] 画布渲染 50 个组件不卡顿
- [ ] 点击元素 → 属性面板显示所有 editable props
- [ ] 修改 label → 预览刷新 → 撤销恢复原值
- [ ] 事件绑定下拉列出 fn / signal / Store action
- [ ] 所有内置组件有元数据 JSON

---

### M3: 拖拽 + 连线 + 代码块 + 响应式 + 设计令牌（5 周）

**目标：** 从零可视化搭建页面，连线编辑数据流，响应式布局

---

#### M3.1 — 组件面板 + 拖拽画布（第 1-2 周）

```typescript
// editor/client/src/panels/ComponentPalette.tsx
export function ComponentPalette() {
  const categories = useMemo(() => {
    const cats = new Map<string, ComponentMeta[]>();
    for (const meta of Object.values(COMPONENT_METADATA)) {
      const list = cats.get(meta.category) || [];
      list.push(meta);
      cats.set(meta.category, list);
    }
    return cats;
  }, []);

  const handleDragStart = (e: React.DragEvent, tag: string) => {
    e.dataTransfer.setData("application/omni-component", tag);
    e.dataTransfer.effectAllowed = "copy";
  };

  return (
    <div style={{ padding: 8 }}>
      {Array.from(categories.entries()).map(([category, components]) => (
        <div key={category}>
          <h4 style={{ fontSize: 11, color: "#6b7280", textTransform: "uppercase", margin: "8px 0 4px" }}>
            {category}
          </h4>
          {components.map((meta) => (
            <div
              key={meta.component}
              draggable
              onDragStart={(e) => handleDragStart(e, meta.component)}
              style={{
                padding: "6px 8px",
                borderRadius: 4,
                cursor: "grab",
                fontSize: 13,
              }}
            >
              {meta.component}
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
```

```typescript
// editor/client/src/canvas/Canvas.tsx (拖放接收)
export function Canvas() {
  const elements = useEditorStore((s) => s.elements);
  const pushUndo = useUndoStore((s) => s.push);

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    const tag = e.dataTransfer.getData("application/omni-component");
    if (!tag) return;

    const id = crypto.randomUUID();
    const newElement: ElementData = {
      id, tag,
      props: { label: tag },
      children: [],
      parentId: null,
    };

    pushUndo(new AddElement(newElement, getCommandContext()));
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
  };

  return (
    <div onDrop={handleDrop} onDragOver={handleDragOver} style={{ width: "100%", height: "100%" }}>
      <LayoutView />
      {showWires && <WireOverlay />}
    </div>
  );
}
```

```typescript
// editor/client/src/panels/OutlinePanel.tsx (组件树)
export function OutlinePanel() {
  const elements = useEditorStore((s) => s.elements);
  const selectedId = useEditorStore((s) => s.selectedElementId);
  const selectElement = useEditorStore((s) => s.selectElement);

  const roots = elements.filter((e) => !e.parentId);

  const renderNode = (el: ElementData, depth: number) => (
    <div key={el.id}>
      <div
        onClick={() => selectElement(el.id)}
        style={{
          paddingLeft: 12 + depth * 16,
          padding: "4px 8px",
          cursor: "pointer",
          background: el.id === selectedId ? "#EEF2FF" : "transparent",
          fontSize: 13,
        }}
      >
        {`<${el.tag}>`}
      </div>
      {el.children.map((childId) => {
        const child = elements.find((e) => e.id === childId);
        return child ? renderNode(child, depth + 1) : null;
      })}
    </div>
  );

  return <div style={{ overflow: "auto", height: "100%" }}>{roots.map((r) => renderNode(r, 0))}</div>;
}
```

---

#### M3.2 — 连线画布 + Canvas-DOM 同步（第 2-3 周）

```typescript
// editor/client/src/canvas/WireOverlay.tsx
/**
 * Canvas2D 透明叠加层，绘制贝塞尔连线和端口。
 *
 * CRT-4 缓解 (Canvas-DOM 同步撕裂):
 *   - ResizeObserver 监听 DOM 元素位置变化
 *   - requestAnimationFrame 双缓冲绘制
 *   - DOM reflow 完成后 50ms debounce 再触发 redraw
 */

import { useRef, useEffect, useCallback } from "react";
import { useEditorStore } from "../store/editor";

interface PortPosition {
  elementId: string;
  portName: string;
  x: number;
  y: number;
  direction: "in" | "out";
}

export function WireOverlay() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const wires = useEditorStore((s) => s.wires);
  const elements = useEditorStore((s) => s.elements);
  const selectedId = useEditorStore((s) => s.selectedElementId);

  // ── 端口位置计算 ─────────────────────────

  const getPortPositions = useCallback((): PortPosition[] => {
    const positions: PortPosition[] = [];
    for (const el of elements) {
      const domEl = document.querySelector(`[data-omni-id="${el.id}"]`);
      if (!domEl) continue;
      const rect = domEl.getBoundingClientRect();
      const canvasRect = canvasRef.current?.getBoundingClientRect();
      if (!canvasRect) continue;

      const offsetX = rect.left - canvasRect.left;
      const offsetY = rect.top - canvasRect.top;

      // 输出端口：右侧中间
      positions.push({
        elementId: el.id,
        portName: "output",
        x: offsetX + rect.width,
        y: offsetY + rect.height / 2,
        direction: "out",
      });
      // 输入端口：左侧中间
      positions.push({
        elementId: el.id,
        portName: "input",
        x: offsetX,
        y: offsetY + rect.height / 2,
        direction: "in",
      });
    }
    return positions;
  }, [elements]);

  // ── 绘制 ─────────────────────────────────

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // DPR 缩放
    const dpr = window.devicePixelRatio || 1;
    const container = canvas.parentElement!;
    canvas.width = container.clientWidth * dpr;
    canvas.height = container.clientHeight * dpr;
    canvas.style.width = `${container.clientWidth}px`;
    canvas.style.height = `${container.clientHeight}px`;
    ctx.scale(dpr, dpr);

    ctx.clearRect(0, 0, canvas.width, canvas.height);

    const ports = getPortPositions();

    // 只绘制选中元素相关的连线 (防蜘蛛网 T0-4)
    const visibleWires = selectedId
      ? wires.filter((w) => w.fromId === selectedId || w.toId === selectedId)
      : wires;

    for (const wire of visibleWires) {
      const fromPort = ports.find((p) => p.elementId === wire.fromId && p.portName === wire.fromPort);
      const toPort = ports.find((p) => p.elementId === wire.toId && p.portName === wire.toPort);
      if (!fromPort || !toPort) continue;

      // 贝塞尔曲线
      ctx.beginPath();
      ctx.moveTo(fromPort.x, fromPort.y);
      const cp1x = fromPort.x + Math.abs(toPort.x - fromPort.x) * 0.5;
      ctx.bezierCurveTo(cp1x, fromPort.y, cp1x, toPort.y, toPort.x, toPort.y);
      ctx.strokeStyle = "#4F46E5";
      ctx.lineWidth = 2;
      ctx.stroke();

      // 箭头
      drawArrow(ctx, toPort.x, toPort.y, fromPort.x < toPort.x ? "right" : "left");
    }

    // 端口圆点
    for (const port of ports) {
      if (selectedId && port.elementId !== selectedId) continue;
      ctx.beginPath();
      ctx.arc(port.x, port.y, 4, 0, Math.PI * 2);
      ctx.fillStyle = port.direction === "out" ? "#4F46E5" : "#10B981";
      ctx.fill();
    }
  }, [wires, selectedId, getPortPositions]);

  // ── ResizeObserver + rAF 同步 ─────────────

  useEffect(() => {
    const container = canvasRef.current?.parentElement;
    if (!container) return;

    let rafId: number;
    let debounceTimer: number;

    const debouncedDraw = () => {
      clearTimeout(debounceTimer);
      debounceTimer = window.setTimeout(() => {
        rafId = requestAnimationFrame(draw);
      }, 50);
    };

    const observer = new ResizeObserver(debouncedDraw);
    observer.observe(container);

    // 初始绘制
    debouncedDraw();

    return () => {
      observer.disconnect();
      cancelAnimationFrame(rafId);
      clearTimeout(debounceTimer);
    };
  }, [draw]);

  return (
    <canvas
      ref={canvasRef}
      style={{
        position: "absolute",
        top: 0, left: 0,
        pointerEvents: "none",       // 不阻挡 DOM 事件
        zIndex: 10,
      }}
    />
  );
}

function drawArrow(ctx: CanvasRenderingContext2D, x: number, y: number, direction: "left" | "right") {
  const size = 6;
  const sign = direction === "right" ? -1 : 1;
  ctx.beginPath();
  ctx.moveTo(x, y);
  ctx.lineTo(x + sign * size, y - size / 2);
  ctx.lineTo(x + sign * size, y + size / 2);
  ctx.closePath();
  ctx.fillStyle = "#4F46E5";
  ctx.fill();
}
```

---

#### M3.3 — 画布代码块（第 3-4 周）

```typescript
// editor/client/src/canvas/CodeBlock.tsx
/**
 * 画布上的代码块节点。
 *
 * signal 块:    输出端口 (当前值) + 输入端口 (setter)
 * computed 块:  输出端口 (派生值) + 输入端口 (依赖)
 * Store signal: 输出端口 (只读) 
 * Store action: 调用端口 (触发)
 *
 * 无连线 → compact chip:  ⬤ count: i32 = 5
 * 有连线 → 展开方块:      ┌──────────┐
 *                          │  signal  │
 *                          │  count   │
 *                          │ ⬤ → i32 │
 *                          │ ⬤ set   │
 *                          └──────────┘
 */

export interface CodeBlockData {
  id: string;
  type: "signal" | "computed" | "store_signal" | "store_action";
  name: string;
  typeAnnotation: string;        // "i32", "String", "Array<Todo>", etc.
  initialValue?: any;
  storeName?: string;            // 仅 store_signal / store_action
  hasWires: boolean;
  isFolded: boolean;
}

export function CodeBlock({ data }: { data: CodeBlockData }) {
  if (data.isFolded || !data.hasWires) {
    // Compact chip 模式
    return (
      <div style={{
        display: "inline-flex", alignItems: "center", gap: 4,
        padding: "2px 8px", borderRadius: 12, fontSize: 12,
        background: "#F3F4F6", border: "1px solid #E5E7EB",
      }}>
        <span style={{ width: 6, height: 6, borderRadius: "50%", background: "#4F46E5" }} />
        {data.name}: {data.typeAnnotation}
        {data.initialValue !== undefined && ` = ${data.initialValue}`}
      </div>
    );
  }

  // 展开方块模式
  const ports = getBlockPorts(data);

  return (
    <div style={{
      padding: 8, borderRadius: 8, minWidth: 120,
      background: "#FFFFFF", border: "2px solid #4F46E5",
      boxShadow: "0 2px 8px rgba(0,0,0,0.1)",
    }}>
      <div style={{ fontSize: 10, color: "#6B7280", textTransform: "uppercase" }}>{data.type}</div>
      <div style={{ fontSize: 13, fontWeight: 600 }}>{data.storeName ? `${data.storeName}.` : ""}{data.name}</div>
      <div style={{ fontSize: 11, color: "#6B7280" }}>{data.typeAnnotation}</div>
      {/* 端口渲染由 WireOverlay 负责，这里是点击目标 */}
    </div>
  );
}

function getBlockPorts(data: CodeBlockData) {
  switch (data.type) {
    case "signal":
      return [
        { name: "output", direction: "out", type: data.typeAnnotation },
        { name: "set", direction: "in", type: data.typeAnnotation },
      ];
    case "computed":
      return [
        { name: "output", direction: "out", type: data.typeAnnotation },
        { name: "deps", direction: "in", type: "dependencies" },
      ];
    case "store_signal":
      return [{ name: "output", direction: "out", type: data.typeAnnotation, readonly: true }];
    case "store_action":
      return [{ name: "call", direction: "in", type: "trigger" }];
  }
}
```

```typescript
// editor/client/src/canvas/WireLabel.tsx
/**
 * fn 调用的连线标签。
 *
 * 不显示为块——在每个调用点显示 [fnName(args)]。
 * 双击跳转到代码面板。
 */

export function WireLabel({ fnName, args, onDoubleClick }: {
  fnName: string; args: string[]; onDoubleClick: () => void;
}) {
  const argsStr = args.join(", ");
  return (
    <span
      onDoubleClick={onDoubleClick}
      style={{
        display: "inline-block", padding: "2px 8px", borderRadius: 4,
        background: "#FEF3C7", border: "1px solid #F59E0B",
        fontSize: 12, cursor: "pointer", fontFamily: "monospace",
      }}
      title="双击跳转到代码面板"
    >
      [{fnName}({argsStr})]
    </span>
  );
}
```

---

#### M3.4 — 响应式约束布局（第 4-5 周）

```typescript
// editor/client/src/canvas/LayoutView.tsx (补充响应式部分)

/** 画布顶栏断点切换器。 */
export function BreakpointSwitcher({ current, onChange }: {
  current: string; onChange: (bp: string) => void;
}) {
  const breakpoints = [
    { name: "Desktop", width: 1440 },
    { name: "Tablet", width: 768 },
    { name: "Mobile", width: 375 },
  ];

  return (
    <div style={{ display: "flex", gap: 4, padding: "4px 12px", borderBottom: "1px solid #e5e7eb" }}>
      {breakpoints.map((bp) => (
        <button
          key={bp.name}
          onClick={() => onChange(bp.name)}
          style={{
            padding: "4px 12px", borderRadius: 4, border: "none", fontSize: 12,
            background: current === bp.name ? "#4F46E5" : "#F3F4F6",
            color: current === bp.name ? "#FFF" : "#374151",
            cursor: "pointer",
          }}
        >
          {bp.name} ({bp.width}px)
        </button>
      ))}
    </div>
  );
}

/** 响应式约束覆盖数据结构。 */
interface ResponsiveOverride {
  elementId: string;
  breakpoint: "tablet" | "mobile";
  overrides: Record<string, any>;
}

// 在 .omni.json 中存储:
// element.props.responsive = {
//   "@tablet": { "gap": "12px", "flexDirection": "row" },
//   "@mobile": { "gap": "8px", "flexDirection": "column" }
// }
```

---

#### M3.5 — 设计令牌系统（第 5 周）

```typescript
// editor/client/src/store/tokens.ts

interface ThemeTokens {
  colors: Record<string, string>;
  spacing: Record<string, string>;
  typography: Record<string, { fontSize: string; fontWeight: string; lineHeight: string }>;
  radius: Record<string, string>;
}

const DEFAULT_TOKENS: ThemeTokens = {
  colors: {
    primary: "#4F46E5",
    primaryHover: "#4338CA",
    surface: "#FFFFFF",
    textPrimary: "#1F2937",
    textSecondary: "#6B7280",
  },
  spacing: {
    xs: "4px", sm: "8px", md: "16px", lg: "24px", xl: "32px",
  },
  typography: {
    heading1: { fontSize: "32px", fontWeight: "bold", lineHeight: "40px" },
    body: { fontSize: "16px", fontWeight: "normal", lineHeight: "24px" },
  },
  radius: {
    sm: "4px", md: "8px", lg: "12px",
  },
};

interface TokenState {
  tokens: ThemeTokens;
  /** 元素属性绑定的令牌引用: elementId.prop → "colors.primary" */
  bindings: Record<string, string>;

  updateToken: (path: string, value: any) => void;
  bindToken: (elementId: string, prop: string, tokenPath: string) => void;
  /** 解析所有绑定，返回更新后的元素属性。 */
  resolveBindings: (elements: ElementData[]) => ElementData[];
}

export const useTokenStore = create<TokenState>((set, get) => ({
  tokens: DEFAULT_TOKENS,
  bindings: {},

  updateToken: (path: string, value: any) => {
    set((state) => {
      const newTokens = { ...state.tokens };
      const parts = path.split(".");
      // path = "colors.primary" → tokens.colors.primary = value
      if (parts.length === 2) {
        (newTokens as any)[parts[0]][parts[1]] = value;
      }
      return { tokens: newTokens };
    });
    // 触发所有绑定此令牌的元素重新渲染
  },

  bindToken: (elementId: string, prop: string, tokenPath: string) => {
    set((state) => ({
      bindings: { ...state.bindings, [`${elementId}.${prop}`]: tokenPath },
    }));
  },

  resolveBindings: (elements: ElementData[]) => {
    const { tokens, bindings } = get();
    return elements.map((el) => {
      const newProps = { ...el.props };
      for (const [key, tokenPath] of Object.entries(bindings)) {
        const [elId, prop] = key.split(".");
        if (elId !== el.id) continue;
        const parts = tokenPath.split(".");
        if (parts.length === 2) {
          newProps[prop] = (tokens as any)[parts[0]][parts[1]];
        }
      }
      return { ...el, props: newProps };
    });
  },
}));
```

```typescript
// editor/client/src/panels/TokenPanel.tsx
/** 设计令牌编辑面板。 */
export function TokenPanel() {
  const tokens = useTokenStore((s) => s.tokens);
  const updateToken = useTokenStore((s) => s.updateToken);

  return (
    <div style={{ padding: 12 }}>
      <h3 style={{ fontSize: 14, margin: "0 0 8px" }}>设计令牌</h3>

      <h4 style={{ fontSize: 11, color: "#6B7280", textTransform: "uppercase", margin: "8px 0 4px" }}>颜色</h4>
      {Object.entries(tokens.colors).map(([name, value]) => (
        <label key={name} style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 4 }}>
          <input type="color" value={value} onChange={(e) => updateToken(`colors.${name}`, e.target.value)} />
          <span style={{ fontSize: 12 }}>{name}</span>
          <code style={{ fontSize: 11, color: "#6B7280" }}>{value}</code>
        </label>
      ))}

      <h4 style={{ fontSize: 11, color: "#6B7280", textTransform: "uppercase", margin: "12px 0 4px" }}>间距</h4>
      {Object.entries(tokens.spacing).map(([name, value]) => (
        <label key={name} style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 4 }}>
          <span style={{ fontSize: 12, width: 24 }}>{name}</span>
          <input
            type="text" value={value}
            onChange={(e) => updateToken(`spacing.${name}`, e.target.value)}
            style={{ flex: 1, padding: 2, fontSize: 12 }}
          />
        </label>
      ))}
    </div>
  );
}
```

---

**M3 验收标准：**
- [ ] 从组件面板拖组件到画布
- [ ] 连线模式：画布上创建事件绑定和信号连线
- [ ] signal/computed/Store 块显示正确端口
- [ ] 200 组件 + 300 连线，编辑延迟 < 50ms
- [ ] Desktop/Tablet/Mobile 切换，布局响应
- [ ] 修改设计令牌 → 所有绑定更新

---

### M4: 代码面板 + 代码片段 + 数据连接器（3 周）

#### M4.1 — 代码面板（Monaco Editor）（第 1 周）

```typescript
// editor/client/src/panels/CodePanel.tsx
import { useEffect, useRef } from "react";
import { useEditorStore } from "../store/editor";

// Monaco 动态导入 (减少初始 bundle)
let monacoPromise: Promise<typeof import("monaco-editor")> | null = null;

function getMonaco() {
  if (!monacoPromise) {
    monacoPromise = import("monaco-editor");
  }
  return monacoPromise;
}

export function CodePanel() {
  const containerRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<any>(null);
  const activeFileContent = useEditorStore((s) => s.activeFileContent);

  useEffect(() => {
    if (!containerRef.current) return;

    getMonaco().then((monaco) => {
      if (editorRef.current) {
        // 更新内容而非重建
        const model = editorRef.current.getModel();
        if (model && model.getValue() !== activeFileContent) {
          model.setValue(activeFileContent || "");
        }
        return;
      }

      editorRef.current = monaco.editor.create(containerRef.current!, {
        value: activeFileContent || "",
        language: "omni",           // 自定义 OmniLang 语法高亮
        theme: "vs",
        minimap: { enabled: false },
        lineNumbers: "on",
        scrollBeyondLastLine: false,
        fontSize: 13,
        tabSize: 4,
        automaticLayout: true,
      });

      // 监听内容变化 → 自动保存
      editorRef.current.onDidChangeModelContent(() => {
        const content = editorRef.current.getValue();
        useEditorStore.setState({ activeFileContent: content });
      });
    });

    return () => {
      editorRef.current?.dispose();
      editorRef.current = null;
    };
  }, [activeFileContent]);

  return <div ref={containerRef} style={{ width: "100%", height: "100%" }} />;
}
```

#### M4.2 — 代码片段系统（第 2 周）

```typescript
// editor/client/src/panels/SnippetPanel.tsx
/**
 * 上下文感知的代码片段列表。
 *
 * 根据当前光标在代码面板中的位置（fn body / store body / computed / view 插值），
 * 显示不同的可用片段集。
 */

interface Snippet {
  id: string;
  name: string;
  description: string;
  template: string;               // Tab 占位符: ${1:name}, ${2:type}
  context: "fn_body" | "store_action" | "computed" | "view_interpolation" | "if_condition";
  category: "state" | "store" | "control_flow" | "async" | "events";
}

const BUILTIN_SNIPPETS: Snippet[] = [
  // ── 状态操作 ──
  { id: "set", name: "set signal", description: "更新信号值",
    template: "${1:signal} = ${2:value};",
    context: "fn_body", category: "state" },
  { id: "toggle", name: "toggle signal", description: "切换布尔信号",
    template: "${1:flag} = !${1:flag};",
    context: "fn_body", category: "state" },
  { id: "increment", name: "increment signal", description: "递增数值信号",
    template: "${1:count} = ${1:count} + 1;",
    context: "fn_body", category: "state" },
  { id: "push", name: "push to Array", description: "追加到数组",
    template: "${1:array}.push(${2:item});",
    context: "fn_body", category: "state" },

  // ── Store 操作 ──
  { id: "store-action", name: "store.action()", description: "调用 Store Action",
    template: "${1:store}.${2:action}(${3:payload});",
    context: "fn_body", category: "store" },
  { id: "store-read", name: "read store signal", description: "读取 Store 状态",
    template: "let ${1:local} = ${2:store}.${3:signal};",
    context: "fn_body", category: "store" },

  // ── 控制流 ──
  { id: "if", name: "if condition", description: "条件分支",
    template: "if ${1:condition} {\n    ${2:// body}\n}",
    context: "fn_body", category: "control_flow" },
  { id: "if-else", name: "if / else", description: "条件分支 + else",
    template: "if ${1:condition} {\n    ${2:// then}\n} else {\n    ${3:// else}\n}",
    context: "fn_body", category: "control_flow" },
  { id: "for", name: "for in", description: "遍历循环",
    template: "for ${1:item} in ${2:list} {\n    ${3:// body}\n}",
    context: "fn_body", category: "control_flow" },
  { id: "match", name: "match", description: "模式匹配",
    template: "match ${1:value} {\n    ${2:pattern} => ${3:result},\n    _ => ${4:default},\n}",
    context: "fn_body", category: "control_flow" },

  // ── 异步 ──
  { id: "await", name: "await", description: "等待异步结果",
    template: "let ${1:result} = await ${2:promise};",
    context: "fn_body", category: "async" },

  // ── 事件 ──
  { id: "emit", name: "event emit", description: "发射组件事件",
    template: "event ${1:eventName}(${2:payload});",
    context: "fn_body", category: "events" },
];

export function SnippetPanel({ currentContext }: { currentContext: Snippet["context"] }) {
  const filteredSnippets = BUILTIN_SNIPPETS.filter((s) => s.context === currentContext);
  const categories = new Map<string, Snippet[]>();
  for (const s of filteredSnippets) {
    const list = categories.get(s.category) || [];
    list.push(s);
    categories.set(s.category, list);
  }

  const handleDoubleClick = (snippet: Snippet) => {
    // 通过 postMessage 或其他机制发送给 CodePanel 的 Monaco 实例
    window.dispatchEvent(new CustomEvent("snippet:insert", { detail: snippet }));
  };

  // 拖到画布 → 只读标签
  const handleDragStart = (e: React.DragEvent, snippet: Snippet) => {
    e.dataTransfer.setData("application/omni-snippet", JSON.stringify(snippet));
  };

  return (
    <div style={{ padding: 8, overflow: "auto", height: "100%" }}>
      <h4 style={{ fontSize: 11, color: "#6B7280", textTransform: "uppercase", margin: 0 }}>
        代码片段 ({currentContext.replace("_", " ")})
      </h4>
      {Array.from(categories.entries()).map(([cat, snippets]) => (
        <div key={cat}>
          <div style={{ fontSize: 10, color: "#9CA3AF", margin: "8px 0 2px", textTransform: "uppercase" }}>
            {cat}
          </div>
          {snippets.map((s) => (
            <div
              key={s.id}
              draggable
              onDragStart={(e) => handleDragStart(e, s)}
              onDoubleClick={() => handleDoubleClick(s)}
              style={{
                padding: "4px 8px", borderRadius: 4, cursor: "pointer", fontSize: 12,
              }}
              title={s.description}
            >
              {s.name}
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
```

#### M4.3 — 数据连接器（第 3 周）

```python
# editor/server/api/connector.py (新增)
"""数据连接器脚手架生成器。POST /api/connector/generate"""

from fastapi import APIRouter
from pydantic import BaseModel

router = APIRouter()


class ConnectorConfig(BaseModel):
    type: str            # "rest" | "graphql" | "firebase"
    name: str            # 连接器名称
    url: str = ""        # REST/GraphQL endpoint
    method: str = "GET"  # REST method
    endpoint: str = ""   # REST path, GraphQL query
    responseShape: dict | None = None  # 示例响应 JSON → 类型推断


@router.post("/connector/generate")
async def generate_connector(config: ConnectorConfig):
    """根据配置生成 OmniLang Store + 类型定义 + API 调用脚手架。"""
    if config.type == "rest":
        return _generate_rest_connector(config)
    elif config.type == "graphql":
        return _generate_graphql_connector(config)
    elif config.type == "firebase":
        return _generate_firebase_connector(config)
    else:
        return {"error": f"Unknown connector type: {config.type}"}


def _generate_rest_connector(config: ConnectorConfig):
    """生成 REST 数据连接器代码。"""
    store_name = f"{config.name}Store"
    type_name = config.name

    # 从示例 JSON 推断类型定义
    type_def = infer_type_from_json(config.responseShape or {}, type_name)

    # 生成代码
    code = f"""
// @editor: managed — 由数据连接器生成
// 数据源: {config.method} {config.url}/{config.endpoint}

struct {type_name} {{
{type_def}
}}

store {store_name} {{
    signal {config.name.lower()}s: Array<{type_name}> = [];
    signal loading: bool = false;
    signal error: String? = null;

    async action fetch{config.name}s() {{
        loading = true;
        error = null;
        try {{
            let response = await fetch("{config.url}/{config.endpoint}");
            let data = await response.json();
            {config.name.lower()}s = data;
        }} catch (e) {{
            error = e.message;
        }} finally {{
            loading = false;
        }}
    }}
}}
"""
    return {"code": code, "storeName": store_name, "typeName": type_name}


def infer_type_from_json(data: dict | list, name: str) -> str:
    """从示例 JSON 推断 OmniLang 类型定义。"""
    if isinstance(data, list):
        if len(data) == 0:
            return f"    // 无法推断列表元素类型（空数组）"
        return infer_type_from_json(data[0], name) if isinstance(data[0], dict) else f"    // List item type"

    fields = []
    for key, value in (data if isinstance(data, dict) else {}).items():
        if isinstance(value, bool):
            fields.append(f"    {key}: bool;")
        elif isinstance(value, int):
            fields.append(f"    {key}: i32;")
        elif isinstance(value, float):
            fields.append(f"    {key}: f64;")
        elif isinstance(value, str):
            fields.append(f"    {key}: String;")
        elif isinstance(value, list):
            fields.append(f"    {key}: Array<Unknown>;  // 需手动指定元素类型")
        elif isinstance(value, dict):
            fields.append(f"    {key}: Struct;  // 嵌套对象，需手动展开")
        else:
            fields.append(f"    {key}: String?;  // null")
    return "\n".join(fields)
```

---

**M4 验收标准：**
- [ ] Monaco Editor 代码面板，语法高亮，自动保存
- [ ] 光标在 fn 体内 → 片段列表显示状态操作 + 控制流
- [ ] 双击 "toggle signal" → 插入 `flag = !flag;`
- [ ] 配置 `GET /todos` → 自动生成 Store + fetch + 类型定义
- [ ] Store 包含 `loading` + `error` 状态

---

### M5: 高级连线 + 调试 + 可逆代码所有权 + a11y + 动画 + 发布（4 周）

#### M5.1 — 高级连线功能（第 1 周）

```typescript
// editor/client/src/canvas/WireOverlay.tsx (补充)

// N-to-M 连线折叠
function foldWires(wires: WireData[], maxVisible: number = 5): {
  visible: WireData[];
  folded: { fromId: string; toId: string; count: number }[];
} {
  // 按 (fromId, toId) 分组
  const groups = new Map<string, WireData[]>();
  for (const w of wires) {
    const key = `${w.fromId}→${w.toId}`;
    const list = groups.get(key) || [];
    list.push(w);
    groups.set(key, list);
  }

  const visible: WireData[] = [];
  const folded: { fromId: string; toId: string; count: number }[] = [];

  for (const [key, list] of groups) {
    if (list.length <= 1) {
      visible.push(list[0]);
    } else if (list.length <= maxVisible) {
      visible.push(...list);
    } else {
      visible.push(...list.slice(0, maxVisible));
      folded.push({ fromId: list[0].fromId, toId: list[0].toId, count: list.length - maxVisible });
    }
  }

  return { visible, folded };
}

// 连线搜索
function searchWires(wires: WireData[], query: string): WireData[] {
  const q = query.toLowerCase();
  return wires.filter((w) =>
    w.fromId.toLowerCase().includes(q) ||
    w.toId.toLowerCase().includes(q) ||
    w.fromPort.toLowerCase().includes(q) ||
    w.toPort.toLowerCase().includes(q)
  );
}
```

#### M5.2 — 调试体验（第 2 周）

```typescript
// editor/client/src/canvas/StateHUD.tsx (实时状态检查)
export function StateHUD({ elementId }: { elementId: string }) {
  // 从 editor store 获取该元素关联的所有 signal / store
  // 从运行时（preview iframe 注入的 debug hook）获取当前值
  const [signalValues, setSignalValues] = useState<Record<string, any>>({});

  useEffect(() => {
    const iframe = document.querySelector("iframe") as HTMLIFrameElement;
    if (!iframe?.contentWindow) return;

    // 通过 postMessage 与预览 iframe 通信，获取运行时状态
    const interval = setInterval(() => {
      iframe.contentWindow?.postMessage({ type: "omni:get-state", elementId }, "*");
    }, 1000);

    const handler = (e: MessageEvent) => {
      if (e.data?.type === "omni:state") {
        setSignalValues(e.data.values);
      }
    };
    window.addEventListener("message", handler);

    return () => {
      clearInterval(interval);
      window.removeEventListener("message", handler);
    };
  }, [elementId]);

  return (
    <div style={{
      position: "absolute", padding: 12, borderRadius: 8,
      background: "#1F2937", color: "#F9FAFB", fontSize: 12,
      boxShadow: "0 4px 12px rgba(0,0,0,0.3)", zIndex: 100,
      minWidth: 200,
    }}>
      <div style={{ fontWeight: 600, marginBottom: 8 }}>状态检查</div>
      {Object.entries(signalValues).map(([name, value]) => (
        <div key={name} style={{ display: "flex", justifyContent: "space-between", marginTop: 4 }}>
          <span>{name}</span>
          <code>{JSON.stringify(value)}</code>
        </div>
      ))}
      {Object.keys(signalValues).length === 0 && (
        <span style={{ color: "#9CA3AF" }}>连接预览以查看运行时状态...</span>
      )}
    </div>
  );
}
```

#### M5.3 — 可逆代码所有权（第 2-3 周）

```python
# editor/server/api/code_ownership.py
"""可逆代码所有权: @editor: managed / @editor: hand-written 区域解析。"""

import re

MANAGED_MARKER = "// @editor: managed"
HANDWRITTEN_MARKER = "// @editor: hand-written"


def parse_ownership_regions(source: str) -> list[dict]:
    """解析源码中的所有权区域标记。

    返回:
        [{ "type": "managed"|"handwritten", "startLine": int, "endLine": int, "content": str }]
    """
    lines = source.split("\n")
    regions = []
    current_type = "managed"  # 默认：编辑器管理
    current_start = 0

    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith(MANAGED_MARKER):
            if current_start < i:
                regions.append({
                    "type": current_type,
                    "startLine": current_start,
                    "endLine": i - 1,
                })
            current_type = "managed"
            current_start = i + 1
        elif stripped.startswith(HANDWRITTEN_MARKER):
            if current_start < i:
                regions.append({
                    "type": current_type,
                    "startLine": current_start,
                    "endLine": i - 1,
                })
            current_type = "handwritten"
            current_start = i + 1

    # 最后一个区域
    if current_start < len(lines):
        regions.append({
            "type": current_type,
            "startLine": current_start,
            "endLine": len(lines) - 1,
        })

    return regions


def apply_canvas_edit(source: str, edit_start_line: int, edit_end_line: int, new_content: str) -> str:
    """应用画布修改，但跳过 hand-written 区域。"""
    regions = parse_ownership_regions(source)
    lines = source.split("\n")

    # 检查编辑区域是否与 hand-written 区域重叠
    for region in regions:
        if region["type"] != "handwritten":
            continue
        if edit_start_line <= region["endLine"] and edit_end_line >= region["startLine"]:
            # 重叠 → 返回冲突信息
            raise OwnershipConflictError(
                f"编辑区域 (lines {edit_start_line}-{edit_end_line}) "
                f"与手写区域 (lines {region['startLine']}-{region['endLine']}) 重叠"
            )

    # 无冲突 → 应用编辑
    new_lines = lines[:edit_start_line] + new_content.split("\n") + lines[edit_end_line + 1:]
    return "\n".join(new_lines)


class OwnershipConflictError(Exception):
    pass
```

```typescript
// editor/client/src/panels/ConflictPanel.tsx (冲突解决 UI)
export function ConflictPanel({ conflict, onResolve }: {
  conflict: { managedContent: string; handwrittenContent: string; lineRange: string };
  onResolve: (choice: "keep-managed" | "keep-handwritten" | "merge") => void;
}) {
  return (
    <div style={{ padding: 16, border: "2px solid #F59E0B", borderRadius: 8, background: "#FFFBEB" }}>
      <h3 style={{ margin: 0, fontSize: 14, color: "#92400E" }}>⚡ 编辑冲突</h3>
      <p style={{ fontSize: 12, color: "#92400E", margin: "4px 0 12px" }}>
        画布修改 ({conflict.lineRange}) 与手写代码区域重叠。
      </p>

      <div style={{ display: "flex", gap: 12 }}>
        <div style={{ flex: 1 }}>
          <div style={{ fontSize: 11, color: "#6B7280", marginBottom: 4 }}>画布版本 (managed)</div>
          <pre style={{ fontSize: 11, padding: 8, background: "#F3F4F6", borderRadius: 4, overflow: "auto", maxHeight: 200 }}>
            {conflict.managedContent}
          </pre>
        </div>
        <div style={{ flex: 1 }}>
          <div style={{ fontSize: 11, color: "#6B7280", marginBottom: 4 }}>手写版本 (hand-written)</div>
          <pre style={{ fontSize: 11, padding: 8, background: "#FEE2E2", borderRadius: 4, overflow: "auto", maxHeight: 200 }}>
            {conflict.handwrittenContent}
          </pre>
        </div>
      </div>

      <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
        <button onClick={() => onResolve("keep-managed")} style={btnStyle}>保留画布版本</button>
        <button onClick={() => onResolve("keep-handwritten")} style={btnStyle}>保留手写版本</button>
        <button onClick={() => onResolve("merge")} style={{ ...btnStyle, background: "#4F46E5", color: "#FFF" }}>
          手动合并 (打开代码面板)
        </button>
      </div>
    </div>
  );
}

const btnStyle: React.CSSProperties = {
  padding: "6px 12px", borderRadius: 4, border: "1px solid #D1D5DB",
  background: "#FFF", cursor: "pointer", fontSize: 12,
};
```

#### M5.4 — 无障碍内建（第 3 周）

```typescript
// editor/client/src/a11y/ContrastChecker.ts
/** WCAG 2.1 对比度计算。 */

function relativeLuminance(r: number, g: number, b: number): number {
  const [rs, gs, bs] = [r, g, b].map((c) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  });
  return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs;
}

export function contrastRatio(fg: string, bg: string): number {
  const fgRgb = hexToRgb(fg);
  const bgRgb = hexToRgb(bg);
  const l1 = relativeLuminance(fgRgb.r, fgRgb.g, fgRgb.b);
  const l2 = relativeLuminance(bgRgb.r, bgRgb.g, bgRgb.b);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

export function wcagLevel(ratio: number, size: "normal" | "large" = "normal"): "AAA" | "AA" | "FAIL" {
  const aaThreshold = size === "large" ? 3.0 : 4.5;
  const aaaThreshold = size === "large" ? 4.5 : 7.0;
  if (ratio >= aaaThreshold) return "AAA";
  if (ratio >= aaThreshold) return "AA";
  return "FAIL";
}

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const clean = hex.replace("#", "");
  return {
    r: parseInt(clean.slice(0, 2), 16),
    g: parseInt(clean.slice(2, 4), 16),
    b: parseInt(clean.slice(4, 6), 16),
  };
}

// 在颜色选择器中嵌入:
// <ContrastBadge fg={textColor} bg={bgColor} />
export function ContrastBadge({ fg, bg }: { fg: string; bg: string }) {
  const ratio = contrastRatio(fg, bg);
  const level = wcagLevel(ratio);
  return (
    <span style={{
      fontSize: 11, padding: "2px 6px", borderRadius: 4,
      background: level === "AAA" ? "#D1FAE5" : level === "AA" ? "#FEF3C7" : "#FEE2E2",
      color: level === "AAA" ? "#065F46" : level === "AA" ? "#92400E" : "#991B1B",
    }}>
      {ratio.toFixed(1)}:1 {level === "AAA" ? "✅ AAA" : level === "AA" ? "⚠ AA" : "❌"}
    </span>
  );
}
```

```typescript
// editor/client/src/a11y/FocusOrder.ts
/** Tab 键焦点顺序可视化叠加层。 */

export function FocusOrderOverlay({ elements }: { elements: ElementData[] }) {
  const focusableElements = getFocusableElements(elements);
  const tabOrder = computeTabOrder(focusableElements);

  return (
    <div style={{ position: "absolute", top: 0, left: 0, pointerEvents: "none", zIndex: 50 }}>
      {tabOrder.map((el, i) => {
        const domEl = document.querySelector(`[data-omni-id="${el.id}"]`);
        if (!domEl) return null;
        const rect = domEl.getBoundingClientRect();
        return (
          <div
            key={el.id}
            style={{
              position: "absolute", left: rect.left - 4, top: rect.top - 4,
              width: rect.width + 8, height: rect.height + 8,
              border: "1px dashed #4F46E5", borderRadius: 4,
            }}
          >
            <span style={{
              position: "absolute", top: -18, left: 0,
              fontSize: 10, background: "#4F46E5", color: "#FFF",
              padding: "1px 4px", borderRadius: 2,
            }}>
              Tab {i + 1}
            </span>
          </div>
        );
      })}
    </div>
  );
}

function getFocusableElements(elements: ElementData[]): ElementData[] {
  const focusableTags = ["Input", "TextArea", "Button", "Select", "Checkbox"];
  return elements.filter((e) => focusableTags.includes(e.tag));
}

function computeTabOrder(elements: ElementData[]): ElementData[] {
  // 按 DOM 出现顺序排序（深度优先遍历元素树）
  // 生产实现：考虑 tabIndex prop 自定义顺序
  return elements;
}
```

#### M5.5 — 动画导入 + 一键发布（第 4 周）

```typescript
// editor/client/src/panels/AnimationImport.tsx (Lottie/Rive 导入)

export function AnimationDropZone({ onImport }: { onImport: (type: "lottie" | "rive", data: any) => void }) {
  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    const file = e.dataTransfer.files[0];
    if (!file) return;

    if (file.name.endsWith(".json")) {
      // Lottie JSON
      const text = await file.text();
      const lottieData = JSON.parse(text);
      onImport("lottie", lottieData);
    } else if (file.name.endsWith(".riv")) {
      // Rive 文件 (二进制)
      const buffer = await file.arrayBuffer();
      onImport("rive", buffer);
    }
  };

  return (
    <div
      onDrop={handleDrop}
      onDragOver={(e) => e.preventDefault()}
      style={{
        padding: 24, border: "2px dashed #D1D5DB", borderRadius: 8,
        textAlign: "center", color: "#6B7280", fontSize: 13,
      }}
    >
      拖入 Lottie (.json) 或 Rive (.riv) 文件
    </div>
  );
}

// 导入后自动生成 .omni 代码:
// component LottiePlayer {
//     signal src: String = "/animations/example.json";
//     signal playing: bool = true;
//     signal loop: bool = true;
//
//     view {
//         <Lottie src={src} playing={playing} loop={loop} />
//     }
// }
```

```python
# editor/server/api/deploy.py (一键发布)
@router.post("/deploy")
async def deploy(req: DeployRequest):
    """推送项目到 Vercel/Netlify。"""
    project_dir = Path(req.projectDir)

    if req.platform == "vercel":
        return await _deploy_vercel(project_dir)
    elif req.platform == "netlify":
        return await _deploy_netlify(project_dir)
    else:
        return {"error": f"Unknown platform: {req.platform}"}


async def _deploy_vercel(project_dir: Path) -> dict:
    """通过 Vercel CLI 部署。"""
    import subprocess

    # 检查 vercel CLI 是否安装
    if subprocess.run(["which", "vercel"], capture_output=True).returncode != 0:
        return {"error": "Vercel CLI not installed. Run: npm i -g vercel"}

    # 执行部署
    result = subprocess.run(
        ["vercel", "--prod", "--confirm"],
        cwd=str(project_dir),
        capture_output=True, text=True, timeout=60,
    )

    # 从输出提取 URL
    url_match = re.search(r"https://[^\s]+\.vercel\.app", result.stdout)
    if url_match:
        return {"url": url_match.group(0), "platform": "vercel"}
    return {"error": result.stderr or "Deploy failed"}
```

---

**M5 验收标准：**
- [ ] 10 条连线折叠为 "＋ 10"
- [ ] HUD 显示实时 signal 值
- [ ] hand-written fn 不被画布修改覆盖
- [ ] 冲突面板弹出选择保留哪个版本
- [ ] 颜色选择器显示 WCAG 对比度评分
- [ ] 拖入 Lottie JSON → 生成动画组件
- [ ] `omnic deploy --platform vercel` → 返回 live URL

---

## §4 测试策略

### 4.1 测试金字塔

```
         ┌──────┐
         │ E2E  │  Playwright: 关键用户流程 × 10
         ├──────┤
         │ 集成  │  Python: API 测试 × 20
         ├──────┤    前端: Zustand store 测试 × 15
         │ 单元  │  前端: Command 撤销/重做 × 30
         │      │  Python: 原子写入、心跳协议 × 10
         └──────┘
```

### 4.2 性能门禁

| 场景 | 指标 | 门禁 | M |
|------|------|------|----|
| 50 组件画布 | fps | > 30 | M2 |
| 200 组件 + 300 连线编辑 | 延迟 | < 50ms | M3 |
| 200 组件 + 300 连线 | fps | > 30 | M3 |
| 撤销栈 500 条 | 内存 | < 50MB | M4 |
| 编译 500 行 .omni | 耗时 | < 2s | M1 |

### 4.3 关键测试用例

```
M1: test_heartbeat_stops_orphan / test_tab_freeze_restore / test_atomic_write / test_diff_single_line
M2: test_property_edit_undo / test_event_binding / test_50_components_30fps
M3: test_drag_palette / test_wire_signal_setter / test_code_block_ports / test_responsive / test_token_propagate / test_200_components_50ms
M4: test_snippet_tab / test_rest_connector_generates_store / test_code_panel_navigation
M5: test_handwritten_not_overwritten / test_contrast_rating / test_lottie_import / test_deploy_returns_url
```

---

## §5 风险矩阵

| # | 风险 | 严重度 | M1 缓解 | 触发条件 | 升级路径 |
|----|------|--------|---------|---------|---------|
| R1 | DOM 画布 < 30fps@300组件 | HIGH | M2 虚拟渲染 + 脏标记 | M3 门禁不达标 | → PixiJS WebGL |
| R2 | Python 子进程孤儿 | MEDIUM | WebSocket heartbeat + 退出 | 用户反馈进程残留 | → 托盘图标 |
| R3 | Tab 冻结丢失状态 | MEDIUM | IndexedDB + visibilitychange | 恢复不一致 | → Web Worker |
| R4 | 文件写入竞态 | HIGH | 原子写入 + 显式通知 | 编译错误 | → fsync + flock |
| R5 | 语义合并冲突 | HIGH | 分块编辑；同块 git 标记 | 多用户协作 | 冲突面板 |
| R6 | 撤销栈内存爆炸 | MEDIUM | zlib 压缩 + IndexedDB | > 100MB | → LRU 驱逐 |

---

## §6 依赖与前置

### 已有资产

| 资产 | 路径 | 用途 |
|------|------|------|
| OmniLang 编译器 | `omnilang/` + `omniview/` | 编译 pipeline |
| `omnic` CLI | `omnilang/cli.py` (548 行), `omniview/cli.py` (753 行) | serve 命令入口（加在 omniview/cli.py，F6） |
| `omnic watch` | `omniview/cli.py:596` watch_command() | 已有 os.path.getmtime 轮询，复用而非引入 watchdog（F5） |
| `omnic init` | `omniview/cli.py:402` init_command() | 已有 Vite+React 脚手架，M1 项目创建直接复用 |
| Playwright | `.playwright-mcp/` | E2E |
| 测试 fixtures | `tests/fixtures/` | 编译器测试 |

### 新增依赖

**Python（pyproject.toml 追加）:**
```toml
[project.optional-dependencies]
editor = ["fastapi>=0.110", "uvicorn[standard]>=0.29"]

[tool.setuptools.packages.find]
include = ["omnilang*", "omniview*", "editor*"]  # ★ G4: 注册 editor 包
```

**前端:**
```json
{
  "react": "^18.3", "react-dom": "^18.3",
  "zustand": "^4.5", "idb": "^8.0",
  "monaco-editor": "^0.47", "pako": "^2.1",
  "vite": "^5.4", "typescript": "^5.5",
  "playwright": "^1.45"
}
```

---

## §7 路线图

```
Week  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30
      ├─────┤├──────────┤├────────────┤├──────────────────┤├──────────┤├──────────────────────┤
      │ M1a ││   M1b    ││    M2      ││       M3         ││    M4    ││         M5           │
      │ 2 周││  3 周    ││  3-4 周    ││     5 周         ││   3 周   ││       4 周           │
```

| M | 可演示 | 可测试 | 可发布 |
|----|--------|--------|--------|
| M1a | ✅ 编译管道端到端 | ✅ API + parser 缓存 | ❌ |
| M1b | ✅ 编辑器骨架 + 预览 + 撤销 | ✅ heartbeat + diff | ❌ |
| M2 | ✅ 属性面板 + 事件绑定 | ✅ 50 组件性能 | ❌ |
| M3 | ✅ 拖拽 + 连线 + 代码块 | ✅ 200 组件门禁 | ❌ |
| M4 | ✅ todo app 接真实 API | ✅ 连接器集成 | ❌ |
| M5 | ✅ 一键发布到线上 | ✅ E2E + a11y | ✅ v1.0 |

---

## §8 工时

| M | Python | React | 测试 | 合计 |
|----|--------|-------|------|------|
| M1a | 5d | 1d | 2d | **8d (2 周)** |
| M1b | 4d | 5d | 2d | **11d (3 周)** |
| M2 | 3d | 10d | 3d | **16d** |
| M3 | 1d | 18d | 4d | **23d** |
| M4 | 3d | 8d | 3d | **14d** |
| M5 | 4d | 12d | 3d | **19d** |
| **合计** | **19d** | **55d** | **17d** | **91d** |

---

## §9 首次启动体验

```
1. 终端: omnic serve --dir ./my-project
2. 浏览器自动打开 → 左右分栏布局
3. 左侧文件列表，右侧实时预览
4. 修改 App.omni → 保存 → 预览 < 2s 刷新
5. 点击元素 → 属性面板编辑
```

---

## §10 总结

**五个里程碑，85 工程日。`omnic serve` 一行命令，浏览器中完成从设计到发布。**

1. M1 极简——50 行代码先跑起来
2. 每 M 独立可演示
3. 性能门禁前置——不达标不进下一 M
4. 复用 > 重写——编译器不改，测试框架不改
5. 先 React 单目标——10 个真实 Vue/Flutter 用户再扩展
