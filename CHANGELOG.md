# Changelog

## 0.2.6 (2026-01-27)

## What's Changed
* fix(ci): construct correct vendor path on Windows by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/106


**Full Changelog**: https://github.com/coffeebeats/build-a-proto/compare/v0.2.5...v0.2.6

## 0.2.5 (2026-01-27)

## What's Changed
* fix(ci): use correct `$CARGO_HOME` path on Windows by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/100
* fix(ci): use a cross-platform compatible Cargo config directory by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/102
* fix(ci): add missing permission when creating directory under `/` by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/103
* fix(ci): add `sudo` when creating Cargo config under `/.cargo` by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/104
* fix(ci): correctly set Cargo vendored dependencies configuration cross-platform by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/105


**Full Changelog**: https://github.com/coffeebeats/build-a-proto/compare/v0.2.4...v0.2.5

## 0.2.4 (2026-01-26)

## What's Changed
* fix(ci): specify `target-dir` when compiling to support non-root crate usage by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/89
* fix(ci): skip build cache when linting as `cargo clippy` ignores it by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/91
* chore(ci): correct mislabeled step name by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/92
* fix(ci): don't overwrite `.cargo/config.toml` with vendor directory by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/93
* fix(ci): add `target-dir` and `vendor-dir` arguments to `cargo-build` action by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/94
* fix(ci): eliminate Cargo config duplication by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/95
* fix(ci): fix syntax in bash conditional by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/96
* chore: export `generate::Writer` trait and string/file implementers by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/97
* chore(lib): export `core` types by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/98
* fix(ci): update CI runner cargo config instead of project's by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/99


**Full Changelog**: https://github.com/coffeebeats/build-a-proto/compare/v0.2.3...v0.2.4

## 0.2.3 (2026-01-17)

## What's Changed
* fix(ci): remove local action dependency from `setup-rust` action by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/84
* fix(ci): vendor crates next to crate manifest by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/86
* fix(ci): use correct vendor directory in `config.toml` by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/88
* fix: remove dependency on `lexical` by disabling unused feature by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/87


**Full Changelog**: https://github.com/coffeebeats/build-a-proto/compare/v0.2.2...v0.2.3

## 0.2.2 (2026-01-17)

## What's Changed
* fix(ci): make `version` input to `setup-rust` action not required by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/80
* chore(ci): rename `manifest` to more common `manifest-path` by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/81
* chore(ci): standardize `manifest-path` argument name in `setup-rust` action by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/83


**Full Changelog**: https://github.com/coffeebeats/build-a-proto/compare/v0.2.1...v0.2.2

## 0.2.1 (2026-01-17)

## What's Changed
* feat: add a `lib.rs` so consumers can use `baproto` as a crate dependency by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/78


**Full Changelog**: https://github.com/coffeebeats/build-a-proto/compare/v0.2.0...v0.2.1

## 0.2.0 (2026-01-16)

