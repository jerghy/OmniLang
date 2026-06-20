# 逻辑食谱

> 即用型 OmniLang 纯计算模式 —— CLI 工具、算法、数据处理。

---

## 食谱：CLI 参数工具

接受参数的命令行工具。

```omni
fn run(command: String, verbose: bool) -> String {
    if verbose {
        return "详细模式执行：" + command;
    }
    return command;
}

fn main() {
    let result = run("build", true);
    print(result);
}
```

```bash
omnic run cli-tool.omni --target python
# 输出：详细模式执行：build
```

**涉及概念**：函数参数、`bool` 类型、`if` 表达式、字符串拼接。

---

## 食谱：斐波那契（递归）

```omni
fn fib(n: i32) -> i32 {
    if n <= 1 {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

fn main() {
    let result: i32 = fib(10);
    println(result.to_string());    // 55
}
```

**涉及概念**：递归函数、`i32` 算术、提前返回、尾表达式返回。

---

## 食谱：冒泡排序

用可变数组实现的原地排序。

```omni
fn main() {
    let mut arr: Array<i32> = [3, 1, 2];
    let mut i: i32 = 0;
    let n: i32 = arr.len();

    while i < n {
        let mut j: i32 = 0;
        while j < n - i - 1 {
            if arr.get(j) > arr.get(j + 1) {
                let tmp: i32 = arr.get(j);
                arr.set(j, arr.get(j + 1));
                arr.set(j + 1, tmp);
            }
            j = j + 1;
        }
        i = i + 1;
    }

    for v in arr {
        println(v.to_string());
    }
    // 输出：1, 2, 3
}
```

**涉及概念**：可变数组、嵌套 `while` 循环、元素交换、`for` 遍历。

---

## 食谱：字符串格式化

使用 `omni/string` 标准库。

```omni
import { to_upper, trim, split, join } from "omni/string";

fn format_name(raw: String) -> String {
    let trimmed = trim(raw);
    return to_upper(trimmed);
}

fn tags_to_string(tags: String) -> String {
    let parts = split(tags, ",");
    return join(parts, " | ");
}

fn main() {
    let name = format_name("  alice  ");
    println(name);                          // ALICE

    let display = tags_to_string("rust,dart,python");
    println(display);                       // rust | dart | python
}
```

**涉及概念**：标准库导入、字符串操作、函数组合。

---

## 食谱：数学工具

使用 `omni/math` 执行常见计算。

```omni
import { abs, floor, ceil, sqrt, pow, PI, clamp, random } from "omni/math";

fn circle_area(radius: f64) -> f64 {
    return PI * radius * radius;
}

fn pythagorean(a: f64, b: f64) -> f64 {
    return sqrt(a * a + b * b);
}

fn clamp_score(raw: i32) -> i32 {
    return clamp(raw, 0, 100);
}

fn main() {
    let area: f64 = circle_area(5.0);
    println(area.to_string());          // ~78.54

    let hyp: f64 = pythagorean(3.0, 4.0);
    println(hyp.to_string());           // 5.0

    let score: i32 = clamp_score(150);
    println(score.to_string());         // 100
}
```

**涉及概念**：标准库常量、`f64` 数学、类型转换。

---

## 食谱：随机数生成器

用 `random()` 和 `floor()` 做骰子。

```omni
import { random, floor } from "omni/math";

fn roll_die(sides: i32) -> i32 {
    return floor(random() * sides) + 1;
}

fn main() {
    let mut i: i32 = 0;
    while i < 5 {
        let result: i32 = roll_die(6);
        println(result.to_string());
        i = i + 1;
    }
    // 输出：5 个 1 到 6 之间的随机数
}
```

**涉及概念**：随机数生成、类型转换、循环。

---

## 食谱：带结构体的计数器

将状态封装在结构体中。

```omni
struct Counter {
    value: i32,
}

fn increment(c: Counter) -> Counter {
    return Counter { value: c.value + 1 };
}

fn decrement(c: Counter) -> Counter {
    return Counter { value: c.value - 1 };
}

fn main() {
    let mut c: Counter = Counter { value: 0 };
    c = increment(c);
    c = increment(c);
    c = decrement(c);
    println(c.value.to_string());    // 1
}
```

**涉及概念**：结构体实例化、不可变更新（返回新结构体）、可变重新绑定。

---

## 如何运行任意食谱

1. 将食谱保存为逻辑项目的 `src/main.omni`：

```bash
omnic create my-recipe --type logic
# 将 src/main.omni 替换为食谱代码
omnic run src/main.omni
```

2. 或编译到特定目标：

```bash
omnic run recipe.omni --target python
omnic run recipe.omni --target js
omnic run recipe.omni --target rust
```

---

## 参见

- [UI 食谱](ui-recipes.md) — 组件、Store 和样式
- [教程 2：语言基础](../tutorial/02-language-basics.md) — 核心语言概念
- [教程 3：结构体与数据类型](../tutorial/03-structs-and-data.md) — 自定义类型和泛型
- [标准库](../stdlib.md) — 完整 API 参考
