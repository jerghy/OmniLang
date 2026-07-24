# Poiesis Design Migration — Design Spec

> 将 Poiesis 编辑器 Chrome 从靛蓝冷色调迁移到陶土暖色调设计系统
>
> Date: 2026-07-24 | Status: Approved

## 一、目标与范围

### 目标

将 `D:\space\OmniLang\Design-System`（Claude/Anthropic 风格的品牌设计系统）的**色彩 DNA 和视觉语言**注入 Poiesis 编辑器自身 Chrome，让 Poiesis 看起来像 Design-System 而非之前的靛蓝冷色调系统。

### 范围

| 包含 | 不包含 |
|------|--------|
| `src/styles/tokens.css` — 全部 `--ol-*` CSS 变量的值 | `design.omni` — 用户项目的设计系统，属于用户内容 |
| 所有 CSS Module 中硬编码的 hex 颜色 → `var(--ol-*)` | `src/core/types/design.ts` — 类型定义不变 |
| ring-shadow 深度系统替换 drop-shadow | `src/store/slices/design-system.ts` — Store 逻辑不变 |
| `src/styles/legacy-bridge.css` — 删除 | `src/store/slices/theme.ts` — 主题切换逻辑不变 |
| Design-System 源文件放入 `docs/design/` 存档 | `.tsx` 组件逻辑代码 |

### 核心原则

- **色彩 DNA 保留** — 陶土 `#c96442` + 羊皮纸底 + 暖灰中性
- **工具调性重新表达** — IDE 信息密度 + 等宽字体 + 功能性布局不变
- **Token 命名空间不变** — 保持 `--ol-*`，只换值不换引用
- **一次到位** — 不搞分批过渡，避免半冷半暖的尴尬期

## 二、设计决策回顾

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 迁移范围 | 完整设计语言 | 不只是换色，是整个编辑器的视觉气质 |
| Token 命名空间 | 保持 `--ol-*` | 所有 CSS Module 不用改引用名，零风险 |
| 视觉调性 | 色彩保留 + 工具调性 | 暖色 DNA 不变，但 IDE 信息密度和等宽字体不碰 |
| 字体 | 保持 Inter + JetBrains Mono | 开发工具黄金组合，换字体风险大于收益 |
| 暗色模式 | Design-System 暗色 + Poiesis 适配 | 接近纯黑底 + 暖灰面板 |
| Shadow 系统 | 全面 ring-shadow | 用暖色环代替投影，区分层级 |
| 硬编码颜色 | 一并规范化 | 不趁现在做以后会有视觉冲突 |
| Info 色 | 与 Primary 同色 | 不采纳独立冷色方案 |
| Ring-shadow 透明度 | 亮色 6%、暗色 8% | 保证雾面屏/低亮度下可见 |
| Surface 层级差 | 加大 | 确保面板边界可感知 |

## 三、Token 映射表

### 3.1 语义色 — 亮色模式

| CSS 变量 | 旧值 | 新值 | 来源 |
|----------|------|------|------|
| `--ol-color-primary` | `#6366f1` | `#c96442` | DS: terracotta-brand |
| `--ol-color-primary-hover` | `#4f46e5` | `#d68869` | DS: brand-color-primary-hover |
| `--ol-color-primary-active` | `#4338ca` | `#a3472e` | DS: brand-color-primary-active |
| `--ol-color-primary-subtle` | `#eef0ff` | `#fff6f0` | DS: brand-color-primary-bg |
| `--ol-color-primary-muted` | `#c7d2fe` | `#f0d2c2` | DS: brand-color-primary-border |
| `--ol-color-success` | `#16a34a` | `#52c41a` | DS: brand-color-success |
| `--ol-color-success-subtle` | `#f0fdf4` | `#f6ffed` | DS: brand-color-success-bg |
| `--ol-color-warning` | `#d97706` | `#faad14` | DS: brand-color-warning |
| `--ol-color-warning-subtle` | `#fffbeb` | `#fffbe6` | DS: brand-color-warning-bg |
| `--ol-color-danger` | `#dc2626` | `#ff4d4f` | DS: brand-color-error |
| `--ol-color-danger-subtle` | `#fef2f2` | `#fff2f0` | DS: brand-color-error-bg |
| `--ol-color-info` | `#0ea5e9` | `#c96442` | 同 primary |
| `--ol-color-info-subtle` | `#f0f9ff` | `#fff6f0` | 同 primary-subtle |

