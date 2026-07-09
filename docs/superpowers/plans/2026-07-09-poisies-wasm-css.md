# Poiesis 代码生成修复 + WASM CSS 画布注入 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Poiesis 生成符合 OmniView 语法的 `.omni` 源码，画布通过 WASM `resolve_styles` 注入 CSS 实现所见即所得。

**Architecture:** 在现有 Poiesis codegen pipeline 上做最小改动——加 `designName` 参数串接各阶段修语法问题，画布端在 `OmniViewRenderer` 中调 WASM 获取 CSS 并注入 `<style>` 标签，`OmniViewElement` 改用 `className` 渲染。

**Tech Stack:** TypeScript (React), Rust (WASM/omnilang)

## Global Constraints

- 不改动现有保存路径（`.omni` 源码直接写盘）
- WASM 不可用时保留现有 TypeScript `buildInlineStyleMap` fallback
- 所有改动向后兼容（旧项目 classRefs 裸名仍可工作）

---

### Task 1: pipeline-types 添加 designName

**Files:**
- Modify: `poiesis/src/services/codegen/pipeline-types.ts`

**Interfaces:**
- Produces: `designName?: string` on `GenerateOmniSourceOptions`, `GenerateOmniFilesOptions`, `GenerateOmniFragmentOptions`

- [ ] **修改 pipeline-types.ts**

`GenerateOmniSourceOptions` 加 `designName?: string`，另外两个接口同改。

```typescript
export interface GenerateOmniSourceOptions {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentName: string;
  rootOrder?: string[];
  forSave?: boolean;
  componentMeta?: ComponentMeta;
  existingPidMap?: PidMap;
  /** Design system name for class reference prefixing and import emission. */
  designName?: string;
}
```

`GenerateOmniFilesOptions` 和 `GenerateOmniFragmentOptions` 同样加 `designName?: string`。

- [ ] **TypeScript 检查**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20
```

- [ ] **Commit**

```bash
git add poiesis/src/services/codegen/pipeline-types.ts
git commit -m "feat: add designName to pipeline option types"
```

---

### Task 2: import-collector 发 design import

**Files:**
- Modify: `poiesis/src/services/codegen/import-collector.ts`

**Interfaces:**
- Consumes: `designName?: string` (from Task 1 type)
- Produces: import string `import { AppDesign } from "./design.omni"`

- [ ] **修改 collectImports**

签名加 `designName?: string`，在 imports 数组里 omniview 后插入 design import：

```typescript
export function collectImports(
  pascalCaseTags: string[],
  storeRefs: string[],
  codeTabs: CodeTab[],
  designName?: string,
): string[] {
  const imports: string[] = ["import omniview;"];

  if (designName) {
    imports.push(`import { ${designName} } from "./design.omni";`);
  }

  for (const tag of pascalCaseTags) {
    imports.push(`import { ${tag} } from "./components/${tag}";`);
  }

  for (const storeName of storeRefs) {
    imports.push(`import { ${storeName} } from "./stores/${storeName}";`);
  }
  // ... rest unchanged
}
```

- [ ] **Commit**

```bash
git add poiesis/src/services/codegen/import-collector.ts
git commit -m "feat: emit design import when designName is set"
```

---

### Task 3: collect-styles 标准化 classRefs

**Files:**
- Modify: `poiesis/src/services/codegen/collect-styles.ts`

**Interfaces:**
- Consumes: `designName?: string`
- Produces: `elementClassMap` with prefixed class names

- [ ] **修改 collectStylesFromEntries**

签名加 `designName?: string`，classRefs 标准化：裸名加 `DesignName.` 前缀。

```typescript
export function collectStylesFromEntries(
  elements: Array<{
    id: string;
    style?: { classRefs?: string[]; inlineProperties?: Array<{ name: string; segments: StyleValueSegment[] }> };
  }>,
  _styleRegistry: StyleRegistry,
  _roots?: ViewNode[],
  designName?: string,
): {
  componentStyle: Record<string, Record<string, string>>;
  elementClassMap: Map<string, string[]>;
  elementInlineMap: Map<string, Record<string, string>>;
} {
  const elementClassMap = new Map<string, string[]>();
  const elementInlineMap = new Map<string, Record<string, string>>();

  for (const el of elements) {
    const classRefs = el.style?.classRefs ?? [];
    if (classRefs.length > 0) {
      const normalized = classRefs.map((ref) => {
        if (ref.includes(".")) return ref;              // already "Design.Class"
        if (designName) return `${designName}.${ref}`;  // prefix bare name
        return ref;                                      // no designName, keep as-is
      });
      elementClassMap.set(el.id, [...new Set(normalized)]);
    }
    const inlineProps = el.style?.inlineProperties ?? [];
    if (inlineProps.length > 0) {
      const inlineMap: Record<string, string> = {};
      for (const prop of inlineProps) {
        inlineMap[prop.name] = compileSegments(prop.segments);
      }
      elementInlineMap.set(el.id, inlineMap);
    }
  }

  return { componentStyle: {}, elementClassMap, elementInlineMap };
}
```

- [ ] **Commit**

```bash
git add poiesis/src/services/codegen/collect-styles.ts
git commit -m "feat: normalize classRefs with designName prefix"
```

---

### Task 4: pipeline 串接 designName

**Files:**
- Modify: `poiesis/src/services/codegen/pipeline.ts`

**Interfaces:**
- Consumes: `designName` from options (Task 1)
- Produces: passes `designName` to `collectImports` (Task 2) and `collectStylesFromEntries` (Task 3)

- [ ] **修改 runPhaseA — 传 designName 给 collectImports**

```typescript
const a3 = runPhase("A", "import-collector", () =>
    collectImports(pascalCaseTags, [...bindings.storeRefs.keys()], options.codeTabs, options.designName),
);
```

- [ ] **修改 runPhaseC — 传 designName 给 collectStylesFromEntries**

```typescript
const styleCollection = c1.errors.length > 0
    ? defaultStyleCollection()
    : collectStylesFromEntries(
        expandedElements,
        { variables: [], classes: [] },
        phaseA.roots,
        options.designName,
      );