## What's Changed
* chore(deps): bump derive_more from 2.0.1 to 2.1.0 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/34
* chore(deps): bump chumsky from 0.10.1 to 0.12.0 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/32
* chore(deps): bump codecov/codecov-action from 5.4.3 to 5.5.2 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/33
* chore(deps): bump actions/download-artifact from 4.3.0 to 7.0.0 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/31
* chore(deps): bump anyhow from 1.0.98 to 1.0.100 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/30
* chore(deps): bump tj-actions/changed-files from 46.0.5 to 47.0.1 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/29
* chore(deps): bump thiserror from 2.0.12 to 2.0.17 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/28
* chore(deps): bump actions/checkout from 4.2.2 to 6.0.1 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/25
* chore(deps): bump ariadne from 0.5.1 to 0.6.0 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/26
* chore(deps): bump googleapis/release-please-action from 4.2.0 to 4.4.0 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/27
* feat(compile): add support for rooted imports by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/36
* feat(core): add `ImportRoot` and `SchemaImport` type wrappers to use paths more safely by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/37
* feat(compile): create a `link` phase to validate module dependencies by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/39
* fix(parse): wrap parser expressions with token spans by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/40
* feat(compile): create a symbol table to support semantic analysis by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/41
* feat(compiler,parse): create and use structured `PackageName` and `Reference` types by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/42
* refactor(syntax): define `PackageName` and `Reference` types in new `syntax` module by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/43
* feat(compile): create `register` function to add new compilation phase by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/44
* refactor: factor `lex` into its own module; use `chumsky`'s `Spanned` by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/45
* refactor(ast): migrate `PackageName` and `Reference` into `core`, factor out `Descriptor` into own file by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/46
* feat(ast): define AST types in `ast` mod by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/47
* fix(parse)!: use new `ast` types in parser; require field indices by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/48
* fix(lex,parse): use `u64` for parsed unsigned integers by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/49
* fix(ast,parse): handle doc comments on package declarations by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/50
* chore(deps): bump tempfile from 3.23.0 to 3.24.0 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/55
* chore(deps): bump clap from 4.5.42 to 4.5.53 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/54
* chore(deps): bump derive_more from 2.1.0 to 2.1.1 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/53
* chore(deps): bump actions/upload-artifact from 4.6.2 to 6.0.0 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/52
* refactor(ast)!: refine AST node types to include span data by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/56
* feat(visit): implement visitor pattern for `ast` nodes by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/57
* fix(ci): prevent `setup-rust` from updatingthe Cargo crate index unnecessarily by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/58
* refactor(mod): modularize lexer and add unit tests by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/59
* refactor(parse): refine parser, change output to `ast` nodes by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/62
* fix: create `LexResult`; using `usize::saturating_sub` to simplify conditional by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/63
* chore: create `AGENTS.md` to capture codebase patterns by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/64
* feat(ir): define IR types and implement lowering from AST nodes by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/65
* fix(ci): download prebuilt binaries in CI by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/66
* fix(ci): cache build artifacts when testing with code coverage by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/67
* chore: upgrade to Rust version `1.92.0` by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/68
* feat(analyze): define `Diagnostic` and associated reporter for issues reported during compilation by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/69
* chore!: remove legacy core IR and compiler implementations by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/70
* fix(compile): streamline `Symbols` and `Descriptor`, update IR types by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/71
* feat(analyze): implement basic field index and type reference analyzers  by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/72
* fix(write): simplify `Writer` implementations by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/73
* fix(compile): create basic compilation scaffold by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/74
* chore: allow/remove unused code by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/75
* feat(cmd,generate): restore end-to-end compilation after re-implementing code generation by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/76
* feat(tests): create integration tests for validating common compilation scenarios by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/77


**Full Changelog**: https://github.com/coffeebeats/build-a-proto/compare/v0.1.4...v0.2.0

## 0.1.4 (2025-08-01)

## What's Changed
* chore(deps): bump clap from 4.5.38 to 4.5.39 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/20
* chore(deps): bump rstest from 0.25.0 to 0.26.1 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/24
* chore(deps): bump clap from 4.5.39 to 4.5.42 by @dependabot[bot] in https://github.com/coffeebeats/build-a-proto/pull/23

## New Contributors
* @dependabot[bot] made their first contribution in https://github.com/coffeebeats/build-a-proto/pull/20

**Full Changelog**: https://github.com/coffeebeats/build-a-proto/compare/v0.1.3...v0.1.4

## 0.1.3 (2025-05-26)

## What's Changed
* feat: add `compile` command to the CLI by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/12
* feat(parse): implement a lexer for tokenizing input by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/14
* feat(parse): implement a parser with basic error reporting by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/15
* feat(core): define types used by the compiler by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/16
* fix(parse): add field index and name validation by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/17
* feat(compile): implement compilation of files by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/18
* feat(generate): implement generation components, add basic GDScript generator by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/19


**Full Changelog**: https://github.com/coffeebeats/build-a-proto/compare/v0.1.2...v0.1.3

## 0.1.2 (2025-05-16)

## What's Changed
* fix(ci): specify CI worklow permissions with least privilege by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/9
* fix(ci): cross-compile Linux builds to support lower `GLIBC` versions by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/11


**Full Changelog**: https://github.com/coffeebeats/build-a-proto/compare/v0.1.1...v0.1.2

## 0.1.1 (2025-05-16)

## What's Changed
* feat(ci): set up GitHub actions and workflows by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/6
* fix(ci): migrate `release-please` to newer action; use correct config paths by @coffeebeats in https://github.com/coffeebeats/build-a-proto/pull/7


**Full Changelog**: https://github.com/coffeebeats/build-a-proto/compare/v0.1.0...v0.1.1

## Changelog
