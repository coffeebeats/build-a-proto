# Commands

## **baproto `compile`**

Compiles the specified build-a-proto definition files into language-specific bindings.

### Usage

`baproto compile [OPTIONS] <--cpp|--gdscript> <FILES>...`

### Options

- `--cpp` — generate c++ bindings
- `--gdscript` — generate GDScript bindings
- `-o`, `--out <OUT_DIR>` — a directory in which to write generated bindings to

### Arguments

- `<FILES>` — one or more filepaths to build-a-proto definitions
