# 16 — 表单与异步数据

本页覆盖两个**功能设计级语义域**（spec 2026-08-08 §4.1 实施收口）：**表单语义（`@form`）**与**数据三态（`@load`）**。编译器理解「这是一个表单」「这是一个异步数据源」的意图，并针对编译目标（React / Flutter 生产级，OmniView 调试级）生成地道生产代码——源码表达意图，选路是 codegen 职责。

> **语义域登记**：字段校验规则（`validate:` 值域闭集 6 名）已登记 vocabulary-registry 域 6；`formErrors`/`formValid`/`formSubmitting`（`@form`）与 `loading`/`error`/`data`/`retry`（`@load`）是**编译器保留名判定**（保留名冲突 = 硬错误），随机制自动生成、用户不可自定义，不属于语义词汇。

## 表单语义（@form）

意图：「这是一个表单——字段有校验规则、提交时收集值并执行提交、错误显示在字段旁」。

```omni
component App {
  signal name: string = ""
  fn submit(e) {}
  view {
    <form @form @onSubmit={submit}>
      <input bind:value={name} validate:{required: true} />
      @if form.errors.name {
        <span id="err-name">{form.errors.name}</span>
      }
      <button disabled={form.submitting}>提交</button>
    </form>
  }
}
```

编译器生成：字段值信号（`bind:value` 接线）、`form.errors` 错误信号 map、提交时校验执行、a11y（aria-invalid/aria-describedby）、`form.submitting` 禁用提交态信号。

### 字段校验（validate: 规则）

`validate:` 是**字段级属性**，值域为 6 名封闭规则集（`VALIDATE_RULES`——编译器白名单，登记册域 6 双向断言）：

| 规则 | 值域 / 约束 | 内建错误消息 |
|------|------------|-------------|
| required | 无参（`true`） | 此项必填 |
| format | 闭集名：`"email"` / `"url"` | 邮箱格式不正确 / URL 格式不正确 |
| min | 数字 | 长度至少为 N |
| max | 数字 | 长度最多为 N |
| pattern | 正则字符串 | RegExp 格式不正确 |
| custom | 组件 fn 名（v0 边界：无字面量自定义消息） | fn 返回消息 |

约束（违规 = 编译期硬错误）：

- **`validate:` 必须与 `bind:value`/`bind:checked` 配对使用**（孤儿规则无从校验）。
- **规则名 ∈ 闭集**：`未知校验规则 "@x"——合法规则: required/format/min/max/pattern/custom`。
- **规则值类型合法**（`validate:{format: "email"}`；闭集外值如 `"tel"` → 硬错误，需 custom）。
- `custom` 规则值为**组件 fn 名**（存在 + 单参 + 返回 String）。

### 错误显式渲染与禁用提交态

- **错误信号显式渲染**：`form.errors` + `@if form.errors.name` 由用户显式渲染（React / OmniView）。
- **Flutter 例外（行为差异注记）**：错误经 `TextFormField.errorText` **自动展示**，语言面 `form.errors` 发射为**常量空 map**——语言面跨目标一致保留，行为差异注记于 semantic-universe 域 8。
- **禁用提交态 v0 形态**：编译器生成 `form.submitting` 信号（v0 恒 false），按钮禁用由用户显式写 `disabled={form.submitting}`——**v0 不做自动按钮接线**。

### 提交（@onSubmit）与保留名

- `@form` **必须配 `@onSubmit`**（`@form 元素必须绑定 @onSubmit`）；每组件至多一个 `@form`。
- `@onSubmit` 事件数据 schema 仅 `{values}`（FormData 收集，`{string: string}`）；旧 DOM 形状字段（`e.value` 等）→ 无效字段硬错误。
- **保留名冲突 = 硬错误**：`formErrors` / `formValid` / `formSubmitting` 与用户信号声明冲突 → `信号名 "@x" 与 @form 生成的保留名冲突`。`form` 对象（`form.errors`/`form.valid`/`form.submitting`）由编译器生成，用户不可声明同名信号。

### 三目标映射

| 目标 | 生成形状 |
|------|---------|
| React | 受控组件 + `validateForm` 执行器（校验规则链 → `setFormErrors` map，`formValid = Object.keys(formErrors).length === 0`） |
| Flutter | `Form` + `TextFormField.validator` 链（`_formKey`；错误经 `errorText` 自动展示，`form.errors` 常量空 map） |
| OmniView | HTML5 校验属性（`required`/`minLength`/`maxLength`/`pattern`…）+ `{Comp}_validateForm` 自绘错误 |

## 数据三态（@load）

意图：「这个数据源是异步的——加载/错误/数据三态由编译器管理」。

```omni
component App {
  fn fetchTodos() -> Array<String> {}
  view {
    @load={fetchTodos}
    @if loading {
      <div>loading</div>
    }
    @if error != "" {
      <div>{error}</div>
    }
    @for item in data {
      <div>{item}</div>
    }
  }
}
```

编译器生成 `loading`（Bool）/ `error`（String）/ `data`（Array）三态信号 + `retry` 重试回调（绑定到用户交互，如 `@onPress={retry}`）。

约束与边界：

- **视图级指令**：`@load={fn}` 绑定组件 fn，mount 后自动装载；每组件至多一个 `@load`。
- fn 必须**存在且无参数**（`@load 引用的函数 "@x" 不存在` / `@load 引用的函数 "@x" 必须无参数`）。
- **v0 边界：仅 Array 返回**（`data` 零值 `[]`）；struct/其他返回 → 硬错误 `@load 仅支持 Array 返回（v0）`。
- **保留名冲突 = 硬错误**：`loading` / `error` / `data` / `retry` 与用户信号声明冲突 → `信号名 "@x" 与 @load 生成的保留名冲突`。
- **增量封装，不替代既有原语**：`@load` 是既有 `async action + await` 之上的三态封装增量——已有 async/await 代码零改动。

### 三目标映射

| 目标 | 生成形状 |
|------|---------|
| React | `useEffect` + 三态 state |
| Flutter | `FutureProvider`（Riverpod 3.0 既有依赖） |
| OmniView | 自绘三态 |

## 语言边界（v0 defer）

- **字段类型检查（V6）**：`value` 类型与控件匹配 → 仅警告级，v0 未实现。
- **`@load` 非 Array 返回**：v0 硬错误（见上），struct 支持属后续版本。
- **多表单 / 跨组件表单共享**：v0 不支持（每组件至多一个 `@form`）。
- **FormData 预填**：v0 不支持。
- **`form.submitting` 异步语义**：v0 恒 false（Flutter 端同步行为注记：常量发射）。
- **`custom` 自定义消息**：v0 仅组件 fn 名，无字面量自定义消息。
