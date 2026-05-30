# Rust Class Inheritance: Boundaries and Patterns

OmniLang compiles class-based OOP to Rust, which has no built-in class inheritance. This document describes the patterns used to bridge this gap, the limitations, and best practices.

## Overview

OmniLang provides class-based OOP syntax that compiles to all four target languages. For Rust, which does not have native classes or inheritance, OmniLang uses code generation patterns to simulate OOP behavior.

### Supported Features

| Feature | Rust Support | Pattern |
|---------|-------------|---------|
| Single inheritance | Partial | Trait + delegation |
| Interface implementation | Yes | Trait implementation |
| Fields (private/public) | Yes | Struct fields |
| Methods | Yes | `impl` blocks |
| Method override | Partial | Trait method override |
| `super` calls | No | — |
| Multiple inheritance | No | Error at compile time |
| Abstract classes | No | Interface-only |

## Rust Mapping Patterns

### Class to Struct + Trait

A simple class:

```omnilang
class Counter {
    value: i32 = 0;

    fn increment(self) -> i32 {
        self.value = self.value + 1;
        return self.value;
    }

    fn get_value(self) -> i32 {
        return self.value;
    }
}
```

Compiles to Rust:

```rust
struct Counter {
    value: i32,
}

impl Counter {
    fn new() -> Self {
        Counter { value: 0 }
    }

    fn increment(&mut self) -> i32 {
        self.value += 1;
        self.value
    }

    fn get_value(&self) -> i32 {
        self.value
    }
}
```

### Inheritance to Trait + Delegation

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

Compiles to Rust using a parent trait:

```rust
trait AnimalTrait {
    fn speak(&self);
    fn name(&self) -> &str;
}

struct Animal {
    name: String,
}

impl AnimalTrait for Animal {
    fn speak(&self) {
        println!("...");
    }
    fn name(&self) -> &str {
        &self.name
    }
}

struct Dog {
    parent: Animal,
}

impl AnimalTrait for Dog {
    fn speak(&self) {
        println!("Woof!");
    }
    fn name(&self) -> &str {
        self.parent.name()
    }
}
```

### Interface to Trait

```omnilang
interface Drawable {
    fn draw() -> void;
    fn get_area() -> f64;
}

class Circle implements Drawable {
    radius: f64 = 1.0;

    fn draw() -> void {
        println("Drawing circle");
    }

    fn get_area() -> f64 {
        return 3.14159 * self.radius * self.radius;
    }
}
```

Compiles to:

```rust
trait Drawable {
    fn draw(&self);
    fn get_area(&self) -> f64;
}

struct Circle {
    radius: f64,
}

impl Drawable for Circle {
    fn draw(&self) {
        println!("Drawing circle");
    }

    fn get_area(&self) -> f64 {
        3.14159 * self.radius * self.radius
    }
}
```

## Inheritance Boundaries

### Single Inheritance Limit

OmniLang enforces single inheritance (only one `extends` clause) across all targets:

```omnilang
// OK: Single inheritance
class Dog extends Animal { }

// Error: Multiple inheritance not allowed
class Dog extends Animal, Mammal { }
```

The static analyzer (`E0401`) rejects multiple inheritance at compile time.

### Interface Multi-Implementation

Multiple `implements` clauses are allowed:

```omnilang
// OK: Multiple interfaces
class File implements Readable, Writable, Closable { }
```

In Rust this generates multiple `impl Trait for Struct` blocks.

### Method Visibility

| Keyword | Rust Mapping | Description |
|---------|-------------|-------------|
| `public` (default) | `pub` | Accessible everywhere |
| `private` | No `pub` / `pub(self)` | Only within the class |

```omnilang
class Account {
    balance: i32 = 0;         // public (default)
    private key: String = ""; // private
}
```

### Static Methods

`static` methods are compiled as associated functions (no `self` parameter):

```omnilang
class MathUtils {
    static fn add(a: i32, b: i32) -> i32 {
        return a + b;
    }
}
```

```rust
struct MathUtils;

impl MathUtils {
    fn add(a: i32, b: i32) -> i32 {
        a + b
    }
}
```

## Patterns for OOP in Rust

### Pattern 1: Composition over Inheritance

Rather than deep inheritance chains, prefer composition:

```omnilang
// Prefer this:
class Engine {
    horsepower: i32 = 100;
}

class Car {
    engine: Engine;          // Composition
    wheels: Array<Wheel>;
}

// Over this:
class Vehicle {
    fn move() -> void { }
}
class Car extends Vehicle {  // Inheritance
    engine: Engine;
}
```

### Pattern 2: Interface-Based Design

Use interfaces for polymorphic behavior:

```omnilang
interface Formatter {
    fn format(data: String) -> String;
}

class JsonFormatter implements Formatter {
    fn format(data: String) -> String {
        return "{ \"data\": \"" + data + "\" }";
    }
}

class XmlFormatter implements Formatter {
    fn format(data: String) -> String {
        return "<data>" + data + "</data>";
    }
}
```

### Pattern 3: Override with Super Call Emulation

In codebases that require calling the parent implementation, use explicit naming:

```omnilang
class Base {
    fn initialize() -> void {
        println("base init");
    }
}

class Derived extends Base {
    @override
    fn initialize() -> void {
        // Call parent explicitly
        Base.initialize();
        println("derived init");
    }
}
```

In Rust this generates a dedicated `Base::initialize()` function call.

## Limitations

### No Dynamic Dispatch (current)

In the current version, method dispatch is static for Rust targets. Dynamic dispatch via `dyn Trait` is planned for a future release.

### No `super` Keyword

The `super` keyword is not supported. Parent implementations can be accessed via `ClassName.method()` syntax.

### No Abstract Classes

Abstract base classes are not supported. Use interfaces instead for defining contracts.

### No Diamond Problem

Since only single inheritance is allowed, the diamond problem does not occur. Multiple interface implementation is safe since interfaces have no state.

## Cross-Language Considerations

| Feature | Rust | Python | JS/TS | Dart |
|---------|------|--------|-------|------|
| Single inheritance | Trait delegation | Native class inheritance | `extends` | Native class |
| Interfaces | Traits | Abstract base classes | Duck typing | Abstract classes |
| Method override | Trait method | `def` override | Prototype chain | `@override` |
| Private fields | Non-pub field | Name mangling | Closure/`#` | `_` prefix |

## Best Practices

1. **Prefer shallow hierarchies:** Maximum 2-3 levels of inheritance
2. **Use interfaces for polymorphism:** Prefer `implements` over `extends`
3. **Composition over inheritance:** Embed objects as fields where possible
4. **Explicit parent calls:** Use `ClassName.method()` syntax for clarity
5. **Avoid state in base classes:** Keep base classes focused on behavior
6. **Mark overrides with `@override`:** Improves code clarity and enables analyzer checks
