# Architecture refactor

This document captures an architectural review of `build-a-proto` (the `baproto`
compiler) and its only real backend, `baproto-gdscript` in
`coffeebeats/godot-plugin-baproto`. It records how the compiler is wired today,
how the reference IDL compilers are architected, what the target architecture
should look like, and a sorted checklist of tasks to get there.

It is written so that someone (or some model) without the surrounding
conversation can pick up any task and do the right thing. Task entries give
context and justification rather than line-level instructions.

## 1. Goals and scope decisions this refactor serves

The refactor is not an end in itself. It exists to unlock a specific feature
set that was decided before this document was written. The decisions are
summarised here because several tasks depend on them.

### 1.1 Vision

`baproto` is one IDL with **two encoding modes**, serving two planes of a
multiplayer game:

| Plane | Mode | What matters |
|---|---|---|
| Realtime state sync (snapshots, inputs) | `packed`: positional, declaration order, no field tags, bit-exact | Bandwidth. Both ends run the same schema version, enforced by a schema hash exchanged at handshake. |
| Service / RPC (account actions, async PvP, matchmaking) | `tagged`: field indices on the wire, unknown fields skippable | Schema evolution. Client and server deploy independently. Bandwidth is nearly irrelevant here. |

Mode is a **per-message** (or per-encoding-declaration) property, never a
per-field one. A message whose fields are partly tagged and partly positional
has no coherent wire format. This mirrors FlatBuffers `table` (evolvable)
versus `struct` (fixed positional) and the book's split of *structure*,
*encoding*, and *language mapping* ("Development and Deployment of Multiplayer
Online Games", Vol. II, IDL chapters).

### 1.2 Keep / cut decisions

| Item | Decision | When to revisit |
|---|---|---|
| Field indices | Keep as `tagged` mode, message-level, required there and forbidden in `packed`. Cut *optional per-field* indices. | Now, with defined skip semantics. |
| Positional encoding | Keep as `packed` mode plus schema hash for handshake. | Now. |
| Structure / encoding / mapping split | Build it. This is the mechanism that gives flexibility without feature sprawl. | Now. |
| `range`, `quantize`, `optional`, `varint`, `smallest_three`, `vec2`/`vec3`/`quat` | Add, driven by the snapshot packer. | After the model and layout passes exist. |
| Baseline-relative encode / decode (delta against an acked baseline) | Add. | After the model and layout passes exist. |
| Maps on the wire | Cut. Wire is an array of key/value pairs (this is how protobuf encodes maps). `Dictionary` accessors can be language-mapping sugar later. | Mapping layer, later. |
| `delta`, `fixed_point`, `pad`, `bits(var(n))`, `bit`, `byte` scalars | Cut. Each is parsed and then dropped or unsupported by the backend. | Never. |
| `include` and flat packages | Keep. | Shared types across client and server. |
| Nested package directory trees and namespace scripts in the backend | Cut. | Never. |
| Services (`service` declaration: method ids, request/response pairing, envelope) | Add, small. | When the async PvP server exists. |
| Rust backend | Cut now. | When a Rust server exists, and only after the conformance corpus exists. |
| External plugin protocol (JSON over stdin/stdout) | Cut now. The library crate is the same door. | When a non-Rust backend author appears; the serialized IR plus a format version is then already the contract. |
| Six-target cross-compilation CI | Reduce to dev platforms plus Linux. | When a release is consumed by someone else. |

The guard against scope creep is a rule, not a list: **nothing enters the
language without a conformance fixture and a consumer that needs it.**

## 2. Current architecture

### 2.1 Facts that reframe the picture

- **The GDScript backend does not use the JSON plugin protocol.** It links
  `baproto` as a library at a pinned git revision, implements the in-process
  `Generator` trait, and calls `baproto::compile(files, roots, out, GDScript)`
  itself. The `ExternalGenerator` subprocess path has no consumer.
- **The Rust backend is a stub.** Its generated bodies are
  `todo!("serialization not yet implemented")`. The integration tests and
  golden files exercise only this stub.
- **The IR does not determine the wire.** The backend contradicts it: the IR
  says strings, arrays, and maps are `LengthPrefixed { prefix_bits: 32 }` but
  the backend always writes a LEB128 varint length; the IR computes an enum
  discriminant width of 8/16/32/64 bits but the backend writes a signed varint.
  `Transform::Delta`, `Transform::FixedPoint`, and `padding_bits` are never
  read by any backend. `WireFormat::Embedded` carries no information.
- **Field indices are decorative.** Uniqueness is checked, then declaration
  order is used for layout. Unindexed fields are silently dropped during
  lowering (`self.index.as_ref()?`).
- **Docs are stale.** `docs/commands.md` describes `--cpp | --gdscript` flags
  that do not exist; the CLI has `--rust | --plugin`.

### 2.2 Data flow

Every stage runs inside one recursive function, `Compiler::compile` in
`src/compile/compiler.rs`, once per file, depth-first through includes.

```
baproto compile --rust|--plugin BIN  -I ROOT  -o OUT  FILES     (src/cmd/compile.rs)
        |
        v
compile::compile(files, roots, out, generator)                   (src/compile/mod.rs)
        |  SchemaImport::try_from(path)   canonical .baproto path  (src/core/import.rs)
        v
Compiler::compile(import)   <- recursive, per file, DFS over includes
  1. SourceCache.insert          read text, keep Rc<String>          src/compile/source.rs
  2. lex::lex(text, import)      Vec<Spanned<Token>>   (chumsky)     src/lex/
  3. parse::parse(tokens)        ast::Schema           (chumsky)     src/parse/
  4. TypeCollector::register     Symbols: Descriptor -> TypeKind     src/compile/collect.rs
  5. for each include            resolve against roots, recurse to 1
  6. run_analyzers               FieldIndexUniqueness                src/analyze/field_index.rs
                                 TypeReferenceResolver(symbols)      src/analyze/type_reference.rs
  7. lower_and_merge             ast.lower(LowerContext) -> ir::Package,
                                 merged by package name into ir::Schema   src/ir/lower/
        |
        v  diagnostics printed via ariadne; any error aborts
ir::Schema { packages: [ Package { name, messages, enums } ] }
        |
        v
Generator::generate(&ir::Schema) -> GeneratorOutput { files: HashMap<PathBuf, String> }
   |- RustGenerator     -> Language<W> hook trait -> StringWriter -> todo!() bodies
   |- ExternalGenerator -> JSON on stdin, JSON on stdout, subprocess     (no consumer)
   '- GDScript          -> in godot-plugin-baproto, links the crate, implements Generator
        |
        v
files written under OUT
```

### 2.3 Representations crossing each boundary

| Boundary | Type | Notes |
|---|---|---|
| Text | `Rc<String>` in `SourceCache`, keyed by `SchemaImport` | |
| Tokens | `Vec<Spanned<Token, Span>>` | `Span = SimpleSpan<usize, SchemaImport>`; every span clones an `Rc<PathBuf>`. |
| AST | `ast::Schema { items: Vec<SchemaItem> }` | Items: `Package`, `Include`, `Message`, `Enum`, `CommentBlock`. Messages hold `MessageItem`s (`Field`, `Message`, `Enum`, `CommentBlock`). A field is `{ index: Option<FieldIndex>, kind: Type, encoding: Option<Encoding> }`. `Type` is `Scalar`, `Reference`, `Array`, `Map`. References are unresolved name paths. |
| Symbols | `Symbols<TypeKind>` = `HashMap<Descriptor, TypeKind>` | `Descriptor { package: PackageName, path: Vec<String> }`. This map is the entire semantic model. |
| IR | `ir::Schema { packages }` | `Message { descriptor, doc, fields, messages, enums }`; `Field { name, index, encoding: Encoding { wire: WireFormat, native: NativeType, transforms, padding_bits }, doc }`; `Enum { descriptor, discriminant: Encoding, variants }`. Type references are `NativeType::Message { descriptor }` with no lookup table. |
| Output | `GeneratorOutput { files: HashMap<PathBuf, String> }` | |

### 2.4 Module dependency graph

Arrows point at what a module imports.

```
        core  <----------------------------- everything
         ^
   lex --'      ast --> lex, core       parse --> lex, ast       visit --> ast

   analyze  --> visit, ast, compile::Symbols, compile::SourceCache, ir::lower::TypeKind
   compile  --> analyze, ast, core, lex, visit, generate::Generator,
                ir::lower::{Lower, LowerContext, TypeKind}
   ir::lower --> ast, core, ir     (defines TypeKind and TypeResolver, which compile
                                    and analyze both import)
   generate --> ir, core
```

Observations:

- `analyze` and `compile` import each other.
- `ir::lower` owns `TypeKind`, a semantic concept; the symbol collector and the
  reference checker reach into the lowering module to borrow it.
- `Diagnostic` lives in `analyze`; the `SourceCache` the reporter needs lives
  in `compile`; lex and parse errors are hand-converted into diagnostics
  inside the compiler; lowering has no diagnostics at all (it returns
  `Option`).
- There is no layer you can point to and say "this is the resolved program."

### 2.5 Module-by-module inventory (~13.2k lines)

| Module | Lines | Role today | Verdict |
|---|---|---|---|
| `core/` | ~900 | `Descriptor`, `PackageName`, `ImportRoot`, `SchemaImport` | Keep `PackageName` validation and import-root resolution. `Descriptor` is overloaded (see 3.6). |
| `lex/` | ~1.7k | chumsky lexer, `Token`, `Keyword` (incl. unused `encoding`), `Span` | Keep. Make `Span` a value type over a `FileId`. |
| `parse/` | ~3.3k | chumsky combinators, one file per construct | Keep. |
| `ast/` | ~500 | Spanned nodes | Keep shapes. Reconsider `CommentBlock` as an item at every level. |
| `visit/` | ~250 | `Visitor`/`Visitable`/`walk` macros over 19 node types | Optional. Used by three visitors. Harmless to keep. |
| `analyze/` | ~660 | `Analyzer` trait, two analyzers, `Diagnostic`, ariadne reporter | Keep diagnostics and reporter; move them. Analyzers become passes over the semantic model. |
| `compile/` | ~1.1k | `Compiler` (DFS driver), `TypeCollector`, `Symbols` (scope-walking resolve), `SourceCache` | Keep the resolve algorithm. Replace the DFS driver with whole-program passes. |
| `ir/` | ~260 data + ~3.0k lowering | IR types; `Lower` trait generic over `TypeResolver` with a `MockResolver` for tests | Redefine IR as descriptor + layout. Lowering becomes a plain function over the model. |
| `generate/` | ~1.2k | `Generator` trait, `RustGenerator`, `ExternalGenerator`, `Language<W>`, `Writer`/`FileWriter`/`StringWriter`, `CodeWriter` | Keep `Generator` and `CodeWriter`. Delete the rest. |
| `cmd/`, `main.rs` | ~120 | clap CLI | Keep, thin. |

## 3. What is unnatural, and why

Each item names the symptom, the cause, and what the reference compilers do
instead.

### 3.1 Phases interleave per file instead of running as whole-program passes

`Compiler::compile` lexes, parses, collects, recurses into includes, analyzes,
and lowers one file before touching the next. Consequences: a cross-file
reference resolves only when the target file is reachable through the current
file's includes; include cycles are guarded by a `processed` set but the
semantics are accidental; the IR is assembled incrementally with a merge step.

Protoc parses every file first and builds the descriptor pool second. This
DFS is the root of the "how do I get from layer to layer" confusion: there is
no point at which a complete program exists before generation.

### 3.2 Name resolution happens twice, and the second time cannot fail loudly

`TypeReferenceResolver` resolves references and reports errors. Lowering then
resolves every reference again through the `TypeResolver` trait and returns
`None` on failure. Every `lower` returns `Option`, so an unresolved reference,
an unindexed field, or a package used as a type silently drops the field.

Protoc's `DescriptorBuilder` resolves once and stores the pointer. Resolution
belongs in a model that later passes read, never redo.

### 3.3 There is no semantic model between AST and IR

The IR is asked to be three things at once: a serializable descriptor for
plugins (protobuf's `FileDescriptorProto` role), the wire specification
(Cap'n Proto's laid-out schema role), and the in-memory structure generators
walk. It does none of them completely. Its doc comment says "self-describing,
serialization-first, no validation", yet it carries `WireFormat` and
`Transform`, yet leaves `Embedded`, prefix widths, and discriminant widths for
the backend to reinterpret. The GDScript backend has its own `collect.rs` to
rebuild a type index because the IR references types by dotted name with no
table.

### 3.4 Encoding semantics are applied but never checked

`ast::Encoding::apply_to_wire` folds annotations onto a per-scalar default with
no rule about which annotation is legal on which type. Nothing can say
"`bits(16)` is meaningless on `f32`" or "`zigzag` requires a signed integer".
The analyzer list has a `TODO` where this check should be. Protoc has a full
validate stage; Cap'n Proto errors in the node translator; flatc errors in the
parser.

### 3.5 Three generator abstractions, and the only real backend uses the simplest one

`Generator` (IR in, file map out) is the contract GDScript implements.
`Language<W>` with begin/end hooks, package-dependency ordering, and
`configure_writer` exists only for the stub Rust backend. `Writer`,
`FileWriter`, and `StringWriter` exist to feed `Language<W>`.
`ExternalGenerator` and its stdin/stdout protocol exist for a plugin nobody
wrote, and the protocol is unversioned.

What was actually built is flatc's architecture: in-process generators over a
library. That is a good choice. The protoc-style plugin layer is a second
architecture bolted alongside.

### 3.6 `Descriptor` does three jobs

It is the symbol-table key, the scope cursor that `push`/`pop` mutate during
traversal, and the IR's identity. Because the package boundary inside a dotted
path is not tracked, absolute resolution has to try every package-versus-path
split. Protoc uses plain full names plus a table. Cap'n Proto uses 64-bit ids
with names as metadata. An interned `TypeId` plus a display name is cheaper
and unambiguous.

### 3.7 Spans are heavy and diagnostic plumbing is scattered

Each `Span` clones an `Rc<PathBuf>`. The conventional shape is a `FileId`
integer plus a byte range, with one `SourceMap` owning paths and text (rustc,
codespan, ariadne's own `Cache`). That also lets `Diagnostic`, the reporter,
and the source cache live together instead of across two mutually-importing
modules.

### 3.8 Files are merged into packages before generation

`lower_and_merge` erases which file declared what. Protoc, capnpc, and flatc
all keep file identity and generate per input file. The Godot
`EditorImportPlugin` runs the compiler per `.baproto` file, so the merge is
undone downstream anyway.

### 3.9 Comment blocks are AST items at every level

`SchemaItem::CommentBlock`, `MessageItem::CommentBlock`, and
`EnumItem::CommentBlock` force a `_ => {}` arm in every consumer, while doc
comments ride separately on `comment: Option<Comment>`. Cap'n Proto attaches
a doc comment to the following declaration and drops free-standing comments.

### 3.10 Lowering is generic over a resolver trait for the sake of a mock

`Lower<'a, T, Ctx>`, `LowerContext<'a, R: TypeResolver<TypeKind>>`,
`FieldTypeContext<'a, 'b, R>`, and `MockResolver` exist so lowering can be
unit-tested without a symbol table. With a concrete resolved model,
`fn lower(&Model) -> ir::Schema` is a plain function and none of this is
needed.

### 3.11 The public API leaked rather than being designed

`lib.rs` re-exports `core::*`, the `Writer` family, `Language`, and
`CodeWriter`. The backend's API surface is whatever escaped. The backend also
re-exposes the entire CLI (`generate -o -I FILES`) because it must call
`baproto::compile` to get an IR.

### 3.12 What is fine and should survive

The chumsky lexer and parser, the spanned AST shapes, `PackageName`
validation, the scope-walking resolution algorithm in `Symbols::resolve`,
ariadne reporting, `CodeWriter`, the `Generator` trait, and the Given/When/Then
test discipline.

## 4. Reference architectures

All three share one shape: parse each file to an unresolved tree, build a
single whole-program resolved model in a dedicated step, compute whatever
layout the wire needs inside the compiler, then hand a finished, serializable
schema to generators.

### 4.1 protobuf (`protoc`)

- `compiler/parser.cc`: hand-written recursive-descent tokenizer and parser.
  Output is a `FileDescriptorProto`, itself a protobuf message. There is no
  separate AST type; names are unresolved strings.
- `compiler/importer.cc`: `Importer`, `SourceTree`, `DiskSourceTree` resolve
  imports against roots; `DescriptorDatabase` abstracts where descriptors come
  from.
- `descriptor.cc`: `DescriptorPool::BuildFile` runs `DescriptorBuilder`, which
  collects symbols, cross-links type names (scope walking, like baproto's
  `Symbols::resolve`), validates, and interprets options. Output is the
  in-memory `FileDescriptor` graph with resolved pointers.
- `code_generator.h`: `CodeGenerator::Generate(const FileDescriptor*, parameter, GeneratorContext*, error)`.
  In-process generators for the core languages.
- `plugin.proto`: `CodeGeneratorRequest { file_to_generate, parameter, proto_file (all files incl. deps), compiler_version }`
  and `CodeGeneratorResponse { file { name, insertion_point, content } }`. The
  protocol is versioned by being a proto.
- Wire encoding is fixed and not represented in descriptors.

Lesson: two representations, one serializable and unresolved
(`FileDescriptorProto`), one in-memory and cross-linked (`FileDescriptor`).
"Build" is a distinct, whole-program step with its own validation.

### 4.2 Cap'n Proto (`capnpc`)

- `compiler/lexer.capnp`, `grammar.capnp`, `lexer.c++`, `parser.c++`: lexer
  output and parsed declarations are Cap'n Proto structs.
- `compiler/compiler.c++`, `node-translator.c++`: `NodeTranslator` turns
  declarations into `schema.capnp` `Node`s, assigning 64-bit ids, resolving
  names through a scope tree, and running `StructLayout` to assign bit offsets
  to every field (including unions and version-stable evolution).
- `capnp compile -o <lang>` sends a `CodeGeneratorRequest { nodes, requestedFiles, sourceInfo }`
  to `capnpc-<lang>` plugin binaries over stdin.

Lesson: this is the closest model to what baproto wants. Layout is computed at
compile time and stored in the schema, so every field carries its offset and
backends never reinvent the wire.

### 4.3 FlatBuffers (`flatc`)

- `src/idl_parser.cpp`: one hand-written `Parser` building `StructDef`,
  `FieldDef`, `EnumDef` in `SymbolTable`s. Resolution, validation, and layout
  (field offsets, vtable slots, alignment, padding) happen during parsing.
- `idl_gen_*.cpp`: all generators in-process, each a `BaseGenerator` over the
  `Parser` object. No plugin protocol.
- `reflection/reflection.fbs`: the binary schema (`.bfbs`) is an optional
  serialized output; newer generators (`bfbs_gen_*.cpp`) consume it.
- Two structure kinds: `table` (vtable-indexed, evolvable, optional fields)
  versus `struct` (inline, fixed layout, no evolution). This is the
  `tagged`/`packed` split.

Lesson: in-process generators over a rich in-memory model is a respectable
architecture, and `table` versus `struct` proves the two-mode idea.

### 4.4 Comparison

| | Parse output | Resolved model | Layout in compiler | Generator interface |
|---|---|---|---|---|
| protoc | `FileDescriptorProto` | `DescriptorPool::BuildFile`, cross-linked, validated | No (fixed wire) | In-process `CodeGenerator`; plugins get all files plus `file_to_generate` |
| capnpc | Cap'n Proto structs | `NodeTranslator` to `schema.capnp` `Node`s with ids | Yes, bit offsets | `CodeGeneratorRequest` to `capnpc-<lang>` |
| flatc | In-memory `Parser` | Same object, resolved during parse | Yes, offsets, vtables | In-process `BaseGenerator`; optional `.bfbs` |
| baproto today | `ast::Schema` per file | `Descriptor -> TypeKind` map | Partially, and the backend overrides it | In-process `Generator`, plus unused JSON subprocess path |

## 5. Target architecture

Four representations, each produced by one pass, each with one job, and
dependencies pointing only downward.

```
 .baproto files
      |  load: resolve include graph from roots, assign FileId, parse ALL files first
      v
 syntax::Ast (per file, spans, unresolved names)                    syntax/
      |  collect: every message and enum gets a TypeId, full name, file, span
      |  resolve: every Reference becomes a TypeId (scope walk), recorded once
      |  check:   indices, encodings vs types, cycles, mode rules, reserved
      v
 sema::Model (whole program, resolved, still has spans)             sema/
      |  layout: pure function of the model and each message's mode
      |          -> bit widths, prefix widths, discriminant widths, presence bits,
      |             tag widths, schema hash
      v
 ir::Schema (serializable, versioned, spans dropped)                ir/
      |  files[], types[] by TypeId with full names, wire fully specified
      v
 Generator::generate(&ir::Schema, request) -> Vec<GeneratedFile>      codegen/
```

### 5.1 Module layout

```
src/
  source/        FileId, SourceMap, Span { file, start, end }, ImportRoot resolution
  diagnostics/   Diagnostic, Severity, Diagnostics (collector), ariadne emitter
  syntax/        token.rs, lexer.rs, ast.rs, parser.rs        (today's lex/, parse/, ast/)
  sema/          model.rs   TypeId, TypeDef, FieldDef, TypeRef, Encoding, Mode
                 collect.rs resolve.rs check/{index,encoding,cycle}.rs
  layout/        wire.rs    WireSpec per field, discriminant, schema hash
  ir/            schema.rs  the contract; built from (&Model, &Layout); format_version
  codegen/       Generator trait, GeneratedFile, CodeWriter
  driver.rs      compile(request) -> Result<ir::Schema, Diagnostics>
  main.rs        CLI, thin
```

Dependency rule: `source <- diagnostics <- syntax <- sema <- layout <- ir <- codegen <- driver`.
No upward references. `TypeKind` lives in `sema`.

### 5.2 Design decisions

- **Whole-program passes replace the per-file DFS.** The driver loads every
  file reachable from the inputs, then runs collect, resolve, check, layout,
  and lower over all of them. Cross-file references and include cycles stop
  being special cases.
- **Every pass returns diagnostics, never `Option`.** Collect, resolve, and
  check append to one `Diagnostics` collector. Lowering runs only on a clean
  model and cannot fail. This is protoc's pool-build contract.
- **The model is the only place resolution lives.** `TypeRef::Named(TypeId)`
  is set once by the resolve pass. Checks, layout, lowering, and generators all
  read it.
- **The IR is descriptor plus layout, and it is versioned.** Each field carries
  a `WireSpec` that says exactly what goes on the wire. Each message carries
  its mode and schema hash. A backend that emits anything the IR did not
  specify is a backend bug. This is the property the conformance corpus
  depends on. It is Cap'n Proto's `StructLayout` idea applied to bit streams.
- **Generation is in-process, library-first.** `baproto::compile` returns an
  IR and the Godot binary stays a thin `Generator` over it, which is what it
  already is. When a non-Rust backend arrives, the serialized IR plus
  `format_version` is already the contract.
- **Files stay in the IR.** Generators default to one output per input file
  like protoc; the GDScript backend can keep splitting per type.

### 5.3 Sketch of the core types

These are shapes, not final signatures. Names are suggestions.

```rust
// source/
pub struct FileId(u32);
pub struct Span { file: FileId, start: u32, end: u32 }
pub struct SourceMap { files: Vec<(PathBuf, String)> }

// sema/model.rs
pub struct TypeId(u32);
pub struct Model {
    files: Vec<FileInfo>,             // path, package, includes, declared TypeIds
    types: Vec<TypeDef>,              // arena; TypeId indexes into it
}
pub enum TypeDef { Message(MessageDef), Enum(EnumDef) }
pub struct MessageDef { name: FullName, file: FileId, mode: Mode, fields: Vec<FieldDef>, span: Span, doc: Option<String> }
pub enum Mode { Packed, Tagged }
pub struct FieldDef { name, index: Option<u32>, ty: TypeRef, encoding: EncodingSpec, span, doc }
pub enum TypeRef { Scalar(Scalar), Named(TypeId), Array { elem: Box<TypeRef>, len: ArrayLen }, /* later: Optional, Vec3, Quat */ }

// layout/
pub enum WireSpec {
    Fixed { bits: u32 },
    Varint { max_bits: u32 },
    Quantized { min: f64, max: f64, bits: u32 },
    LengthPrefixed { len_bits: u32, elem: Box<WireSpec> },
    FixedArray { len: u32, elem: Box<WireSpec> },
    Optional { inner: Box<WireSpec> },              // 1 presence bit
    Embedded { ty: TypeId },
    Tagged { tag_bits: u32, inner: Box<WireSpec> }, // tagged mode only
}

// ir/schema.rs   (serde; the contract)
pub struct Schema { format_version: u32, files: Vec<File>, types: Vec<Type> }
pub struct Message { id: TypeId, full_name: String, file: usize, mode: Mode, schema_hash: u64, fields: Vec<Field> }
pub struct Field { name, index: Option<u32>, native: NativeType, wire: WireSpec, doc }
```

### 5.4 Conformance corpus

Independent of any backend: a directory of `.baproto` schemas, each paired
with sample values and the expected wire bytes (hex) plus bit length. The Rust
crate owns an in-memory reference encoder that computes expected bytes from
the IR and a value tree. Every backend runs the corpus. This is the only test
that proves the IR determines the wire.

## 6. Task checklist

Sorted by dependency and by value. Each task is meant to be one PR. Earlier
tasks are mostly deletion and mechanical moves; later ones change how the code
reads. `cargo test` should stay green after each.

Conventions for whoever implements these: follow `AGENTS.md` (comment
headers, Given/When/Then tests, doc-comment style). Keep the chumsky lexer and
parser. Do not add features to the language until task 12.

### Phase A: remove the second architecture

- [ ] **A1. Delete the external plugin path.** Remove `ExternalGenerator`,
  the `--plugin` CLI flag, `is_executable` dependency, and the JSON protocol
  docs. *Why:* no consumer exists; the only backend links the crate. The
  serialized IR with a format version (task C3) is the future plugin contract
  if one is ever needed.
- [ ] **A2. Delete `Language<W>` and the `Writer` family.** Remove
  `generate/language/`, `generate/write/`, and the `RustGenerator` that uses
  them. Keep `Generator`, `GeneratorOutput`, and `CodeWriter`. Remove the Rust
  golden tests and goldens (they test a `todo!()` stub). *Why:* three
  generator abstractions for one real backend. If a Rust backend returns
  later it will be written against the new IR.
- [ ] **A3. Fix stale docs.** Rewrite `docs/commands.md` to match the actual
  CLI. Replace the README "How it works" TODO with a short pipeline summary
  pointing at this document. *Why:* three different CLI surfaces are
  documented or implemented today.
- [ ] **A4. Reduce CI matrix.** Keep the developer platforms plus Linux for
  the cross builds; drop the rest. *Why:* release engineering has consumed
  most recent commits; nobody consumes the other targets.

### Phase B: straighten the plumbing

- [ ] **B1. Introduce `FileId` and `SourceMap`; make `Span` a value type.**
  Replace `SimpleSpan<usize, SchemaImport>` with `Span { file: FileId, start, end }`.
  `SourceMap` owns paths and text and implements ariadne's `Cache`. Keep
  `ImportRoot` resolution. *Why:* spans currently clone an `Rc<PathBuf>`;
  source cache and diagnostics are split across modules that import each
  other. This also removes `SchemaImport` from the lexer's type signatures.
- [ ] **B2. Create a `diagnostics` module.** Move `Diagnostic`, `Severity`,
  and the ariadne reporter there; add a `Diagnostics` collector with
  `error(span, msg)` and `has_errors()`. Have lex and parse convert chumsky
  `Rich` errors into `Diagnostic` at their own boundary rather than in the
  compiler. *Why:* one place to accumulate and report; breaks the
  `analyze <-> compile` cycle.
- [ ] **B3. Merge `lex/`, `parse/`, `ast/` into `syntax/`.** Pure move; keep
  file granularity inside. Attach doc comments to the following declaration in
  the parser and drop `CommentBlock` as an item variant (keep a lexer token so
  spans and tests still work). *Why:* the three modules are one layer; the
  `CommentBlock` item forces a skip arm in every consumer.
- [ ] **B4. Split the DFS driver into whole-program passes.** Replace
  `Compiler::compile` with: load (resolve the include graph, parse every file,
  detect cycles explicitly), then run the existing collector, analyzers, and
  lowering over *all* parsed files. Outputs must be unchanged. *Why:* section
  3.1. This is a behaviour-preserving step that makes task C1 possible.

### Phase C: introduce the semantic model

- [ ] **C1. Add `sema::Model` and the collect/resolve passes.** Define
  `TypeId`, `TypeDef`, `FieldDef`, `TypeRef` (section 5.3). Move `Symbols` and
  its scope-walking `resolve` into `sema::resolve`. Collect assigns ids and
  full names; resolve rewrites every `Reference` to `TypeRef::Named(TypeId)`
  and reports unresolved or wrong-kind references. `TypeKind` moves here.
  Retire `Descriptor` as a mutable scope cursor; keep a `FullName` display
  type. *Why:* sections 3.2, 3.3, 3.6. After this there is exactly one answer
  to "where is the resolved program."
- [ ] **C2. Make lowering a plain function.** `fn lower(&Model, &Layout) -> ir::Schema`
  with no `Lower` trait, no `TypeResolver`, no `LowerContext`, no
  `MockResolver`, no `Option` returns. Port the lowering unit tests to build a
  small `Model` directly. *Why:* section 3.10; lowering must not be able to
  drop fields silently.
- [ ] **C3. Redefine the IR as descriptor plus layout.** Add `format_version`,
  a `files` list, a `types` arena indexed by `TypeId`, and `full_name` on
  every type. Replace `WireFormat`/`Transform`/`padding_bits` with a
  `WireSpec` that fully determines the bits (section 5.3). Remove
  `Map` from `NativeType` (arrays of pair messages are the wire; see 1.2).
  *Why:* section 3.3; this is the contract the conformance corpus and every
  backend depend on. Bump the format version whenever it changes.
- [ ] **C4. Add the check passes.** Field indices (uniqueness, plus the mode
  rules once C6 lands), encoding-versus-type legality (`bits(n)` only on
  integers and within native width; `zigzag` only on signed; `varint` only on
  integers), recursion without indirection, array length bounds, package
  declared before other items. Remove the corresponding ad-hoc checks from the
  collector. *Why:* section 3.4; today nothing rejects a nonsensical
  annotation.
- [ ] **C5. Add the layout pass.** A pure function from `Model` to per-field
  `WireSpec`, per-enum discriminant width (minimum bits for the variant
  count), and a per-message `schema_hash` (hash of the fully laid-out
  structure, stable across doc changes and field renames in packed mode).
  *Why:* the IR must state what the backend emits; the hash is the handshake
  guard for packed mode.
- [ ] **C6. Add message mode (`packed` | `tagged`).** Parser support for a
  per-message mode (the unused `encoding` keyword is the natural slot), model
  field, check rules (indices required in tagged, forbidden in packed), and
  tagged layout (tag width from max index; a length or wire-type prefix so
  unknown fields can be skipped). Default is `packed`. *Why:* section 1.1;
  this replaces optional per-field indices with a coherent choice.
- [ ] **C7. Define the public API.** `lib.rs` exports exactly: `compile(request) -> Result<ir::Schema, Diagnostics>`
  (with a `CompileRequest { files, import_roots }`), the `ir` types,
  `Generator`, `GeneratedFile`, `CodeWriter`, and `Diagnostics` rendering.
  Nothing from `syntax` or `sema`. *Why:* section 3.11; the backend should
  not need the CLI or internals.

### Phase D: prove the wire

- [ ] **D1. Build the conformance corpus.** `tests/conformance/*.baproto`
  each with a values file and expected bytes. Implement a reference encoder in
  the crate that walks the IR and a value tree. Golden-test the IR JSON for
  each schema as well. *Why:* section 5.4; without this no claim about
  bandwidth or compatibility is testable.
- [ ] **D2. Bring the GDScript backend onto the new IR.** Delete its own
  defaults (`collect.rs` type index, varint-for-everything lengths, varint
  discriminants) and make it obey `WireSpec` exactly. Delete nested-package
  directory and namespace generation (section 1.2). Run the conformance corpus
  through generated GDScript in CI (headless Godot). *Why:* this is the first
  moment the IR and a backend agree, and the first end-to-end test.
- [ ] **D3. Remove dead language features.** `delta`, `fixed_point`, `pad`,
  `bits(var(n))`, `bit`, `byte`, maps. Keep the parser able to give a good
  error for the removed spellings. *Why:* section 1.2; each is parsed then
  dropped or unsupported.

### Phase E: features from the keep column (in this order, each with fixtures)

- [ ] **E1. `optional T`** with a presence bit.
- [ ] **E2. `varint`** as an explicit encoding on integers.
- [ ] **E3. `range(min, max)`** on integers (bits derived from range) and
  **`quantize(min, max, bits)`** on floats.
- [ ] **E4. Bounded arrays** `[N]T` with the length bits derived from `N`,
  and a check that unbounded arrays are rejected in packed mode.
- [ ] **E5. Native vector types** `vec2`, `vec3`, `quat` with a
  `smallest_three` encoding for `quat`. Language mapping to Godot `Vector3`
  and `Quaternion` in the backend.
- [ ] **E6. Baseline-relative codec API.** `encode(writer, baseline)` and
  `decode(reader, baseline)` where a field equal to the baseline costs one
  bit; `delta(range)` for integers relative to the baseline. This is the
  codec's share of delta compression; acks and history stay in the sync layer.
- [ ] **E7. Envelope and type ids.** A generated registry mapping message
  `TypeId` to codec, and an envelope message with a type id field, so a
  packet can carry heterogeneous messages.
- [ ] **E8. `service` declarations.** Method ids, request and response
  pairing, generated dispatch table. No transport binding.

### Deferred, with the trigger that revisits them

- Rust backend: when a Rust server exists (after D1).
- Serialized-IR plugin protocol: when a non-Rust backend author appears
  (after C3 the contract exists; only the subprocess shim is missing).
- Map language-mapping sugar: when a service message needs a `Dictionary`
  accessor (after E8).
- GDExtension runtime: when GDScript codec throughput is the measured
  bottleneck.