> hover/active/muted 语义色也同步更新，此处不逐一列举

### 3.2 灰度系 — 亮色模式

| CSS 变量 | 旧值（冷灰） | 新值（暖灰） |
|----------|-------------|-------------|
| `--ol-gray-50` | `#f9fafb` | `#f1f0e9` |
| `--ol-gray-100` | `#f3f4f6` | `#ecebe4` |
| `--ol-gray-200` | `#e5e7eb` | `#e7e7e0` |
| `--ol-gray-300` | `#d1d5db` | `#d3d2cc` |
| `--ol-gray-400` | `#9ca3af` | `#bdbcb7` |
| `--ol-gray-500` | `#6b7280` | `#908f8b` |
| `--ol-gray-600` | `#4b5563` | `#63625f` |
| `--ol-gray-700` | `#374151` | `#2f2f2d` |
| `--ol-gray-800` | `#1f2937` | `#141413` |
| `--ol-gray-900` | `#111827` | `#141413` |
| `--ol-gray-950` | `#030712` | `#141413` |

> 灰度来源于 DS 的 `colorFill` / `colorText` 系列，保持暖黄底色。

### 3.3 Surface / Border / Text — 亮色模式

| CSS 变量 | 旧值 | 新值 | DSL 来源 |
|----------|------|------|---------|
| `--ol-surface` | `#ffffff` | `#f5f4ed` | DS: colorBgContainer / parchment |
| `--ol-surface-hover` | `#f3f4f6` | `#ecebe4` | DS: colorFillTertiary |
| `--ol-surface-active` | `#eef0ff` | `#fff6f0` | DS: colorPrimaryBg |
| `--ol-surface-raised` | `#ffffff` | `#ecebe4` | 增大层级差 |
| `--ol-surface-overlay` | `#ffffff` | `#f5f4ed` | |
| `--ol-surface-inverse` | `#111827` | `#141413` | DS: near-black |
| `--ol-border` | `#e5e7eb` | `#e8e5dc` | 增大层级差（比 DS border 稍深） |
| `--ol-border-hover` | `#d1d5db` | `#d3d2cc` | DS: colorFill |
| `--ol-border-active` | `#6366f1` | `#c96442` | DS: primary |
| `--ol-border-subtle` | `#f3f4f6` | `#f0eee6` | DS: border-cream |
| `--ol-border-strong` | `#9ca3af` | `#908f8b` | |
| `--ol-text` | `#111827` | `#141413` | DS: near-black |
| `--ol-text-secondary` | `#4b5563` | `#5e5d59` | DS: olive-gray |
| `--ol-text-muted` | `#9ca3af` | `#908f8b` | DS: colorTextTertiary |
| `--ol-text-inverse` | `#ffffff` | `#f5f4ed` | DS: parchment |
| `--ol-text-link` | `#6366f1` | `#c96442` | DS: colorLink |
| `--ol-text-link-hover` | `#4f46e5` | `#d68869` | DS: colorLinkHover |

### 3.4 Shadow 系统 — 亮色模式

所有 drop-shadow 替换为 ring-shadow（暖色环），透明度最低 6%：

