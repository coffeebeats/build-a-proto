# baproto Roadmap

A production roadmap for the baproto schema compiler. This document consolidates architecture restructuring and feature development into a unified, ordered plan.

---

## Overview

**Current state**: baproto parses `.baproto` schema files and generates GDScript data classes. Core infrastructure (rooted imports, dependency validation) is complete. Phase 1 architecture foundation (lex, ast, core, parse modules) is complete. Next: semantic analysis (`analyze` module).

**Target state**: A clean compiler pipeline (`lex → parse → analyze → lower`) that produces a JSON-serializable IR with computed layouts. External plugins consume the IR to generate language-specific code.

**Pipeline architecture**:
```
┌─────┐    ┌───────┐    ┌─────────┐    ┌───────┐    ┌──────────┐
│ lex │ → │ parse │ → │ analyze │ → │ lower │ → │ emit/out │
└─────┘    └───────┘    └─────────┘    └───────┘    └──────────┘
  │            │             │             │              │
  │            │             │             │              └─ Plugin system (JSON stdin/stdout)
  │            │             │             └─ IR with computed layouts
  │            │             └─ Symbol table, validation, resolution
  │            └─ Owned AST (String, not &str)
  └─ Zero-copy tokens (borrowed &str)
```

---

## Completed Work

### Infrastructure ✅

**Rooted Imports**: Protoc-style import resolution with type-safe path handling.
- CLI flags: `-I` / `--import_root` (repeatable)
- Type wrappers: `ImportRoot` (directories), `SchemaImport` (validated `.baproto` paths)
- Security: symlink escape prevention, path traversal defense, canonicalization
- Files: `src/core/path.rs`, `src/cmd/compile.rs`

**Dependency Validation**: Module dependency graph validation before type resolution.
- Link phase validates all `include` statements
- Cycle detection with full cycle path reporting
- Type safety: `Module.deps: Vec<SchemaImport>`
- Files: `src/compile/linker.rs`

### Phase 1 (Partial): Architecture Foundation ✅

**1.1 `lex` Module**: Extracted lexer into standalone module with zero-copy tokens (`src/lex/`)

**1.2 `core` Module**: Renamed from `syntax`, consolidated foundational types (`src/core/`)

**1.3 `ast` Module**: New owned AST types with inline spans, two-file organization (`src/ast/`)

**1.4 `parse` Module**: Updated to produce owned AST types, removed old `Expr` types

**1.4.5 Integer Types**: Fixed cross-platform representation using `u64` for tokens/AST

**1.4.6 Package Doc Comments**: Added documentation comment support to package declarations

---

## Phase 1: Architecture Restructure

Refactor the compiler into a clean, modular architecture. This is foundational work that enables all subsequent features.

### Design Decisions

| Aspect | Decision |
|--------|----------|
| Tokens | Borrowed `&str` for zero-copy lexing |
| AST | Owned `String`; conversion at parse boundary |
| Spans | chumsky's `SimpleSpan`; inline `span` field on AST nodes |
| Parser | Keep chumsky; output structured `SourceFile` |
| Modules | Rename `syntax` → `core`; replace old `core` with `ir` |
| AST construction | Struct literals (drop `derive_builder`) |

### Compilation Model

| Aspect | Decision |
|--------|----------|
| Compilation unit | Per-file; each `.baproto` produces one `Schema` |
| Symbol table | Shared across all files; built during registration |
| Cross-file resolution | Symbol table contains all types; lowering resolves references |
| Error handling | Accumulate errors; continue analysis with `Invalid` AST nodes |
| Lowering precondition | Name resolution must succeed (no unresolved references) |

### Error Recovery Strategy

| Stage | Strategy |
|-------|----------|
| Lexer | Skip invalid characters, emit `Token::Invalid` |
| Parser | Use chumsky's recovery; emit `TypeKind::Invalid` for bad types |
| Analysis | Continue with errors; skip validation on `Invalid` nodes |
| Lowering | Fail if unresolved references remain; otherwise produce IR |


### 1.5 Create `analyze` Module

Semantic analysis in two stages with full validation.

**Stage 1: Cross-file passes** (run once across all files)
- Registration: populate symbol table with all type names
- Cycle detection: validate no circular module dependencies

**Stage 2: Per-file passes** (run on each file)
- Name resolution: resolve `Reference` → `Descriptor`
- Type checking: validate field types and encoding compatibility
- Index validation: validate field indices (no duplicates, within range)

#### Symbol Table Design