```

- [ ] **Commit**

```bash
git add poiesis/src/services/codegen/pipeline.ts
git commit -m "feat: thread designName through pipeline phases"
```

---

### Task 5: ClassesSection 存储带 DesignName 前缀的 classRefs

**Files:**
- Modify: `poiesis/src/components/panels/inspector/ClassesSection.tsx`

**Interfaces:**
- Consumes: `activeDesign.name` from store
- Produces: `classRefs` stored as `["AppDesign.Card", "AppDesign.Row"]`

- [ ] **修改 handleAddClass — 加前缀**

```typescript
const handleAddClass = useCallback(
    (className: string) => {
      const prefixed = `${activeDesign.name}.${className}`;
      if (currentRefs.includes(prefixed)) return;
      setElementClassRefs(element.id, [...currentRefs, prefixed]);
      setSearch("");
      setShowDropdown(false);
    },
    [element.id, currentRefs, setElementClassRefs, activeDesign.name],
);
```

- [ ] **Commit**

```bash
git add poiesis/src/components/panels/inspector/ClassesSection.tsx
git commit -m "feat: store classRefs with DesignName.ClassName prefix"
```

---

### Task 6: source-emitter — label→文本内容 + skip label attr

**Files:**
- Modify: `poiesis/src/services/codegen/source-emitter.ts`

**Interfaces:**
- Consumes: `elNode.props.label`
- Produces: text content inside element instead of `label="..."` attr

- [ ] **修改 emitViewNode — HTML 属性遍历中 skip label**

在 props 遍历的 skip 列表（~369行）加上 `label`：

```typescript
for (const [key, val] of Object.entries(elNode.props)) {
    if (key === "className" || key === "style" || key === "key") continue;
    if (key === "label") continue;  // handled as text content below
    if (key.startsWith("on")) continue;
    // ... rest unchanged
}
```

- [ ] **修改 emitViewNode — contentText 判定加 label fallback**

在现有的 `style?.contentText` 判定后（~437行）加：

```typescript
// Determine content text: prefer resolvedStyle, fall back to props.label
let contentText = style?.contentText;
if (!contentText && elNode.props.label && typeof elNode.props.label === "string") {
    contentText = elNode.props.label;
}
```

- [ ] **Commit**

```bash
git add poiesis/src/services/codegen/source-emitter.ts
git commit -m "feat: emit label prop as text content, not HTML attribute"
```

---

### Task 7: WASM html.rs — element_classes 键改为 source id

**Files:**
- Modify: `omnilang/src/codegen/omniview/html.rs`

**Interfaces:**
- Consumes: `el.props.get("id")` in `css_only` mode
- Produces: `element_classes` / `element_tags` keyed by source element `id` prop

- [ ] **修改 void element css_only 路径（~741 行）**

```rust
// Before:
if self.css_only {
    self.element_classes
        .insert(nid.clone(), class_name_parts.clone());
    self.element_tags
        .insert(nid.clone(), html_tag.to_string());
    return String::new();
}

