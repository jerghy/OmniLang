# Poiesis 代码生成修复 + WASM CSS 画布注入

## 目标

1. Poiesis 生成的 `.omni` 源码符合 OmniView 语法规范
2. 画布通过 WASM `resolve_styles` 加载 CSS，实现所见即所得

## 当前问题

### 代码生成语法错误

```omni
// 当前 Poiesis 输出
import omniview;

component main {
    view {
        <p id="Text_c13705" class: card label="Text"></p>
    }
}
```

| 问题 | 现状 | 正确写法 |
|------|------|---------|
| 缺少 design import | 无 | `import { AppDesign } from "./design.omni";` |
| class 引用格式错误 | `class: card` | `class: AppDesign.Card` |
| 文本内容当属性 | `label="Text"` | `<p>Text</p>` |

### 画布样式加载

当前 TypeScript `buildInlineStyleMap()` 直接在 JS 侧解析 token/class → 内联 style 对象，完全绕过 WASM 编译器。画布渲染效果与最终编译产物不一致。

## 设计

### 总体架构

```
                    ┌──────────────────────────┐
                    │   Poiesis Codegen Pipeline │
                    │   (Phase A → B → C)        │
                    └──────────┬───────────────┘
                               │ 产出正确的 .omni 源码
                               │ (import + class: Design.Class)
                               ▼
              ┌────────────────────────────────────┐
              │         .omni 源码 (内存)            │
              │  design.omni + main.omni + ...     │
              └────────────────┬───────────────────┘
                               │
          ┌────────────────────┼────────────────────┐
          │                    │                    │
          ▼                    ▼                    ▼
   保存到磁盘              WASM resolve_styles    编译 (omnic)
   (源码即正确)            → 返回 CSS + classes     → HTML/JS/CSS
                               │
                               ▼
                    画布 <style> 注入 + className
```

### 改动 1：修复代码生成器（Poiesis TypeScript）

**1a. classRefs 存储格式**

`ClassesSection.tsx`：绑定 class 时存 `DesignName.ClassName` 格式（如 `"AppDesign.Card"`），而非裸名 `"Card"`。

**1b. 代码生成**

- `import-collector.ts`：新增 design import（`import { AppDesign } from "./design.omni"`）
- `source-emitter.ts`：`label` prop 转为文本子节点，不发为 HTML 属性
- `pipeline.ts`：向各阶段传递 `designName`

### 改动 2：WASM element_classes 键改为 source id

`omnilang/src/codegen/omniview/html.rs`：`css_only` 模式下，`element_classes` / `element_tags` 的 key 从编译器内部 nid (`el-div-001`) 改为元素的 `id` prop 值。此改动仅影响 `resolve_styles` 路径，不影响 JS 代码生成。

### 改动 3：画布 WASM CSS 注入（Poiesis React）

**3a. OmniViewRenderer**

```tsx
// 取设计源码 + 组件源码
const designSource = useMemo(() => emitDesignSource(activeDesign), [activeDesign]);
const componentSource = useLiveCodePreview(); // 已有的 hook

// WASM 解析 CSS
const wasmStyles = useMemo(() => {
  if (!designSource || !componentSource) return null;
  const sources = {
    "src/design.omni": designSource,
    "src/main.omni": componentSource,
  };
  return rustCompiler.resolveStylesMulti(sources, {});
}, [designSource, componentSource]);

// 注入 <style> + 传递 className 映射
```

**3b. OmniViewElement**

将 `inlineStyleMap` prop 替换为 `classMap: Map<string, string[]>`，元素渲染时用 `className` 替代内联 `style`。

**3c. 保留 fallback**

WASM 不可用时（初始化中、编译错误），回退到现有的 `buildInlineStyleMap` TypeScript 路径。

### 改动 4：WASM resolve_styles 调整

确认 `resolve_styles_multi` ：
- 正常处理跨文件 import（`import { AppDesign } from "./design.omni"`）
- `element_classes` 的 key 使用 source `id`（改动 2 已覆盖）
- 返回的 `design_css` + `component_css` 可合并注入画布

## 不改的部分

- 保存路径不变：代码生成器产出正确的 `.omni` 源码直接写盘
- 编译路径不变：`compileToOmniViewFull` 继续用于完整编译
- React 组件树结构不变：仅替换样式注入方式

## 验证

1. 在 Poiesis 中绑定 class → 代码面板显示 `class: AppDesign.Card` + `import { AppDesign }`
2. 画布元素用 className 渲染，CSS 来自 `<style>` 标签
3. 保存的 `.omni` 文件可通过 `omnic` 编译
4. WASM 不可用时画布仍可渲染（fallback 路径）
