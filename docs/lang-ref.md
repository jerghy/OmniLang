# OmniLang Language Reference

OmniLang is a cross-platform programming language that transpiles to Rust, Dart, JavaScript, and Python. It combines Rust-like syntax and ownership semantics with high-level expressiveness.

## Syntax

### Comments

```omnilang
// Single-line comment
/* Multi-line
   comment */
```

### Variables and Mutability

Variables are immutable by default. Use `mut` to make them mutable.

```omnilang
let x: i32 = 42;          // Immutable variable
let mut y: i32 = 10;      // Mutable variable
y = y + 1;                // Mutation allowed
```

### Primitive Types

| Type | Description | Example |
|------|-------------|---------|
| `i32` | 32-bit signed integer | `42`, `-1`, `0xFF` |
| `f64` | 64-bit floating point | `3.14`, `1.0e-5` |
| `bool` | Boolean | `true`, `false` |
| `String` | UTF-8 string | `"hello"` |
| `void` | Unit / no value | — |
| `null` | Null value | — |

### Generic Types

```omnilang
Array<i32>           // Array of integers
Map<String, i32>     // Map from string to integer
Array<Array<bool>>   // Nested arrays
```

### Union Types

```omnilang
let x: i32 | String = 42;     // Union of types
let y: i32? = null;           // Optional type (T | null)
```

### Operators

**Arithmetic:** `+`, `-`, `*`, `/`, `%`

**Comparison:** `==`, `!=`, `<`, `>`, `<=`, `>=`

**Logical:** `&&`, `||`, `!`

**Assignment:** `=`, `+=`, `-=`, `*=`, `/=`, `%=`

**Null coalescing:** `??`

**Spread:** `...`

### Strings

```omnilang
let s: String = "hello";
let len: i32 = s.len();
let upper: String = s.to_upper();
let combined: String = s + " world";
```

### Template Strings

Backtick-delimited strings with `${expr}` interpolation:

```omnilang
let name: String = "World";
let msg: String = `Hello, ${name}!`;
// "Hello, World!"
```

### Arrays

```omnilang
let arr: Array<i32> = [1, 2, 3];
arr.push(4);
let first: i32 = arr.get(0);
let len: i32 = arr.len();
let popped: i32 = arr.pop();
```

### Maps (Dictionaries)

```omnilang
let m: Map<String, i32> = {"a": 1, "b": 2};
m.insert("c", 3);
let v: i32 = m.get("a");
let exists: bool = m.contains("b");
let len: i32 = m.len();
m["key"] = value;        // Indexed assignment
```

### Functions

```omnilang
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// Implicit return (tail expression)
fn double(x: i32) -> i32 {
    x * 2
}

// No return value
fn greet(name: String) {
    println("Hello, " + name);
}
```

### Async Functions

```omnilang
async fn fetch(url: String) -> String {
    return "data";
}

async fn main() {
    let result: String = await fetch("http://example.com");
    println(result);
}
```

### Control Flow

#### If/Else

```omnilang
if x > 5 {
    println("big");
} else if x == 0 {
    println("zero");
} else {
    println("small");
}
```

#### While Loops

```omnilang
let mut i: i32 = 0;
while i < 5 {
    println(i.to_string());
    i = i + 1;
}
```

#### For-In Loops

```omnilang
for item in items {
    println(item);
}

// With index
for (i, item) in items.enumerate() {
    println(item);
}
```

#### Break and Continue

```omnilang
while true {
    if done { break; }
    if skip { continue; }
    process();
}
```

### Match Expressions

```omnilang
let result: String = match value {
    1 => "one",
    2 => "two",
    _ => "other",
};

// With blocks
match value {
    1 => { println("one"); }
    2 => { println("two"); }
    _ => { println("other"); }
}
```

Patterns support:
- Literal patterns: `1`, `"hello"`, `true`
- Variable bindings: `x`
- Wildcard: `_`
- Struct patterns: `Some { value }`
- Or-patterns: `1 | 2`

### Closures

```omnilang
// Pipe syntax closure
let add_one = |x: i32| -> i32 { x + 1 };

// No-param closure
let get_value = || -> i32 { 42 };

// Arrow function syntax
let multiply = (a: i32, b: i32) => a * b;
```

### Structs

```omnilang
struct Point {
    x: i32;
    y: i32;
}

// Struct literal
let p: Point = Point { x: 10, y: 20 };
println(p.x.to_string());
```

#### Nominal Structs

```omnilang
nominal struct UserId {
    value: i32;
}
```

#### Impl Blocks

```omnilang
struct Vec2 {
    x: f64;
    y: f64;
}

impl Vec2 {
    fn magnitude(self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}
```

### Tagged Unions (Enums)

```omnilang
type Option<T> = Some { value: T } | None
type Result<T, E> = Ok { value: T } | Err { error: E }
```

### Classes

```omnilang
class Animal {
    name: String = "";
    fn speak(self) -> void {
        println("...");
    }
}

class Dog extends Animal {
    @override
    fn speak(self) -> void {
        println("Woof!");
    }
}
```

### Interfaces

```omnilang
interface Drawable {
    fn draw() -> void;
    fn get_area() -> f64;
}

class Circle implements Drawable {
    radius: f64 = 1.0;
    fn draw() -> void { /* ... */ }
    fn get_area() -> f64 { return 3.14 * radius * radius; }
}
```

### Destructuring

```omnilang
// Array destructuring
let [a, b, ...rest]: [i32, i32, Array<i32>] = [1, 2, 3, 4];

// Object/struct destructuring
let { x, y } = point;
```

### Optional Chaining

```omnilang
let value: i32? = obj?.field;
let result: String? = obj?.method();
```

### Spread Expressions

```omnilang
let combined: Array<i32> = [...arr1, ...arr2];
```

### If Expressions (Ternary)

```omnilang
let max: i32 = if a > b { a } else { b };
```

### Modules and Imports

```omnilang
import { greet, farewell } from "./utils";
import type { MyType } from "./types";
export fn helper() -> void { /* ... */ }
```

## Ownership System

OmniLang uses a simplified ownership model inspired by Rust:

- **Move semantics:** Assigning a value transfers ownership
- **No references/borrowing** in v0.2 (planned for future)
- Values are moved, not copied (except primitive types like `i32`, `f64`, `bool`)

```omnilang
let s1: String = "hello";
let s2: String = s1;    // s1 is moved to s2
// println(s1);          // Error: use after move
```

## Print Functions

```omnilang
println("Hello");        // Print with newline
print("Hello");          // Print without newline
```

## Code Generation Targets

| Target | Extension | Status |
|--------|-----------|--------|
| Rust | `.rs` | Full support |
| Python | `.py` | Full support |
| JavaScript | `.js` | Full support |
| Dart | `.dart` | Full support |

## Toolchain

```bash
omnic build file.omni              # Compile to all targets
omnic build file.omni -t python    # Compile to specific target
omnic check file.omni              # Static analysis
omnic run file.omni -t python      # Compile and run
omnic fmt file.omni                # Format source
omnic init ./my_project            # Initialize project
```
