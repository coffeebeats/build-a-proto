# **build-a-proto** ![GitHub release (with filter)](https://img.shields.io/github/v/release/coffeebeats/build-a-proto) ![GitHub](https://img.shields.io/github/license/coffeebeats/build-a-proto) [![Build Status](https://img.shields.io/github/actions/workflow/status/coffeebeats/build-a-proto/check-commit.yml?branch=main)](https://github.com/coffeebeats/build-a-proto/actions?query=branch%3Amain+workflow%3Acheck) [![codecov](https://codecov.io/gh/coffeebeats/build-a-proto/graph/badge.svg)](https://codecov.io/gh/coffeebeats/build-a-proto)

An interface definition language (IDL) compiler for creating custom, bit-level binary encodings with an emphasis on supporting networked simulations.

> ⚠️ **WARNING:** This project is in a very early stage. API instability, missing features, and bugs are to be expected for now.

## **How it works**

TODO

## **Getting started**

These instructions will help you install `build-a-proto` and compile your own custom binary encodings.

### **Example usage**

TODO

### **Installation**

See [docs/installation.md](./docs/installation.md#installation) for detailed instructions on how to download `build-a-proto`.

## **API Reference**

### **Commands**

See [docs/commands.md](./docs/commands.md) for a detailed reference on how to use each command.

## **Development**

### Setup

The following instructions outline how to get the project set up for local development:

1. [Follow the instructions](https://www.rust-lang.org/tools/install) to install Rust (see [Cargo.toml](./Cargo.toml) for the minimum required version).
2. Clone the [coffeebeats/build-a-proto](https://github.com/coffeebeats/build-a-proto) repository.
3. Install the tools [used below](#code-submission) by following each of their specific installation instructions.

### Testing

Integration tests use comparisons to golden output files; these need to be regenerated when `baproto`'s implementation is updated. To regenerate golden files, run the tests with the `BAPROTO_UPDATE_GOLDENS` environment variable set to `1`:

```sh
BAPROTO_UPDATE_GOLDENS=1 cargo test
```

### Code submission

When submitting code for review, ensure the following requirements are met:

1. The project is correctly formatted using [rustfmt](https://github.com/rust-lang/rustfmt):

    ```sh
    cargo fmt
    ```

2. All [clippy](https://github.com/rust-lang/rust-clippy) linter warnings are addressed:

    ```sh
    cargo clippy \
        --all-features \
        --all-targets \
        --no-deps \
        -- \
            --deny=warnings
    ```

3. All unit tests pass:

    ```sh
    cargo test \
        --all-features \
        --all-targets \
        --frozen \
        --release
    ```

4. The `build-a-proto` binary successfully compiles using [Cross](https://github.com/cross-rs/cross) (release artifacts will be available in `./target`). Follow the [installation instructions](https://github.com/cross-rs/cross#installation) to ensure `cross` is installed on the development system.

    ```sh
    cross build \
        --manifest-path=Cargo.toml \
        --profile=release \
        --frozen \
        --all-targets
    ```

## **Contributing**

All contributions are welcome! Feel free to file [bugs](https://github.com/coffeebeats/build-a-proto/issues/new?assignees=&labels=bug&projects=&template=bug-report.md&title=) and [feature requests](https://github.com/coffeebeats/build-a-proto/issues/new?assignees=&labels=enhancement&projects=&template=feature-request.md&title=) and/or open pull requests.

## **Version history**

See [CHANGELOG.md](https://github.com/coffeebeats/build-a-proto/blob/main/CHANGELOG.md).

## **License**

[MIT License](https://github.com/coffeebeats/build-a-proto/blob/main/LICENSE)
