# Commands

## **baproto `compile`**

Compiles the specified build-a-proto definition files into language-specific bindings.

### Usage

`baproto compile [OPTIONS] <PATH>...`

### Options

- `--cpp` — generate c++ bindings (cannot be specified with other languages; default=`true`)
- `--gdscript` — generate GDScript bindings (cannot be specified with other languages; default=`false`)
- `-o`, `--out` — a directory in which to write generated bindings to

### Arguments

- `<PATH>` — one or more filepaths to build-a-proto definitions