| CSS 变量 | 旧值 | 新值 |
|----------|------|------|
| `--ol-shadow-sm` | `0 1px 2px rgba(0,0,0,0.05)` | `0 0 0 1px rgba(0,0,0,0.06)` |
| `--ol-shadow-md` | `0 4px 6px -1px rgba(0,0,0,0.07), 0 2px 4px -2px rgba(0,0,0,0.05)` | `0 0 0 1px rgba(0,0,0,0.08)` |
| `--ol-shadow-lg` | `0 10px 15px -3px rgba(0,0,0,0.08), 0 4px 6px -4px rgba(0,0,0,0.04)` | `0 0 0 1px rgba(0,0,0,0.10)` |
| `--ol-shadow-xl` | `0 20px 25px -5px rgba(0,0,0,0.08), 0 8px 10px -6px rgba(0,0,0,0.04)` | `0 0 0 1px rgba(0,0,0,0.12)` |
| `--ol-shadow-2xl` | `0 25px 50px -12px rgba(0,0,0,0.15)` | `0 0 0 2px rgba(0,0,0,0.10)` |
| `--ol-shadow-popover` | `0 8px 24px rgba(0,0,0,0.12), 0 0 0 1px rgba(0,0,0,0.05)` | `0 0 0 2px rgba(0,0,0,0.10), 0 4px 12px rgba(0,0,0,0.06)` |
| `--ol-shadow-modal` | `0 20px 60px rgba(0,0,0,0.18), 0 0 0 1px rgba(0,0,0,0.08)` | `0 0 0 2px rgba(0,0,0,0.12), 0 8px 32px rgba(0,0,0,0.08)` |
| `--ol-shadow-drawer` | `0 0 40px rgba(0,0,0,0.12)` | `0 0 0 2px rgba(0,0,0,0.10)` |
| `--ol-shadow-dropdown` | `0 4px 16px rgba(0,0,0,0.1), 0 0 0 1px rgba(0,0,0,0.04)` | `0 0 0 2px rgba(0,0,0,0.08), 0 2px 8px rgba(0,0,0,0.06)` |

> popover/modal/dropdown 保留微弱的 drop-shadow 辅助层，因为高层级浮层仍需"浮起"感，但主结构是 ring-shadow。

### 3.5 暗色模式 Token 映射

| CSS 变量 | 旧值 | 新值 | 来源 |
|----------|------|------|------|
| `--ol-gray-50` | `#1a1a1e` | `#1d1d1d` | DS: colorFillQuaternary |
| `--ol-gray-100` | `#27272b` | `#272727` | DS: colorFillTertiary |
| `--ol-gray-200` | `#3f3f46` | `#303030` | DS: colorFillSecondary |
| `--ol-gray-300` | `#52525b` | `#3e3e3e` | DS: colorFill |
| `--ol-gray-400` | `#71717a` | `#4f4f4f` | DS: colorTextQuaternary |
| `--ol-gray-500` | `#a1a1aa` | `#7e7e7e` | DS: colorTextTertiary |
| `--ol-gray-600` | `#d4d4d8` | `#adadad` | DS: colorTextSecondary |
| `--ol-gray-700` | `#e4e4e7` | `#dcdcdc` | DS: colorText |
| `--ol-gray-800` | `#f0f0f3` | `#dcdcdc` | |
| `--ol-gray-900` | `#fafafc` | `#dcdcdc` | |
| `--ol-gray-950` | `#ffffff` | `#ffffff` | |
| `--ol-color-primary` | `#818cf8` | `#b9775c` | DS dark: colorPrimary |
| `--ol-color-primary-hover` | `#6366f1` | `#ce9e87` | DS dark: colorPrimaryHover |
| `--ol-color-primary-active` | `#4f46e5` | `#925f4b` | DS dark: colorPrimaryActive |
| `--ol-color-primary-subtle` | `rgba(99,102,241,0.15)` | `rgba(185,119,92,0.15)` | |
| `--ol-color-primary-muted` | `rgba(99,102,241,0.3)` | `rgba(185,119,92,0.3)` | |
| `--ol-surface` | `#18181b` | `#141414` | DS dark: colorBgContainer |
| `--ol-surface-hover` | `#27272b` | `#272727` | DS dark: colorFillTertiary |
| `--ol-surface-raised` | `#1f1f23` | `#222222` | 增大层级差 |
| `--ol-surface-overlay` | `#27272b` | `#272727` | DS dark: colorBgElevated |
| `--ol-surface-inverse` | `#fafafc` | `#f5f4ed` | |
| `--ol-border` | `#3f3f46` | `#343434` | 增大层级差 |
| `--ol-border-hover` | `#52525b` | `#3e3e3e` | |
| `--ol-border-active` | `#818cf8` | `#b9775c` | DS dark: colorPrimary |
| `--ol-border-subtle` | `#27272b` | `#303030` | |
| `--ol-border-strong` | `#71717a` | `#4f4f4f` | |
| `--ol-text` | `#fafafc` | `#dcdcdc` | DS dark: colorText |
| `--ol-text-secondary` | `#d4d4d8` | `#adadad` | DS dark: colorTextSecondary |
| `--ol-text-muted` | `#71717a` | `#7e7e7e` | DS dark: colorTextTertiary |
| `--ol-text-inverse` | `#111827` | `#141413` | |
| `--ol-text-link` | `#818cf8` | `#b9775c` | DS dark: colorLink |
| `--ol-text-link-hover` | `#6366f1` | `#ce9e87` | DS dark: colorLinkHover |
| `--ol-focus-ring-color` | `rgba(129,140,248,0.4)` | `rgba(185,119,92,0.4)` | |

