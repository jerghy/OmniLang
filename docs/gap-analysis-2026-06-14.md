# OmniLang 可视化编辑器 — 成熟度差距分析总报告

**日期**: 2026-06-14  
**方法论**: 3 个子代理并行分析，对标 GrapesJS（设计画布）、Node-RED / UE Blueprints / Unity VS（节点流图）、Figma / Webflow / Framer（UX 和架构）  
**数据来源**: 读取 30+ 源文件，交叉验证

---

## 总览：按严重程度排序的差距

| 优先级 | 类别 | 严重度 | 归属 | 预计工作量 |
|--------|------|--------|------|-----------|
| 🔴 P0 | 测试基础设施 | **CRITICAL** | 架构 | 大 |
| 🔴 P0 | 画布交互（拖拽/多选/缩放句柄） | **CRITICAL** | 设计模式 | 大 |
| 🔴 P0 | 样式编辑（可视化控件） | **CRITICAL** | 设计模式 | 大 |
| 🔴 P0 | 复制/粘贴 | **CRITICAL** | 代码模式 | 中 |
| 🟠 P1 | 节点组织（分组/注释/子图） | **HIGH** | 代码模式 | 大 |
| 🟠 P1 | 调试/追踪（画布断点） | **HIGH** | 代码模式 | 中 |
| 🟠 P1 | 组件层级（图层面板） | **HIGH** | 设计模式 | 中 |
| 🟠 P1 | 响应式设计 | **HIGH** | 设计模式 | 中 |
| 🟠 P1 | 插件系统 | **HIGH** | 架构 | 大 |
| 🟠 P1 | 无障碍（ARIA/键盘导航） | **HIGH** | 架构 | 大 |
| 🟠 P1 | 错误恢复（崩溃恢复/自动保存） | **HIGH** | 架构 | 中 |
| 🟡 P2 | 序列化格式（版本化） | **MEDIUM** | 架构 | 中 |
| 🟡 P2 | 撤销/重做（复合命令/持久化） | **MEDIUM** | 全局 | 小 |
| 🟡 P2 | 键盘快捷键 | **MEDIUM** | 全局 | 小 |
| 🟡 P2 | 主题系统（暗色模式） | **MEDIUM** | 架构 | 中 |
| 🟡 P2 | 性能架构（虚拟化/懒加载） | **MEDIUM** | 架构 | 大 |
| 🟡 P2 | 自动布局 | **MEDIUM** | 代码模式 | 小 |
| 🟡 P2 | 验证 UX（节点错误徽章） | **MEDIUM** | 代码模式 | 小 |
| 🟡 P2 | 通知/Toast | **MEDIUM** | 全局 | 小 |
| 🟡 P2 | 选中文模型（多选） | **MEDIUM** | 代码模式 | 中 |
| 🟡 P2 | 资源管理（图片上传） | **MEDIUM** | 设计模式 | 中 |
| 🟡 P2 | 上下文菜单 | **MEDIUM** | 全局 | 中 |
| 🟡 P2 | 新手引导 | **MEDIUM** | UX | 中 |
| 🟢 P3 | 连线管理（路由/reroute） | **LOW-MEDIUM** | 代码模式 | 小 |
| 🟢 P3 | 类型系统（强制转换/泛型） | **LOW-MEDIUM** | 代码模式 | 中 |
| 🟢 P3 | 视觉抛光（动画） | **LOW-MEDIUM** | 代码模式 | 小 |
| 🟢 P3 | 代码输出（设计模式预览） | **LOW-MEDIUM** | 设计模式 | 小 |
| 🟢 P3 | 搜索/命令面板 | **LOW** | 全局 | 小 |
| 🟢 P3 | 缩放/平移 | **LOW** | 全局 | 小 |
| 🟢 P3 | 分屏/多窗 | **LOW-MEDIUM** | UX | 中 |
| 🟢 P3 | 拖放改进 | **LOW-MEDIUM** | 全局 | 中 |
| 🟢 P3 | 多用户协作 | **LOW** (单人场景) | 架构 | 巨大 |

---

## 详细分析

### 🔴 P0 — 阻塞性问题（必须优先解决）

