# Production Roadmap

This document outlines the features and architectural changes required to make
`baproto` production-ready. Items are organized by phase, with dependencies
explicitly noted.

---

## Phase 0: Foundation

These changes have no dependencies on each other and establish the groundwork
for everything else. They can be implemented in parallel.

### 0.1 Migrate to Rooted Paths

**Goal**: Replace relative path resolution with protoc-style rooted imports.

**Current State**: Imports are resolved relative to the importing file (see
`parse_include_path` in `src/compile/prepare.rs:180-217`). This breaks when
schemas are in different directory trees.

**What Changes**:

1. **CLI flags**: Add `-I` / `--proto_path` to specify import roots (can be
   repeated). Modify `src/cmd/compile.rs:27-39`.
2. **Path resolution**: Change `parse_include_path` to search import roots in
   order, returning the first match.
3. **Canonicalization**: Imports become rooted (e.g., `import "foo/bar.proto"`
   resolves as `<import_root>/foo/bar.proto`).
4. **Default behavior**: If no `-I` specified, use current working directory.

**Key Decisions**:

- Whether to support relative imports at all (recommend: no, for simplicity)
- Error message format when file not found in any root

**Files to Modify**: `src/main.rs`, `src/cmd/compile.rs`, `src/compile/prepare.rs`

---

### 0.2 Resolution Pass: Validate Type References

**Goal**: Ensure all `Type::Reference` values resolve to actual definitions
before generation.

**Current State**: `Type::Reference(String)` is never validated. Invalid
references silently produce broken code.

**What Changes**:

1. **New phase**: Add `src/compile/resolve.rs` that runs after `prepare`, before
   `compile`.
2. **Symbol table**: Build `HashMap<String, Descriptor>` mapping fully-qualified
   names to their definitions.
3. **Type walking**: Recursively visit all types in messages/enums, validate
   each `Reference` exists in symbol table.
4. **Error reporting**: Use spans to report undefined type errors with precise
   locations.

**Key Decisions**:

- Fully-qualified name format (recommend: `package.path.Name`, matching current
  `Descriptor` string format)
- Whether to support relative references within a message scope (recommend: yes,
  resolve in order: local scope → parent scopes → package root → imports)

**Files to Add**: `src/compile/resolve.rs`
**Files to Modify**: `src/compile/mod.rs`, `src/cmd/compile.rs`

---

### 0.3 Field-Level Optionality Encoding

**Goal**: Support optional fields with presence tracking or default values,
enabling schema evolution.

**Current State**: All fields are implicitly required. No presence bits, no
defaults.

**What Changes**:

1. **Parser syntax**: Extend field grammar to support:
   ```
   required uint32 id = 1;           // Must be present (current behavior)
   optional uint32 count = 2;        // Presence bit, omit if unset
   optional uint32 retries = 3 [default = 3];  // Write default if unset
   ```
2. **Core types**: Extend `Field` struct in `src/core/message.rs`:
   ```rust
   pub enum Optionality {
       Required,
       Optional,                    // Uses presence bit
       OptionalWithDefault(Value),  // Writes default value
   }

   pub struct Field {
       // ... existing fields ...
       pub optionality: Optionality,
   }
   ```
3. **Wire format impact**: Optional fields require a presence bitmap at the
   start of each message. The bitmap is `ceil(optional_field_count / 8)` bytes.

**Key Decisions**:

- Presence bitmap location: Start of message (recommend) vs. before each
  optional field
- Default value syntax: `[default = X]` attribute vs. inline `= X`
- Whether `required` is explicit or default (recommend: explicit for clarity)

**Files to Modify**: `src/parse/parser.rs`, `src/parse/expr.rs`,
`src/core/message.rs`, `src/compile/prepare.rs`

---

## Phase 1: Validation

Depends on Phase 0 (Resolution Pass).

### 1.1 Encoding Validator: Check Encoding/Type Compatibility

**Goal**: Validate that encoding annotations are compatible with their field
types.

**Current State**: Encodings are parsed and stored (`Encoding` enum in
`src/core/message.rs:70-81`) but never validated against field types.

**What Changes**:

