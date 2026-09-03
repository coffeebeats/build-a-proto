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

Mode is a property of the **outermost message being encoded**, never of a
field. A message declares the mode it uses when it is that outermost message,
and every message embedded inside it takes the enclosing mode. A message
whose fields are partly tagged and partly positional has no coherent wire
format, and a type reached from both planes simply has two layouts. This is
ASN.1's model, one abstract type with the encoding rules chosen at the root
(7.3), rather than FlatBuffers' `table` versus `struct`, which fixes the mode
on the type. Section 1.3 gives the rules. The book's split of *structure*,
*encoding*, and *language mapping* ("Development and Deployment of
Multiplayer Online Games", Vol. I, chapter 3(d)) is why the encoding hints
stay separate from the data inside the compiler even though they are written
next to the field.

### 1.2 Keep / cut decisions

| Item | Decision | When to revisit |
|---|---|---|
| Field indices | Keep, as part of the data declaration: optional in the syntax, required on every field of a message that has a tagged layout, ignored by the packed layout (1.3). What is cut is the *decorative* index that no layout reads. | Now, with defined skip semantics. |
| Positional encoding | Keep as `packed` mode plus schema hash for handshake. | Now. |
| Structure / encoding / mapping split | Build it. This is the mechanism that gives flexibility without feature sprawl. | Now. |
| `range`, `quantize`, `optional`, `varint`, `smallest_three`, `vec2`/`vec3`/`quat` | Add, driven by the snapshot packer. | After the model and layout passes exist. |
| Baseline-relative encode / decode (delta against an acked baseline) | Add. | After the model and layout passes exist. |
| Maps on the wire | Cut. Wire is an array of key/value pairs (this is how protobuf encodes maps). `Dictionary` accessors can be language-mapping sugar later. | Mapping layer, later. |
| `delta`, `fixed_point`, `pad`, `bits(var(n))`, `bit`, `byte` scalars | Cut. Each is parsed and then dropped or unsupported by the backend. | Never. |
| `include` and flat packages | Keep. | Shared types across client and server. |
| Embedding across modes (`packed` inside `tagged`, `tagged` inside `packed`) | Cut as a wire shape. An embedded message takes the enclosing mode, so a type used from both planes has two layouts (1.3, 5.3). | When a packed snapshot must carry an independently evolving blob. |
| Nested package directory trees and namespace scripts in the backend | Cut. | Never. |
| Services (`service` declaration: method ids, request/response pairing, envelope) | Add, small. | When the async PvP server exists. |
| Rust backend | Cut now. | When a Rust server exists, and only after the conformance corpus exists. |
| External plugin protocol (JSON over stdin/stdout) | Cut now. The library crate is the same door. | When a non-Rust backend author appears; the serialized IR plus a format version is then already the contract. |
| Six-target cross-compilation CI | Reduce to dev platforms plus Linux. | When a release is consumed by someone else. |

The guard against scope creep is a rule, not a list: **nothing enters the
language without a conformance fixture and a consumer that needs it.**

### 1.3 Where encoding choices live

The references were read for one question: one encoding per message, or
several (7.1, 7.2, 7.3). The decision below is biased toward the smallest
scope that keeps the flexibility a solo developer will actually use.

1. **Two modes, chosen at the root.** A message declares `packed` (the
   default) or `tagged`, and the declaration applies when the message is the
   outermost thing encoded. Embedded messages take the enclosing mode. A
   message reached from roots of both modes gets both layouts and two
   generated codecs. Cross-mode embedding as a distinct wire shape is cut.
   The declared layout is always built, so a helper type used only from
   tagged roots should say `tagged`: otherwise the packed rules (bounded
   arrays, no recursion) apply to a layout nothing uses.
2. **Size hints are facts about the data and stay on the field.** `range`,
   `quantize`, and array and string bounds describe the value, so they are
   written next to it and mean the same under both modes. This is what an
   ASN.1 constraint is, and unaligned PER derives bit widths from
   constraints the way the packed layout does here. Named per-field
   encoding blocks separate from the data (Ignatchenko's `ENCODING`
   declarations, ASN.1's ECN) are not built: they need a rule for nested
   messages, they multiply generated code and fixtures, and ECN is the
   standardised form of that idea that nobody adopted.
3. **Full versus delta is a runtime argument, not a declaration.** Whether
   to send everything or only what changed depends on what the peer has
   acknowledged, which only the sync layer knows. E6 adds a baseline
   parameter to the packed codec and the schema does not change.
4. **Field indices belong to the data declaration**, not to tagged mode.
   They are optional in the syntax. The check pass requires them on every
   field of a message that has a tagged layout and on every variant of an
   enum used from one; the packed layout ignores them.
5. **Escape hatches that need no compiler feature.** A genuinely different
   shape for the same data, such as a low-detail variant for distant
   objects, is a second message type. Data that must survive years of schema
   changes, a replay or a save, is sent under a tagged root.

Revisit trigger for the cut in item 1: a packed snapshot that must carry an
independently evolving blob. That is the one case the old cross-mode rule
served, and it has no consumer today.

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
| `generate/` | ~1.2k | `Generator` trait, `RustGenerator`, `ExternalGenerator`, `Language<W>`, `Writer`/`FileWriter`/`StringWriter`, `CodeWriter` | Keep `CodeWriter` and the shape of `Generator`, which becomes a function signature (5.2). Delete the rest. |
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
ariadne reporting, `CodeWriter`, the `Generator` contract (IR in, files
out, kept as a function signature rather than a trait, see 5.2), and the
Given/When/Then test discipline.

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
      |  collect: every message and enum gets a TypeId and a full name (NameTable)
      |  build:   one pass constructs the Model, resolving each reference as it goes
      v
 sema::Model (whole program, resolved, immutable, still has spans)  sema/
      |  modes:  which modes reach each type (ModeSet, a side table by TypeId)
      |  check:  indices, encodings vs types, recursion, mode rules, reserved
      |  layout: pure function of the model and the mode set
      |          -> bit widths, prefix widths, discriminant widths, presence bits,
      |             tag widths, layout hash
      v
 ir::Schema (serializable, versioned, spans kept as byte ranges)    ir/ + lower/
      |  files[], types[] by TypeId with full names, wire fully specified
      v
 generate(&ir::Schema, &GenerateRequest) -> Vec<GeneratedFile>       backend crate
```

### 5.1 Module layout

```
src/
  span.rs        LEAF. FileId, Span { file, start, end }. Depends on serde only.
  source/        SourceMap (paths + text, implements ariadne Cache), ImportRoot resolution
  diagnostics/   Diagnostic (severity, code, span, message), Diagnostics (collector),
                 ariadne renderer over a SourceMap
  ir/            The contract: TypeId, Mode, WireSpec, LenSpec, Schema, File, Type,
                 Message, Enum, PackedLayout, TaggedLayout, verify(), format_version.
                 Depends on span, diagnostics, and serde.
  syntax/        token.rs, lexer.rs, ast.rs, parser.rs        (today's lex/, parse/, ast/)
  sema/          model.rs   TypeDef, FieldDef, TypeRef (incl. Error), EncodingSpec
                 names.rs   NameTable: full name -> TypeId, scope parents
                 build.rs   Ast + NameTable -> Model (resolves references while building)
                 modes.rs   Model -> ModeSet (which modes reach each type)
                 check/{index,encoding,cycle,mode}.rs
  layout/        wire.rs    Model + ModeSet -> ir layouts per type, hash per packed layout
  lower/         lower(&Model, &Layout) -> ir::Schema
  codegen/       GeneratedFile, GenerateRequest (+ its validation), CodeWriter
  driver.rs      compile(&CompileRequest) -> Compiled { schema, sources, diagnostics }
  main.rs        CLI: check | ir --json | encode | decode
```

Dependency rule: a DAG, not a chain, in this order with no upward references:
`span <- source <- diagnostics <- ir <- syntax <- sema <- layout <- lower <- codegen <- driver`.
`span` is the leaf, as `rustc_span` is for rustc and the `span` crate is for
rust-analyzer: every layer, the lexer and the serialized IR included, shares
one location type without the lexer importing it from the contract module.
`ir` sits directly above `diagnostics` because `verify` returns
`Diagnostics`; an earlier draft made `ir` the leaf and declared it
serde-only, which that signature contradicts. `ir` still sits below
`sema` because `WireSpec::Message { ty: TypeId }` places a type id inside
the serialized contract and the `check` pass needs `Mode`, and this keeps
the public API free of `sema` (task C7). `TypeKind` lives in `sema`.

Nothing inside one crate enforces this order; only a crate boundary does.
The crate stays single at this size. If the rule is ever broken in review,
the first split is `span` plus `ir` into a serde-only `baproto-ir` crate,
which is also the crate a backend would depend on for fixtures without the
compiler. It is listed under the deferred items.

### 5.2 Design decisions

- **Whole-program passes replace the per-file DFS.** The driver loads every
  file reachable from the inputs, then runs collect, build, modes, check,
  layout, and lower over all of them. Cross-file references and include
  cycles stop being special cases.
- **Every pass returns diagnostics, never `Option`.** Collect, build, and
  check append to one `Diagnostics` collector. Lowering runs only on a clean
  model and cannot fail. This is protoc's pool-build contract.
- **The model is built once and never mutated.** Collect produces only a
  `NameTable` (full name to `TypeId`, plus each scope's parent). One build
  pass then constructs the `Model` from the AST and resolves every reference
  as it converts it, so `TypeRef` is `Named(TypeId)` or `Error` from the
  moment it exists and has no unresolved state. Facts derived later, such
  as which modes reach a type, are returned as side tables indexed by
  `TypeId` (`ModeSet`) and passed to the passes that need them, never
  written back into the model. This is rustc's shape (name resolution
  produces `Res`, lowering consumes it, everything else is a map keyed by
  id) and rust-analyzer's (`hir` is "a static, fully resolved view"). It
  removes the "which pass has run" question from every later pass, and it
  keeps each pass a pure function of immutable inputs, which is what a
  query system would need if one were ever wanted.
- **The name table is keyed by one dotted full name.** protoc's pool keeps
  a flat `symbols_by_name` map and resolves a relative name by trying the
  enclosing scope's full name plus the reference, then stripping one
  trailing component at a time. With a single string key the package
  boundary is not part of the key, so the "try every package split" loop of
  section 3.6 disappears. `Descriptor` is retired; `FullName` is a display
  and key type only.
- **The IR is descriptor plus layout, and it is versioned.** Each layout a
  type has carries a `WireSpec` per field that says exactly what goes on
  the wire, and each packed layout carries its hash. Each message carries
  its declared mode. A backend that emits anything the IR did not
  specify is a backend bug. This is the property the conformance corpus
  depends on. It is Cap'n Proto's `StructLayout` idea applied to bit streams.
- **Generation is in-process, library-first, and a backend is a function.**
  `baproto::compile` returns an IR and the Godot binary stays a thin
  translation over it, which is what it already is. The `Generator` trait
  goes: it has one implementor, in another crate, and after Phase A nothing
  in this crate dispatches over it, so it is ceremony (rust-analyzer's
  style guide calls these "doer objects"). The crate exports the shapes a
  backend consumes and produces, `GenerateRequest` and `GeneratedFile`, and
  a backend is `fn generate(&ir::Schema, &GenerateRequest) ->
  Result<Vec<GeneratedFile>, Diagnostics>` with typed configuration held by
  whatever owns that function. When a non-Rust backend arrives, the
  serialized IR plus `format_version` is already the contract.
- **The public result carries warnings.** `compile` returns
  `Compiled { schema: Option<ir::Schema>, sources: SourceMap, diagnostics }`
  rather than `Result<_, Diagnostics>`: a `Result` drops every diagnostic on
  success, and the E3 raw-float check is a warning that must reach the
  Godot importer on a successful compile. `schema` is `Some` exactly when
  `diagnostics` has no error.
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
  C-like; there is no per-variant payload. No variant states a fact twice:
  a float's width is its host type, a varint's signedness is its host
  type's, and a `Message` or `Enum` leaf names the type and nothing else,
  because every duplicated fact is one more verifier rule and one more way
  for a backend to read the wrong copy. `WireSpec` is deliberately not
  `#[non_exhaustive]`: a backend must fail to compile when a variant is
  added, since a new variant means a new runtime primitive.
- **Mode decides layout at the use site, not on the type.** A message
  declares the mode it uses as a root; everything embedded in it takes the
  enclosing mode (1.3). So the wire form of an enum discriminant, an
  `optional`, an array length, and an embedded message is chosen by the
  layout pass from the *enclosing* mode, and a type reached from both kinds
  of root carries both layouts. A packed layout gets the minimum-width
  discriminant and a presence bit; a tagged layout gets a varint
  discriminant and expresses optionality as tag absence, so adding an enum
  variant in the service plane is a compatible change. The table in 5.3
  states each case. An enum's discriminant is stored on the enum's own
  layouts, not repeated at every field that uses it: a field's
  `WireSpec::Enum { ty }` means "call that type's codec for the layout
  being generated", exactly as `Message { ty }` does.
- **A type has at most two layouts, and only the packed one has a hash.**
  Which layouts exist is decided by reachability (`sema::modes`, C1c): a
  message always has its declared mode's layout, plus the other mode's if
  any message of that mode embeds it, transitively. `layout_hash` and the
  runtime hash composition cover the packed layout only; a tagged layout is
  self-describing and needs no handshake. A recursive type is an error in a
  packed layout and legal in a tagged one, because the tagged shape
  length-prefixes every field. Backends generate one codec pair per layout.
- **Layouts hang on the type, not on the field.** A message is
  `fields: Vec<Field>` (declarations: name, span, doc) plus
  `packed: Option<PackedLayout>` and `tagged: Option<TaggedLayout>`, each a
  vector of wire specs aligned with `fields`. The packed layout owns the
  hash; the tagged layout owns the index of every field as a plain `u32`.
  An enum has the same pair. An earlier draft put `packed: Option<WireSpec>`
  and `tagged: Option<WireSpec>` on every field with `layout_hash` and
  `index` as further `Option`s, and needed four verifier rules to say
  "all or none per message", "hash present iff packed", "every index
  present iff tagged", and "an enum used only from tagged roots has no
  hash". A per-message fact encoded per field is the pattern the API
  guidelines call conveying meaning "through types, not bool or Option";
  hoisting it makes three of the four rules structural and leaves the
  verifier checking lengths and that the declared layout is present. A
  backend's "one codec pair per layout" becomes a loop over two options.
- **The IR is a flat arena, not a tree.** Types live in one `Vec<Type>`
  indexed by `TypeId`; nesting is `parent: Option<TypeId>` and
  `nested: Vec<TypeId>` (Cap'n Proto's `Node.scopeId`). `Type` is one
  header shared by messages and enums (wire id, full name, file, parent,
  span, doc) plus a `body` enum, as Cap'n Proto's `Node` is a common header
  plus a union, so a consumer reads the header without matching. Backends
  that want one file per type flatten for free; backends that want inner
  classes walk `nested`. The GDScript `collect.rs` and the prefix-string
  hack disappear.
- **The IR carries source locations, and there is one location type.**
  protoc keeps `SourceCodeInfo`, capnpc keeps `sourceInfo`. Backends produce
  real errors (a Godot `int` is 64-bit signed, so `u64` cannot round-trip)
  and the Godot importer shows compiler output to the user. Each type and
  field carries the same `Span { file: FileId, start, end }` the compiler
  uses; byte offsets are serializable and cheap, and line and column are the
  renderer's job. A backend returns `Diagnostics` with `Span`s, not
  `anyhow` strings, and the same renderer prints them. Each `Diagnostic`
  also carries a short stable code (`E0007`-style or a snake-case name) so
  tests assert on the code rather than on prose.
- **Stable ids for the wire are not arena indexes.** `TypeId` is an arena
  index assigned by file order then declaration order, so inserting one type
  renumbers every type after it. Anything that goes on the wire (the E7
  envelope, the registry) uses `wire_id: u32`, a hash of the full name (or a
  declared id later), stored next to `TypeId` and checked unique by the
  verifier. Cap'n Proto assigns 64-bit ids for exactly this reason, and
  rustc keeps a `DefPathHash` that is stable across sessions next to the
  `DefId` that is not.
- **Every hash is computed over bytes the specification defines.** Neither
  `layout_hash` nor `wire_id` may come from `#[derive(Hash)]` or
  `DefaultHasher`: `std::hash` output is not stable across Rust releases,
  and the GDScript runtime and any other peer must reproduce both values.
  `docs/wire.md` defines the canonical byte encoding of a packed layout
  (the wire spec sequence, no names, docs, or spans) and of a full name,
  and names the hash function over those bytes (64-bit FNV-1a for the
  layout, 32-bit FNV-1a for the wire id, unless the corpus work finds a
  reason to change). The reference codec computes both so the corpus
  checks them.
- **Passes tolerate a dirty model.** Build and check run after collect has
  reported errors, so the user sees every error in one compile. An
  unresolvable reference becomes `TypeRef::Error` (rustc's `Res::Err`) and
  later checks never report on an `Error` reference, which stops cascades.
  Lowering still runs only on a clean model.
- **There is an IR verifier, and it checks what the types cannot.** LLVM
  has `Verifier`, rustc validates MIR at each phase. An
  `ir::verify(&Schema) -> Diagnostics` checks id ranges, `wire_id`
  uniqueness, that each `WireSpec` is well-formed for its position and
  layout (a presence-bit `Optional` in a tagged layout is an error), that
  every `Message { ty }` and `Enum { ty }` inside a packed layout names a
  type that has a packed layout and likewise for tagged, that layout
  vectors match their declarations in length, that the declared layout is
  present, and that `nested`/`parent` agree. It runs in tests and before
  generation. It is cheap and protects every backend and the reference
  encoder. The rule for the split between types and verifier is
  rust-analyzer's: a field is public when any value is valid, and an
  invariant the type cannot carry is documented and verified. IR structs
  therefore have public fields, which deviates from the API guideline that
  prefers private fields, and the ids and `Span` are newtypes with
  constructors.
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
// span.rs   (leaf; serde)
pub struct FileId(u32);                                     // Copy, Ord, Hash; constructor only
pub struct Span { file: FileId, start: u32, end: u32 }      // byte offsets

// diagnostics/
pub struct Diagnostic { severity: Severity, code: &'static str, span: Span, message: String }
pub struct Diagnostics { items: Vec<Diagnostic> }           // error(code, span, msg), warn(..), has_errors()

// ir/   (the contract; serde; every type derives Debug, Clone, PartialEq, Eq, Hash)
pub struct TypeId(u32);                                     // Copy, Ord, Hash; indexes Model and Schema
pub enum Mode { Packed, Tagged }
pub enum IntType { U8, U16, U32, U64, I8, I16, I32, I64 }
pub enum FloatType { F32, F64 }
pub struct Real(u64);                                       // f64 as its bit pattern, so Eq and Hash derive

// one variant == one runtime primitive; self-sufficient; no fact stated twice (5.2)
pub enum WireSpec {
    Bool,                                                   // 1 bit
    Uint { bits: u32, host: IntType },                      // fixed width, unsigned; bits <= host width
    Int { bits: u32, host: IntType },                       // fixed width, two's complement
    ZigZag { bits: u32, host: IntType },                    // fixed width, zigzag-mapped
    Varint { host: IntType },                               // LEB128; zigzag iff host is signed
    Float(FloatType),                                       // IEEE bit pattern, width from host
    Quantized { min: Real, max: Real, bits: u32, host: FloatType },
    Bytes { len: LenSpec },                                 // raw bytes
    Utf8 { len: LenSpec },                                  // string
    Array { len: LenSpec, elem: Box<WireSpec> },
    Optional { inner: Box<WireSpec> },                      // 1 presence bit (packed layouts only; verify)
    Message { ty: TypeId },                                 // call that type's codec for this layout
    Enum { ty: TypeId },                                    // same; discriminant lives on the enum's layout
}
pub enum LenSpec { Fixed(u32), Prefix { bits: u32 }, Varint }

pub struct Schema { format_version: u32, files: Vec<File>, types: Vec<Type> }   // impl Index<FileId>, Index<TypeId>
pub struct File { path: String, package: String, includes: Vec<FileId>, types: Vec<TypeId> }
pub struct Type {                                           // one header, as Cap'n Proto's Node
    wire_id: u32, full_name: String, file: FileId, parent: Option<TypeId>,
    span: Span, doc: Option<String>, body: TypeBody,
}
pub enum TypeBody { Message(Message), Enum(Enum) }
pub struct Message {
    mode: Mode,                                             // declared: the layout used when this message is the root
    nested: Vec<TypeId>,
    fields: Vec<Field>,                                     // declarations only
    packed: Option<PackedLayout>,                           // present iff a packed root reaches this type
    tagged: Option<TaggedLayout>,                           // present iff a tagged root reaches this type
}
pub struct Field { name: String, span: Span, doc: Option<String> }
pub struct PackedLayout { hash: u64, fields: Vec<WireSpec> }                 // aligned with `fields`; hash is the layout_hash of 5.2
pub struct TaggedLayout { fields: Vec<TaggedField> }                         // aligned with `fields`
pub struct TaggedField { index: u32, spec: WireSpec }
pub struct Enum {
    variants: Vec<Variant>,                                 // ordinal = position
    packed: Option<PackedEnumLayout>,
    tagged: Option<TaggedEnumLayout>,
}
pub struct Variant { name: String, span: Span, doc: Option<String> }
pub struct PackedEnumLayout { hash: u64, bits: u32 }                          // Uint { bits } carrying the ordinal
pub struct TaggedEnumLayout { indices: Vec<u32> }                             // Varint carrying the declared index
pub fn verify(schema: &Schema) -> Diagnostics;

// source/
pub struct SourceMap { files: Vec<(PathBuf, String)> }   // indexed by FileId; ariadne Cache

// sema/   (internal; never exported)
pub struct NameTable { by_name: BTreeMap<FullName, TypeId>, parent_scope: Vec<Option<TypeId>> }
pub fn collect(asts: &[(FileId, Ast)], diags: &mut Diagnostics) -> NameTable;
pub fn build(asts: &[(FileId, Ast)], names: &NameTable, diags: &mut Diagnostics) -> Model;   // resolves as it builds
pub struct Model { files: Vec<FileInfo>, types: Vec<TypeDef> }   // arena; TypeId indexes into it; immutable after build
pub enum TypeDef { Message(MessageDef), Enum(EnumDef) }
pub struct MessageDef { name: FullName, file: FileId, mode: Mode, fields: Vec<FieldDef>, span: Span, doc: Option<String> }
pub struct FieldDef { name, index: Option<u32>, ty: TypeRef, encoding: EncodingSpec, span, doc }
pub enum TypeRef {
    Scalar(Scalar), Named(TypeId), Array { elem: Box<TypeRef>, len: ArrayLen },
    Error,                            // build reports the failure and leaves this; checks skip it
    /* later: Optional, Vec3, Quat */
}
pub struct Modes { packed: bool, tagged: bool }             // at least one is true (constructor)
pub struct ModeSet(Vec<Modes>);                             // side table indexed by TypeId
pub fn modes(model: &Model) -> ModeSet;                     // declared mode plus reachability, to a fixed point
pub fn check(model: &Model, modes: &ModeSet, diags: &mut Diagnostics);

// layout/
pub fn layout(model: &Model, modes: &ModeSet) -> Layout;    // ir layouts per type per used mode; hash per packed layout

// lower/
pub fn lower(model: &Model, layout: &Layout) -> ir::Schema;

// codegen/
pub struct GenerateRequest { files_to_generate: Vec<FileId> }
pub struct GeneratedFile { path: PathBuf, contents: String }
// a backend is: fn generate(&ir::Schema, &GenerateRequest) -> Result<Vec<GeneratedFile>, Diagnostics>

// driver.rs
pub struct CompileRequest { files: Vec<PathBuf>, import_roots: Vec<ImportRoot> }
pub struct Compiled { schema: Option<ir::Schema>, sources: SourceMap, diagnostics: Diagnostics }
pub fn compile(req: &CompileRequest) -> Compiled;           // schema is Some iff no error
```

Id newtypes are hand-rolled with a small macro rather than taken from
`la-arena`: its `Idx<T>` is typed per element, and one `TypeId` indexes
both `Model.types` and `Schema.types`, which is the point of lowering being
one-to-one. Storing an id inside the element it indexes (`Message.id`) is
omitted for the same reason cranelift's `PrimaryMap` and la-arena omit it:
it is one more fact to verify. The JSON form may add it for readability.

`EncodingSpec` in `sema` is what the user wrote (`bits(12)`, `zigzag`,
`range(0, 100)`); `WireSpec` in `ir` is what the layout pass derived from it
and the enclosing mode. That difference is the reason `Model` and `Schema`
are separate types rather than one struct with an optional layout: the model
may churn freely, the IR is versioned.

Per-mode layout of the constructs whose wire form depends on the enclosing
message's mode:

| Construct | In a `packed` message | In a `tagged` message |
|---|---|---|
| Enum discriminant (on the enum's own layout; the field says `Enum { ty }`) | `PackedEnumLayout { bits: max(1, ceil(log2(variants))) }`: fixed width carrying the variant's ordinal | `TaggedEnumLayout { indices }`: unsigned varint carrying the variant's declared index |
| `optional T` | `Optional { inner }`: 1 presence bit then `T` | tag absent means absent; no presence bit |
| Array length | `LenSpec::Fixed(N)` for `[N]T`; `Prefix { bits }` or `Varint` otherwise | `LenSpec::Varint` |
| Embedded message | inline, bit-exact, the child's packed layout, hash composed | the child's tagged shape as the field payload |

Tagged mode wire shape (fixed by task C6b, stated here so the sketch is
complete). Tagged messages are byte-oriented. The message starts on a byte
edge, which holds trivially: a tagged message is only ever a root or a field
payload inside another tagged message. Each present field is
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
  `FileId` and `Span` are defined in the leaf `span` module (5.1) so every
  layer, the serialized IR included, shares one location type. `SourceMap`
  owns paths and text and implements ariadne's `Cache`. Keep `ImportRoot`
  resolution. *Why:* spans currently clone an `Rc<PathBuf>`;
  source cache and diagnostics are split across modules that import each
  other. This also removes `SchemaImport` from the lexer's type signatures.
- [ ] **B2. Create a `diagnostics` module.** Move `Diagnostic`, `Severity`,
  and the ariadne reporter there; add a `Diagnostics` collector with
  `error(code, span, msg)`, `warn(code, span, msg)`, and `has_errors()`,
  where `code` is a short stable identifier that tests match on (5.2).
  Have lex and parse convert chumsky `Rich` errors into `Diagnostic` at
  their own boundary rather than in the compiler. *Why:* one place to
  accumulate and report; breaks the `analyze <-> compile` cycle.
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

- [ ] **C1. Add `sema::Model` and the collect and build passes.** Define
  `TypeId`, `TypeDef`, `FieldDef`, `TypeRef` (section 5.3). Collect walks
  the ASTs, assigns ids in file then declaration order, and produces a
  `NameTable` keyed by dotted full name with each scope's parent. Build
  constructs the `Model` from the ASTs in one pass and resolves each
  `Reference` as it converts the field, using the scope-walking algorithm
  from `Symbols::resolve` rewritten over the new key (try the enclosing
  scope's full name plus the reference, then strip one trailing component
  at a time; an absolute reference is looked up directly). An unresolved or
  wrong-kind reference is reported and left as `TypeRef::Error` so later
  passes run without cascading errors; `TypeRef` has no unresolved variant
  and the model is never mutated after build. `TypeKind` moves here.
  Retire `Descriptor`; keep `FullName` as a display and key type. *Why:*
  sections 3.2, 3.3, 3.6, and 5.2. After this there is exactly one answer
  to "where is the resolved program", and no pass has to ask whether
  resolution has run.
- [ ] **C1b. Add message mode to the syntax and the model.** Parser support
  for a per-message mode (the unused `encoding` keyword is the natural
  slot), `Mode` on `MessageDef`, default `packed`. The declared mode is
  the one used when the message is the outermost one encoded (1.3). No wire
  change yet. *Why:* the layout pass (C5) is a function of mode and the
  index rules (C4) depend on it, so mode must exist before either; the
  tagged wire shape itself waits for the corpus (C6b).
- [ ] **C1c. Add the `sema::modes` pass.** `modes(&Model) -> ModeSet`, a
  side table indexed by `TypeId` holding `Modes { packed, tagged }` for
  every message and enum: start from each message's declared mode and walk
  its embedded types transitively, marking each with the enclosing mode;
  iterate to a fixed point so recursive types terminate. The result is
  passed to check and layout, not written into the model. *Why:* section
  1.3 and 5.2; the index check (C4) and the layout pass (C5) both need to
  know which layouts a type has, and this is the only place that answer is
  computed.
- [ ] **C2. Make lowering a plain function in its own module.**
  `lower::lower(&Model, &Layout) -> ir::Schema` with no `Lower` trait, no
  `TypeResolver`, no `LowerContext`, no `MockResolver`, no `Option` returns.
  `lower/` sits above `sema` and `layout`; the `ir` types it builds sit
  below all three (5.1). Port the lowering unit tests to build a small
  `Model` directly. *Why:* section 3.10; lowering must not be able to drop
  fields silently.
- [ ] **C3. Redefine the IR as descriptor plus layout.** Add `format_version`,
  a `files` list keyed by `FileId`, a flat `types` arena indexed by `TypeId`
  whose elements are one `Type` header (`wire_id`, `full_name`, `file`,
  `parent`, `span`, `doc`) plus a `TypeBody`, with `nested` links on
  messages. Replace `WireFormat`/`Transform`/`padding_bits`/`NativeType`
  with the single `WireSpec` tree of section 5.3, whose scalar leaves carry
  the host type and state no fact twice. Put layouts on the type:
  `Message { fields, packed: Option<PackedLayout>, tagged: Option<TaggedLayout> }`
  and the enum equivalents, with the hash inside the packed layout and the
  field indices inside the tagged one. Remove maps (arrays of pair messages
  are the wire; see 1.2). Assign `TypeId`s deterministically (file order,
  then declaration order), implement `Index<TypeId>` and `Index<FileId>` on
  `Schema`, derive `Debug, Clone, PartialEq, Eq, Hash, Serialize,
  Deserialize` on every IR type (`Copy, Ord` on ids; `Real` wraps a float's
  bit pattern so the derives hold), keep `#[serde(tag = "kind")]` on
  enums, and use ordered collections everywhere. *Why:* sections 3.3 and
  5.2; this is the contract the conformance corpus and every backend
  depend on. Bump the format version whenever it changes.
- [ ] **C3b. Add `ir::verify`.** A pure function from `&Schema` to
  `Diagnostics` that checks every `TypeId` and `FileId` is in range,
  `wire_id`s are unique, each `WireSpec` is well-formed for its position
  and layout (a presence-bit `Optional` inside a tagged layout is an
  error; `bits` never exceeds the host width), every `Message { ty }` and
  `Enum { ty }` inside a packed layout names a type with a packed layout
  and likewise for tagged, each layout's vector has the same length as its
  declarations, a tagged layout's indices are unique, the declared mode's
  layout is present, and `nested`/`parent` agree. Request validation
  (`files_to_generate` naming real files) lives in `codegen`, not here.
  Run `verify` in every IR test and in the driver before generation.
  *Why:* section 5.2; the IR is a contract shared by the reference encoder
  and every backend, and a verifier is the standard way to keep an IR
  honest about the invariants its types cannot carry.
- [ ] **C4. Add the check passes.** `check(&Model, &ModeSet, &mut
  Diagnostics)`, run after C1c because several rules depend on which
  layouts a type has. Field indices (uniqueness; required on every field
  of a message, and every variant of an enum, that has a tagged layout;
  ignored otherwise), encoding-versus-type legality (`bits(n)` only on
  integers and within native width; `zigzag` only on signed; `varint` only
  on integers), recursion in a packed layout, array length bounds, package
  declared before other items. Remove the corresponding ad-hoc checks from
  the collector. *Why:* section 3.4; today nothing rejects a nonsensical
  annotation.
- [ ] **C5. Add the layout pass.** `layout(&Model, &ModeSet) -> Layout`, a
  pure function producing, for each type and each mode that reaches it,
  the `ir` layout of section 5.3 (a `WireSpec` per field, the enum
  discriminant width or index list, the hash for each packed layout),
  applying the per-mode table at each use site (optional, array length,
  embedded message and enum). Define the hash precisely: every type with a
  packed layout, enums included, has one; it is computed over the
  canonical byte encoding of the type's own packed layout that
  `docs/wire.md` defines, never over Rust's `Hash` (5.2); it excludes
  names, docs, and spans; an embedded field contributes a constant marker
  and its position, never the referenced type's name or layout. The
  handshake hash is composed at runtime, structurally: a packed parent
  mixes in each embedded child's composed hash in field order, and
  recursion cannot occur because C4 rejects it in a packed layout. The
  combine function (for example 64-bit FNV-1a over the child hashes) is
  specified in `docs/wire.md` and computed by the reference codec so the
  corpus checks it. This is what keeps per-file Godot import from going
  stale (section 5.2). *Why:* the IR must state what the backend emits;
  the hash is the handshake guard for packed mode, and peers in different
  languages must compute the same value.
- [ ] **C6b. Implement the tagged wire shape.** With mode already in the
  model (C1b) and the corpus in place (D1), implement the byte-oriented
  tagged shape from section 5.3: byte-aligned start, LEB128 tag, LEB128
  payload length in bytes, byte-padded payload, terminator tag `0`, unknown
  tags skipped with `read_bytes`. Add a conformance fixture that decodes a
  message with an unknown field and one for a type reached from both a
  packed and a tagged root, encoded under each.
  *Why:* section 1.1; this replaces optional per-field indices with a
  coherent choice, and "skippable" is only meaningful once the skip scheme is
  written down and tested from the decode side. This task is ordered after
  D1 in practice even though it is numbered here.
- [ ] **C7. Define the public API and the CLI.** `lib.rs` exports exactly:
  `compile(&CompileRequest) -> Compiled { schema, sources, diagnostics }`
  with `CompileRequest { files, import_roots }` (5.2: a `Result` would drop
  warnings on success); the `ir` module (types, `verify`) and `Span` and
  `FileId` from `span`; `GenerateRequest { files_to_generate }` with its
  validation and `GeneratedFile`; `Diagnostics` with a renderer that takes
  the `SourceMap`. No `Generator` trait: a backend is a function of
  `&ir::Schema` and `&GenerateRequest` (5.2). Nothing from `syntax` or
  `sema`; backend config is typed and held by the backend, not passed
  through string parameters. `CodeWriter` is optional: the
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
  the value file format and its number rules, the canonical byte encoding
  of a packed layout and of a full name, the hash functions over them for
  `layout_hash` and `wire_id`, and the hash combine function (5.2).
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
  runtime primitive and obey it. Generate one codec pair per layout the type
  has, named by mode (`encode_packed`, `decode_tagged`); the unsuffixed
  `encode` and `decode` alias the declared mode so the common case reads
  plainly. Delete nested-package directory and
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
  **`quantize(min, max, bits)`** on floats. In the same PR add a
  warning-level check: an `f32` or `f64` field in a packed message with no
  `quantize` reports "unbounded float in packed message; consider
  `quantize(min, max, bits)`". Silence it per field with an explicit `raw`
  encoding, which is also the spelling that makes the choice visible in
  review. *Why:* both references treat a raw float in a snapshot as a
  mistake (7.1); the check is one arm in the check pass and it steers users
  toward the encoding packed mode exists for.
- [ ] **E4. Bounded arrays** `[N]T` with the length bits derived from `N`,
  and a check that unbounded arrays are rejected in packed mode.
- [ ] **E5. Native vector types** `vec2`, `vec3`, `quat` with a
  `smallest_three` encoding for `quat`. Language mapping to Godot `Vector3`
  and `Quaternion` in the backend.
- [ ] **E6. Baseline-relative codec API.** `encode(writer, baseline)` and
  `decode(reader, baseline)` where a field equal to the baseline costs one
  bit; `delta(range)` for integers relative to the baseline. This is the
  codec's share of delta compression; acks and history stay in the sync layer.
  The baseline parameter exists on the packed codec only (1.3).
- [ ] **E7. Envelope and type ids.** A generated registry mapping `wire_id`
  to codec, and an envelope message whose type id field carries `wire_id`,
  so a packet can carry heterogeneous messages. Never the arena `TypeId`,
  which renumbers on every unrelated edit (section 5.2). The registry is a
  whole-program artifact with one writer: an `EditorPlugin` action plus an
  `EditorExportPlugin._export_begin` hook in the Godot plugin, not the
  per-file importer. The envelope is a message like any other, so its mode
  decides which codec the registry dispatches to: a packed envelope selects
  packed codecs and its handshake is the composed packed hash of every
  registered type; a tagged envelope selects tagged codecs and needs none.
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
- `baproto-ir` crate split (`span` plus `ir`, serde-only): when the module
  dependency rule in 5.1 is broken in review, or when a backend wants IR
  fixtures without linking the compiler.

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
  delta encoding against an acknowledged baseline. Backs E3, including its
  raw-float warning, E5, and E6, and is the source of the split in E6
  between what the codec owns (baseline comparison) and what the sync layer
  owns (acks and history).
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
  chosen per message. That is the packed and tagged split (1.2, 5.2). The
  same part proposes named encoding declarations separate from the data;
  1.3 records why that is not built.
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

### 7.3 ASN.1 (ITU-T X.680 series)

Not a game reference, but the one IDL with decades of experience of several
encodings over one type declaration, so it settles the question in 1.3.
X.680 defines the abstract syntax with constraints in the type
(`INTEGER (0..100)`). X.690 BER and DER are tagged, byte-oriented rules;
X.691 unaligned PER is a positional, bit-packed rule that derives field
widths from those constraints. The rules are chosen at the root of the
encode call, never per type, which is the model 1.3 adopts. X.692 ECN adds
per-field encoding control separate from the data, which is Ignatchenko's
encoding declaration in standardised form, and its lack of adoption is the
evidence against building it. Cap'n Proto's packed variant is the same
root-level choice in a modern system: a transport option, not a schema one.

### 7.4 Compiler data models and Rust type design

Sources for the shape of sections 5.1 to 5.3. Each entry names what was
taken from it.

- **rustc dev guide**, chapters *Overview of the compiler*, *The HIR*,
  *The `ty` module*, *Memory management*, *Name resolution*, *Queries*.
  <https://rustc-dev-guide.rust-lang.org/>. One representation per job
  (AST, HIR, THIR, MIR); HIR bodies kept out-of-band in maps keyed by
  `HirId`; `hir::Ty` is the syntax of a type and `ty::Ty` its semantics,
  which is the `EncodingSpec` versus `WireSpec` split; `Res::Err` continues
  compilation after a failed resolution (`TypeRef::Error`); `DefPathHash`
  stable across sessions beside a `DefId` that is not (`wire_id` beside
  `TypeId`); queries need pure functions of their inputs, which the
  immutable model gives without adopting a query system.
- **rust-analyzer**, `docs/book/src/contributing/architecture.md` and
  `style.md`, and `lib/la-arena`.
  <https://github.com/rust-lang/rust-analyzer>. The `syntax` crate "knows
  nothing" about semantics; `hir` is "a static, fully resolved view" and the
  API boundary; derived data lives in `ArenaMap` side tables next to an
  `Arena`; "if a field can have any value without breaking invariants,
  make the field public", otherwise enforce it in a constructor; avoid
  "doer objects". These fix the immutable model, the `ModeSet` side table,
  the public IR fields, and the removal of the `Generator` trait.
- **cranelift-entity** (`PrimaryMap`, `SecondaryMap`, `entity_impl!`) and
  **Carbon toolchain docs** (`toolchain/docs/idioms.md`).
  <https://github.com/bytecodealliance/wasmtime/tree/main/cranelift/entity>,
  <https://github.com/carbon-language/carbon-lang/tree/trunk/toolchain/docs>.
  Typed 32-bit ids into dense vectors, side tables keyed by the same id, no
  id stored inside the element, and a linear pass pipeline chosen on
  purpose.
- **Cap'n Proto `schema.capnp`**, **protobuf `descriptor.proto` and
  `descriptor.h`**, **FlatBuffers `reflection.fbs`**. A `Node` is one
  header (`id`, `scopeId`, `displayName`, `nestedNodes`) plus a union of
  bodies, which is the `Type` header plus `TypeBody`; protoc's pool keys
  symbols by dotted full name and resolves relative names by stripping
  scope components, which is the `NameTable`; FlatBuffers references types
  by index into flat tables.
- **LLVM `Verifier.cpp`** and **rustc MIR validation**
  (`rustc_mir_transform/src/validate.rs`). A verifier checks the
  well-formedness that the type system cannot carry, and an IR may be
  legal in one phase and not another, which is why `Optional` stays one
  variant guarded by `verify` rather than two `WireSpec` enums.
- **Rust API Guidelines**, *Type safety* (C-NEWTYPE, C-CUSTOM-TYPE),
  *Interoperability* (C-COMMON-TRAITS, C-SERDE), *Future proofing*
  (C-STRUCT-PRIVATE). <https://rust-lang.github.io/api-guidelines/>.
  "Arguments convey meaning through types, not `bool` or `Option`" is the
  case against per-field `Option<WireSpec>` pairs; the derive list on IR
  types; the private-field guideline this document knowingly deviates
  from for the IR, with the verifier as the invariant guard.
- **The Rust Reference**, `#[non_exhaustive]`, and **serde**, *Enum
  representations*. A non-exhaustive enum removes downstream exhaustiveness
  checking, which is why `WireSpec` is exhaustive; internally tagged
  representation (`#[serde(tag = "kind")]`) for the JSON form.
- **`std::hash` documentation.** The output of `Hash` plus `DefaultHasher`
  is not specified to be stable across Rust releases, which rules it out
  for `layout_hash` and `wire_id` (5.2).