#### 1. 测试基础设施 (CRITICAL)
- **现状**: package.json 中无测试脚本、无测试运行器依赖，git log 中有测试文件但已删除
- **影响**: 零回归保护，任何修改都可能引入不可见 bug
- **方案**: 加 vitest 做单元测试（store/undo/commands/validators），Playwright 做 E2E，恢复 02dede56 的测试文件

#### 2. 设计模式画布交互 (CRITICAL)
- **现状**: `LayoutView.tsx` 只支持根级放置，单选，无缩放句柄，无对齐引导线，无右键菜单
- **对标**: GrapesJS 原生支持目标区域解析、多选、四角缩放句柄、对齐吸附、右键上下文菜单
- **方案**: 引入 `@dnd-kit` 做可排序拖放系统，添加缩放句柄组件，实现对齐引导线和多选框架

#### 3. 可视化样式编辑器 (CRITICAL)
- **现状**: `PropertyPanel.tsx` 所有样式属性都是纯文本 input，无滑块、无阴影编辑器、无渐变色选择器
- **对标**: GrapesJS Style Manager 有 padding/margin 象限滑块、阴影编辑器、颜色色板、排版控件
- **方案**: 构建专用 Style Panel，padding/margin 四边独立拖动、颜色选择器 + 透明度、box-shadow 可视化编辑

#### 4. 复制/粘贴 (CRITICAL)
- **现状**: 零剪贴板功能，无 CopySelectionCommand 或 PasteCommand
- **对标**: 所有成熟节点编辑器都支持 Ctrl+C/V/D 并保留连线
- **方案**: 创建 CopySelectionCommand + PasteCommand，序列化选中节点+内部连线为 JSON clipboard buffer，新 ID + 40px 偏移粘贴
- **位置**: 新增 `commands/CopySelection.ts` + `commands/PasteSelection.ts`

---

### 🟠 P1 — 高优先级

#### 5. 节点组织（分组/注释/子图）(HIGH)
- **现状**: `isFolded` 字段存在但无渲染，`nodeType: "group"` 在 schema 中有定义但无渲染器
- **方案**: BaseNode 添加折叠/展开按钮，实现 Group 节点类型（矩形+标题+半透明背景），子图用 "macro" 节点 + drill-down 导航

#### 6. 调试 UX（画布断点指示器）(HIGH)
- **现状**: DebugPanel 有断点基础设施，但画布节点上看不到断点标记
- **方案**: BaseNode 左上角渲染红色圆点（断点标记），连线 step-through 逐节点高亮，执行路径覆盖层（遍历过的边/节点半透明高亮）

#### 7. 组件层级 / 图层面板 (HIGH)
- **现状**: `ElementData` 有 `parentId` 和 `children` 但无树形 UI
- **方案**: 新增 Layers 面板（基于 `elements` + `parentId` 构建树），实现拖拽重排，添加面包屑导航

#### 8. 响应式设计 (HIGH)
- **现状**: App.tsx 三个硬编码断点仅改变画布宽度，无 per-breakpoint 样式，无 @media 生成
- **方案**: Store 添加 `propsByBreakpoint`，断点配置 UI，编译器生成 @media 查询

#### 9. 插件系统 (HIGH)
- **现状**: 所有节点类型硬编码在 `nodeRegistry.ts`，面板静态 import
- **方案**: 定义 Plugin 接口（registerNodeType, registerPanel, registerCommand, registerStorage），动态 import + 沙箱加载

#### 10. 无障碍 (HIGH)
- **现状**: 无可用的 screen reader 支持，ARIA 属性缺失，FocusOrder/ContrastChecker 工具存在但未接 UI
- **方案**: 加 ARIA roles/labels、焦点陷获、aria-live 状态通知、ContrastChecker 接入颜色选择器

#### 11. 错误恢复 (HIGH)
- **现状**: 自动保存只在编译后触发，undo 栈仅内存，无崩溃恢复，无 ErrorBoundary
- **方案**: IndexedDB 持久化 undo 栈（实现 undo.ts 中已注释的 stub），React ErrorBoundary + 恢复 UI，beforeunload 确认未保存更改

---

### 🟡 P2 — 中优先级