1. **Validation rules**: Define compatibility matrix:
   | Encoding | Valid Types |
   | ------------------- | ----------------------------------------------- |
   | `Bits(n)` | integers (n ≤ type bit width), bool, enum |
   | `BitsVariable(max)` | unsigned integers |
   | `FixedPoint(i, f)` | floats (i + f bits total) |
   | `Delta` | integers, floats (requires previous value) |
   | `ZigZag` | signed integers only |
   | `Pad(n)` | any (inserts n zero bits) |
2. **Validator location**: Add to `resolve.rs` or new `validate.rs`.
3. **Error reporting**: Report type/encoding mismatches with spans.

**Key Decisions**:

- Whether to allow `Bits(n)` where n > type width (recommend: error)
- Whether `Delta` requires runtime state tracking annotation

**Files to Add**: `src/compile/validate.rs` (or extend `resolve.rs`)
**Files to Modify**: `src/compile/mod.rs`

---

## Phase 2: Intermediate Representation

Depends on Phase 0 (Optionality) and Phase 1 (Validation).

### 2.1 Serializable IR with Computed Layout

**Goal**: Create a typed, serializable IR that includes computed layout
information (bit offsets, sizes) and enables external generator plugins.

**Current State**: `core::*` types are the IR but lack:

- Serde serialization
- Computed fields (bit offsets, sizes)
- Stable schema for external tools

**What Changes**:

1. **Serde derives**: Add `#[derive(Serialize, Deserialize)]` to all core types.
2. **Computed layout**: Add layout computation pass that calculates:
   ```rust
   pub struct ComputedField {
       pub field: Field,
       pub bit_offset: usize,    // Offset from message start
       pub bit_size: usize,      // Size in bits (computed from type + encoding)
   }

   pub struct ComputedMessage {
       pub message: Message,
       pub presence_bitmap_bits: usize,  // Number of optional fields
       pub total_bits: usize,            // Fixed size (if determinable)
       pub fields: Vec<ComputedField>,
   }
   ```
3. **JSON schema**: Document the IR JSON format for external tool authors.
4. **IR versioning**: Add schema version field for forwards compatibility.

**Key Decisions**:

- Whether to keep parse IR separate from generation IR (recommend: single IR
  with optional computed fields)
- IR JSON format: flat vs. nested (recommend: nested, mirrors Descriptor
  hierarchy)
- How to handle variable-size fields in layout computation

**Files to Add**: `src/ir/mod.rs`, `src/ir/layout.rs`
**Files to Modify**: `src/core/*.rs` (add serde), `Cargo.toml` (add serde dep)

---

## Phase 3: Core Functionality

Depends on Phase 2 (IR with layout).

### 3.1 Serialize/Deserialize Code Generation

**Goal**: Generate actual serialization code, not just data classes. This is the
primary value of the project.

**Current State**: GDScript generator (`src/generate/lang/gdscript.rs`)
generates data classes but completely ignores the `encoding` field.

**Architecture Note**: Serialization code generation happens in **plugins**, not
in baproto itself. The `Generator` trait in `src/generate/generator.rs` is an
internal implementation detail (or vestigial code). Plugins consume the JSON IR
and implement their own code generation logic in whatever language they choose.

**What Plugins Must Generate**:

1. **Runtime libraries**: Each target language needs a runtime (~100-200 LOC):
   - `BitWriter`: Write N bits, variable-length integers, etc.
   - `BitReader`: Read N bits, variable-length integers, etc.
   - Endianness handling (recommend: little-endian for games)
2. **Encoding implementations**: Generate code for each encoding type:
   - `Bits(n)`: `writer.write_bits(value, n)`
   - `BitsVariable(max)`: Variable-length encoding
   - `FixedPoint(i, f)`: `writer.write_bits((value * 2^f) as int, i + f)`
   - `ZigZag`: `(n << 1) ^ (n >> 31)` transform
   - `Delta`: Store/compare with previous value
   - `Pad(n)`: `writer.write_bits(0, n)`
3. **Optionality handling**:
   - Write presence bitmap at message start
   - Skip optional fields not present
   - Write defaults for `OptionalWithDefault`

**Key Decisions**:

