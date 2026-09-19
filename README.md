<div align="center">
  <h1>⚡ FastLang</h1>
  <p>A modern, high-performance, statically typed systems programming language with fine-grained memory management and explicit scope engines.</p>

  <a href="https://ko-fi.com/W5V0236W86" target="_blank">
      <img src="https://storage.ko-fi.com/cdn/kofi2.png?v=3" alt="Support me on Ko-fi" height="36" style="border:0px;height:36px;" />
  </a>
</div>

<br />

## 🌟 What is FastLang?

FastLang is a compiled, statically typed language designed with an emphasis on **explicit control**, **zero runtime ambiguity**, and **raw execution speed**. It eliminates unpredictable hidden overhead while providing powerful modern abstractions like Custom Scopes, Smart Pointer semantics (`ref<T>`, `mutRef<T>`, `copy<T>`), Pattern Matching, and Built-in Macros (`default()`).

Source files use the official **`.fast`** file extension.

---

## 🚀 Key Language Features

- **Direct Type Bitwidth Syntax**: `int32`, `int64`, `int16`, `int8`, `float32`, `float64`, or generic parameterized forms `int<32>`, `float<64>`.
- **Zero-Arrow Scope Declarations**: Clean struct, class, enum, and custom scope definitions (`struct Point { ... }`, `class Node { ... }`, `enum Status { ... }`).
- **Unified `using` System**: Instant namespace imports for enum variants (`using Status;`), static class methods, and zero-parameter usable micros.
- **Explicit Memory Model**:
  - `ref<T>`: Immutable tracked pointer reference / borrow.
  - `mutRef<...T>`: Mutable pointer borrow with reference counting and capability tracking.
  - `copy<T>`: Strict deep value snapshot.
- **Algebraic Data Types & Pattern Matching**: Enums with tuple payloads and exhaustive `match` branches.
- **Custom Scopes & Operator Overloading**: Extensible scopes with lifecycle handles (`add`, `sub`, `mul`, `display`, `default`, etc.).
- **Built-in `default()` Macro**: Universal clean zero-initialization or user-overridden handle default values.

---

## 📖 Syntax & Examples

### 1. Functions & Variables
```rust
fn add(a: int32, b: int32) -> int32 {
    int32 result = a + b;
    return result;
}

fn main() -> int32 {
    int32 x = 10;
    int32 y = 20;
    int32 z = add(x, y);
    log("Result: ", z);
    return 0;
}
```

### 2. Enums, Using, and Pattern Matching
```rust
enum Status {
    Idle,
    Running(int32),
    Success(string),
}

fn main() -> int32 {
    using Status;

    Status state = Running(75);

    match (state) -> {
        Idle => {
            log("Waiting...");
        }
        Running(progress) => {
            log("Progress: ", progress, "%");
        }
        Success(msg) => {
            log("Done: ", msg);
        }
    }
    return 0;
}
```

### 3. Custom Scopes & Operator Overloading
```rust
custom Vector2D {
    enable [public, handle, static];

    public {
        int32 x = 0;
        int32 y = 0;
    }

    constructor {
        init(x: int32, y: int32) -> {
            this.x = x;
            this.y = y;
        }
    }

    handle -> {
        fn add(other: Vector2D) -> Vector2D {
            return new Vector2D(this.x + other.x, this.y + other.y);
        }

        fn display() -> void {
            log("Vector2D(", this.x, ", ", this.y, ")");
        }
    }
}

fn main() -> int32 {
    Vector2D v1 = new Vector2D(10, 20);
    Vector2D v2 = new Vector2D(5, 15);
    Vector2D sum = v1 + v2;
    sum.display();
    return 0;
}
```

---

## 🛠️ The Compiler Architecture

The FastLang compiler is written in Rust:

- **Frontend**: High-speed Lexer and Recursive-Descent Parser producing a strict AST.
- **Middle-End**: Robust Semantic Analyzer, Spatial Control Flow Analyzer, Scope Environment Table, and Type Checker.
- **Backend (C++20)**: High-performance code generation yielding native C++ binaries with zero memory leaks.

---

## 🧪 Testing & Execution

Run the complete test suite:
```bash
cargo test --test integration_test
```

Run a specific `.fast` script:
```bash
cargo run -- tests/01_variables.fast
```

---

## 🗺️ Roadmap & To-Do List

### ✅ Phase 1: Core Engine & Type System (Completed)
- [x] **Strict Type Bitwidth**: Direct `int32`, `int64`, `int16`, `int8`, `float32`, `float64` and generic parameterized forms `int<32>`.
- [x] **Zero-Arrow Clean Declarations**: Removed mandatory `->` on `struct`, `class`, `blueprint`, `custom`, `enum`, and visibility blocks (`public`, `private`, `static`).
- [x] **Standard `.fast` Extension**: Fully migrated all 47 tests and standard library modules to `.fast`.
- [x] **Unified `using` System**: Instant namespace imports for enums (`using Status;`), static scope methods, and usable zero-param micros.
- [x] **Eradication of `null`**: Universal zero-cost tag structs (`fastlang_tag_Enum_Variant`) for pure typed variants.
- [x] **Universal `default()` Macro**: Zero-initialization & user-overridden `handle -> { fn default() -> T { ... } }`.
- [x] **100% English Codebase**: Completely cleaned and translated all internal source code doc-comments to English.
- [x] **Editor Ecosystem**: Built-in VS Code and Antigravity IDE TextMate syntax highlighter extension.

### ✅ Phase 2: Function Pointers & Memory Unification (Completed)
- [x] **Unified `name<T>` Pointer System**:
  - Replaced legacy `scope<T>` wrappers with universal `name<Fn<(Args), Ret>>` and `name<method>`.
  - Type-inferred function and method references.
- [x] **First-Class Lambdas & Anonymous Functions**:
  - Support inline anonymous functions: `fn _(x: int32, y: int32) -> int32 { return x + y; }`.
  - Seamless passing to Higher-Order Functions and `set` reassignment.

### ⏳ Phase 3: Error System & Control Flow Safety
- [ ] **First-Class `Error` Class & `Result<T, E>` ADT**:
  - Built-in typed errors with stack trace and error codes.
  - Ergonomic `?` try operator or explicit matching.
- [ ] **Total Analyzer Strictness**:
  - Full eradication of `auto` / loose type inferences in the middle-end analyzer.
  - Exhaustive control flow validation across all branch combinations.

### ⏳ Phase 4: Standard Library & Tooling
- [ ] Collections (`list<T>`, `map<K, V>`, `set<T>`, `string` methods).
- [ ] Asynchronous Task & Coroutine runtime scheduler.
- [ ] FastLang CLI Package Manager & Formatter.

<br />

<div align="center">
  <a href="https://ko-fi.com/W5V0236W86" target="_blank">
      <img src="https://storage.ko-fi.com/cdn/kofi2.png?v=3" alt="Support me on Ko-fi" height="36" style="border:0px;height:36px;" />
  </a>
  <br /><br />
  <sub>Built with ❤️ by TreZone1t.</sub>
</div>
