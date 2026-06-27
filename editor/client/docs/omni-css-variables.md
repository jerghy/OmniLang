# OmniLang CSS Variables — 语言级理解

> 基于 `test/projects/learn-omniview/src/components/` 中的实际示例分析。
> 编辑器样式面板必须忠实反映 OmniLang 语言能力，不能创造语言不支持的模式。

---

## 一、CSS 变量的三种用法

### 1. 声明（Declaration）

在 `style {}` 块中声明 `--*` 属性，值固定，不能动态变化。

```css
.wrapper {
    --primary: #6366f1;
    --border-color: #cbd5e1;
    --radius-sm: 6px;
    --radius-md: 12px;
}
```

- 这些值**不可通过运行时改变**
- 编译直接在 style block 中输出 CSS 规则
- 作用域由选择器层级决定（级联继承）

### 2. 注入（Injection）

通过 `<div style: { --name: signal }>` 将信号值动态注入为 CSS 变量。

```html
<div style: { --accent: accentColor, --theme-bg: ThemeStore.currentBg }>
```

- 值可以是信号引用、字符串表达式、或字面量
- 编译器为每个 `--*` 信号引用生成独立的 `setProperty()` 效果代码
- 注入的变量可被元素自身及其子元素通过 `var()` 访问

**代码路径：**
```
style: { --var: signal }
  → __omni_effect(() => el.style.setProperty("--var", signal.value))
```

### 3. 引用（Reference）

在任何属性值中使用 `var(--name)` 来引用 CSS 变量。

```css
/* style block 中引用 */
.themed-card {
    background: var(--theme-bg);
    color: var(--theme-text);
    border-radius: var(--card-radius);
    padding: 16px;
}

/* 内联 style 中引用 */
<div style: { background: "var(--theme-bg)", border: "2px solid var(--theme-border)" }>
```

- `var()` 是 CSS 标准函数，遵循 CSS 级联规则
- 可提供回退值：`var(--name, fallback)`
- 可参与 `calc()` 等函数组合：`calc(var(--theme-border-w) * 2px)`

---

## 二、CSS 变量在文件中的位置

### 2.1 在 style block 中声明 + 引用

```
component X {
    view { ... }
    style {
        /* 声明变量 */
        .wrapper { --primary: #6366f1; }

        /* 引用变量 */
        .card { color: var(--primary); background: var(--surface, white); }
    }
}
```

### 2.2 在内联 style 中注入（静态）

```html
<div style: {
    --box-bg: "#e0e7ff",
    --box-text: "#3730a3",
    --box-radius: "8px"
}>
```

值带引号 `"..."`，编译为字符串常量用 `setProperty()` 设置。

### 2.3 在内联 style 中注入（动态——信号）

```html
<div style: {
    --accent: accentColor,                            /* 信号 */
    border: "2px solid " + accentColor                 /* 表达式 */
}>
```

值不带引号时识别为信号引用或表达式。

---

## 三、StyleEntry 三种类型与 CSS 变量的关系

| StyleEntry 类型 | 对应编译产物 | 是否支持 `--*` |
|----------------|-------------|----------------|
| `inline` | `<div style: { prop: value, --custom: value }>` | ✅ 支持，属性名以 `--` 开头时识别为 CSS 变量 |
| `class` | `.xxx { prop: value; --custom: value }` 在 style block + `class: xxx` 在元素 | ✅ 支持，class 属性集中可包含 `--*` |
| `variable` | `._v-xxx { --xxx: value }` 在 style block + `class: _v-xxx` 在元素 | ✅ 专门为 CSS 变量设计 |

### 关键区别

- **`inline` 中的 `--*`** → 对应用法 2（注入）。用在某个元素的 `<div style: {}>` 中，作用域为该元素及其子元素。
- **`class` 中的 `--*`** → class 属性集的一部分。编译为 style block 中 class 规则的一部分，`--*` 和普通属性混写。
- **`variable`** → 对应用法 1（声明）。专门创建一个设 `--*` 的类，目的是让其他地方能通过 `var()` 引用。也可通过注入覆盖。

---

## 四、编译映射

### `class` 类型 entry：`{ name: "card", properties: [{ name: "padding", value: "16px" }, { name: "--radius", value: "8px" }] }`

```css
/* style block 输出 */
.card { padding: 16px; --radius: 8px; }
```

```html
<!-- 元素上 -->
<div class: card>
```

### `variable` 类型 entry：`{ name: "--primary", properties: [{ name: "value", value: "#6366f1" }] }`

```css
/* style block 输出 */
._v-primary { --primary: #6366f1; }
```

```html
<!-- 元素上 -->
<div class: _v-primary>
```

### `inline` 类型 entry：`{ properties: [{ name: "padding", value: "16px" }, { name: "--accent", value: "accentColor" }] }`

```html
<!-- inline 输出 -->
<div style: { padding: "16px", --accent: accentColor }>
```

---

## 五、关键设计约束

1. **SSOT（单一事实来源）**：class 和 variable 的定义存储在全局注册表中。所有引用同名的 entry 共享同一份定义。修改定义时所有引用同步更新。
2. **平台忠实性**：编辑器 UI 的设计必须对应 OmniLang 实际编译产物。不能创建编译器不支持的虚拟概念。
3. **`var()` 是消费者侧的引用**：`var(--name)` 出现在属性值中，由编辑器中的**属性值自动补全**支持，而非作为一个独立的 StyleEntry 类型。
4. **信号引用是 inline 的值类型**：`style: { --accent: accentColor }` 中 `accentColor` 是信号引用，属于值的一种类型，对应 inline entry 中属性值的「信号引用」模式。