### 3.6 Shadow 系统 — 暗色模式

| CSS 变量 | 旧值 | 新值 |
|----------|------|------|
| `--ol-shadow-sm` | `0 1px 2px rgba(0,0,0,0.3)` | `0 0 0 1px rgba(255,255,255,0.08)` |
| `--ol-shadow-md` | `0 4px 6px -1px rgba(0,0,0,0.35), ...` | `0 0 0 1px rgba(255,255,255,0.10)` |
| `--ol-shadow-lg` | `0 10px 15px -3px rgba(0,0,0,0.4), ...` | `0 0 0 1px rgba(255,255,255,0.12)` |
| `--ol-shadow-xl` | `0 20px 25px -5px rgba(0,0,0,0.4), ...` | `0 0 0 1px rgba(255,255,255,0.14)` |
| `--ol-shadow-2xl` | `0 25px 50px -12px rgba(0,0,0,0.5)` | `0 0 0 2px rgba(255,255,255,0.12)` |
| `--ol-shadow-popover` | `0 8px 24px rgba(0,0,0,0.4), ...` | `0 0 0 2px rgba(255,255,255,0.12), 0 4px 12px rgba(0,0,0,0.3)` |
| `--ol-shadow-modal` | `0 20px 60px rgba(0,0,0,0.5), ...` | `0 0 0 2px rgba(255,255,255,0.14), 0 8px 32px rgba(0,0,0,0.4)` |
| `--ol-shadow-drawer` | `0 0 40px rgba(0,0,0,0.4)` | `0 0 0 2px rgba(255,255,255,0.12)` |
| `--ol-shadow-dropdown` | `0 4px 16px rgba(0,0,0,0.35), ...` | `0 0 0 2px rgba(255,255,255,0.10), 0 2px 8px rgba(0,0,0,0.3)` |

### 3.7 结构 Token — 保持不变

| 类别 | Token | 说明 |
|------|-------|------|
| **间距** | `--ol-space-0` ~ `--ol-space-24` | 保持 4px 网格 |
| **字体** | `--ol-font-family-ui` / `--ol-font-family-mono` | Inter + JetBrains Mono |
| **字号** | `--ol-font-size-10` ~ `--ol-font-size-36` | 保持 10-36px 梯队 |
| **字重** | `--ol-font-weight-400` ~ `700` | 不变 |
| **行高** | `--ol-line-height-*` | 不变 |
| **圆角** | `--ol-radius-*` | 标准 6px 微调至 8px |
| **动画** | `--ol-duration-*` | 时长不变，easing 用 DS 的 |
| **Z-Index** | `--ol-z-*` | 不变 |
| **布局** | `--ol-header-height` 等 | 不变 |

### 3.8 动画 Easing