- Runtime library location: Inline in generated code vs. separate import
- Error handling strategy: Exceptions vs. Result types
- Buffer management: Pre-allocated vs. growable

**baproto's role**: Produce validated IR with computed layout. Plugins do the
rest.

---

## Phase 4: Tooling

Can be implemented in parallel after Phase 2 or Phase 3.

### 4.1 Wire Format Spec CLI Command

**Goal**: Output human-readable wire format documentation for debugging and
documentation.

**Depends on**: Phase 2 (IR with layout)

**What Changes**:

1. **New subcommand**: `baproto wire-format <schema.proto> [MessageName]`
2. **Output format**:
   ```
   Message: PlayerState
   Total size: 48 bits (6 bytes) + variable

   Presence bitmap: 2 bits (fields: health, position)

   Fields:
     [0:8]   id: uint8 (Bits(8))
     [8:16]  health: optional uint8 (Bits(8), presence bit 0)
     [16:48] position: optional Vec3 (presence bit 1)
       [0:16]  x: float (FixedPoint(8, 8))
       [16:32] y: float (FixedPoint(8, 8))
       [32:48] z: float (FixedPoint(8, 8))
   ```

**Files to Add**: `src/cmd/wire_format.rs`
**Files to Modify**: `src/cmd/mod.rs`, `src/main.rs`

---

### 4.2 Schema Versioning CLI Command

**Goal**: Check compatibility between schema versions to prevent breaking
changes.

**Depends on**: Phase 0 (Resolution, Optionality)

**What Changes**:

1. **New subcommand**: `baproto check-compat <old.proto> <new.proto>`
2. **Compatibility rules**:
   - Field indices must not change for existing fields
   - Field types must be compatible (widening OK: u8→u16, narrowing breaks)
   - New `required` fields break old readers
   - Removed fields should use `reserved` indices
   - Encoding changes that affect wire format break compatibility
3. **Output**:
   ```
   INCOMPATIBLE: 2 breaking changes found

   ERROR: Field 'health' index changed from 2 to 3
   ERROR: New required field 'armor' added (use optional instead)

   WARNING: Field 'name' encoding changed (Bits(8) → Bits(16))
   ```

**Key Decisions**:

- Whether to support `reserved` keyword for removed field indices
- Strictness levels: `--strict` vs. `--lenient`

**Files to Add**: `src/cmd/check_compat.rs`
**Files to Modify**: `src/cmd/mod.rs`, `src/main.rs`

---

### 4.3 Plugin System Refactor

**Goal**: Enable external generator plugins, similar to `protoc`.

**Depends on**: Phase 2 (Serializable IR)

**Architecture Note**: The JSON IR schema is the contract between baproto and
plugins. The `Generator` trait in `src/generate/generator.rs` is **not** part of
this contract—it's an internal Rust abstraction that plugins don't use. Plugins
can be written in any language; they just need to parse JSON and emit JSON.

**What Changes**:

1. **CLI flags**:
   ```
   baproto compile --plugin=path/to/generator schema.proto
   baproto compile --out-ir=schema.json schema.proto  # IR only, no codegen
   ```
2. **Plugin protocol**:
   - baproto writes IR JSON to plugin's stdin
   - Plugin writes output JSON to stdout:
     ```json
     {
       "files": {
         "path/to/output.gd": "file contents...",
         "path/to/other.gd": "file contents..."
       }
     }
     ```
   - baproto reads output and writes files
3. **Remove built-in generators**: The existing GDScript generator moves to a
   separate `baproto-gdscript` repository. baproto becomes IR-only.

**Key Decisions**:

- IR transport: stdin/stdout (recommend) vs. temp file vs. named pipe
- Plugin discovery: Explicit path only vs. PATH search vs. plugin directory
- Error handling: Plugin exit codes, stderr handling

**Files to Add**: `src/generate/plugin.rs`
**Files to Modify**: `src/cmd/compile.rs`
**Files to Remove**: `src/generate/lang/*`, `src/generate/generator.rs` (after
migration)

---

## Phase 5: Language Support

Depends on Phase 2 (IR), Phase 4.3 (Plugin System).

