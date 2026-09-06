# Memory Management

This document describes the memory management model used by the LLVM compiler backend: automatic reference counting (refcounting) for composite types and strings, ownership rules when passing values around, and known limitations.

## 1. Primitives vs. heap

- **Primitives** (`i8`/`i16`/`i32`/`i64`, `u8`/`u16`/`u32`/`u64`, `f64`, `bool`, `char`) live entirely on the stack / in registers. Copying them is a plain `store`, with no bookkeeping at all.
- **`str`, `T[]` (vector), struct** always live on the heap and are managed by the refcounting scheme described below.

## 2. Heap object layout

Every managed object has a header with a reference count `refcount: i64`.

### `StrHeader`
```
struct StrHeader {
    i64 refcount;
    i8* data;   // NUL-terminated character buffer, separate malloc
}
```
`LlvmValue::Str` **always** points at a `StrHeader`, never directly at the character buffer. Access to the characters always goes through the `data` field (see `Compiler::str_data_ptr`).

### `VecHeader`
```
struct VecHeader {
    i64 refcount;
    T*  data;       // backing array, separate malloc
    i64 length;
    i64 capacity;
}
```

### User struct
The declared fields plus one extra, hidden field appended at the end:
```
struct <Name> {
    <fields declared by the user>;
    i64 refcount;   // appended automatically by struct_llvm_type
}
```

## 3. Value vs. reference semantics

| Type | `let b = a;` | argument by value | argument by `&` (reference) |
|---|---|---|---|
| primitive | bitwise copy | bitwise copy | pointer to the caller's variable |
| `str` | **full, deep copy** of a new buffer | **full, deep copy** | pointer to the caller's `StrHeader` |
| `T[]` | retain (shared reference) | **shallow copy**: new `VecHeader` + new backing array; composite elements (`str`/`T[]`/struct) are *retained*, not deep-copied | pointer to the caller's `VecHeader` |
| `struct` | retain (shared reference) | **shallow copy**: new struct, primitive fields copied bitwise, composite fields *retained* | pointer to the caller's struct |

Strings **always** get full value semantics - never shared, even though they're refcounted under the hood. This is intentional: the refcount on a string exists mainly so releasing goes through one consistent mechanism (`release_value`), not so strings can be shared.

## 4. Retain / release - when they happen

### Base rule: "new reference +1"
Any expression whose result is **anything other than a bare variable read** (`Expression::Variable`) returns an owned (+1) reference by construction:
- a fresh literal (`[1,2,3]`, `Struct { ... }`),
- a function call result,
- a struct field read (`obj.field`) or vector element read (`arr[i]`) - these two specifically retain at the point of reading, precisely to satisfy this rule,
- a cast, concatenation, vector stringify result, etc.

Reading a bare variable (`x`) is **not** a "new reference" - it's a borrow of the variable's own reference.

Consequence: whenever an expression's result lands in a new owning slot (`let`, assignment, struct field, vector element, by-value argument, `return`), an extra `retain` is needed **only** if the source expression was a bare variable - in every other case the result already carries its own +1 and nothing further is needed.

See: `Compiler::expr_needs_retain` / `Compiler::expr_needs_release` in `memory.rs`.

### Scope release
Every block `{ ... }` is its own lexical scope (`Compiler::push_scope` / `pop_scope_and_release`, wired up through `visit_block`). Owning variables declared in that block are released automatically at the end of the block, in reverse declaration order - unless the block already ended via `return`/`break`/`continue` (in which case release already happened earlier, see below).

By-value function parameters live in their own, outer scope (opened before the function body) - the function owns them and releases them at the end, unless an explicit `return` releases them earlier.

### `return`
1. Evaluate the return value.
2. If the returned expression is a bare variable (`return x;`) and its type is managed - `retain` it **before** releasing scopes (so it survives the release of its own local variable).
3. Release **every** active scope of the function (parameters + all nested blocks).
4. Build the `ret` instruction.

