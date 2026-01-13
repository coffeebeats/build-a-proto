# CLAUDE.md

> `baproto`: An IDL compiler for custom bit-level binary encodings targeting networked simulations.

## Build & Test

```bash
cargo build          # Build
cargo test           # Run tests
cargo clippy         # Lint
cargo fmt --check    # Check formatting
```

## Comment Headers

80-char delimited, centered text. Types: `Struct`, `Enum`, `Type`, `Trait`, `Fn`, `Impl`, `Mod`, `Macro`
Type name is the "context" (e.g. function name, trait being impl'ed, function being tested, etc.)

```rust
/* -------------------------------------------------------------------------- */
/*                              Struct: TypeName                              */
/* -------------------------------------------------------------------------- */

/* ------------------------------ Mod: SubName ------------------------------ */
mod subname;
pub use subname::*;
```

## Tests

Inline with Given/When/Then comments:

```rust
/* -------------------------------------------------------------------------- */
/*                                 Mod: Tests                                 */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    /* ---------------------------- Tests: feature -------------------------- */

    #[test]
    fn test_feature_scenario_outcome() {
        // Given: Setup description.
        // When: Action description.
        // Then: Assertion description.
    }
}
```

## Patterns

- Lexer/parser use `chumsky` combinators; parser fns return `impl Parser<...>`
- All AST nodes include `span: Span` for source location
- Errors use `thiserror`: `#[derive(Error)]`
- Visibility: `pub(self)` module-private, `pub(super)` test helpers
- Doc comments: `` `Name` `` or `` [`Name`] `` for links
- Doc comments (cont.): start doc comments with identifier:

    ```rust
    /// `ident` ...
    fn ident() {}
    ```
