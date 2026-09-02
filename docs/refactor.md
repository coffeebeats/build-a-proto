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
| Embedding across modes (`packed` inside `tagged`, `tagged` inside `packed`) | Allow both. A tagged child inside a packed parent is byte-aligned and contributes a constant to the parent's hash; a packed child inside a tagged parent is a length-prefixed opaque payload. | Now, with the layout rules in 5.3. |
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
- **The backend re-does instruction selection.** `codec/wire.rs` in the
  backend pattern-matches on `(NativeType, WireFormat)` pairs, roughly twenty
  arms, and bails with "unsupported encoding combination" on anything else.
  `bits(12)` on an unsigned integer without `zigzag` is rejected even though
  the runtime has `write_bits`. The 64-bit arms emit calls to `write_u64` and
  `read_u64`, which the runtime does not define. Nothing compiles generated
  GDScript in CI, so neither was caught.
- **Docs are stale.** `docs/commands.md` describes `--cpp | --gdscript` flags
  that do not exist; the CLI has `--rust | --plugin`.

### 2.1.1 How the Godot plugin drives the compiler

The plugin's `EditorImportPlugin` (`import/plugin.gd`) runs once per
`.baproto` file Godot imports. It invokes
`baproto-gdscript generate -o <project-wide output dir> -I res:// <file>`,
the compiler emits every type reachable from that file, including types from
included files, and the importer then scans the entire output directory and
registers every `.gd` it finds as that one import's generated files. It also
writes a comment-only stub script as the import's own save file.

Consequences:

- Importing file A rewrites and claims file B's outputs. Harmless while the
  contents are identical, but ownership is wrong and cleanup on removal is
  undefined.
- No import ever sees the whole program, so whole-program artifacts (a type
  registry, a project-wide schema hash) cannot be generated correctly.
- Godot's importer does not track include dependencies, so editing B does not
  reimport A. Anything A's generated code copies from B's layout goes stale.

The backend also contains a full GDScript syntax tree and emitter
(`src/gdscript/ast/`, ~2.3k lines). That is a sound backend design; it means
the library's `CodeWriter` is used only for indent and comment tokens.

The types the backend actually imports from `baproto` today, which is the
de-facto public API: the whole `ir` module (`Schema`, `Package`, `Message`,
`Enum`, `Field`, `Variant`, `Encoding`, `NativeType`, `WireFormat`,
`Transform`), `Descriptor` and `DescriptorBuilder`, `PackageName`,
`Generator`, `GeneratorError`, `GeneratorOutput`, `CodeWriter` and its
builder, `Writer`, `StringWriter`, and `compile`.

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
| `cmd/`, `main.rs` | ~120 | clap CLI (`compile --rust\|--plugin`) | Keep, thin, and repurpose: after Phase A there is no in-crate backend, so `compile` goes and the binary becomes `check`, `ir --json`, and `encode`/`decode` (task C7). |

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
the backend to reinterpret.

Two further symptoms in the backend. It has a `collect.rs` whose job is to
flatten the IR's nested message tree into one entry per type with
`Outer_Inner` file stems, and a `types.rs` that computes relative preload
paths from `Descriptor` and detects "this is one of my nested types" by a
string-prefix test on the file stem. Both exist because the IR nests types as
vectors with no parent link and no flat table. And `WireSpec`-level facts
(signedness, zigzag, float versus integer) are split between `NativeType` and
`WireFormat`, so the backend must pattern-match on the pair to choose a
runtime primitive (section 2.1). The IR is not lowered far enough for a
backend to be a syntax-directed translation.

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

Note for whoever deletes the `Writer` family: `CodeWriter` is generic over
the `Writer` trait, and the GDScript backend's own emitter is written as
`emit<W: Writer>` over `StringWriter`. They cannot simply be removed; see
task A2.

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
 ir::Schema (serializable, versioned, spans kept as byte ranges)    ir/ + lower/
      |  files[], types[] by TypeId with full names, wire fully specified
      v
 Generator::generate(&ir::Schema, request) -> Vec<GeneratedFile>      codegen/