```rust
pub struct SymbolTable {
    /// Type name → descriptor + metadata
    types: HashMap<String, TypeEntry>,
    /// Module path → module metadata
    modules: HashMap<SchemaImport, ModuleEntry>,
}

pub struct TypeEntry {
    pub descriptor: Descriptor,
    pub kind: TypeKind,          // Message or Enum
    pub span: Span,              // Definition location
    pub source: SchemaImport,    // Which file defined this
}

pub enum TypeKind {
    Message { fields: Vec<FieldEntry>, nested: Vec<Descriptor> },
    Enum { variants: Vec<VariantEntry> },
}

pub struct FieldEntry {
    pub name: String,
    pub index: u64,
    pub typ: ResolvedType,       // Scalar, Array, Map, or Descriptor
    pub encoding: Option<Encoding>,
    pub span: Span,
}

/// Type after name resolution
pub enum ResolvedType {
    Scalar(ScalarType),
    Array { element: Box<ResolvedType>, size: Option<u64> },
    Map { key: Box<ResolvedType>, value: Box<ResolvedType> },
    Named(Descriptor),           // Resolved reference
}
```

#### Implementation

- [ ] Create `src/analyze/mod.rs`
  - `AnalysisContext { symbols: SymbolTable, errors: Vec<AnalysisError> }`
  - `analyze(files: &[SourceFile], ctx: &mut AnalysisContext)`
  - `AnalysisError { file: SchemaImport, span: Span, kind: AnalysisErrorKind }`
- [ ] Create `src/analyze/symbol.rs`
  - `SymbolTable` with resolution logic (ported from `compile/symbol.rs`)
  - `find(scope: &Descriptor, reference: &Reference) -> Option<Descriptor>`
  - Support: absolute refs (`.foo.Bar`), relative refs (`Bar`, `Outer.Inner`)
- [ ] Create `src/analyze/passes/mod.rs`
  - `run_cross_file_passes(files, ctx)`
  - `run_per_file_passes(file, ctx)`
- [ ] Create `src/analyze/passes/registration.rs`
  - Walk all files, register type names in symbol table
  - Detect duplicate type definitions
- [ ] Create `src/analyze/passes/cycle_detection.rs`
  - Port from `compile/linker.rs`
  - DFS to detect circular dependencies
- [ ] Create `src/analyze/passes/name_resolution.rs`
  - Walk AST, resolve each `Reference` to `Descriptor`
  - Update `FieldEntry.typ` with `ResolvedType::Named`
  - Error on unresolved references
- [ ] Create `src/analyze/passes/type_check.rs`
  - Validate map keys are scalars
  - Validate encoding/type compatibility (see table below)
  - Validate array sizes are positive
- [ ] Create `src/analyze/passes/index_validation.rs`
  - Validate no duplicate indices within a message
  - Validate indices are within reasonable range (see note below)
- [ ] Create `src/analyze/passes/name_validation.rs`
  - Validate no duplicate field names within a message
  - Validate no duplicate variant names within an enum
  - Validate no duplicate nested type names within a message
  - Note: This validation was previously in parser (if present); moving to semantic analysis
- [ ] Create `src/analyze/passes/value_validation.rs`
  - Validate field indices fit in reasonable range (e.g., < 2^29 like protobuf)
  - Validate encoding size parameters are valid for their types
  - Validate array sizes are reasonable (non-zero, within limits)
  - Note: Integrates integer range validation from Phase 1.4.5
- [ ] Create `src/analyze/passes/resource_limits.rs`
  - Validate structural and encoding limits to prevent abuse and ensure reasonable schemas
  - Message field count limits (e.g., max 512 fields per message)
  - Enum variant count limits (e.g., max 256 variants per enum)
  - Nested message depth limit (e.g., max 32 levels deep)
  - Bit type width limits (e.g., `Bits(n)` where n ≤ 64)
  - Fixed-point encoding limits (e.g., `FixedPoint(i, f)` where i + f ≤ 64)
  - Encoding parameter validation (e.g., `BitsVariable(max)` where max is reasonable)
  - Note: Size-based limits (serialized message size, array item size) require layout computation and are validated in Phase 1.6 during IR lowering
- [ ] Update `src/main.rs` - add `mod analyze;`
- [ ] Run tests

#### Encoding Validation Rules

| Encoding | Valid Types |
|----------|-------------|
| `Bits(n)` | integers (n ≤ type width), bool, enum |
| `BitsVariable(max)` | unsigned integers |
| `FixedPoint(i, f)` | floats (i + f = total bits) |
| `Delta` | integers, floats |
| `ZigZag` | signed integers only |
| `Pad(n)` | any type |

### 1.6 Create `ir` Module

JSON-serializable IR with computed layouts.

- [ ] Create `src/ir/mod.rs`
- [ ] Create `src/ir/schema.rs`
  - `Schema { package, includes, messages, enums }` + serde derives
  - `MessageIR { name, fqn, doc, fields, nested_messages, nested_enums, layout }`
  - `FieldIR { name, index, doc, typ, encoding, layout }`
  - `EnumIR { name, fqn, doc, variants, layout }`
  - `VariantIR { name, index, doc, data }`
- [ ] Create `src/ir/types.rs`
  - `TypeIR` enum (Scalar, Array, Map, Message, Enum)
  - `ScalarIR` enum
  - `EncodingIR`