// After:
if self.css_only {
    let key = el.props.get("id")
        .and_then(|v| {
            if let ExprIR::LiteralExpr { value } = v {
                value.as_str().map(|s| s.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| nid.clone());
    self.element_classes
        .insert(key.clone(), class_name_parts.clone());
    self.element_tags
        .insert(key, html_tag.to_string());
    return String::new();
}
```

- [ ] **同样修改 non-void element css_only 路径（~794 行）**

相同改动。

- [ ] **构建 WASM**

```bash
cd omnilang && cargo build --release --features wasm,codegen-omniview --target wasm32-unknown-unknown
```

- [ ] **Commit**

```bash
git add omnilang/src/codegen/omniview/html.rs
git commit -m "feat: use source element id as key in css_only element_classes"
```

---

### Task 8: OmniViewRenderer — WASM CSS 注入

**Files:**
- Modify: `poiesis/src/components/design/OmniViewRenderer.tsx`

**Interfaces:**
- Consumes: `rustCompiler.resolveStylesMulti()`, `emitDesignSource()`, `useLiveCodePreview()`
- Produces: `<style>` tag + `classMap` prop 传给 `OmniViewElement`

- [ ] **修改 OmniViewRenderer — 加 WASM style 解析 + classMap + <style> 注入**

```tsx
import { useMemo } from "react";
import { usePoiesisStore } from "../../store";
import { useElements, useRootOrder, useSelectedElementIds, useComponentDefs } from "../../store/hooks";
import { OmniViewElement } from "./OmniViewElement";
import { buildInlineStyleMap } from "./render-utils";
import { rustCompiler } from "../../commands/rustCompiler";
import { useLiveCodePreview } from "../../hooks/useLiveCodePreview";
import { emitDesignSource } from "../../services/codegen/design-emitter";
import type { ElementData, ViewElementData } from "../../core/types/element";
import type { ComponentDef } from "../../core/types/project-json";

export function OmniViewRenderer() {
  // ... existing hooks ...

  const activeDesign = usePoiesisStore((s) => s.activeDesign);
  const componentSource = useLiveCodePreview();

  // Build design.omni source from activeDesign
  const designSource = useMemo(() => {
    if (!activeDesign || activeDesign.tokens.length === 0) return null;
    return emitDesignSource(activeDesign);
  }, [activeDesign]);

  // WASM CSS resolution
  const wasmStyleResult = useMemo(() => {
    if (!componentSource || !designSource || !rustCompiler.isReady) return null;
    const sources: Record<string, string> = {
      "src/design.omni": designSource,
      "src/main.omni": componentSource,
    };
    const pathMapping: Record<string, string> = {
      "./design.omni": "src/design.omni",
    };
    const result = rustCompiler.resolveStylesMulti(sources, pathMapping);
    if (!result.success) return null;
    return result;
  }, [componentSource, designSource]);

  // Build classMap from WASM result (element id → class names)
  const classMap = useMemo(() => {
    if (!wasmStyleResult) return null;
    const map = new Map<string, string[]>();
    for (const [elId, classes] of Object.entries(wasmStyleResult.element_classes)) {
      map.set(elId, classes);
    }
    return map;
  }, [wasmStyleResult]);

  // Combined CSS for <style> injection
  const combinedCss = useMemo(() => {
    if (!wasmStyleResult) return null;
    const parts: string[] = [];
    if (wasmStyleResult.design_css) parts.push(wasmStyleResult.design_css);
    for (const css of Object.values(wasmStyleResult.component_css)) {
      if (css) parts.push(css);
    }
    return parts.join("\n");
  }, [wasmStyleResult]);

  // Fallback: existing TS-based inline style resolution
  const inlineStyleMap = useMemo(
    () => buildInlineStyleMap(expandedElements, activeDesign),
    [expandedElements, activeDesign],
  );

  // ... existing rootElements, callbacks ...

  return (
    <div data-renderer="omniview" style={{ width: "100%", height: "100%", overflow: "auto", padding: 8, boxSizing: "border-box" }}>
      {combinedCss && <style dangerouslySetInnerHTML={{ __html: combinedCss }} />}
      {rootElements.length === 0 ? (
        <div style={{ /* ... existing empty canvas ... */ }}>
          空画布 — 从组件面板拖入元素
        </div>
      ) : (
        rootElements.map((el) => (
          <OmniViewElement
            key={el.id}
            element={el}
            expandedElements={expandedElements}
            customChildIds={customChildIds}
            selectedIds={selectedIdSet}
            inlineStyleMap={inlineStyleMap}
            classMap={classMap}
            onSelect={handleSelect}
            onDoubleClick={handleDoubleClick}
          />
        ))
      )}
    </div>
  );
}
```

- [ ] **Commit**

```bash
git add poiesis/src/components/design/OmniViewRenderer.tsx
git commit -m "feat: inject WASM-resolved CSS into canvas via <style> tag"
```

---

### Task 9: OmniViewElement — className 渲染

**Files:**
- Modify: `poiesis/src/components/design/OmniViewElement.tsx`

**Interfaces:**
- Consumes: `classMap?: ReadonlyMap<string, string[]>` (new prop)
- Produces: `className` on rendered elements instead of inline `style` for design classes

- [ ] **修改 OmniViewElement — 加 classMap prop + className 渲染**

```tsx
export interface OmniViewElementProps {
  element: ViewElementData;
  expandedElements: ReadonlyMap<string, ElementData>;
  customChildIds: ReadonlySet<string>;
  selectedIds: ReadonlySet<string>;
  inlineStyleMap: ReadonlyMap<string, Record<string, string>>;
  /** WASM-resolved class names per element id. When available, className is used. */
  classMap?: ReadonlyMap<string, string[]> | null;
  onSelect: (id: string) => void;
  onDoubleClick: (id: string) => void;
}

export const OmniViewElement = React.memo(function OmniViewElement({
  element,
  expandedElements,
  customChildIds,
  selectedIds,
  inlineStyleMap,
  classMap,
  onSelect,
  onDoubleClick,
}: OmniViewElementProps) {
  const isInsideCustom = customChildIds.has(element.id);
  const isSelected = !isInsideCustom && selectedIds.has(element.id);

  const inlineStyles = inlineStyleMap.get(element.id) ?? {};

  // WASM class names take priority over inline styles for design classes
  const wasmClasses = classMap?.get(element.id);
  const className = wasmClasses ? wasmClasses.join(" ") : undefined;

  const style: React.CSSProperties = {
    outline: isSelected ? "2px solid #7C3AED" : "none",
    outlineOffset: "-2px",
    cursor: isInsideCustom ? "default" : "pointer",
    // When WASM CSS is active, only apply non-design inline styles
    // (inline style properties that are NOT from design classes)
    ...(className ? {} : inlineStyles),
  };

  // ... rest unchanged (tagName, callbacks, children) ...

  return React.createElement(
    tagName,
    {
      key: element.id,
      "data-pid": element.id,
      className,
      style,
      onClick: handleClick,
      onDoubleClick: handleDoubleClick,
    },
    element.props?.label != null ? String(element.props.label) : null,
    ...childElements,
  );
});
```

- [ ] **Commit**

```bash
git add poiesis/src/components/design/OmniViewElement.tsx
git commit -m "feat: support className rendering from WASM classMap in canvas"
```

---

### Task 10: 端到端验证

- [ ] **启动 Poiesis + 验证代码生成**

```bash
bash D:/space/OmniLang/.claude/skills/start-poiesis/start-poiesis.sh
```

用 Victauri 检查：
1. 在画布上选一个元素，在 Class 绑定面板绑定一个 design class
2. 查看代码面板：应有 `import { AppDesign } from "./design.omni"` + `class: AppDesign.Card`
3. 查看画布 DOM：元素应有 `class="Card"`（来自 WASM CSS），而非纯内联 style
4. 保存项目，检查 `.omni` 文件内容正确

- [ ] **构建 WASM + 运行 Rust 测试**

```bash
cd omnilang && cargo test
```

- [ ] **Commit 最终调整**

```bash
git add -A
git commit -m "chore: final verification and cleanup"
```
