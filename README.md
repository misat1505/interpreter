# Raptor

Raptor is a custom interpreted and compiled programming language written in Rust.

It is a strongly and statically typed language with mutable variables, scoped execution, functions, references, structs, multidimensional vectors, type conversions, and structured control flow.

The project provides **two executables**:

* `raptor` — the Raptor compiler and interpreter
* `lsp` — the Language Server Protocol (LSP) server for editor integration

## Table of contents

* [Language pipeline](#language-pipeline)
* [Executables](#executables)
* [Documentation](#documentation)
* [Quick start](#quick-start)
* [CLI options](#cli-options)
* [Example program](#example-program)
* [Language overview](#language-overview)
* [Errors and diagnostics](#errors-and-diagnostics)
* [Testing](#testing)
* [LLVM](#llvm)
* [Cargo targets](#cargo-targets)

## Language pipeline

![](./docs/diagram/compiler_pipeline.png)

The semantic checker performs **type checking** and other static validation before the program is interpreted or compiled, unless `--unsafe` is explicitly used.

## Executables

### `raptor`

The main Raptor command-line tool. It provides both the interpreter and the compiler, and can:

* interpret Raptor source files;
* compile Raptor programs to native executables;
* compile and immediately run programs;
* control compiler optimization levels;
* optionally skip semantic checking with `--unsafe`.

### `lsp`

The Raptor Language Server Protocol implementation. It provides language-server functionality for editors and IDEs that support LSP, allowing Raptor source files to be integrated with development environments.

The LSP server is a separate executable from the `raptor` compiler/interpreter.

## Documentation

| Component          | Documentation                                           |
| ------------------ | -------------------------------------------------------- |
| Grammar             | [docs/grammar.md](docs/grammar.md)                       |
| Lexer               | [docs/lexer.md](docs/lexer.md)                           |
| Parser              | [docs/parser.md](docs/parser.md)                         |
| Semantic Checker    | [docs/semantic-checker.md](docs/semantic-checker.md)     |
| Interpreter         | [docs/interpreter.md](docs/interpreter.md)               |
| Compiler            | [docs/compiler.md](docs/compiler.md)                     |
| Memory Management   | [docs/memory-management.md](docs/memory-management.md)   |

## Quick start

### Build

The project defines two executable targets, `raptor` and `lsp`, plus the shared `raptor_lib` library crate they're both built on.

```bash
# Build everything
cargo build --release

# Build only the compiler/interpreter
cargo build --release --bin raptor

# Build only the LSP server
cargo build --release --bin lsp
```

The resulting executables are written to `target/release/raptor` and `target/release/lsp`.

### Run a Raptor program

```bash
# Interpret (default behavior)
./target/release/raptor examples/basic.rp

# Compile to a native executable (written to build/)
./target/release/raptor --compile examples/basic.rp

# Compile and immediately run the result
./target/release/raptor --run examples/basic.rp
# equivalent to:
./target/release/raptor --compile --run examples/basic.rp
```

### Development

During development, `cargo run` builds and runs directly from source, without a separate release build:

```bash
cargo run --bin raptor -- examples/basic.rp             # interpret
cargo run --bin raptor -- --compile examples/basic.rp    # compile
cargo run --bin raptor -- --run examples/basic.rp         # compile and run
cargo run --bin lsp                                        # LSP server
```

For everyday local use, prefer building once in release mode and invoking the resulting binaries directly, as shown above — debug builds of the compiler are noticeably slower.

## CLI options

The `raptor` executable supports the following options:

```text
-h, --help          Show this help message
-v, --verbose       Show execution time of each phase
--unsafe            Skip semantic checking
--compile           Compile the source file instead of interpreting it
--run               After compiling, build and run the resulting executable (implies --compile)
-o <FILE>           Set output executable path
--link <FILE>       Link an additional object file (compilation mode only)
-O0                 No optimization (default)
-O1                 Basic optimization
-O2                 Default optimization
-O3                 Aggressive optimization
--overflow <POLICY> Integer overflow policy: ignore, warn, error
```

The compiler writes generated artifacts to `build/`.

The `lsp` executable is a separate LSP server and does not use the `raptor` command-line interface described above.

## Example program

The following program demonstrates several core Raptor features: variables, functions, references, loops, conditionals, vectors, and static typing.

```text
fn sum(i64[] values): i64 {
    i64 total = 0;

    for (i64 i = 0; i < vector_size(&values); i += 1) {
        total = total + values[i];
    }

    return total;
}

fn max(i64[] values): i64 {
    i64 result = values[0];

    for (i64 i = 1; i < vector_size(&values); i += 1) {
        if (values[i] > result) result = values[i];
    }

    return result;
}

fn average(i64[] values): f64 {
    return sum(values) as f64 / vector_size(&values) as f64;
}

fn add_bonus(&i64 score, i64 bonus): void {
    score = score + bonus;

    if (score > 100) score = 100;
}

fn main(): void {
    i64[] scores = [72, 85, 91, 68, 94];

    i64 total = sum(scores);
    i64 best = max(scores);
    f64 avg = average(scores);

    println("Results:");
    println("---------");

    print("Total: ");
    println(total as str);

    print("Best: ");
    println(best as str);

    print("Average: ");
    println(avg as str);

    i64 final_score = best;
    add_bonus(&final_score, 5);

    print("Final score: ");
    println(final_score as str);

    if (final_score >= 90) {
        println("Status: excellent");
    } else if (final_score >= 75) {
        println("Status: good");
    } else {
        println("Status: needs improvement");
    }
}

main();
```

Save the program as `examples/demo.rp`, then run it with any of:

```bash
./target/release/raptor examples/demo.rp             # interpret
./target/release/raptor --compile examples/demo.rp    # compile
./target/release/raptor --run examples/demo.rp         # compile and run
```

## Language overview

Raptor currently supports:

* `i64`, `f64`, `str`, `bool`, and `void`;
* mutable variables with block-based scoping;
* functions and recursion;
* parameters passed by value or by reference;
* structs, including fields of composite (`str`, vector, struct) type;
* `if`, `for`, `while`, and `switch`;
* `break`, `continue`, and `return`;
* arithmetic, comparison, and logical operators;
* explicit casts with `as`;
* vectors, including multidimensional types such as `i64[][]`;
* built-in functions such as `print`, `input`, and `mod`.

### Vectors

Vector types may have multiple dimensions:

```text
i64[]       # one-dimensional vector
i64[][]     # two-dimensional vector
i64[][][]   # three-dimensional vector
```

When a vector is passed **by value**, the language uses a **shallow copy**: the vector's own structure is copied, while any composite elements it contains are shared, not recursively deep-copied.

### Memory management

`str`, vectors, and structs are heap-allocated and managed automatically through reference counting — there is no manual `free`/`delete` and no garbage collector pause. Assigning or passing these types follows consistent value/reference rules (e.g. strings are always deep-copied, vectors and structs are shared or shallow-copied depending on context). See [docs/memory-management.md](docs/memory-management.md) for the full model, including its current known limitations (reference cycles are not collected).

## Errors and diagnostics

Raptor diagnostics use a compact compiler-style format:

```text
error: <message>
  --> <file>:<line>:<column>
```

Different pipeline stages report different classes of errors:

* the lexer reports malformed lexical input;
* the parser reports syntax errors;
* the semantic checker performs static type checking and related validation;
* the interpreter reports runtime errors;
* the compiler reports code-generation and compilation errors.

See the individual component documentation for examples and details.

## Testing

Run the complete test suite with:

```bash
cargo test
```

The project includes unit tests for core components and integration tests for the language pipeline.

## LLVM

The native compilation pipeline currently targets **LLVM 18** and invokes `llc-18` and `clang-18`. These tools must be available on `PATH` when using `--compile` or `--run`.

The LLVM toolchain is only required for native compilation — running a program through the interpreter does not require it.

## Cargo targets

```toml
[lib]
name = "raptor_lib"
path = "src/lib.rs"

[[bin]]
name = "raptor"
path = "src/main.rs"

[[bin]]
name = "lsp"
path = "src/bin/lsp.rs"
```

```text
┌─────────────────────┐   ┌─────────────────────┐   ┌─────────────────────┐
│        raptor       │   │          lsp        │   │      raptor_lib     │
│                     │   │                     │   │                     │
│  Interpreter        │   │  Language Server    │   │  Shared library     │
│  Compiler           │   │  Protocol (LSP)     │   │  used by both       │
│  CLI                │   │                     │   │  executables        │
└─────────────────────┘   └─────────────────────┘   └─────────────────────┘
```