- [ ] Create `src/ir/layout.rs`
  - `MessageLayout { min_bits, max_bits, is_variable_length }`
  - `FieldLayout { bit_offset, bit_size, alignment }`
  - `EnumLayout { discriminant_bits, max_variant_bits }`
- [ ] Create `src/ir/lower.rs`
  - `lower(file: &SourceFile, symbols: &SymbolTable) -> Result<Schema, LowerError>`
  - Transform validated AST + resolved types → IR
  - Compute layouts during lowering
  - Validate size-based resource limits (requires computed layouts):
    - Serialized message size limits (e.g., max message ≤ 64KB on wire)
    - Array item size limits (e.g., fixed-size arrays must fit in reasonable bounds)
    - Total message complexity (nested size calculations)
  - Note: Static limits (field counts, nesting depth) validated in Phase 1.5
- [ ] Update `src/main.rs` - add `mod ir;`
- [ ] Compile check

### 1.7 Update `compile` Module

Simplify to pipeline orchestration. The `compile` module provides a clean API boundary between CLI and compiler internals.

- [ ] Create `src/compile/pipeline.rs`
  - `compile_files(sources: &[(PathBuf, String)]) -> CompileResult`
  - Orchestrate: lex all → parse all → analyze all → lower each
  - Return `Vec<Schema>` + accumulated errors
- [ ] Update `src/compile/mod.rs`
  - Export `compile_files()`, `CompileResult`, `CompileError`
  - `CompileError` enum unifies lex/parse/analyze/lower errors
- [ ] Delete `src/compile/symbol.rs` (moved to analyze)
- [ ] Delete `src/compile/register.rs` (moved to analyze/passes)
- [ ] Delete `src/compile/linker.rs` (moved to analyze/passes)
- [ ] Delete `src/compile/compiler.rs` (distributed)
- [ ] Delete `src/compile/prepare.rs` (deprecated)
- [ ] Run tests

### 1.8 Clean Up Old `core` Types

Remove deprecated runtime types replaced by `ir/`.

- [ ] Delete `src/core/types.rs` (old)
- [ ] Delete `src/core/message.rs` (old)
- [ ] Delete `src/core/enumeration.rs` (old)
- [ ] Delete `src/core/registry.rs` (old)
- [ ] Verify `core/` contains only: `PackageName`, `Reference`, `Descriptor`, path utils
- [ ] Run tests

### 1.9 Update Consumers

Update CLI and generators to use new pipeline.

- [ ] Update `src/cmd/compile.rs`
  - Use `compile::compile_files()`
  - Collect all errors before reporting (better UX)
  - Handle unified `CompileError` type
- [ ] Update `src/generate/generator.rs`
  - Use `ir::Schema` instead of old types
- [ ] Update `src/generate/lang/gdscript.rs`
  - Use IR types for code generation
- [ ] Run full test suite
- [ ] Manual end-to-end test

---

## Phase 2: Field Optionality

Support optional fields with presence tracking or default values.

### Context

Currently all fields are implicitly required. Schema evolution requires optional fields with presence bits or default values.

### 2.1 Parser Syntax

- [ ] Extend field grammar:
  ```
  required uint32 id = 1;           // Must be present
  optional uint32 count = 2;        // Presence bit
  optional uint32 retries = 3 [default = 3];  // Default value
  ```
- [ ] Files: `src/parse/parser.rs`

### 2.2 AST Types

- [ ] Add `Optionality` to AST:
  ```rust
  pub enum Optionality {
      Required,
      Optional,
      OptionalWithDefault(Value),
  }
  ```
- [ ] Update `ast::Field` with `optionality` field
- [ ] Files: `src/ast/mod.rs`

### 2.3 IR Types

- [ ] Add optionality to IR types
- [ ] Add presence bitmap computation to layout:
  ```rust
  pub struct MessageLayout {
      pub presence_bitmap_bits: usize,  // ceil(optional_count)
      pub min_bits: Option<u64>,
      pub max_bits: Option<u64>,
      pub is_variable_length: bool,
  }
  ```
- [ ] Files: `src/ir/schema.rs`, `src/ir/layout.rs`

### Design Decisions

| Decision | Recommendation |
|----------|----------------|
| Presence bitmap location | Start of message |
| Default value syntax | `[default = X]` attribute |
| Required explicit? | Yes, for clarity |

---

## Phase 3: Plugin System

Enable external generator plugins via JSON IR.

### Context

The JSON IR is the contract between baproto and plugins. Plugins can be any language that reads JSON from stdin and writes JSON to stdout.

### 3.1 CLI Interface

- [ ] Add flags:
  ```
  baproto compile --plugin=path/to/gen schema.baproto
  baproto compile --emit-ir schema.baproto  # IR to stdout
  baproto compile --out-ir=schema.json schema.baproto  # IR to file
  ```
- [ ] Files: `src/cmd/compile.rs`