```

### 5.1 Module layout

```
src/
  ir/            LEAF. The shared vocabulary and the contract: FileId, TypeId, Span,
                 Mode, WireSpec, LenSpec, Schema, File, Type, Field, verify(),
                 format_version. Depends on serde only.
  source/        SourceMap (paths + text, implements ariadne Cache), ImportRoot resolution
  diagnostics/   Diagnostic, Severity, Diagnostics (collector), ariadne emitter
  syntax/        token.rs, lexer.rs, ast.rs, parser.rs        (today's lex/, parse/, ast/)
  sema/          model.rs   TypeDef, FieldDef, TypeRef (incl. Error), EncodingSpec
                 collect.rs resolve.rs check/{index,encoding,cycle,mode}.rs
  layout/        wire.rs    Model + Mode -> ir::WireSpec per field, layout_hash per type
  lower/         lower(&Model, &Layout) -> ir::Schema
  codegen/       Generator trait, GeneratedFile, GenerateRequest, CodeWriter
  driver.rs      compile(request) -> Result<ir::Schema, Diagnostics>
  main.rs        CLI: check | ir --json | encode | decode
```

Dependency rule: a DAG, not a chain. `ir` is a leaf that `sema`, `layout`,
`lower`, and `codegen` all import for vocabulary (`TypeId`, `Mode`,
`WireSpec`, `Span`). Above it the order is
`source <- diagnostics <- syntax <- sema <- layout <- lower <- codegen <- driver`
with no upward references. The earlier draft of this rule put `ir` above
`layout` and `sema`, which cannot hold: `WireSpec::Message { ty: TypeId }`
places a type id inside the serialized contract, the `check` pass needs
`Mode`, and a generator that reports an error needs a `Span`. Making `ir` the
leaf resolves all three and keeps the public API free of `sema` (task C7).
`TypeKind` lives in `sema`.

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
- **`WireSpec` is a closed instruction set, is self-sufficient, and is the
  only type tree on a field.** A backend must be able to pick the runtime
  primitive from the `WireSpec` alone. `f32` and `u32` must not both be "32
  fixed bits"; signedness and zigzag are variants, not side flags. Each
  variant maps to exactly one primitive in a runtime (`write_bits`,
  `write_varint`, `write_bytes`, and so on). There is no parallel
  `NativeType` tree: scalar leaves carry the host type as data
  (`Uint { bits, host: IntType }`) that a backend may read for its language
  mapping and may ignore for the wire. Two parallel trees would have to be
  kept structurally aligned by the verifier, which is the backend's
  `(NativeType, WireFormat)` pair matching relocated rather than removed.
  Composite features (`smallest_three`, baseline-relative fields) are
  expressed in existing variants where possible; a genuinely new variant
  requires a new runtime primitive and a conformance fixture. Enums are
  C-like; there is no per-variant payload.
- **Mode decides layout at the use site, not on the type.** Mode is a
  per-message property, so the wire form of an enum discriminant, an
  `optional`, an array length, and an embedded message is chosen by the
  layout pass from the *enclosing* message's mode. A packed message gets
  the minimum-width discriminant and a presence bit; a tagged message gets a
  varint discriminant and expresses optionality as tag absence, so adding an
  enum variant in the service plane is a compatible change. The table in 5.3
  states each case. Layout stored on the enum type itself is only what a
  packed reader needs.
- **Embedding across modes is allowed both ways.** A tagged child inside a
  packed parent starts on a byte edge and contributes a constant to the
  parent's `layout_hash`, so the child may evolve without breaking the
  parent's handshake. A packed child inside a tagged parent is a
  length-prefixed opaque payload whose hash is the child's own.
- **The IR is a flat arena, not a tree.** Types live in one `Vec<Type>`
  indexed by `TypeId`; nesting is `parent: Option<TypeId>` and
  `nested: Vec<TypeId>` (Cap'n Proto's `Node.scopeId`). Backends that want
  one file per type flatten for free; backends that want inner classes walk
  `nested`. The GDScript `collect.rs` and the prefix-string hack disappear.
- **The IR carries source locations, and there is one location type.**
  protoc keeps `SourceCodeInfo`, capnpc keeps `sourceInfo`. Backends produce
  real errors (a Godot `int` is 64-bit signed, so `u64` cannot round-trip)
  and the Godot importer shows compiler output to the user. Each type and
  field carries the same `Span { file: FileId, start, end }` the compiler
  uses; byte offsets are serializable and cheap, and line and column are the
  renderer's job. `Generator` returns `Diagnostics` with `Span`s, not
  `anyhow` strings, and the same renderer prints them.
- **Stable ids for the wire are not arena indexes.** `TypeId` is an arena
  index assigned by file order then declaration order, so inserting one type
  renumbers every type after it. Anything that goes on the wire (the E7
  envelope, the registry) uses `wire_id: u32`, a hash of the full name (or a
  declared id later), stored next to `TypeId` and checked unique by the
  verifier. Cap'n Proto assigns 64-bit ids for exactly this reason.
- **Passes tolerate a dirty model.** Resolve and check run after collect has
  reported errors, so the user sees every error in one compile. An
  unresolvable reference becomes `TypeRef::Error` (rustc's `TyKind::Error`)
  and later checks never report on an `Error` reference, which stops
  cascades. Lowering still runs only on a clean model.
- **There is an IR verifier.** LLVM has `Verifier`, rustc validates MIR. An
  `ir::verify(&Schema) -> Diagnostics` checks id ranges, `wire_id`
  uniqueness, that each `WireSpec` is well-formed for its position and mode,
  mode rules, and index rules. It runs in tests and before generation. It is cheap and protects every backend and the reference
  encoder.
- **Everything is deterministic.** `TypeId`s are assigned by input-file order
  then declaration order; output collections are ordered (`BTreeMap` or
  `Vec`). Godot reimports on every save, so unstable output means spurious
  churn.
- **Files stay in the IR, and generation takes a request.** protoc separates
  `file_to_generate` from the full file set.
  `GenerateRequest { files_to_generate: Vec<FileId> }` does the same:
  dependencies are visible, only the named files are emitted. protoc's
  stringly typed `parameter` is not copied; it exists because protoc cannot
  know a foreign plugin's options, whereas an in-process generator is
  constructed by its caller with typed config (`GDScript { runtime_path }`
  replaces the hardcoded `res://addons/baproto/runtime`).
- **Generated code never inlines another file's layout.** Godot's importer
  does not reimport A when an included B changes. So A's generated code calls
  B's codec for embedded fields and composes B's schema hash at runtime; it
  never copies B's field layout or hash constant. Under this rule per-file
  import cannot go stale.
- **An import compiles its include closure; the registry is a build step.**
  Each Godot import passes the imported file and everything it transitively
  includes as the compile set, with `files_to_generate` = the imported file
  only. It does not compile every `.baproto` under `res://`: that would give
  the registry one writer per schema file (Godot 4 can import on several
  threads, see `editor/import/use_multiple_threads`), fail every import when
  any one file has an error, and do quadratic work. Whole-program artifacts
  (the registry, E7) have exactly one writer: an `EditorPlugin` action and an
  `EditorExportPlugin._export_begin` hook that compile everything and emit
  `files_to_generate` = the registry.
- **Default output is one script per input file.** With `files_to_generate`
  the natural GDScript shape is one `.gd` per `.baproto` using inner classes,
  and that script may be the importer's own save file. Then
  `preload("res://schemas/game.baproto")` yields the script and
  `Game.Player.new()` works, with no output-directory setting, no `mod.gd`
  tree, no `gen_files`, no directory scan, and no orphan cleanup. The risk is
  developer experience, not loading: a script that lives under
  `.godot/imported/` is invisible to the script editor, the language server,
  and the global class cache, so autocompletion on `Game.Player`,
  go-to-definition, breakpoints in generated code, and `class_name` may all
  stop working. The spike (task D2) passes only if autocompletion on an inner
  class and a breakpoint in generated code both work; otherwise output stays
  a visible `.gd` per schema file and ownership is tracked by a manifest
  rather than a directory scan.
- **Versioning is of the crate, not only of the JSON.** The plugin's real
  contract is the Rust API it links. Crate releases are tagged with semver
  and the plugin pins a tag instead of a git revision. `format_version`
  governs the golden files and the future subprocess contract, and bumps on
  any change to `ir::Schema`.

### 5.3 Sketch of the core types

These are shapes, not final signatures. Names are suggestions.

```rust
// ir/   (leaf; serde; the contract and the shared vocabulary)
pub struct FileId(u32);
pub struct TypeId(u32);
pub struct Span { file: FileId, start: u32, end: u32 }
pub enum Mode { Packed, Tagged }
pub enum IntType { U8, U16, U32, U64, I8, I16, I32, I64 }
pub enum FloatType { F32, F64 }

// one variant == one runtime primitive; self-sufficient; the only tree on a field (5.2)
pub enum WireSpec {
    Bool,                                                   // 1 bit
    Uint { bits: u32, host: IntType },                      // fixed width, unsigned
    Int { bits: u32, host: IntType },                       // fixed width, two's complement
    ZigZag { bits: u32, host: IntType },                    // fixed width, zigzag-mapped
    Varint { signed: bool, host: IntType },                 // LEB128, zigzag if signed
    Float { bits: u32, host: FloatType },                   // 32 or 64, IEEE bit pattern
    Quantized { min: f64, max: f64, bits: u32, host: FloatType },
    Bytes { len: LenSpec },                                 // raw bytes
    Utf8 { len: LenSpec },                                  // string
    Array { len: LenSpec, elem: Box<WireSpec> },
    Optional { inner: Box<WireSpec> },                      // 1 presence bit (packed only; see table)
    Message { ty: TypeId },                                 // call that type's codec
    Enum { ty: TypeId, discriminant: Box<WireSpec> },       // discriminant chosen by the enclosing mode
}
pub enum LenSpec { Fixed(u32), Prefix { bits: u32 }, Varint }

pub struct Schema { format_version: u32, files: Vec<File>, types: Vec<Type> }
pub struct File { id: FileId, path: String, package: String, includes: Vec<FileId>, types: Vec<TypeId> }
pub enum Type { Message(Message), Enum(Enum) }
pub struct Message {
    id: TypeId, wire_id: u32, full_name: String, file: FileId,
    parent: Option<TypeId>, nested: Vec<TypeId>,
    mode: Mode, layout_hash: u64,       // own laid-out fields only; composed at runtime (C5)
    fields: Vec<Field>, span: Span, doc: Option<String>,
}
pub struct Enum {
    id: TypeId, wire_id: u32, full_name: String, file: FileId, parent: Option<TypeId>,
    layout_hash: u64, variants: Vec<Variant>, span: Span, doc: Option<String>,
}
pub struct Field { name: String, index: Option<u32>, wire: WireSpec, span: Span, doc: Option<String> }
pub fn verify(schema: &Schema) -> Diagnostics;

// source/
pub struct SourceMap { files: Vec<(PathBuf, String)> }   // indexed by FileId; ariadne Cache

// sema/model.rs   (internal; never exported)
pub struct Model { files: Vec<FileInfo>, types: Vec<TypeDef> }   // arena; TypeId indexes into it
pub enum TypeDef { Message(MessageDef), Enum(EnumDef) }
pub struct MessageDef { name: FullName, file: FileId, mode: Mode, fields: Vec<FieldDef>, span: Span, doc: Option<String> }
pub struct FieldDef { name, index: Option<u32>, ty: TypeRef, encoding: EncodingSpec, span, doc }
pub enum TypeRef {
    Scalar(Scalar), Named(TypeId), Array { elem: Box<TypeRef>, len: ArrayLen },
    Error,                            // set by resolve on failure; checks skip it
    /* later: Optional, Vec3, Quat */
}

// layout/
pub fn layout(model: &Model) -> Layout;   // per-field WireSpec, per-type layout_hash

// lower/
pub fn lower(model: &Model, layout: &Layout) -> ir::Schema;

// codegen/
pub struct GenerateRequest { files_to_generate: Vec<FileId> }
pub trait Generator {
    fn generate(&self, schema: &ir::Schema, req: &GenerateRequest) -> Result<Vec<GeneratedFile>, Diagnostics>;
}
```

`EncodingSpec` in `sema` is what the user wrote (`bits(12)`, `zigzag`,
`range(0, 100)`); `WireSpec` in `ir` is what the layout pass derived from it
and the enclosing mode. That difference is the reason `Model` and `Schema`
are separate types rather than one struct with an optional layout: the model
may churn freely, the IR is versioned.

Per-mode layout of the constructs whose wire form depends on the enclosing
message's mode:

| Construct | In a `packed` message | In a `tagged` message |
|---|---|---|
| Enum discriminant | `Uint { bits: ceil(log2(variants)) }` | `Varint { signed: false }` |
| `optional T` | `Optional { inner }`: 1 presence bit then `T` | tag absent means absent; no presence bit |
| Array length | `LenSpec::Fixed(N)` for `[N]T`; `Prefix { bits }` or `Varint` otherwise | `LenSpec::Varint` |
| Embedded packed message | inline, bit-exact, hash composed | length-prefixed opaque payload |
| Embedded tagged message | pad to byte edge, then the tagged shape below; hash constant | the tagged shape below |

Tagged mode wire shape (fixed by task C6b, stated here so the sketch is
complete). Tagged messages are byte-oriented. The message starts on a byte
edge (a packed parent pads before it). Each present field is
`LEB128 tag, LEB128 payload length in bytes, payload padded to a byte edge`;
the message ends with tag `0`. A reader skips an unknown tag with
`read_bytes(len)`, and a writer buffers an embedded message as one
`PackedByteArray` append. An earlier draft measured the length in bits and
kept payloads unaligned; that forced a size pass or a bit-offset splice on
every codec and a bit seek the runtime does not have, to save at most seven
bits per field on the plane where bandwidth is nearly irrelevant.

### 5.4 Conformance corpus

Independent of any backend: a directory of `.baproto` schemas, each paired
with sample values, the expected wire bytes (hex) plus bit length, and, when
decoding does not reproduce the input (unknown fields dropped), the expected
decoded values. The Rust crate owns an in-memory reference codec that
encodes a value tree to bytes and decodes bytes back to a value tree
according to the IR, in both directions, because half of what a backend can
get wrong is reading and the unknown-field fixture (C6b) is a decode test.
The value file format and its number rules (how a JSON number becomes an
`f32` or a `u64`) are part of `docs/wire.md`. The reference also computes
the composed `layout_hash` for each fixture so peers in other languages can
check their handshake value. Every backend runs the corpus. This is the only
test that proves the IR determines the wire.

The IR cannot express the bit-stream conventions themselves: bit order within
a byte (the GDScript runtime packs least-significant bit first), byte order of
fixed-width integers, float bit patterns, the varint form, string bytes
(UTF-8), what a length counts (bits, bytes, or elements), the byte
alignment rule for tagged messages, and the hash combine function (C5).
Today these live only in `runtime/writer.gd`. They belong in a
`docs/wire.md` that the reference codec implements and the corpus checks;
the document is the specification and the codec is its executable form.

## 6. Task checklist

Sorted by dependency and by value. Each task is meant to be one PR. Earlier
tasks are mostly deletion and mechanical moves; later ones change how the code
reads. `cargo test` should stay green after each.

Conventions for whoever implements these: follow `AGENTS.md` (comment
headers, Given/When/Then tests, doc-comment style). Keep the chumsky lexer and
parser. Do not add features to the language until Phase E. Two ordering
notes: message mode enters the syntax and model in C1b, before the checks and
layout that depend on it, while the tagged wire shape (C6b) lands after the
corpus (D1) so it is tested from the decode side on day one; and the IR JSON
goldens added in A2 are the pipeline's only end-to-end test until D1.

### Phase A: remove the second architecture

- [ ] **A1. Delete the external plugin path.** Remove `ExternalGenerator`,
  the `--plugin` CLI flag, `is_executable` dependency, and the JSON protocol
  docs. *Why:* no consumer exists; the only backend links the crate. The
  serialized IR with a format version (task C3) is the future plugin contract
  if one is ever needed.
- [ ] **A2. Delete `Language<W>`, `FileWriter`, and the `RustGenerator`.**
  Remove `generate/language/` and the Rust golden tests and goldens (they test
  a `todo!()` stub). Retarget `CodeWriter` from the `Writer` trait onto
  `std::fmt::Write` so a plain `String` is the sink, then delete `Writer`,
  `FileWriter`, and `StringWriter`. In the same PR add an IR JSON golden per
  schema in `tests/testdata` so the pipeline keeps an end-to-end test; B4
  diffs against these and C3 rewrites them deliberately. *Why:* three
  generator abstractions for one real backend, and the Rust goldens are the
  only test that runs the whole pipeline, so they must be replaced, not just
  removed. *Caution:* the GDScript backend's emitter is generic over
  `baproto::Writer` and uses `StringWriter` throughout; it pins a git revision
  so nothing breaks immediately, but the next bump must swap those for
  `String` and `fmt::Write`. Note in the changelog.
- [ ] **A3. Fix stale docs.** Rewrite `docs/commands.md` to match the actual
  CLI. Replace the README "How it works" TODO with a short pipeline summary
  pointing at this document. *Why:* three different CLI surfaces are
  documented or implemented today.
- [ ] **A4. Reduce CI matrix.** Keep the developer platforms plus Linux for
  the cross builds; drop the rest. *Why:* release engineering has consumed
  most recent commits; nobody consumes the other targets.

### Phase B: straighten the plumbing

- [ ] **B0. Compile generated GDScript in the plugin's CI.** In
  `godot-plugin-baproto`, generate output for a small fixture set and run
  headless Godot over it (at minimum a parse check; the runtime already has
  Gut tests to hang a smoke test on). *Why:* today the backend emits calls to
  runtime methods that do not exist (`write_u64`) and nothing notices. This is
  cheap, backend-only, and independent of every other task; it should land
  before the IR changes so D2 has a safety net.
- [ ] **B1. Introduce `FileId` and `SourceMap`; make `Span` a value type.**
  Replace `SimpleSpan<usize, SchemaImport>` with `Span { file: FileId, start, end }`.
  `FileId` and `Span` are defined in the leaf `ir` module (5.1) so every
  layer, the serialized IR included, shares one location type. `SourceMap`
  owns paths and text and implements ariadne's `Cache`. Keep `ImportRoot`
  resolution. *Why:* spans currently clone an `Rc<PathBuf>`;
  source cache and diagnostics are split across modules that import each
  other. This also removes `SchemaImport` from the lexer's type signatures.
- [ ] **B2. Create a `diagnostics` module.** Move `Diagnostic`, `Severity`,
  and the ariadne reporter there; add a `Diagnostics` collector with
  `error(span, msg)` and `has_errors()`. Have lex and parse convert chumsky
  `Rich` errors into `Diagnostic` at their own boundary rather than in the
  compiler. *Why:* one place to accumulate and report; breaks the
  `analyze <-> compile` cycle.
- [ ] **B3a. Merge `lex/`, `parse/`, `ast/` into `syntax/`.** Pure move; keep
  file granularity inside; no behaviour change. *Why:* the three modules are
  one layer.
- [ ] **B3b. Attach doc comments in the parser.** Attach a doc comment to the
  following declaration and drop `CommentBlock` as an item variant (keep a
  lexer token so spans and tests still work). Separate from B3a because it
  changes the AST and every consumer. *Why:* the `CommentBlock` item forces a
  skip arm in every consumer.
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
  and reports unresolved or wrong-kind references, leaving `TypeRef::Error`
  behind so later passes run without cascading errors. `TypeKind` moves
  here. Retire `Descriptor` as a mutable scope cursor; keep a `FullName`
  display type. *Why:* sections 3.2, 3.3, 3.6. After this there is exactly
  one answer to "where is the resolved program."
- [ ] **C1b. Add message mode to the syntax and the model.** Parser support
  for a per-message mode (the unused `encoding` keyword is the natural
  slot), `Mode` on `MessageDef`, default `packed`. No wire change yet.
  *Why:* the layout pass (C5) is a function of mode and the index rules (C4)
  depend on it, so mode must exist before either; the tagged wire shape
  itself waits for the corpus (C6b).
- [ ] **C2. Make lowering a plain function in its own module.**
  `lower::lower(&Model, &Layout) -> ir::Schema` with no `Lower` trait, no
  `TypeResolver`, no `LowerContext`, no `MockResolver`, no `Option` returns.
  `lower/` sits above `sema` and `layout`; the `ir` types it builds live in
  the leaf module and depend on nothing (5.1). Port the lowering unit tests to build a
  small `Model` directly. *Why:* section 3.10; lowering must not be able to
  drop fields silently.
- [ ] **C3. Redefine the IR as descriptor plus layout.** Add `format_version`,
  a `files` list keyed by `FileId`, a flat `types` arena indexed by `TypeId`
  with `parent`/`nested` links, `full_name`, `wire_id`, and a `Span` on every
  type and field. Replace `WireFormat`/`Transform`/`padding_bits`/`NativeType`
  with the single `WireSpec` tree of section 5.3, whose scalar leaves carry
  the host type and whose `Enum` variant carries its discriminant spec.
  Remove maps (arrays of pair messages are the wire; see 1.2). Assign
  `TypeId`s deterministically (file order, then declaration order) and use
  ordered collections everywhere. *Why:* sections 3.3 and 5.2; this is the
  contract the conformance corpus and every backend depend on. Bump the
  format version whenever it changes.
- [ ] **C3b. Add `ir::verify`.** A pure function from `&Schema` to
  `Diagnostics` that checks every `TypeId` and `FileId` is in range,
  `wire_id`s are unique, each `WireSpec` is well-formed for its position (a
  presence-bit `Optional` inside a tagged message is an error; a
  discriminant spec is an integer spec), tagged messages have unique indices
  and packed ones have none, and `nested`/`parent` are consistent. Request
  validation (`files_to_generate` naming real files) lives in `codegen`, not
  here. Run `verify` in every IR test and in the driver before generation. *Why:* section 5.2; the
  IR is a contract shared by the reference encoder and every backend, and a
  verifier is the standard way to keep an IR honest.
- [ ] **C4. Add the check passes.** Field indices (uniqueness; required in
  tagged, forbidden in packed, using the mode from C1b), encoding-versus-type
  legality (`bits(n)` only on
  integers and within native width; `zigzag` only on signed; `varint` only on
  integers), recursion without indirection, array length bounds, package
  declared before other items. Remove the corresponding ad-hoc checks from the
  collector. *Why:* section 3.4; today nothing rejects a nonsensical
  annotation.
- [ ] **C5. Add the layout pass.** A pure function from `Model` to per-field
  `WireSpec` and per-type `layout_hash`, applying the per-mode table in 5.3
  at each use site (enum discriminant, optional, array length, embedded
  message). Define the hash precisely: every type, enums included, has one;
  it covers the type's own laid-out `WireSpec` sequence and mode; it excludes
  names, docs, and spans; an embedded field contributes a constant marker
  and its position, never the referenced type's name or layout. The
  handshake hash is composed at runtime, structurally: a packed parent mixes
  in each packed child's composed hash in field order; a tagged child
  contributes a fixed constant so it may evolve freely; recursive types are
  handled with a visited set. The combine function (for example 64-bit
  FNV-1a over the child hashes) is specified in `docs/wire.md` and computed
  by the reference codec so the corpus checks it. This is what keeps
  per-file Godot import from going stale (section 5.2). *Why:* the IR must
  state what the backend emits; the hash is the handshake guard for packed
  mode, and peers in different languages must compute the same value.
- [ ] **C6b. Implement the tagged wire shape.** With mode already in the
  model (C1b) and the corpus in place (D1), implement the byte-oriented
  tagged shape from section 5.3: byte-aligned start, LEB128 tag, LEB128
  payload length in bytes, byte-padded payload, terminator tag `0`, unknown
  tags skipped with `read_bytes`. Add a conformance fixture that decodes a
  message with an unknown field and one for each cross-mode embedding.
  *Why:* section 1.1; this replaces optional per-field indices with a
  coherent choice, and "skippable" is only meaningful once the skip scheme is
  written down and tested from the decode side. This task is ordered after
  D1 in practice even though it is numbered here.
- [ ] **C7. Define the public API and the CLI.** `lib.rs` exports exactly:
  `compile(&CompileRequest) -> Result<(ir::Schema, SourceMap), Diagnostics>`
  with `CompileRequest { files, import_roots }`; the `ir` module (types,
  `Span`, `verify`); `GenerateRequest { files_to_generate }`; the
  `Generator` trait returning `Result<Vec<GeneratedFile>, Diagnostics>`;
  `Diagnostics` with a renderer that takes the `SourceMap`. Nothing from
  `syntax` or `sema`; backend config is typed and passed to the backend's
  constructor, not through string parameters. `CodeWriter` is optional: the
  GDScript backend has its own emitter and uses it only for tokens, so
  either keep it over `fmt::Write` or drop it. The binary becomes
  `baproto check` (diagnostics only), `baproto ir --json` (goldens and
  debugging), and `baproto encode` / `decode` (the reference codec as a
  corpus tool); `compile` is removed. Tag a crate release and move the
  plugin's pin from a git revision to the tag. *Why:* section 3.11; the
  backend should not need the CLI or internals, the list in section 2.1.1 is
  what it reaches for today, and after Phase A the CLI has no backend to run.

### Phase D: prove the wire

- [ ] **D1. Write `docs/wire.md` and build the conformance corpus.** First
  write the bit-stream specification (section 5.4): bit order, byte order,
  float representation, varint form, string bytes, length units, the byte
  alignment rule for tagged messages, the packed and tagged message shapes,
  the value file format and its number rules, and the hash combine function.
  Match the existing GDScript runtime where it has already chosen
  (least-significant-bit-first packing, LEB128). Then
  `tests/conformance/*.baproto`, each with `values`, `bytes`, `bits`, an
  optional `decoded`, and the expected composed hash, and a reference codec
  in the crate that encodes and decodes according to that document. Keep the
  IR JSON goldens from A2 current. *Why:* section 5.4; without this no claim
  about bandwidth or compatibility is testable, without the decode direction
  the unknown-field fixture cannot exist, and without the document the
  runtime is the only specification.
- [ ] **D2. Bring the GDScript backend onto the new IR.** Delete
  `collect.rs`, the prefix-string dependency detection, the
  `(NativeType, WireFormat)` matching in `codec/wire.rs`, and the
  varint-for-everything defaults; map each `WireSpec` variant to exactly one
  runtime primitive and obey it. Delete nested-package directory and
  namespace generation (section 1.2). Honour `files_to_generate`. Spike the
  one-script-per-schema-file output with inner classes as the importer's own
  save file (section 5.2) and adopt it if an imported `Script` loads by its
  source path; otherwise keep per-type files but only for the requested
  file. The spike passes only if editor autocompletion on an inner class
  and a breakpoint in generated code both work (section 5.2). Change
  `import/plugin.gd` to pass the imported file's include closure as the
  compile set with `files_to_generate` = that file, register only its own
  outputs, and stop scanning the output directory. Enforce the rule that
  generated code never inlines another file's layout: embedded fields call
  the other type's codec, and hashes compose at runtime with the combine
  from `docs/wire.md`. Run the conformance corpus, decode direction
  included, through generated GDScript in CI (headless Godot). *Why:* this
  is the first moment the IR and a backend agree, the first end-to-end test,
  and the fix for the import ownership and staleness problems in section
  2.1.1.
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
- [ ] **E7. Envelope and type ids.** A generated registry mapping `wire_id`
  to codec, and an envelope message whose type id field carries `wire_id`,
  so a packet can carry heterogeneous messages. Never the arena `TypeId`,
  which renumbers on every unrelated edit (section 5.2). The registry is a
  whole-program artifact with one writer: an `EditorPlugin` action plus an
  `EditorExportPlugin._export_begin` hook in the Godot plugin, not the
  per-file importer. The handshake for the packed plane is the registry's
  composed hash.
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

## 7. References

Outside sources that inform the wire design and the keep column. Each entry
names the part of this document it backs so a reader can check the claim
against the original. Transport, reliability, and session material from the
same authors is deliberately absent: it sits above `baproto` and only shapes
what the IR must be able to express.

### 7.1 Gaffer on Games (Glenn Fiedler)

- **Reading and Writing Packets**, from *Building a Game Network Protocol*.
  <https://gafferongames.com/post/reading_and_writing_packets/>. The
  bitpacker: 64-bit scratch word, LSB-first bit order, little-endian flush,
  bits derived from a value range, and range validation on every read. Backs
  the packed-mode column of the layout table in 5.3 and the `reader.gd` and
  `writer.gd` runtime contract (D1).
- **Serialization Strategies**, same series.
  <https://gafferongames.com/post/serialization_strategies/>. Bounded
  integers, floats compressed to a normalized integer range, byte-aligned
  length-prefixed strings and arrays, sparse array subsets with relative index
  encoding, and protocol id plus checksum plus in-stream serialization checks.
  Backs `Quantized`, `LenSpec`, `Optional`, and the `layout_hash` and `wire_id`
  decisions in 5.2 and E7. The protocol id is folded into the checksum but
  never transmitted; that trick is the alternative to sending the composed
  hash in the handshake.
- **Snapshot Compression**, from *Networked Physics*.
  <https://gafferongames.com/post/snapshot_compression/>. Smallest-three
  quaternions, bounded position and velocity quantization, at-rest flags, and
  delta encoding against an acknowledged baseline. Backs E3, E5, and E6, and
  is the source of the split in E6 between what the codec owns (baseline
  comparison) and what the sync layer owns (acks and history).
- **Snapshot Interpolation** and **State Synchronization**, same series.
  <https://gafferongames.com/post/snapshot_interpolation/>,
  <https://gafferongames.com/post/state_synchronization/>. Define the
  publishable-state message and its send rate, which is the traffic packed
  mode exists for (1.1, 1.2).
- **Deterministic Lockstep** and **Floating Point Determinism**.
  <https://gafferongames.com/post/deterministic_lockstep/>,
  <https://gafferongames.com/post/floating_point_determinism/>. Relevant only
  to the `Quantized` decode path: the formula must be bit-exact across
  platforms, so the corpus (5.4) checks decoded values, not just bytes.

### 7.2 Development and Deployment of Multiplayer Online Games (Ignatchenko)

Vol. I, *GDD, Authoritative Servers, Communications*. Chapter 3,
"Communications", is the relevant chapter; its four parts are listed in the
order they matter here.

- **3(d) Protocols. IDL: Encodings, Mappings, and Backward Compatibility.**
  Argues for a declarative IDL with three separate concerns: the data
  declaration, the encoding declaration (per-field bit widths, fixed-point
  ranges, bit-oriented versus byte-oriented streams), and the mapping to
  language types. That triad is `Field`, `WireSpec`, and the host type on
  scalar leaves (5.3). The same part contrasts field-tagged extensible
  encodings with compact positional ones and recommends keeping both,
  chosen per message. That is the packed and tagged split (1.2, 5.2) and the
  reason cross-mode embedding is specified rather than forbidden.
- **3(b) Protocols. World States and Reducing Traffic.** Server state versus
  publishable state versus client state, delta compression against a
  reference base, dead reckoning as compression, and bit-level encoding as
  the final step. Same conclusion as Snapshot Compression: the codec owns
  the leaf encodings and the baseline comparison (E6), nothing more.
- **3(c) Protocols. Point-to-Point Communications and Non-blocking RPCs.**
  Message-type discriminators and request ids in the envelope. Backs E7 and
  E8.
- **3(a) Protocols. RTT, Input Lag, and Mitigation.** Background for why
  packed mode exists; no direct design consequence.

Vol. II, Chapter 5 "(Re)Actors" and Chapter 6 "Client-Side Architecture",
touch this work only through the plugin: deterministic reactors want
recordable, replayable message logs, which argues for the `decode` CLI (C7)
and a stable IR JSON format (A2). The author's shorter "64 Network DO's and
DON'Ts for Game Engines" series, part II "Protocols and APIs", is a condensed
version of 3(d).
<https://leanpub.com/development-and-deployment-of-multiplayer-online-games-vol1>
