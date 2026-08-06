# OmniLang Standard Library

> **⚠️ 已废弃（v4，2026-08-06）**：本文档教授 `omni/*` 导入路径——编译器事实为 `std/<module>`（crates/omnilang-core/src/stdlib/mod.rs）。正确文档：`docs/omniview/12-standard-library.md`（`std/math`、`std/string`、`std/async`、`std/http` 等）。本文件保留至迁移完成。

The OmniLang standard library provides cross-platform modules that work across all compilation targets. Each module is documented with available functions, types, and usage examples.

## Module: `std/math`

Mathematical operations and constants.

### Constants

| Name | Type | Value | Description |
|------|------|-------|-------------|
| `PI` | `f64` | `3.141592653589793` | Archimedes' constant |
| `E` | `f64` | `2.718281828459045` | Euler's number |
| `TAU` | `f64` | `6.283185307179586` | Tau = PI * 2 |
| `INFINITY` | `f64` | Infinity | Positive infinity |
| `NEG_INFINITY` | `f64` | -Infinity | Negative infinity |
| `EPSILON` | `f64` | `2.220446049250313e-16` | Machine epsilon |

### Functions

#### `abs(x: i32 | f64) -> i32 | f64`

Returns the absolute value.

```omnilang
import { abs } from "std/math";
let n: i32 = abs(-5);          // 5
let f: f64 = abs(-3.14);       // 3.14
```

#### `floor(x: f64) -> i32`

Returns the largest integer less than or equal to `x`.

```omnilang
import { floor } from "std/math";
let n: i32 = floor(3.7);       // 3
let n2: i32 = floor(-1.2);     // -2
```

#### `ceil(x: f64) -> i32`

Returns the smallest integer greater than or equal to `x`.

```omnilang
import { ceil } from "std/math";
let n: i32 = ceil(3.2);        // 4
```

#### `round(x: f64) -> i32`

Rounds to the nearest integer (ties round away from zero).

```omnilang
import { round } from "std/math";
let n: i32 = round(3.5);       // 4
```

#### `sqrt(x: f64) -> f64`

Returns the square root.

```omnilang
import { sqrt } from "std/math";
let s: f64 = sqrt(16.0);       // 4.0
```

#### `pow(base: f64, exp: f64) -> f64`

Raises `base` to the power of `exp`.

```omnilang
import { pow } from "std/math";
let p: f64 = pow(2.0, 8.0);    // 256.0
```

#### `sin(x: f64) -> f64`, `cos(x: f64) -> f64`, `tan(x: f64) -> f64`

Trigonometric functions (radians).

```omnilang
import { sin, cos, PI } from "std/math";
let s: f64 = sin(PI / 2.0);    // 1.0
```

#### `asin(x: f64) -> f64`, `acos(x: f64) -> f64`, `atan(x: f64) -> f64`

Inverse trigonometric functions.

#### `atan2(y: f64, x: f64) -> f64`

Returns the angle from the positive x-axis to the point `(x, y)`.

#### `log(x: f64, base: f64) -> f64`

Logarithm with specified base.

```omnilang
import { log } from "std/math";
let l: f64 = log(100.0, 10.0); // 2.0
```

#### `ln(x: f64) -> f64`

Natural logarithm.

```omnilang
import { ln } from "std/math";
let l: f64 = ln(2.71828);      // ~1.0
```

#### `log2(x: f64) -> f64`, `log10(x: f64) -> f64`

Base-2 and base-10 logarithms.

#### `min(a: i32|f64, b: i32|f64) -> i32|f64`

Returns the smaller of two values.

#### `max(a: i32|f64, b: i32|f64) -> i32|f64`

Returns the larger of two values.

#### `clamp(value: i32|f64, min: i32|f64, max: i32|f64) -> i32|f64`

Clamps a value within a range.

```omnilang
import { clamp } from "std/math";
let n: i32 = clamp(15, 0, 10); // 10
```

#### `random() -> f64`

Returns a pseudo-random number in `[0.0, 1.0)`.

```omnilang
import { random, floor } from "std/math";
let dice: i32 = floor(random() * 6.0) + 1;
```

#### `random_int(min: i32, max: i32) -> i32`

Returns a random integer in `[min, max]`.

#### `is_nan(x: f64) -> bool`

Returns true if the value is NaN.

#### `is_infinite(x: f64) -> bool`

Returns true if the value is infinite.

#### `is_finite(x: f64) -> bool`

Returns true if the value is finite.

---

## Module: `std/async`

Async runtime and utilities.

### Functions

#### `sleep(ms: i32) -> void`

Pauses execution for the given number of milliseconds.

```omnilang
import { sleep } from "std/async";

async fn delayed_greeting() {
    await sleep(1000);
    println("Hello after 1 second!");
}
```

#### `spawn(future: fn() -> T) -> Future<T>`

Spawns a task for concurrent execution.