### 3.2 Plugin Protocol

- [ ] Create `src/generate/plugin.rs`
- [ ] Protocol:
  - baproto writes IR JSON to plugin stdin
  - Plugin writes output to stdout:
    ```json
    {
      "files": {
        "path/to/output.gd": "contents...",
        "path/to/other.gd": "contents..."
      },
      "errors": [],
      "warnings": ["optional warning messages"]
    }
    ```
  - baproto writes files, reports errors/warnings

### 3.3 Remove Built-in Generators

- [ ] Migrate GDScript generator to `baproto-gdscript` repo
- [ ] Delete `src/generate/lang/*`
- [ ] Delete `src/generate/generator.rs`

### Design Decisions

| Decision | Recommendation |
|----------|----------------|
| IR transport | stdin/stdout |
| Plugin discovery | Explicit path only |
| Error handling | Exit codes + stderr + JSON errors field |

---

## Phase 4: CLI Tooling

Additional CLI commands for debugging and production safety.

### 4.1 Wire Format Command

Human-readable wire format documentation.

- [ ] New command: `baproto wire-format <schema.baproto> [MessageName]`
- [ ] Output:
  ```
  Message: PlayerState
  Total size: 48 bits (6 bytes) + variable

  Presence bitmap: 2 bits (fields: health, position)

  Fields:
    [0:8]   id: uint8 (Bits(8))
    [8:16]  health: optional uint8 (Bits(8), presence bit 0)
    [16:48] position: optional Vec3 (presence bit 1)
  ```
- [ ] Files: `src/cmd/wire_format.rs`

### 4.2 Schema Compatibility Command

Check compatibility between schema versions.

- [ ] New command: `baproto check-compat <old.baproto> <new.baproto>`
- [ ] Compatibility rules:
  - Field indices must not change
  - Type widening OK (u8→u16), narrowing breaks
  - New required fields break old readers
  - Encoding changes that affect wire format break
- [ ] Output:
  ```
  INCOMPATIBLE: 2 breaking changes

  ERROR: Field 'health' index changed from 2 to 3
  ERROR: New required field 'armor' added

  WARNING: Field 'name' encoding changed (Bits(8) → Bits(16))
  ```
- [ ] Files: `src/cmd/check_compat.rs`

---

## Phase 5: Language Generators

External plugins for each target language. These are **separate repositories**.

### 5.1 GDScript Generator

Repository: `baproto-gdscript`

- [ ] Consume JSON IR from stdin
- [ ] Generate serialize/deserialize methods
- [ ] Runtime library: `BitReader`/`BitWriter`
- [ ] Package as Godot addon

### 5.2 Rust Generator

Repository: `baproto-rust`

- [ ] Consume JSON IR from stdin
- [ ] Type mappings: `uint8`→`u8`, `optional T`→`Option<T>`, etc.
- [ ] Generate `serialize()`/`deserialize()` methods
- [ ] Runtime crate: `baproto-runtime` on crates.io

### Plugin Requirements

Each plugin must generate:

1. **Runtime library** (~100-200 LOC):
   - `BitWriter`: write N bits, variable-length integers
   - `BitReader`: read N bits, variable-length integers
   - Endianness handling (little-endian recommended)

2. **Encoding implementations**:
   - `Bits(n)`: `writer.write_bits(value, n)`
   - `BitsVariable(max)`: variable-length encoding
   - `FixedPoint(i, f)`: `writer.write_bits((value * 2^f) as int, i + f)`
   - `ZigZag`: `(n << 1) ^ (n >> 31)`
   - `Delta`: compare with previous value
   - `Pad(n)`: `writer.write_bits(0, n)`

3. **Optionality handling**:
   - Write presence bitmap at message start
   - Skip optional fields not present
   - Write defaults for `OptionalWithDefault`

---

## Phase 6: Advanced Encodings

Extended encoding support for game networking.

### 6.1 Quantized Floats

Range-limited floats in fewer bits.

- [ ] Syntax: `float32 rotation @Quantized(0.0, 360.0, 10);`
- [ ] Encoding:
  ```rust
  Quantized { min: f64, max: f64, bits: usize }
  ```
- [ ] Serialization:
  ```
  normalized = (value - min) / (max - min)
  quantized = (normalized * (2^bits - 1)) as uint
  ```

### 6.2 Dead Reckoning

Only transmit values differing from predictions.

- [ ] Syntax:
  ```
  message PlayerState {
    Vec3 position = 1;
    Vec3 velocity = 2;
    Vec3 predicted_position = 3 @DeadReckoning(position, velocity, 0.1);
  }
  ```
- [ ] Runtime:
  - Track previous values
  - Compute prediction: `predicted = previous + velocity * dt`
  - Transmit if `abs(actual - predicted) > threshold`

**Note**: Details deferred; architecture supports this model.

---

## Phase 7: Integration Testing

End-to-end validation across the pipeline.