### `break` / `continue`
Release every scope opened **since** entering the loop/switch (not counting the loop's own declaration scope, e.g. `for (let i = ...)`), before jumping to the target. The exact threshold (`scope_depth`) is recorded in `ControlFrame` at the point the loop/switch is entered.

### User function call (by value)
1. Evaluate the argument.
2. Build a copy (string: deep; vector/struct: shallow - see section 3).
3. If the source expression was **not** a bare variable (i.e. it already carried its own +1 per the base rule), release the original - copying already consumed its contents and nothing else needs it anymore.
4. The copied value is handed to the callee as a brand new, fully independent owner (parameter), released by the callee on exit.

### Built-in functions (`vector_size`, `str_len`, `vector_stringify`, `vector_push`, ...)
These functions do **not** go through the argument-copying mechanism above - they call `visit_expression` on the argument directly and only "peek" at the value (length, data pointer), never storing it anywhere. If the argument was a `FieldAccess`/`Index`/fresh expression (i.e. something carrying a +1 per the base rule), the function must release that +1 after use, or it leaks. Reading a bare variable requires no release (nothing was retained).

## 5. A quick sanity check

Practical test for "is this balanced": compare the behavior of `f(x)` (bare variable) vs. `f(obj.field)` / `f(arr[i])` / `f(literal)` - the source object's refcount after the call should be identical in both cases. If it differs, some place isn't balancing retain/release correctly.

## 6. Known limitations

### 6.1. Reference cycles - memory leak
Plain refcounting **does not detect cycles**. Two objects holding onto each other (directly, or through a vector) will never reach refcount 0, even once nothing external can reach them anymore:

```raptor
struct Node {
    i64 value,
    Node[] next
};

fn main() {
    let a = Node { value: 1, next: [] };
    let b = Node { value: 2, next: [] };

    a.next = [b];   // retains b
    b.next = [a];   // retains a
}
// end of main: release(a), release(b) bring both rc from 2 down to 1 - never to 0.
// a, b, and their next vectors leak forever.
```

This is a known, accepted weakness of the current model (no weak references / cycle collector). Future fix, if needed: weak references for designated fields, or a separate cycle collector run periodically.

### 6.2. Self-referential types - **compiler** crash, not just a runtime leak
This is a more serious, currently unfixed problem: `release_struct` / `release_vector` / `release_value` generate the release code through **recursive Rust function calls at compile time** (inlining the release logic instead of generating one LLVM function per type and calling it).

For a type that (directly, or through a cycle of several structs) contains itself inside a vector - e.g. `struct Node { Node[] next }` - compiling **any** code that requires generating a release for `Node` enters infinite recursion:

```
release_struct(Node)
  -> field next: Vector<Node> -> release_vector(Node)
    -> element: Struct(Node) -> release_struct(Node)
      -> field next: Vector<Node> -> release_vector(Node)
        -> ... (forever)
```

Symptom: the compiler (the `raptor` process, not the generated program) crashes with a `stack overflow` during compilation, before anything ever gets to run.

**Fix direction** (not implemented): instead of inlining release/retain for every type at each use site, generate one LLVM function per type (`@release_Node`, `@retain_Node`) once, with memoization ("already generated for this type") - analogous to how `declare_functions` generates user functions once regardless of how many times they're called. Inside such a generated function, recursion (e.g. `Node` releasing a `Node[]` containing `Node`) is fully safe, since it's an ordinary LLVM `call` to the same function, handled by the *program's* stack, not the *compiler's* stack. This would also, as a side effect, cut down on generated-IR bloat (release for deeply nested types is currently fully inlined, repeatedly, at every use site).

Until this is fixed: **avoid struct types that (directly or through a cycle) contain a vector/field of their own type** - compiling such a program will not succeed.

### 6.3. Intermediate expressions with no assignment
If the result of a +1-producing expression (field read, indexing, function call) is used in the middle of a larger expression and never lands anywhere covered by the rules in section 4, that +1 may never be released. Any new place in the compiler that "peeks" at a managed value without storing it (similar to `vector_size`/`str_len`) must manually release that value if `expr_needs_release` returns `true` for the source expression.

### 6.4. The `??` operator (`Alternative`)
Not covered by the "is this result a new +1 reference or a borrow" analysis - if both operands are bare variables, the result may not have a correctly accounted-for refcount. Needs a dedicated review when implementing this operator for managed types.