| 项目 | 方案 |
|------|------|
| 序列化格式 | 画布 JSON 添加 `version` 字段和 JSON Schema 验证 |
| 撤销/重做 | 添加 `BatchCommand`，IndexedDB 持久化 |
| 键盘快捷键 | Delete/Ctrl+C/V/D/方向键微移，快捷键速查表 |
| 主题系统 | CSS custom properties 统一颜色，ThemeProvider 上下文 |
| 性能架构 | react-virtuoso 虚拟化列表，React.lazy 懒加载 Monaco |
| 自动布局 | 将 dagre 引擎接入 UI 触发按钮（现存死代码） |
| 验证 UX | 问题节点渲染红色 "!" 徽章 |
| 通知系统 | react-hot-toast 或 sonner 替换单行状态栏 |
| 多选模型 | selectedElementId → selectedElementIds: Set\<string\> |
| 资源管理 | Assets store + 文件上传端点 + 缩略图网格面板 |
| 上下文菜单 | ContextMenu 组件接入 FileExplorer/画布/列表项 |
| 新手引导 | localStorage 首次标记 + 欢迎对话框 + react-joyride 导览 |

---

### 🟢 P3 — 低优先级 / 锦上添花

| 项目 | 方案 |
|------|------|
| 连线路由 | bezier/step/straight 可选 + Reroute 节点 |
| 类型强制转换 | i32→f64 自动允许，插入转换节点 |
| 视觉动画 | CSS keyframes 节点出现动画，事件源节点脉冲 |
| 代码预览 | 设计模式分屏显示编译后 HTML/CSS |
| 搜索面板 | 匹配字符高亮，最近使用排序，工具栏可见按钮 |
| 缩放控制 | 底部缩放滑块 + 百分比标签 |
| 分屏模式 | 设计+代码垂直分屏（替代全页切换） |
| 拖放改进 | @dnd-kit 结构化 DnD，拖拽预览，触屏支持 |
| 协作 | Yjs CRDT 集成（仅团队场景需要） |

---

## 已具备的优势（不要破坏）

1. **搜索系统** — `SearchModal` + `SearchIndex` 的模糊评分、源分组、键盘导航质量很高
2. **节点注册表** — `nodeRegistry.ts` 的 24 类型 + 6 分类 + 默认端口 + 关键词结构良好
3. **Command 模式** — `commands/types.ts` 的 `execute()`/`undo()` 接口设计正确，为持久化和协作打好基础
4. **.omni.json 格式** — 带版本号、广覆盖的 schema，比 GrapesJS 原生格式更丰富
5. **无障碍基元** — `FocusOrder.ts` 和 `ContrastChecker.ts` 显示有意为之的无障碍思考
6. **连线打包** — `WireOverlay.tsx` 的 `foldWires` 将多条同源线折叠为单条虚线+N 徽章
7. **PerfMonitor** — 帧计时、FPS 滚动平均、性能门控检查，比同阶段工具更成熟
8. **节点搜索面板** — `NodePalette` 模糊搜索+键盘导航+端口预览，与 Unity VS 同级
9. **类型系统** — flow-vs-data 端口分离、类型着色、6 项连接验证

---

## 建议实施路线图

### Phase 1: 基础质量 (1-2 周)
- [ ] 复制/粘贴 (P0)
- [ ] 键盘快捷键 (P2)
- [ ] 测试基础设施 (P0)
- [ ] 通知/Toast (P2)
- [ ] 撤销持久化 + ErrorBoundary (P1)

### Phase 2: 设计画布升级 (2-4 周)
- [ ] 画布交互重写 (P0) — 目标区域拖放、多选、缩放句柄
- [ ] 可视化样式编辑器 (P0)
- [ ] 图层面板 (P1)
- [ ] 响应式设计 (P1)

### Phase 3: 代码画布深化 (2-3 周)
- [ ] 节点分组/注释 (P1)
- [ ] 断点画布指示器 (P1)
- [ ] 自动布局接入 (P2)
- [ ] 验证 UX 改进 (P2)

### Phase 4: 架构与生态 (3-4 周)
- [ ] 插件系统 (P1)
- [ ] 主题系统 (P2)
- [ ] 无障碍 (P1)
- [ ] 性能虚拟化 (P2)
- [ ] 上下文菜单 (P2)