### 7.1 Test Harness

- [ ] Create `tests/integration/` structure:
  ```
  tests/integration/
    gdscript/test_roundtrip.gd
    rust/test_roundtrip.rs
  ```
- [ ] CI integration

### 7.2 Test Cases

- [ ] All scalar types
- [ ] Nested messages
- [ ] Enums with variants
- [ ] Optional fields (with/without values)
- [ ] Arrays and maps
- [ ] All encoding types
- [ ] Multi-file imports
- [ ] Error cases with correct spans

---

## Phase 8: Schema Formatting

Enable lossless AST representation for schema formatting, refactoring tools, and code organization features.

### Context

Currently, the AST is designed for compilation: it organizes fields by type (all fields together, all nested messages together, etc.) and discards line comments. This is optimal for semantic analysis and IR generation, but prevents:
- Schema formatting tools (`baproto format`)
- Automatic refactoring tools
- IDE features that preserve user's code organization
- Round-trip parsing (parse → modify → write back)

This phase redesigns the AST to preserve **source file ordering** and **all comments**, enabling a full suite of schema tooling.

### Design Philosophy

**Key insight**: The AST serves two masters:
1. **Compilation**: Needs typed access to fields, messages, enums
2. **Formatting**: Needs original source structure and comments

**Solution**: Store items in source order, provide helper methods for typed access.

```rust
// Current AST (optimized for compilation)
pub struct Message {
    pub fields: Vec<Field>,           // All fields grouped
    pub nested_messages: Vec<Message>, // All messages grouped
    pub nested_enums: Vec<Enum>,      // All enums grouped
}

// Phase 8 AST (preserves source order)
pub struct Message {
    pub items: Vec<MessageItem>,      // Source order preserved!
}

pub enum MessageItem {
    Field(Field),
    NestedMessage(Message),
    NestedEnum(Enum),
    LineComment(LineComment),         // NEW: preserved!
    BlankLine,                        // NEW: preserved!
}

// Helper methods for semantic passes (unchanged usage)
impl Message {
    pub fn fields(&self) -> impl Iterator<Item = &Field> { ... }
    pub fn messages(&self) -> impl Iterator<Item = &Message> { ... }
    pub fn enums(&self) -> impl Iterator<Item = &Enum> { ... }
}
```

**Result**: Semantic analysis code remains clean (uses helpers), but AST can now reconstruct original source exactly.

### 8.1 Update AST for Ordering

Redesign AST node types to store items in source order.

- [ ] Update `src/ast/mod.rs` - Message ordering:
  ```rust
  pub struct Message {
      pub span: Span,
      pub doc: Option<DocComment>,
      pub name: Ident,
      pub items: Vec<MessageItem>,  // Changed from separate vecs
  }

  pub enum MessageItem {
      Field(Field),
      NestedMessage(Message),
      NestedEnum(Enum),
      LineComment(LineComment),
      BlankLine,
  }

  impl Message {
      /// Get all fields in source order
      pub fn fields(&self) -> impl Iterator<Item = &Field> {
          self.items.iter().filter_map(|item| match item {
              MessageItem::Field(f) => Some(f),
              _ => None,
          })
      }

      /// Get all nested messages in source order
      pub fn messages(&self) -> impl Iterator<Item = &Message> {
          self.items.iter().filter_map(|item| match item {
              MessageItem::NestedMessage(m) => Some(m),
              _ => None,
          })
      }

      /// Get all nested enums in source order
      pub fn enums(&self) -> impl Iterator<Item = &Enum> {
          self.items.iter().filter_map(|item| match item {
              MessageItem::NestedEnum(e) => Some(e),
              _ => None,
          })
      }
  }
  ```

- [ ] Update `src/ast/mod.rs` - SourceFile ordering:
  ```rust
  pub struct SourceFile {
      pub span: Span,
      pub package: Package,
      pub includes: Vec<Include>,
      pub items: Vec<TopLevelItem>,  // Changed from separate vecs
  }

  pub enum TopLevelItem {
      Message(Message),
      Enum(Enum),
      LineComment(LineComment),
      BlankLine,
  }

  impl SourceFile {
      pub fn messages(&self) -> impl Iterator<Item = &Message> { ... }
      pub fn enums(&self) -> impl Iterator<Item = &Enum> { ... }
  }
  ```

- [ ] Add `src/ast/mod.rs` - Comment types:
  ```rust
  /// Line comment (// ...)
  pub struct LineComment {
      pub span: Span,
      pub text: String,  // Without leading "//"
  }

  /// Blank line marker
  pub struct BlankLine {
      pub span: Span,
  }
  ```

- [ ] Update `src/ast/mod.rs` - Enum ordering (if enums support nested items in future):
  ```rust
  // Currently enums only have variants, but structure for future:
  pub struct Enum {
      pub span: Span,
      pub doc: Option<DocComment>,
      pub name: Ident,
      pub items: Vec<EnumItem>,
  }

  pub enum EnumItem {
      Variant(Variant),
      LineComment(LineComment),
      BlankLine,
  }

  impl Enum {
      pub fn variants(&self) -> impl Iterator<Item = &Variant> { ... }
  }
  ```