**Note**: Each language generator is a **separate repository** that consumes
baproto's JSON IR via stdin and produces generated code. These implement the
serialization requirements defined in Phase 3.

### 5.1 GDScript Language Generator

**Goal**: Migrate existing GDScript generator to a plugin and add serialization.

**Architecture Note**: This is a **separate project** (`baproto-gdscript`). The
current `src/generate/lang/gdscript.rs` gets migrated out and rewritten to:
1. Consume JSON IR from stdin (instead of using Rust Generator trait)
2. Generate serialize/deserialize methods (currently missing)
3. Depend on a GDScript runtime library for BitReader/BitWriter

**Repository**: `baproto-gdscript` (based on `godot-plugin-template`)

---

### 5.2 Rust Language Generator

**Goal**: Generate Rust structs with serialization.

**Architecture Note**: This is a **separate project** (`baproto-rust`), not part
of the baproto repository. It's a plugin binary that consumes JSON IR from stdin
and emits generated Rust code.

**What the Plugin Generates**:

1. **Type mappings**:
   | Schema Type | Rust Type |
   | ----------- | --------- |
   | `uint8` | `u8` |
   | `int32` | `i32` |
   | `float32` | `f32` |
   | `string` | `String` |
   | `[T]` | `Vec<T>` |
   | `[K]V` | `HashMap<K, V>` |
   | `optional T` | `Option<T>` |
2. **Generated code structure**:
   ```rust
   #[derive(Debug, Clone, PartialEq)]
   pub struct PlayerState {
       pub id: u8,
       pub health: Option<u8>,
       pub position: Option<Vec3>,
   }

   impl PlayerState {
       pub fn serialize(&self, writer: &mut BitWriter) -> Result<()> { ... }
       pub fn deserialize(reader: &mut BitReader) -> Result<Self> { ... }
   }
   ```
3. **Runtime crate**: `baproto-runtime` crate with `BitReader`/`BitWriter`.

**Repository**: `baproto-rust` (separate from `baproto`)

---

## Phase 6: Advanced Encodings

Depends on Phase 3 (Serialize/Deserialize).

### 6.1 Quantized Floats

**Goal**: Support range-limited floats encoded in fewer bits.

**What Changes**:

1. **Parser syntax**:
   ```
   float32 rotation @Quantized(0.0, 360.0, 10);  // 10 bits, 0-360 range
   ```
2. **Encoding**: Extend `Encoding` enum:
   ```rust
   Quantized { min: f64, max: f64, bits: usize }
   ```
3. **Serialization**:
   ```
   normalized = (value - min) / (max - min)
   quantized = (normalized * (2^bits - 1)) as uint
   writer.write_bits(quantized, bits)
   ```

---

### 6.2 Dead Reckoning Support

**Goal**: Only transmit values when they differ from predicted values.

**What Changes**:

1. **Parser syntax**:
   ```
   message PlayerState {
     Vec3 position = 1;
     Vec3 velocity = 2;
     Vec3 predicted_position = 3 @DeadReckoning(position, velocity, 0.1);
   }
   ```
2. **Runtime support**:
   - Track previous values per field
   - Compute predicted value: `predicted = previous + velocity * dt`
   - Compare with threshold: `if abs(actual - predicted) > threshold`
   - Transmit delta or full value based on prediction accuracy
3. **State management**: Runtime must maintain prediction state.

**Key Decisions**:

- State ownership: Per-message instance vs. external state store
- Prediction function: Built-in linear vs. custom callbacks

---

## Phase 7: Quality Assurance

Ongoing, starts after Phase 3.

### 7.1 Integration Tests

**Goal**: End-to-end validation of parse → generate → compile → roundtrip.

**What Changes**:

1. **Test harness per language**:
   ```
   tests/
     integration/
       gdscript/
         test_roundtrip.gd
       rust/
         test_roundtrip.rs
   ```
2. **Test cases**:
   - All scalar types
   - Nested messages
   - Enums with variants
   - Optional fields with/without values
   - Arrays and maps
   - All encoding types
3. **CI integration**: Run integration tests on PR.

**Files to Add**: `tests/integration/`

---

## Dependency Graph