```omnilang
import { spawn } from "std/async";

async fn task(id: i32) -> i32 {
    await sleep(100);
    return id * 2;
}

async fn main() {
    let handle = spawn(|| -> i32 { return await task(1); });
    // Do other work...
    let result: i32 = await handle;
    println(result.to_string());
}
```

#### `all(futures: Array<Future<T>>) -> Future<Array<T>>`

Waits for all futures to complete, returning their results.

```omnilang
import { all } from "std/async";

async fn main() {
    let futures: Array<Future<i32>> = [fetch(1), fetch(2), fetch(3)];
    let results: Array<i32> = await all(futures);
    println(results.len().to_string());
}
```

#### `race(futures: Array<Future<T>>) -> Future<T>`

Returns the result of the first future to complete.

```omnilang
import { race, sleep } from "std/async";

async fn fast() -> String {
    await sleep(100);
    return "fast";
}

async fn slow() -> String {
    await sleep(500);
    return "slow";
}

async fn main() {
    let winner: String = await race([fast(), slow()]);
    println(winner);  // "fast"
}
```

#### `timeout(future: Future<T>, ms: i32) -> Future<T | null>`

Returns `null` if the future doesn't complete within the timeout.

```omnilang
import { timeout, sleep } from "std/async";

async fn slow_op() -> String {
    await sleep(2000);
    return "done";
}

async fn main() {
    let result: String? = await timeout(slow_op(), 1000);
    if result == null {
        println("timed out");
    }
}
```

#### `yield_now() -> void`

Yields execution to allow other tasks to run.

```omnilang
import { yield_now } from "std/async";

async fn cooperative() {
    let mut i: i32 = 0;
    while i < 1000 {
        // Do some work
        i = i + 1;
        if i % 100 == 0 {
            await yield_now();
        }
    }
}
```

### Types

#### `Future<T>`

Represents an asynchronous computation. Created by calling `async fn` or `spawn`.

---

## Module: `std/string`

String manipulation utilities.

### Functions

#### `len(s: String) -> i32`

Returns the length of the string in bytes.

```omnilang
import { len } from "std/string";
let n: i32 = len("Hello");    // 5
```

#### `is_empty(s: String) -> bool`

Returns true if the string is empty.

#### `to_upper(s: String) -> String`

Converts to uppercase.

```omnilang
import { to_upper } from "std/string";
let s: String = to_upper("hello");  // "HELLO"
```

#### `to_lower(s: String) -> String`

Converts to lowercase.

```omnilang
import { to_lower } from "std/string";
let s: String = to_lower("HELLO");  // "hello"
```

#### `trim(s: String) -> String`

Removes leading and trailing whitespace.

```omnilang
import { trim } from "std/string";
let s: String = trim("  hello  ");  // "hello"
```

#### `substring(s: String, start: i32, end: i32) -> String`

Returns a substring from `start` (inclusive) to `end` (exclusive).

```omnilang
import { substring } from "std/string";
let s: String = substring("hello", 1, 4);  // "ell"
```

#### `contains(haystack: String, needle: String) -> bool`

Returns true if `haystack` contains `needle`.

#### `starts_with(s: String, prefix: String) -> bool`

Returns true if the string starts with the given prefix.

#### `ends_with(s: String, suffix: String) -> bool`

Returns true if the string ends with the given suffix.

#### `index_of(haystack: String, needle: String) -> i32`

Returns the index of the first occurrence, or `-1` if not found.

```omnilang
import { index_of } from "std/string";
let idx: i32 = index_of("hello", "ll");  // 2
```

#### `split(s: String, delimiter: String) -> Array<String>`

Splits the string by a delimiter.

```omnilang
import { split } from "std/string";
let parts: Array<String> = split("a,b,c", ",");
// ["a", "b", "c"]
```

#### `replace(s: String, from: String, to: String) -> String`

Replaces all occurrences of a substring.

```omnilang
import { replace } from "std/string";
let s: String = replace("hello world", "world", "there");
// "hello there"
```

#### `join(elements: Array<String>, separator: String) -> String`

Joins an array of strings with a separator.

```omnilang
import { join } from "std/string";
let parts: Array<String> = ["a", "b", "c"];
let s: String = join(parts, ", ");
// "a, b, c"
```

#### `pad_start(s: String, total_width: i32, pad_char: String) -> String`

Pads the string to the specified width by adding `pad_char` at the start.

#### `pad_end(s: String, total_width: i32, pad_char: String) -> String`

Pads the string to the specified width by adding `pad_char` at the end.

#### `reverse(s: String) -> String`

Returns the reversed string.

#### `char_at(s: String, index: i32) -> String`

Returns the character at the given byte index as a string.

#### `to_string(value: i32 | f64 | bool) -> String`

Converts a value to its string representation. Also available as a method:

```omnilang
let s: String = 42.to_string();    // "42"
let s: String = true.to_string();  // "true"
```