### 8.2 Update Parser for Comment Preservation

Modify parser to capture and preserve line comments and blank lines.

- [ ] Update `src/parse/parser.rs` - Line comment parser:
  - Add parser for `//` comments (capture until newline)
  - Store text without leading `//` and trailing whitespace
  - Create `LineComment` AST nodes

- [ ] Update `src/parse/parser.rs` - Blank line tracking:
  - Detect consecutive newlines (blank lines)
  - Create `BlankLine` AST nodes
  - Preserve in source order with other items

- [ ] Update `src/parse/parser.rs` - Message item parsing:
  - Parse fields, nested messages, nested enums, line comments, blank lines
  - Store all items in `Vec<MessageItem>` in source order
  - Maintain existing error recovery

- [ ] Update `src/parse/parser.rs` - Top-level item parsing:
  - Parse messages, enums, line comments, blank lines
  - Store all items in `Vec<TopLevelItem>` in source order

- [ ] Run tests - verify parser produces ordered AST

### 8.3 Update Analysis Passes

Update semantic analysis to use helper methods instead of direct field access.

- [ ] Update `src/analyze/passes/registration.rs`:
  - Change `message.nested_messages` → `message.messages()`
  - Change `message.nested_enums` → `message.enums()`
  - Change `file.messages` → `file.messages()`
  - Change `file.enums` → `file.enums()`

- [ ] Update `src/analyze/passes/name_resolution.rs`:
  - Use `message.fields()` instead of `message.fields`
  - Use helper methods throughout

- [ ] Update `src/analyze/passes/type_check.rs`:
  - Use `message.fields()` instead of `message.fields`

- [ ] Update `src/analyze/passes/index_validation.rs`:
  - Use `message.fields()` instead of `message.fields`
  - Use `enum.variants()` instead of `enum.variants`

- [ ] Update `src/analyze/passes/name_validation.rs`:
  - Use helper methods for iteration

- [ ] Update `src/analyze/passes/value_validation.rs`:
  - Use helper methods for iteration

- [ ] Run tests - verify analysis passes still work correctly

**Note**: Because analysis code uses helper methods, changes should be minimal (mostly find-replace of direct field access).

### 8.4 Update IR Lowering

Update lowering logic to use helper methods.

- [ ] Update `src/ir/lower.rs`:
  - Use `message.fields()` instead of `message.fields`
  - Use `message.messages()` instead of `message.nested_messages`
  - Use `message.enums()` instead of `message.nested_enums`
  - Use `file.messages()` and `file.enums()` for top-level items

- [ ] Run tests - verify IR generation unchanged
- [ ] Verify IR remains comment-free (formatting is source concern only)

**Design note**: The IR intentionally does **not** include comments or ordering. The IR is the semantic representation for code generation. Comments and ordering are preserved in the AST for source-level tooling.

### 8.5 Create Schema Formatter

Implement schema formatting based on ordered AST.

- [ ] Create `src/format/mod.rs`:
  ```rust
  pub struct FormatOptions {
      pub indent_size: usize,      // Spaces per indent level
      pub max_line_width: usize,   // Target line width
      pub preserve_blank_lines: usize, // Max consecutive blank lines
  }

  impl Default for FormatOptions {
      fn default() -> Self {
          Self {
              indent_size: 2,
              max_line_width: 100,
              preserve_blank_lines: 1,
          }
      }
  }

  pub fn format(ast: &SourceFile, options: &FormatOptions) -> String {
      // AST → formatted source reconstruction
  }
  ```

- [ ] Create `src/format/writer.rs`:
  - Helper for indentation tracking
  - Helper for line wrapping
  - Helper for blank line management

- [ ] Implement formatting rules:
  - Package declaration: doc comments, then `package name;`
  - Includes: grouped together, sorted alphabetically
  - Top-level items: preserve ordering and comments
  - Messages: doc, name, then ordered items with proper indentation
  - Fields: doc, type, name, index, encoding on one line (wrap if needed)
  - Nested items: indent by `indent_size`
  - Line comments: preserve, re-indent to match context
  - Blank lines: preserve up to `preserve_blank_lines` consecutive

- [ ] Add tests:
  - Round-trip: parse → format → parse (should be identical)
  - Formatting: unformatted input → formatted output
  - Comment preservation: verify all comments present in output
  - Ordering preservation: verify source order maintained

### 8.6 Add Format CLI Command

Add `baproto format` command to CLI.