```
Phase 0 (parallel)
├── 0.1 Rooted Paths
├── 0.2 Resolution Pass ──────────────┐
└── 0.3 Field Optionality ────────────┤
                                      │
Phase 1                               │
└── 1.1 Encoding Validator ◄──────────┘
              │
Phase 2       │
└── 2.1 IR with Layout ◄──────────────┘
              │
              ├────────────────────────────────────────────┐
Phase 4       │                                            │
├── 4.1 Wire Format CLI ◄──────────────────────────────────┤
├── 4.2 Schema Versioning CLI (needs 0.2, 0.3)             │
└── 4.3 Plugin System ◄────────────────────────────────────┘
              │
Phase 5 (implements Phase 3 requirements)
├── 5.1 GDScript Generator ◄───────────────────────────────┘
└── 5.2 Rust Generator ◄───────────────────────────────────┘
              │
Phase 6       │
├── 6.1 Quantized Floats ◄─────────────────────────────────┘
└── 6.2 Dead Reckoning ◄───────────────────────────────────┘
              │
Phase 7       │
└── 7.1 Integration Tests ◄────────────────────────────────┘
```

**Note**: Phase 3 describes the serialization requirements that plugins must
implement. The actual implementation happens in Phase 5 (language-specific
plugin repositories).

---

## Recommended Implementation Order

1. **0.1 Rooted Paths** - Quick win, improves usability immediately
2. **0.2 Resolution Pass** - Critical for catching errors early
3. **0.3 Field Optionality** - Foundational for wire format
4. **1.1 Encoding Validator** - Catches more errors
5. **2.1 IR with Layout** - Enables everything else
6. **4.3 Plugin System** - Required before any generator work
7. **5.1 GDScript Generator** - Migrate existing code to plugin, add ser/de
8. **7.1 Integration Tests** - Validate the above works
9. **5.2 Rust Generator** - Second language
10. **4.1 Wire Format CLI** - Debugging tool
11. **4.2 Schema Versioning CLI** - Production safety
12. **6.1 Quantized Floats** - Encoding extension
13. **6.2 Dead Reckoning** - Advanced feature

**Note**: Phase 3.1 (Serialize/Deserialize) describes what plugins must generate,
not work in the baproto repo itself. That work happens in Phase 5 (language
generator plugins).

---

## Runtime Library Strategy

Each target language requires a small runtime library (~100-200 LOC) containing:

- `BitWriter`: Buffer + bit-level write operations
- `BitReader`: Buffer + bit-level read operations
- Variable-length integer encoding/decoding
- Endianness handling

**Recommended approach**: Separate repository per language using appropriate
package managers:

| Language | Repository               | Package        |
| -------- | ------------------------ | -------------- |
| GDScript | `baproto-gdscript`       | Godot addon    |
| Rust     | `baproto-runtime`        | crates.io      |
| C++      | `baproto-cpp`            | CMake/vcpkg    |

The `baproto` compiler itself only generates code; it does not bundle runtimes.
Generated code imports the runtime as a dependency.

---

## Progress Checklist

### Phase 0: Foundation

- [x] 0.1 Migrate to Rooted Paths
- [ ] 0.2 Resolution Pass: Validate Type References
- [ ] 0.3 Field-Level Optionality Encoding

### Phase 1: Validation

- [ ] 1.1 Encoding Validator: Check Encoding/Type Compatibility

### Phase 2: Intermediate Representation

- [ ] 2.1 Serializable IR with Computed Layout

### Phase 3: Core Functionality

- [ ] 3.1 Serialize/Deserialize Code Generation (plugin requirements)

### Phase 4: Tooling

- [ ] 4.1 Wire Format Spec CLI Command
- [ ] 4.2 Schema Versioning CLI Command
- [ ] 4.3 Plugin System Refactor

### Phase 5: Language Support

- [ ] 5.1 GDScript Language Generator (separate repo)
- [ ] 5.2 Rust Language Generator (separate repo)

### Phase 6: Advanced Encodings

- [ ] 6.1 Quantized Floats
- [ ] 6.2 Dead Reckoning Support

### Phase 7: Quality Assurance

- [ ] 7.1 Integration Tests
