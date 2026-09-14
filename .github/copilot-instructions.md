# Coding Standards

This document defines coding standards for this project. Follow these rules in
every file, every commit, every PR.

---

## General Principles

When a specific rule is unclear, fall back to these:

| # | Principle | Meaning |
|---|-----------|---------|
| 1 | Safety first | Prefer safe Rust. Only reach for `unsafe` when performance measurements or FFI boundaries leave no alternative. |
| 2 | Idiomatic Rust | Follow standard library conventions. Avoid clever abstractions that fight the borrow checker or obscure intent. |
| 3 | Clarity over brevity | Write code for the next person reading it. Use descriptive names, extract complex expressions, and comment the *why*. |
| 4 | Explicit imports | Always import types, functions, and macros at the top of the file. Never rely on wildcard or implicit imports. |

## Naming

Names are documentation. Choose them carefully.

| Category | Case | Example |
|----------|------|---------|
| Types (structs, enums, traits) | `PascalCase` | `TraitTypeMap`, `VecFamily` |
| Functions, variables, modules | `snake_case` | `get_storage`, `type_map` |
| Constants & statics | `SCREAMING_SNAKE_CASE` | `DEFAULT_CAPACITY` |
| Generics | short uppercase, or descriptive | `T`, `K`, `V` |

Abbreviations are forbidden. `env`, `dt`, `tex`, `ctx`, `fmt`, `rot`, `pos`,
and any other truncation must never appear in identifiers, module paths, or
type names.

The only permitted abbreviations are Rust language keywords whose canonical
form is already short: `mut`, `ref`, `ptr`, `len`, `idx`, `dyn`. These are
allowed because they are part of the language's vocabulary, not because they
happen to be short.

Long, verbose names are encouraged. Let the compiler do the typing; humans
need to understand.

## File Layout

Every `.rs` file follows a fixed top-to-bottom order:

| # | Section | Separator | Contents |
|---|---------|-----------|----------|
| 1 | Header Comment | *(none)* | `//!` module doc with `# Responsibilities` and `# Design` sections |
| 2 | Module Attributes | *(none)* | `#![]` crate-level lints |
| 3 | Imports | `// Group` comments | Three groups: `// Standard library` → `// External crates` → `// Current crate` |
| 4 | Constants | `// =====…=====` | `const` & `static`; `pub` before private |
| 5 | Types + Impls | `// =====…=====` per type | struct/enum/trait → inherent `impl` → trait `impl`. Repeat per type. |
| 6 | Free Functions | `// =====…=====` | Module-level helpers |
| 7 | Tests | `// =====…=====` | `#[cfg(test)] mod tests { use super::*; … }` at the bottom |

## Comments

### Module-Level (`//!`)

Every file opens with a `//!` block:

```rust
//! Brief summary.
//!
//! # Responsibilities
//!
//! - What this module owns.
//!
//! # Design
//!
//! Architectural notes.
```

### Item-Level (`///`)

Every public API item must have a `///` doc comment with at least one runnable
example. All examples must compile via `cargo test --doc`.

Types (2-4 lines): summary + elaboration. Enum variants each get `///`.

Functions: third-person summary line + `# Errors` / `# Safety` / `# Examples`
as applicable.

### Inline (`//`)

- Explain *what* and *why* above non-obvious blocks. Never restate the obvious.
- Multi-phase functions use `// Step N: …` labels.

### Safety (`// SAFETY:`)

Every `unsafe` block must be preceded by a `// SAFETY:` comment in prose
justifying aliasing, lifetime, bounds, concurrency, and type validity.

## Imports

Three groups in order, each with a `// Group name` comment:

```rust
// Standard library
use std::any::TypeId;

// External crates
use serde::{Deserialize, Serialize};

// Current crate
use crate::storage::VecFamily;
```

## Error Handling

- `Result<T, E>` for recoverable errors. Never panic for expected failures.
- Use `?` extensively.
- Define specific error types so callers can match on variants.

## Testing

- `#[cfg(test)] mod tests { use super::*; }` at the bottom of each file.
- Test names describe behaviour, not implementation.
- Every `#[test]` carries a `///` doc comment.
- Test pure functions, state transitions, edge cases, and error paths.
- `# Examples` in doc comments are run as doc-tests by `cargo test`.

## Library-Specific

- Keep the public API minimal and intuitive. Prefer a single clear approach
  over multiple ways to do the same thing.
- Every public type, trait, and function must have a `///` doc comment with
  at least one `# Examples` section that compiles.
- Re-export key types at the crate root so users don't need deep import paths.
- Use intra-doc links to reference related types.

## Formatting

- Run `cargo fmt` and `cargo clippy` before committing.
- Do not suppress clippy lints without documenting why.
- Keep function bodies short (~40 lines max).
- Keep files small (~500 lines max).

## Version Control

- Write commit messages explaining *what* changed and *why*.
- Keep commits atomic - one thing, buildable.
- Never commit generated files.

Pull request titles follow Conventional Commits:

```
<type>(<optional-scope>): <description>
```

| Type | Use for |
|------|---------|
| `feat` | New functionality |
| `fix` | Bug fixes |
| `docs` | Documentation |
| `refactor` | Internal restructuring |
| `perf` | Performance improvements |
| `test` | Adding or updating tests |
| `build` | Dependency or build-tooling changes |
| `ci` | CI changes |
| `chore` | Other maintenance |