- [ ] Create `src/cmd/format.rs`:
  ```rust
  pub struct FormatCommand {
      pub input: PathBuf,
      pub check: bool,        // Check mode (exit 1 if unformatted)
      pub write: bool,        // Write back to file
      pub indent_size: usize,
  }

  pub fn format(cmd: FormatCommand) -> Result<(), Error> {
      // Parse schema
      // Format AST
      // Either: print to stdout, write to file, or check mode
  }
  ```

- [ ] Update `src/cmd/mod.rs`:
  - Add `format` subcommand

- [ ] Update `src/main.rs`:
  - Wire up format command

- [ ] Add CLI flags:
  ```
  baproto format <schema.baproto>              # Print formatted to stdout
  baproto format --write <schema.baproto>      # Write formatted back to file
  baproto format --check <schema.baproto>      # Check if formatted (CI mode)
  baproto format --indent-size=4 <schema.baproto>  # Custom indent
  ```

- [ ] Document behavior:
  - Default: print to stdout
  - `--write`: modify file in-place (with backup?)
  - `--check`: exit 0 if formatted, exit 1 if would change (for CI)
  - `--indent-size`: override default indentation

### 8.7 Integration Testing

- [ ] Test format command:
  - Format a schema, verify output is valid
  - Round-trip: format → parse → format (idempotent)
  - Comment preservation across formatting
  - Blank line management

- [ ] Test with real schemas:
  - Format existing test schemas
  - Verify no semantic changes (same IR output)

- [ ] CI integration:
  - Add format check to CI: `baproto format --check schemas/**/*.baproto`

### Benefits

After Phase 8:
- ✅ `baproto format` standardizes schema style across projects
- ✅ IDE integrations can preserve comments during refactoring
- ✅ Schema evolution tools can modify fields without losing comments
- ✅ Round-trip editing: parse → modify AST → write back
- ✅ Foundation for future refactoring tools (rename, extract, inline)

### Design Rationale

**Why defer to Phase 8?**
- Semantic analysis (Phase 1.5-1.7) is complex enough without ordering concerns
- IR generation doesn't need comments or ordering
- Helper methods insulate analysis code from AST structural changes
- Formatting is a distinct feature that benefits from focused implementation

**Why not use a separate "formatting AST"?**
- Single source of truth is simpler
- Helper methods make semantic code clean regardless of storage
- Avoids conversion overhead between AST representations
- Enables future tools that need both semantic and structural information

---

## Appendix A: Module Structure

Target directory layout after Phase 1:

```
src/
├── lex/
│   ├── mod.rs       # lex(), Token, Keyword, Span
│   ├── token.rs     # Token<'src>, Keyword
│   └── lexer.rs     # Lexer implementation
│
├── ast/
│   ├── mod.rs       # All node types (SourceFile, Message, Field, etc.)
│   └── types.rs     # Type, TypeKind, ScalarType
│
├── parse/
│   ├── mod.rs       # parse(), ParseResult, ParseError
│   └── parser.rs    # Parser (chumsky)
│
├── analyze/
│   ├── mod.rs       # analyze(), AnalysisContext, AnalysisError
│   ├── symbol.rs    # SymbolTable, TypeEntry, FieldEntry
│   └── passes/
│       ├── mod.rs
│       ├── registration.rs
│       ├── cycle_detection.rs
│       ├── name_resolution.rs
│       ├── type_check.rs
│       ├── index_validation.rs
│       ├── name_validation.rs
│       ├── value_validation.rs
│       └── resource_limits.rs
│
├── ir/
│   ├── mod.rs       # Schema, MessageIR, etc.
│   ├── schema.rs    # Schema, MessageIR, FieldIR, EnumIR
│   ├── types.rs     # TypeIR, ScalarIR, EncodingIR
│   ├── layout.rs    # MessageLayout, FieldLayout
│   └── lower.rs     # AST → IR transformation
│
├── compile/
│   ├── mod.rs       # compile_files(), CompileResult, CompileError
│   └── pipeline.rs  # Pipeline orchestration
│
├── core/            # Foundational types (renamed from syntax/)
│   ├── mod.rs
│   ├── package.rs   # PackageName
│   ├── reference.rs # Reference
│   ├── descriptor.rs # Descriptor
│   └── path.rs      # SchemaImport, ImportRoot
│
├── generate/        # Plugin invocation (after Phase 3)
│   └── plugin.rs
│
├── format/          # Schema formatting (Phase 8)
│   ├── mod.rs       # format(), FormatOptions
│   └── writer.rs    # Formatting helpers
│
└── cmd/             # CLI commands
    ├── mod.rs
    ├── compile.rs
    ├── format.rs        # (Phase 8)
    ├── wire_format.rs   # (Phase 4)
    └── check_compat.rs  # (Phase 4)
```

---

## Appendix B: Dependency Graph