| CSS 变量 | 旧值 | 新值 |
|----------|------|------|
| `--ol-ease-default` | `cubic-bezier(0.4, 0, 0.2, 1)` | `cubic-bezier(0.645, 0.045, 0.355, 1)` |
| `--ol-ease-in` | `cubic-bezier(0.4, 0, 1, 1)` | 不变 |
| `--ol-ease-out` | `cubic-bezier(0, 0, 0.2, 1)` | `cubic-bezier(0.215, 0.61, 0.355, 1)` |
| `--ol-ease-in-out` | `cubic-bezier(0.4, 0, 0.2, 1)` | `cubic-bezier(0.645, 0.045, 0.355, 1)` |
| `--ol-ease-spring` | `cubic-bezier(0.34, 1.56, 0.64, 1)` | 不变 |
| `--ol-ease-bounce` | `cubic-bezier(0.34, 1.56, 0.64, 1)` | 不变 |

## 四、文件变更清单

### 4.1 重写

| 文件 | 内容 |
|------|------|
| `poiesis/src/styles/tokens.css` | 全部 Token 值替换为上文映射表 |

### 4.2 删除

| 文件 | 原因 |
|------|------|
| `poiesis/src/styles/legacy-bridge.css` | 旧的 `--color-*` / `--dp-*` 桥接已不需要 |
| `poiesis/src/App.tsx` 中 `import './styles/legacy-bridge.css'` | 移除引用 |
| `poiesis/index.html` 中如果引用 legacy-bridge | 移除引用 |

### 4.3 硬编码颜色替换

扫描并替换以下文件中硬编码的 hex 颜色为 `var(--ol-*)` Token：

| 优先级 | 文件 | 典型硬编码值 |
|--------|------|-------------|
| P0 | `CodeCardEditor.module.css` | `#6366f1`, `#e2e8f0`, `#fff` |
| P0 | `ComponentDefPanel.css` | 混合硬编码 |
| P1 | 其他 CSS Module 中绕过 Token 的值 | TBD（通过 grep 扫描） |

### 4.4 新增

| 文件 | 内容 |
|------|------|
| `docs/design/DESIGN.md` | Design-System 原始设计文档 |
| `docs/design/tokens.default.json` | Design-System 原始 Token（参考） |
| `docs/design/variables.css` | Design-System 原始 CSS（参考） |
| `docs/design/guide.md` | Design-System 品牌指南 |

### 4.5 不碰

- `src/core/types/design.ts` — OmniLang 设计系统类型定义
- `src/store/slices/design-system.ts` — Token/Class/Variant CRUD
- `src/services/codegen/design-emitter.ts` — design.omni 发射器
- `src/services/codegen/parser.ts` — 解析器
- `src/store/slices/theme.ts` — 主题切换逻辑
- 所有 `.tsx` 组件逻辑代码
- `design.omni` 文件 — 用户项目内容

## 五、验收标准

1. **亮色模式**：Poiesis 编辑器 Chrome 呈现暖羊皮纸底色 + 陶土强调色 + ring-shadow 深度
2. **暗色模式**：接近纯黑底色 + 暖灰面板 + 柔白文字 + 亮环深度
3. **Token 完整性**：所有 CSS Module 中引用的 `--ol-*` 变量在新 `tokens.css` 中均存在
4. **零硬编码颜色**：`grep -rP '#[0-9a-fA-F]{3,6}' src/ --include='*.css'` 仅返回 Token 定义文件中的值
5. **TypeScript 编译**：`pnpm tsc --noEmit` 零错误
6. **Poiesis 正常启动**：编辑器 Chrome 渲染无异常
7. **主题切换**：亮色 ↔ 暗色切换正常，无视觉断层
8. **Design-System 源文件**：已存档至 `docs/design/`

## 六、风险与缓解

| 风险 | 缓解 |
|------|------|
| Token 遗漏 | 全局 grep `var(--ol-` 验证所有引用都有定义 |
| 硬编码颜色遗漏 | grep hex 模式扫描全项目 |
| 颜色在显示器上表现不一致 | 用 OKLCH 转换验证关键颜色在不同色温下的表现 |
| 已有功能衰退 | 不碰任何 `.tsx` 逻辑代码 |
