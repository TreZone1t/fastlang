<div align="center">
  <h1> FastLang</h1>
  <p>A modern, fast, and highly customizable programming language built for performance and ultimate control over data and scopes.</p>

  <a href="https://ko-fi.com/W5V0236W86" target="_blank">
      <img src="https://storage.ko-fi.com/cdn/kofi2.png?v=3" alt="Support me on Ko-fi" height="36" style="border:0px;height:36px;" />
  </a>
</div>

<br />

## What is FastLang?

FastLang is a compiled, statically typed programming language designed with an emphasis on **explicit control**, **modular scope management**, and **raw execution speed**. It blends the familiarity of C-like syntax with advanced meta-programming capabilities like Custom Scopes and explicit type definitions.

### Language Features

- **Strict Explicit Typing**: Define types precisely, e.g., `int(32)`, `int(64)`, `float(32)`.
- **Advanced Scope Engine**: Use the `scope` keyword to create powerful, customizable structures (`custom`, `object`, `enum`) with rich settings and flags.
- **Access Control**: Fine-grained access modifiers (`private`, `public`, `restricted`) natively integrated into scopes.
- **Built-in Handles**: Handle specific lifecycle events like `Error`, `Copy`, `Destruct` directly in your scope declarations.

##  Syntax & Examples

### 1. Functions & Basic Types
FastLang embraces explicit typing for safety and clarity:

```rust
fn my_add(a: int(32), b: int(32)) -> int(32) {
    int(32) result = a + b;
    return result;
}

fn main() -> void {
    int(32) x = 10;
    int(32) y = 20;
    int(32) z = my_add(x, y);
}
```

### 2. Custom Scopes (Advanced Objects)
FastLang allows you to build custom types with extreme precision using the `scope` syntax. You can enable specific capabilities and handle initialization:

```rust
custom DataBuffer -> {
    enable [index_access, length, private, public, constructor];
    private -> {
        int(32) size;
        int(32) capacity;
    }

    public -> {
        fn get_size() -> int(32) {
            return this.size;
        }
    }

    handle -> {
        constructor() {
            this.size = 0;
            this.capacity = 16;
        }
    }
}
```

---

## 🛠️ The Compiler

Behind the language is a cutting-edge compiler built with Rust. It provides massive flexibility whether you want to run code on the fly or generate native machine code.

- **Frontend**: Custom Lexer and Parser generating a strict AST.
- **Middle-End**: Robust Semantic Analyzer and Environment resolver lowering to a Custom IR.
- **Backend (Cranelift)**: Directly lowers Custom IR to Cranelift CLIF for fast code generation.
- **Execution Modes**:
  - **JIT**: Executes directly in memory.
  - **AOT**: Compiles down to native Object (`.o`) files.
  - **Transpiler**: Experimental C++ source code generation.

## 🚀 Quick Start

### Build the Compiler
```bash
cargo build --release
```

### Run a Script (JIT)
Execute FastLang code directly in memory using the Cranelift JIT engine:
```bash
cargo run -- -b cranelift tests/ir_test.fs
```

### Compile Ahead-of-Time (AOT)
Generate a native object file (`.o`) in the `build/` folder next to your script:
```bash
cargo run -- -b cranelift --aot tests/ir_test.fs
```

### View Custom IR
To preview the generated Intermediate Representation before lowering to Cranelift:
```bash
cargo run -- -b cranelift --emit-ir tests/ir_test.fs
```

## 🗺️ Roadmap
- [x] Custom IR Lowering to Cranelift CLIF
- [x] JIT Execution Support
- [x] AOT Object Generation
- [ ] Middle-End Scope & Array Lowering
- [ ] Standard Library (std) Integration
- [ ] Advanced Optimizations Pass

---

<div align="center">
  <sub>Built with ❤️ by TreZone1t.</sub>
</div>
