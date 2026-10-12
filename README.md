<div align="center">
  <h1>⚡ FastLang</h1>
  <p>A statically typed, high-performance systems language with native reactive web primitives and zero-overhead compilation.</p>

  <a href="https://ko-fi.com/W5V0236W86" target="_blank">
      <img src="https://storage.ko-fi.com/cdn/kofi2.png?v=3" alt="Support me on Ko-fi" height="36" style="border:0px;height:36px;" />
  </a>
</div>

<br />

> [!NOTE]
> FastLang is an open, experimental systems programming language developed from scratch in Rust, designed to explore compiler architectures, ergonomic type systems, and native reactive document runtimes.

---

## 🚀 Key Highlights

- **Modern Statically Typed Syntax**: Expressive type inference, first-class functions, pipelines, and explicit memory semantics (`ref<T>`, `mutRef<T>`).
- **Blueprints & Object Orientation**: Data-first structs (`blueprint`), encapsulation, and decoupled method definitions (`impl`).
- **Algebraic Data Types & Matching**: Exhaustive pattern matching on tagged enums, `Option<T>`, and `Result<T>`.
- **First-Class Reactive Web & Foreign Scopes**: Native `@html`, `@json`, and `@safe` blocks with `@f{...}` interpolation, paired with FastReact declarative directives (`onclick`, `onsubmit`, `live`, `target`, `swap`).
- **Comprehensive Standard Library**: Native support for threads, synchronization mutexes, network sockets, HTTP servers, collections (`list`, `HashSet`, `HashMap`), and embedded SQLite (`std::sql`).
- **Zero-Config Project Tooling**: Built-in CLI workflow with `fast new`, `fast run`, `fast build`, and `fast test` managed via `Fast.toml`.
- **High-Performance Backend**: Emits optimized C++20 code, compiled directly with native toolchains.

---

## 🔍 Language Tour

### 1. Functions & Control Flow
```rust
import std::fmt;

fn fibonacci(n: int32) -> int32 {
    if (n <= 1) {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn main() -> void {
    int32 result = fibonacci(10);
    fmt::println("Fibonacci(10) = " + result);
}
```

### 2. Blueprints & Methods
```rust
import std::fmt;

blueprint User -> {
    int32 id;
    str name;
    bool active = true;
}

impl User -> {
    fn display() -> void {
        fmt::println("User #" + this.id + ": " + this.name);
    }
}

fn main() -> void {
    User u = new User(1, "Hakim");
    u.display();
}
```

### 3. Pattern Matching & Enums
```rust
import std::fmt;

enum Status -> {
    Pending,
    Active(int32),
    Completed(str)
}

fn handle_status(s: Status) -> void {
    match (s) -> {
        Pending => fmt::println("Waiting..."),
        Active(id) => fmt::println("In progress with task: " + id),
        Completed(msg) => fmt::println("Done: " + msg),
        _ => fmt::println("Unknown state")
    }
}
```

### 4. Native Reactive Web & FastReact
```rust
import std::http;

blueprint ItemCard -> {
    str name = "Calculator";
    int32 price = 350;
}

impl ItemCard -> {
    fn render() -> HtmlDocument {
        return @html {
            <div id="card" class="item">
                <h3>@f{this.name}</h3>
                <p>Price: @f{this.price} EGP</p>
                <button onclick="/buy" target="#card" swap="outerHTML">
                    Purchase
                </button>
            </div>
        };
    }
}
```

### 5. Embedded SQLite Database
```rust
import std::fmt;
import std::sql;

fn query_users() -> void {
    db := sql::open("data.db");
    db.exec("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT);");
    db.exec("INSERT INTO users (name) VALUES ('Hakim');");

    stmt := db.prepare("SELECT id, name FROM users;");
    while (stmt.step()) -> {
        int32 id = stmt.get_int(0);
        str name = stmt.get_str(1);
        fmt::println("User: " + id + " - " + name);
    }
    stmt.finalize();
    db.close();
}
```

---

## 🛠️ Quick Start

### Requirements
- [Rust Toolchain](https://rustup.rs/) (edition 2021+)
- Modern C++20 compiler (`g++` or `clang++`)

### Installation & Build
```bash
# Clone the repository
git clone https://github.com/Trezone/fast_lang.git
cd fast_lang

# Build release binary
cargo build --release
```

### Project Workflow
```bash
# Create a new FastLang binary project
fast new my_app -y
cd my_app

# Run the project
fast run

# Run tests
fast test
```

---

<div align="center">
  <p>Crafted with passion for fast, expressive systems programming.</p>

  <a href="https://ko-fi.com/W5V0236W86" target="_blank">
      <img src="https://storage.ko-fi.com/cdn/kofi2.png?v=3" alt="Support me on Ko-fi" height="36" style="border:0px;height:36px;" />
  </a>
</div>