```
Completed ─────────────────────────────────────────────────────┐
  ├─ Infrastructure (Rooted Imports, Dependency Validation) ✅ │
  └─ Phase 1.1-1.4.6 (lex, ast, core, parse modules) ✅       │
                                                               │
Phase 1.5-1.9: Remaining Architecture ◄────────────────────────┘
  │
  ├──────────────────────────────┐
  │                              │
  ▼                              ▼
Phase 2: Optionality       (can parallelize)
  │
  ▼
Phase 3: Plugin System
  │
  ├───────────────┬───────────────┬───────────────┐
  │               │               │               │
  ▼               ▼               ▼               ▼
Phase 4       Phase 5         Phase 5         Phase 8
CLI Tools     GDScript        Rust Gen        Schema
              Generator       (external)      Formatting
              (external)
                    │
                    ├───────────────┐
                    │               │
                    ▼               ▼
            Phase 6: Advanced   Phase 7:
                 Encodings      Integration
                                   Tests
```

---

## Appendix C: Design Notes

### Architectural Decisions

1. **Two-stage analysis**: Cross-file passes (registration, cycles) run first, then per-file passes (resolution, type check, indices). This ensures all types are known before resolution.

2. **Symbol table as source of truth**: After analysis, the symbol table contains fully resolved type information. Lowering reads from it, not from the AST.

3. **Error accumulation**: All stages collect errors rather than fail-fast. The CLI reports all errors at once for better developer experience.

4. **chumsky spans**: Parser uses `(T, Span)` tuples internally; AST uses inline `span` fields. `Span` is re-exported from chumsky as `SimpleSpan`.

5. **Visitor pattern deferred**: Initial passes use direct pattern matching. Add `AstVisitor` trait only when shared traversal logic is needed.

6. **Thin compile module**: The `compile` module is intentionally minimal—it provides a clean API boundary. Complex logic lives in `analyze` and `ir`.

### Rust Best Practices

- **Owned AST**: No lifetime parameters; AST can be stored/transformed freely
- **Exhaustive matching**: Use `match` on enums rather than visitor pattern where feasible
- **Serde for IR**: Standard serialization, JSON-debuggable output
- **Spans on errors**: All errors carry span + file information for rich diagnostics
- **Type-safe paths**: `SchemaImport` and `ImportRoot` enforce validation at construction

### Resource Limits and Constraints

To prevent abuse, ensure reasonable resource usage, and catch schema design issues early, baproto enforces the following limits:

#### Static Limits (Phase 1.5 - Semantic Analysis)

Validated during semantic analysis without requiring layout computation:

| Limit | Recommended Value | Rationale |
|-------|------------------|-----------|
| Max fields per message | 512 | Prevents excessive memory usage; encourages better data modeling |
| Max variants per enum | 256 | Fits discriminant in u8; reasonable for most use cases |
| Max nesting depth | 32 | Prevents stack overflow during traversal; encourages flatter schemas |
| Max bits in `Bits(n)` | 64 | Matches largest integer type (u64/i64) |
| Max bits in `FixedPoint(i,f)` | i + f ≤ 64 | Must fit in u64 for computation |
| Max in `BitsVariable(max)` | 64 | Reasonable upper bound for variable-length encoding |
| Max field index | 2^29 - 1 (~536M) | Matches protobuf limit; reserves top bits for flags |
| Max array declared size | 2^16 (65536) | Prevents accidental huge allocations |

#### Computed Limits (Phase 1.6 - IR Lowering)

Validated during IR lowering after layout computation:

| Limit | Recommended Value | Rationale |
|-------|------------------|-----------|
| Max serialized message size | 64 KB (configurable) | Network packet size; prevents DoS |
| Max array item size | 16 KB | Prevents large inline allocations |
| Max message complexity | TBD | Sum of nested message sizes; prevents exponential blowup |

**Note**: These are **recommended defaults**. The compiler should:
- Make limits configurable via CLI flags or config file
- Provide clear error messages with actual vs. limit values
- Allow different limit profiles (strict, moderate, permissive)

**Example error message**:
```
error: message exceeds field count limit
  --> game/protocol.baproto:15:1
   |
15 | message PlayerState {
   | ^^^^^^^^^^^^^^^^^^^ message has 600 fields, but limit is 512
   |
   = help: consider breaking this message into smaller, focused messages
   = note: use `--max-fields=1024` to increase the limit if necessary
```

---

## Appendix D: Testing Strategy

### Unit Tests

| Module | Test Approach |
|--------|---------------|
| `lex` | Input strings → expected tokens |
| `ast` | Pure data structures; no tests needed |
| `parse` | Tokens → expected AST |
| `analyze/symbol` | Insert/resolve operations |
| `analyze/passes/*` | Constructed ASTs → expected errors |
| `ir/lower` | AST + symbols → expected IR |

### Integration Tests

- End-to-end: `.baproto` files → IR verification
- Error cases: Invalid schemas → expected errors with spans
- Multi-file: Imports → correct cross-file resolution
- Roundtrip: Serialize → deserialize → compare

### Test Helpers

Create `tests/util/` with:
- AST builder helpers for concise construction
- Snapshot testing for IR comparison
