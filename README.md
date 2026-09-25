<div align="center">
  <h1>⚡ FastLang</h1>
  <p>A fun, experimental systems programming language written in Rust to explore compiler design, custom memory models, and compile-time execution.</p>

  <a href="https://ko-fi.com/W5V0236W86" target="_blank">
      <img src="https://storage.ko-fi.com/cdn/kofi2.png?v=3" alt="Support me on Ko-fi" height="36" style="border:0px;height:36px;" />
  </a>
</div>

<br />

> [!NOTE]
> **Disclaimer:** FastLang is a personal, educational hobby project created for learning and experimenting with compiler construction. It is **not** intended for production use.

---

## 💡 About the Project

FastLang is a statically typed language designed from scratch in Rust. The primary goal of the project is to experiment with:
- Hand-written recursive descent parsing and lexical scanning.
- Robust semantic analysis with multi-pass symbol tables and dependency resolution.
- Scoped Enums, Algebraic Data Types with Struct & Tuple payloads, and Variant Proxies.
- Standard Library `Result<T>` and `Option<T>` with implicit prelude exports via `std`.
- Rust-style implicit returns in functions and block expressions.
- Ergonomic `?` inline error handler operator with zero-overhead exception propagation.
- Ternary conditional expressions (`cond ? then : else`) as first-class syntactic sugar.
- Zero-overhead runtime panic safety shield with clean diagnostics.
- Explicit memory semantics (`ref<T>`, `mutRef<T>`, `copy<T>`).
- Compile-time reflection and validation blocks (`@compile { ... }`, `typeof`, `sizeof`).
- Code generation targeting modern C++20 (with experimental Cranelift AOT support).

Source files use the **`.fast`** extension.

---

## 🔍 Code Examples

### 1. Variables & Basic Functions
```rust
fn add(a: int32, b: int32) -> int32 {
    return a + b;
}

fn main() -> int32 {
    int32 x = 10;
    int32 y = 20;
    int32 sum = add(x, y);
    println("Sum: ", sum);
    return 0;
}
```

### 2. Blueprints & Objects
```rust
object Point = {
    int32 x = 0;
    int32 y = 0;
};

impl Point {
    fn display() -> void {
        println("Point(", this.x, ", ", this.y, ")");
    }
}

fn main() -> int32 {
    auto pt = Point();
    pt.x = 10;
    pt.y = 20;
    pt.display();
    return 0;
}
```

### 3. Super-Types & Inference
```rust
// Numbers with auto-inferred bitwidths
number count = 42;       // Inferred as number::int32
number pi = 3.14159;     // Inferred as number::float64

// Functions as first-class citizens
function greet() -> void {
    println("Hello, FastLang!");
}

// Namespaces
object MathUtils :: {
    fn square(n: int32) -> int32 {
        return n * n;
    }
}

fn main() -> int32 {
    greet();
    println("Square of 5: ", MathUtils::square(5));
    return 0;
}
```

### 4. Compile-time Introspections & Return Types
```rust
object User = {
    int32 id = 1;
    array<char> name = "Hakim";
};

// Return type derived from member access at compile-time
fn get_user_id(u: User) -> typeof(u).id {
    return u.id;
}

fn main() -> int32 {
    auto u = User();
    println("User ID: ", get_user_id(u));
    return 0;
}
```

---

## 🚀 Building & Running

### Prerequisites
- [Rust](https://rustup.rs/) (edition 2021)
- C++20 compiler (`g++` or `clang++`)

### Build FastLang
```bash
cargo build --release
```

### Run Tests
```bash
# Run all integration test suites
cargo test run_all_fs_tests

# Test error accumulation diagnostics
cargo test test_debug_error_accumulation
```

### Compile a FastLang Program
```bash
# Compile and run
cargo run -- main.fast

# Compile with a specific output binary
cargo run -- main.fast -o build/app.exe

# Diagnostics mode: collect all semantic errors without compiling
cargo run -- main.fast --debug-error

# Emit generated C++ code without invoking backend compiler
cargo run -- main.fast --emit-cpp
```

---

## 🛠️ CLI Options

```text
FastLang Compiler (fast_lang) v0.1.5-stable
High-performance compiled language with fine-grained memory management and explicit scopes.

USAGE:
    fast_lang [OPTIONS] <SOURCE_FILE>

ARGUMENTS:
    <SOURCE_FILE>                  Path to the main entry source file (.fast or .fs)

OPTIONS:
    -h, --help, -help              Print help information and exit
    -d, --debug                    Enable verbose debug and compilation trace output
    --debug-error, --debug-errors  Accumulate and report all semantic errors without generating code
    -o, --output <PATH>            Specify output binary or build directory path
    -I, --include <DIR>            Add module search directory
    -b, --backend <BACKEND>        Set code generator backend: 'cpp' (default) or 'cranelift'
    --ast, --emit-ast [FILE]       Dump parsed AST as formatted JSON
    --ast-file, --json-ast <FILE>  Export parsed AST directly to FILE
    --emit-cpp, --cpp-only         Generate C++ source file without compiling binary
    --emit-ir, --print-ir          Print intermediate representation (IR) output
    --aot                          Enable Ahead-Of-Time (AOT) compilation
    --clean                        Remove temporary build files

SUBCOMMANDS:
    clean [PATH]                   Delete build/ directory and cached artifacts
```

---

<div align="center">
  <a href="https://ko-fi.com/W5V0236W86" target="_blank">
      <img src="https://storage.ko-fi.com/cdn/kofi2.png?v=3" alt="Support me on Ko-fi" height="36" style="border:0px;height:36px;" />
  </a>
</div>
