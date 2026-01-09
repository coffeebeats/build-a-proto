# Minimal Migration Plan: `generate` → `generate2` Behavior

Adapt `src/generate` (original from main) to work with modern IR types while maintaining the original style and structure.

## Goal

Keep the elegant visitor pattern and consolidated traits from `src/generate`, but adapt to work with `ir::*` types instead of `crate::core::*`. The result should behave like `generate2` (tested, working) while preserving the original architecture.

## What We're Keeping (Original Style)

- **`Generator<W: Writer>` trait** - Visitor pattern with begin/end hooks (11 methods)
- **`generate()` orchestrator** - Centralized traversal logic in `generator.rs`
- **`CodeWriter`** - Indentation/formatting abstraction in `code.rs`
- **Module structure** - `code.rs`, `write/`, `lang/` organization
- **Consolidated approach** - Single generator trait, no layering

## Minimal Changes Required

### 1. Writer Trait (`src/generate/write/mod.rs`)

**Keep lifecycle, adapt types:**

```diff
+use crate::ir;
-use crate::core::Module;

 pub trait Writer: Default {
-    fn configured(self, module: &Module) -> Result<Self>;
+    fn configured(self, pkg: &ir::Package) -> Result<Self>;
     fn open(&mut self, path: &Path) -> Result<()>;
     fn write(&mut self, text: &str) -> anyhow::Result<()>;
     fn close(&mut self) -> Result<()>;
 }
```

**Why:** Writers manage file lifecycle and flushing. Keep existing pattern, just change `Module` → `ir::Package`.

### 2. FileWriter & StringWriter

**Update `configured()` parameter:**
```diff
-impl Writer for FileWriter {
-    fn configured(self, module: &Module) -> Result<Self> { ... }
-}
+impl Writer for FileWriter {
+    fn configured(self, pkg: &ir::Package) -> Result<Self> { ... }
+}

-impl Writer for StringWriter {
-    fn configured(self, module: &Module) -> Result<Self> { ... }
-}
+impl Writer for StringWriter {
+    fn configured(self, pkg: &ir::Package) -> Result<Self> { ... }
+}
```

### 3. Generator Trait (`src/generate/generator.rs`)

**Replace Registry with Schema, Module with Package, add overridable methods:**

```diff
 pub trait Generator<W: Writer> {
+    // Configuration & file paths
-    fn configure_writer(&self, out_dir: &Path, module: (&Descriptor, &mut Module), writer: W)
-        -> Result<(PathBuf, W)>;
+    fn configure_writer(&self, out_dir: &Path, pkg: &ir::Package, writer: W)
+        -> Result<(PathBuf, W)>;

+    // Schema-level hooks
-    fn gen_begin(&mut self, r: &Registry, w: Vec<(&PathBuf, &mut W)>) -> Result<()>;
-    fn gen_end(&mut self, r: &Registry, w: Vec<(&PathBuf, &mut W)>) -> Result<()>;
+    fn gen_begin(&mut self, schema: &ir::Schema, w: Vec<(&PathBuf, &mut W)>) -> Result<()>;
+    fn gen_end(&mut self, schema: &ir::Schema, w: Vec<(&PathBuf, &mut W)>) -> Result<()>;

+    // Package-level hooks
-    fn mod_begin(&mut self, r: &Registry, m: Described<&Module>, w: &mut W) -> Result<()>;
-    fn mod_end(&mut self, r: &Registry, m: Described<&Module>, w: &mut W) -> Result<()>;
+    fn pkg_begin(&mut self, schema: &ir::Schema, pkg: &ir::Package, w: &mut W) -> Result<()>;
+    fn pkg_end(&mut self, schema: &ir::Schema, pkg: &ir::Package, w: &mut W) -> Result<()>;

+    // Cross-package references
-    fn gen_include(&mut self, r: &Registry, m: Described<&Module>, w: &mut W) -> Result<()>;
+    fn gen_include(&mut self, schema: &ir::Schema, pkg: &ir::Package, w: &mut W) -> Result<()>;

+    // Message hooks
-    fn gen_msg_begin(&mut self, r: &Registry, m: Described<&Message>, w: &mut W) -> Result<()>;
-    fn gen_msg_end(&mut self, r: &Registry, m: Described<&Message>, w: &mut W) -> Result<()>;
+    fn gen_msg_begin(&mut self, schema: &ir::Schema, msg: &ir::Message, w: &mut W) -> Result<()>;
+    fn gen_msg_end(&mut self, schema: &ir::Schema, msg: &ir::Message, w: &mut W) -> Result<()>;

+    // Enum hooks
-    fn gen_enum_begin(&mut self, r: &Registry, e: Described<&Enum>, w: &mut W) -> Result<()>;
-    fn gen_enum_end(&mut self, r: &Registry, e: Described<&Enum>, w: &mut W) -> Result<()>;
+    fn gen_enum_begin(&mut self, schema: &ir::Schema, e: &ir::Enum, w: &mut W) -> Result<()>;
+    fn gen_enum_end(&mut self, schema: &ir::Schema, e: &ir::Enum, w: &mut W) -> Result<()>;

+    // Field/variant hooks
-    fn gen_field(&mut self, r: &Registry, f: &Field, w: &mut W) -> Result<()>;
-    fn gen_variant(&mut self, r: &Registry, v: &Variant, w: &mut W) -> Result<()>;
+    fn gen_field(&mut self, schema: &ir::Schema, field: &ir::Field, current_pkg: &str, w: &mut W) -> Result<()>;
+    fn gen_variant(&mut self, schema: &ir::Schema, variant: &ir::Variant, current_pkg: &str, w: &mut W) -> Result<()>;
+
+    // NEW: Overridable generation methods (default impls provided)
+    fn gen_pkg(&mut self, schema: &ir::Schema, pkg: &ir::Package, w: &mut W) -> Result<()> {
+        // Default impl: generate enums, then messages, with includes
+        self.pkg_begin(schema, pkg, w)?;
+
+        // Generate includes for cross-package references
+        for dep_pkg in find_package_dependencies(schema, pkg) {
+            self.gen_include(schema, dep_pkg, w)?;
+        }
+
+        for e in &pkg.enums {
+            self.gen_enum(schema, e, &pkg.path, w)?;
+        }
+
+        for msg in &pkg.messages {
+            self.gen_msg(schema, msg, &pkg.path, w)?;
+        }
+
+        self.pkg_end(schema, pkg, w)?;
+        Ok(())
+    }
+
+    fn gen_msg(&mut self, schema: &ir::Schema, msg: &ir::Message, current_pkg: &str, w: &mut W) -> Result<()> {
+        // Default impl: generate nested types BEFORE parent (for Rust)
+        for e in &msg.enums {
+            self.gen_enum(schema, e, current_pkg, w)?;
+        }
+
+        for nested in &msg.messages {
+            self.gen_msg(schema, nested, current_pkg, w)?;
+        }
+
+        self.gen_msg_begin(schema, msg, w)?;
+
+        for field in &msg.fields {
+            self.gen_field(schema, field, current_pkg, w)?;
+        }
+
+        self.gen_msg_end(schema, msg, w)?;
+        Ok(())
+    }
+
+    fn gen_enum(&mut self, schema: &ir::Schema, e: &ir::Enum, current_pkg: &str, w: &mut W) -> Result<()> {
+        // Default impl: generate enum with variants
+        self.gen_enum_begin(schema, e, w)?;
+
+        for variant in &e.variants {
+            self.gen_variant(schema, variant, current_pkg, w)?;
+        }
+
+        self.gen_enum_end(schema, e, w)?;
+        Ok(())
+    }
 }
+
+// Helper to find cross-package dependencies
+fn find_package_dependencies<'a>(schema: &'a ir::Schema, pkg: &ir::Package) -> Vec<&'a ir::Package> {
+    // Scan all messages/enums in pkg for references to other packages
+    // Return unique list of referenced packages
+    // Implementation details...
+}
```

**Key changes:**
- ✅ Keep `configure_writer()` - handles filepath management per package
- ✅ Keep `gen_include()` - needed for cross-package type references
- ✅ Keep `Vec<(&PathBuf, &mut W)>` in gen_begin/end - needed for multi-file coordination
- ✂️ Remove `Described<T>` wrapper - IR types self-describe
- ✂️ Remove `&Registry` - replace with `&ir::Schema` where needed
- 🔄 Rename `mod_*` → `pkg_*` (packages, not modules)
- ➕ Add `&ir::Schema` parameter to most methods - for cross-package lookups
- ➕ Add `current_pkg: &str` to field/variant - for type resolution
- ➕ Add `gen_pkg()`, `gen_msg()`, `gen_enum()` with default impls - overridable for languages with different nesting rules

### 4. Orchestrator Function (`src/generate/generator.rs`)

**Simplify orchestrator - most logic moves to trait default impls:**

```diff
-pub fn generate<P, W, G>(out_dir: P, r: &mut Registry, g: &mut G) -> Result<()>
+pub fn generate<P, W, G>(out_dir: P, schema: &ir::Schema, g: &mut G) -> Result<()>
 where
     P: AsRef<Path>,
     W: Writer,
     G: Generator<W>,
 {
     let out_dir = out_dir.as_ref().to_path_buf();
     if !out_dir.is_dir() {
         std::fs::create_dir_all(&out_dir)?;
     }

     let mut writers = HashMap::<PathBuf, W>::default();

-    for (d, m) in r.iter_modules_mut() {
-        let w = W::default().configured(m)?;
-        let (path, w) = g.configure_writer(&out_dir, (d, m), w)?;
+    for pkg in &schema.packages {
+        let w = W::default().configured(pkg)?;
+        let (path, w) = g.configure_writer(&out_dir, pkg, w)?;
         w.open(&path)?;
         writers.insert(path.clone(), w);
     }

-    g.gen_begin(r, writers.iter_mut().collect())?;
+    g.gen_begin(schema, writers.iter_mut().collect())?;

-    for (d, m) in r.iter_modules() {
-        let w = writers.get_mut(&m.path).ok_or(anyhow!("missing module: {}", d))?;
+    for pkg in &schema.packages {
+        // Need to look up writer by path from configure_writer result
+        let w = writers.iter_mut()
+            .find(|(p, _)| p.ends_with(&format!("{}.{}", pkg.path.replace('.', "/"), "ext")))
+            .map(|(_, w)| w)
+            .ok_or(anyhow!("missing writer for package: {}", pkg.path))?;

-        g.mod_begin(r, (d, m), w)?;
-
-        for dep_path in &m.deps {
-            let (dep_desc, dep) = r.get_module_by_path(dep_path)?;
-            g.gen_include(r, (dep_desc, dep), w)?;
-        }
-
-        for d in &m.enums {
-            let enm = r.get_enum(d)?;
-            gen_enum(r, (d, enm), g, w)?;
-        }
-
-        for d in &m.messages {
-            let msg = r.get_message(d)?;
-            gen_msg(r, (d, msg), g, w)?;
-        }
-
-        g.mod_end(r, (d, m), w)?;
+        // Delegate to trait method (default impl handles nested types & includes)
+        g.gen_pkg(schema, pkg, w)?;
     }

-    g.gen_end(r, writers.iter_mut().collect())?;
+    g.gen_end(schema, writers.iter_mut().collect())?;

     for (_, mut w) in writers {
         w.close()?;
     }

     Ok(())
 }
```

**Note:** Helper functions `gen_msg()` and `gen_enum()` are now trait methods with default implementations. Languages can override these for custom nesting behavior (e.g., Python might want fields inside class bodies, while Rust needs nested types defined separately).

### 6. CodeWriter (`src/generate/code.rs`)

**Enhance with tested helpers from generate2:**

```diff
 impl CodeWriter {
     // Change return type for builder pattern
-    pub fn indent(&mut self) -> Result<()> {
+    pub fn indent(&mut self) -> &mut Self {
         self.indented += 1;
-        Ok(())
+        self
     }

-    pub fn outdent(&mut self) -> Result<()> {
+    pub fn outdent(&mut self) -> &mut Self {
-        if self.indented == 0 {
-            return Err(anyhow!("cannot outdent further"));
-        }
+        debug_assert!(self.indented > 0, "cannot outdent below zero");
-        self.indented -= 1;
-        Ok(())
+        self.indented = self.indented.saturating_sub(1);
+        self
     }

     // Fix indentation: write BEFORE text, not after newline
-    pub fn newline<W: Writer>(&self, writer: &mut W) -> Result<()> {
-        writer.write(&self.newline_token)?;
-        writer.write(&self.get_indent())?;
+    fn write_indent<W: Writer>(&self, w: &mut W) -> Result<()> {
+        w.write(&self.get_indent())?;
         Ok(())
     }

+    pub fn newline<W: Writer>(&self, w: &mut W) -> Result<()> {
+        w.write(&self.newline_token)?;
+        Ok(())
+    }
+
+    pub fn writeln<W: Writer>(&self, w: &mut W, text: &str) -> Result<()> {
+        self.write_indent(w)?;
+        w.write(text)?;
+        w.write(&self.newline_token)?;
+        Ok(())
+    }
+
+    pub fn writeln_no_indent<W: Writer>(&self, w: &mut W, text: &str) -> Result<()> {
+        w.write(text)?;
+        w.write(&self.newline_token)?;
+        Ok(())
+    }
+
-    pub fn comment<W: Writer>(&mut self, writer: &mut W, input: &str) -> Result<()> {
-        writer.write(&format!("{} {}", self.comment_token, input))?;
-        self.newline(writer)?;
+    pub fn comment<W: Writer>(&self, w: &mut W, text: &str) -> Result<()> {
+        self.write_indent(w)?;
+        if text.is_empty() {
+            w.write(&self.comment_token)?;
+        } else {
+            w.write(&format!("{} {}", self.comment_token, text))?;
+        }
+        w.write(&self.newline_token)?;
         Ok(())
     }
+
+    pub fn comment_block<W: Writer>(&self, w: &mut W, text: &str) -> Result<()> {
+        for line in text.lines() {
+            self.comment(w, line)?;
+        }
+        Ok(())
+    }
+
+    pub fn comment_opt<W: Writer>(&self, w: &mut W, text: Option<&str>) -> Result<()> {
+        if let Some(text) = text {
+            self.comment_block(w, text)?;
+        }
+        Ok(())
+    }
+
+    pub fn blank_line<W: Writer>(&self, w: &mut W) -> Result<()> {
+        w.write(&self.newline_token)?;
+        Ok(())
+    }
+
+    pub fn indented<W, F>(&mut self, w: &mut W, f: F) -> Result<()>
+    where
+        W: Writer,
+        F: FnOnce(&mut Self, &mut W) -> Result<()>,
+    {
+        self.indent();
+        let result = f(self, w);
+        self.outdent();
+        result
+    }
+
+    pub fn block<W, F>(&mut self, w: &mut W, header: &str, footer: &str, f: F) -> Result<()>
+    where
+        W: Writer,
+        F: FnOnce(&mut Self, &mut W) -> Result<()>,
+    {
+        self.writeln(w, header)?;
+        self.indented(w, f)?;
+        self.writeln(w, footer)?;
+        Ok(())
+    }
+
+    pub fn braced_block<W, F>(&mut self, w: &mut W, header: &str, f: F) -> Result<()>
+    where
+        W: Writer,
+        F: FnOnce(&mut Self, &mut W) -> Result<()>,
+    {
+        self.block(w, &format!("{} {{", header), "}", f)
+    }
 }
```

**Key fixes:**
- Write indentation BEFORE each line of text (not after newlines)
- `indent()`/`outdent()` return `&mut Self` for chaining
- Add `writeln()`, `comment_opt()`, `comment_block()`, `blank_line()`
- Add block helpers: `indented()`, `block()`, `braced_block()`

### 7. Module Exports (`src/generate/mod.rs`)

**Update re-exports:**

```diff
+use std::collections::HashMap;
+use std::path::PathBuf;
+
+use serde::{Deserialize, Serialize};
+use thiserror::Error;
+
 mod code;
 mod generator;
 mod lang;
 mod write;

 pub use generator::*;
 pub use write::*;
 pub use code::*;

+// Re-export for convenience
+pub use crate::ir;
+
-pub fn gdscript<W>() -> impl Generator<W>
-where W: Writer
-{
-    lang::GDScript::default()
-}
+pub use lang::{rust, gdscript};
+
+#[derive(Debug, Clone, Default, Serialize, Deserialize)]
+pub struct GeneratorOutput {
+    pub files: HashMap<PathBuf, String>,
+}
+
+impl GeneratorOutput {
+    pub fn new() -> Self {
+        Self::default()
+    }
+
+    pub fn add(&mut self, path: impl Into<PathBuf>, content: impl Into<String>) {
+        self.files.insert(path.into(), content.into());
+    }
+}
+
+#[derive(Debug, Error)]
+pub enum GeneratorError {
+    #[error("generation failed: {0}")]
+    Generation(String),
+
+    #[error("I/O error: {0}")]
+    Io(#[from] std::io::Error),
+}
```

### 8. GDScript Generator (`src/generate/lang/gdscript.rs`)

**Update to use IR types:**

```diff
+use std::path::PathBuf;
+
-use crate::core::Type;
+use crate::ir;
 use crate::generate::{CodeWriter, CodeWriterBuilder, Generator, Writer};

 #[derive(Clone, Debug)]
 pub struct GDScript(CodeWriter);

 impl Default for GDScript {
     fn default() -> Self {
         Self(
             CodeWriterBuilder::default()
                 .comment_token("##".to_owned())
                 .indent_token("  ".to_owned())
                 .newline_token("\n".to_owned())
                 .build()
                 .unwrap(),
         )
     }
 }

 impl<W: Writer> Generator<W> for GDScript {
-    fn configure_writer(&self, out_dir: &Path, (_, module): (&Descriptor, &mut Module), w: W)
-        -> Result<(PathBuf, W)>
-    {
-        let path = out_dir.join(module.package[0].clone()).with_extension("gd");
-        module.path = path.clone();
-        let w = w.configured(module)?;
-        Ok((path, w))
+    fn file_path(&self, pkg: &ir::Package) -> PathBuf {
+        let parts: Vec<&str> = pkg.path.split('.').collect();
+        PathBuf::from(parts.last().unwrap_or(&pkg.path)).with_extension("gd")
     }

-    fn gen_begin(&mut self, _: &Registry, _: Vec<(&PathBuf, &mut W)>) -> Result<()> {
+    fn gen_begin(&mut self, _: &ir::Schema) -> Result<()> {
         Ok(())
     }

-    fn gen_end(&mut self, _: &Registry, _: Vec<(&PathBuf, &mut W)>) -> Result<()> {
+    fn gen_end(&mut self, _: &ir::Schema) -> Result<()> {
         Ok(())
     }

-    fn mod_begin(&mut self, _: &Registry, _: (&Descriptor, &Module), w: &mut W) -> Result<()> {
+    fn pkg_begin(&mut self, _: &ir::Package, w: &mut W) -> Result<()> {
         self.0.write(w, "extends RefCounted")?;
         self.0.newline(w)?;
         self.0.newline(w)?;
         Ok(())
     }

-    fn mod_end(&mut self, _: &Registry, _: (&Descriptor, &Module), w: &mut W) -> Result<()> {
+    fn pkg_end(&mut self, _: &ir::Package, w: &mut W) -> Result<()> {
         self.0.newline(w)?;
         Ok(())
     }

-    fn gen_include(&mut self, _: &Registry, (desc, m): (&Descriptor, &Module), w: &mut W)
-        -> Result<()>
-    {
-        // Remove this method - no longer in trait
-    }
-
-    fn gen_msg_begin(&mut self, _: &Registry, (_, msg): (&Descriptor, &Message), w: &mut W)
-        -> Result<()>
-    {
+    fn gen_msg_begin(&mut self, msg: &ir::Message, w: &mut W) -> Result<()> {
+        self.0.comment_opt(w, msg.doc.as_deref())?;
         self.0.write(w, &format!("class {}:", msg.name))?;
         self.0.newline(w)?;
-        self.0.indent()?;
+        self.0.indent();
         Ok(())
     }

-    fn gen_msg_end(&mut self, _: &Registry, _: (&Descriptor, &Message), w: &mut W)
-        -> Result<()>
-    {
-        self.0.outdent()?;
+    fn gen_msg_end(&mut self, _: &ir::Message, w: &mut W) -> Result<()> {
+        self.0.outdent();
         self.0.newline(w)?;
         Ok(())
     }

-    fn gen_enum_begin(&mut self, _: &Registry, (_, e): (&Descriptor, &Enum), w: &mut W)
-        -> Result<()>
-    {
+    fn gen_enum_begin(&mut self, e: &ir::Enum, w: &mut W) -> Result<()> {
+        self.0.comment_opt(w, e.doc.as_deref())?;
         self.0.write(w, &format!("enum {}:", e.name))?;
         self.0.newline(w)?;
-        self.0.indent()?;
+        self.0.indent();
         Ok(())
     }

-    fn gen_enum_end(&mut self, _: &Registry, _: (&Descriptor, &Enum), w: &mut W)
-        -> Result<()>
-    {
-        self.0.outdent()?;
+    fn gen_enum_end(&mut self, _: &ir::Enum, w: &mut W) -> Result<()> {
+        self.0.outdent();
         self.0.newline(w)?;
         Ok(())
     }

-    fn gen_field(&mut self, r: &Registry, f: &Field, w: &mut W) -> Result<()> {
-        let type_name = self.type_name(r, &f.r#type)?;
+    fn gen_field(&mut self, field: &ir::Field, current_pkg: &str, w: &mut W) -> Result<()> {
+        let type_name = self.type_name(&field.encoding.native, current_pkg);
+        self.0.comment_opt(w, field.doc.as_deref())?;
-        self.0.write(w, &format!("var {}: {}", f.name, type_name))?;
+        self.0.write(w, &format!("var {}: {}", field.name, type_name))?;
         self.0.newline(w)?;
         Ok(())
     }

-    fn gen_variant(&mut self, _: &Registry, v: &Variant, w: &mut W) -> Result<()> {
+    fn gen_variant(&mut self, variant: &ir::Variant, _: &str, w: &mut W) -> Result<()> {
-        self.0.write(w, &format!("{} = {}", v.name, v.tag))?;
+        match variant {
+            ir::Variant::Unit { name, tag, doc, .. } => {
+                self.0.comment_opt(w, doc.as_deref())?;
+                self.0.write(w, &format!("{} = {}", name, tag))?;
+            }
+            ir::Variant::Field { name, tag, doc, .. } => {
+                // GDScript doesn't support variant fields, treat as unit
+                self.0.comment_opt(w, doc.as_deref())?;
+                self.0.write(w, &format!("{} = {}", name, tag))?;
+            }
+        }
         self.0.newline(w)?;
         Ok(())
     }
 }

 impl GDScript {
-    fn type_name(&self, r: &Registry, t: &Type) -> Result<String> {
-        match t {
-            Type::Array(inner) => Ok(format!("Array[{}]", self.type_name(r, inner)?)),
-            Type::Map { .. } => Ok("Dictionary".to_owned()),
-            Type::Reference(desc) => Ok(desc.name().to_owned()),
-            Type::Scalar(s) => Ok(match s {
-                Scalar::Bool => "bool",
-                Scalar::Float32 | Scalar::Float64 => "float",
-                Scalar::Int8 | Scalar::Int16 | Scalar::Int32 | Scalar::Int64 => "int",
-                Scalar::String => "String",
-                Scalar::UInt8 | Scalar::UInt16 | Scalar::UInt32 | Scalar::UInt64 => "int",
-            }.to_owned()),
+    fn type_name(&self, native: &ir::NativeType, current_pkg: &str) -> String {
+        match native {
+            ir::NativeType::Bool => "bool".to_owned(),
+            ir::NativeType::Int { .. } => "int".to_owned(),
+            ir::NativeType::Float { .. } => "float".to_owned(),
+            ir::NativeType::String => "String".to_owned(),
+            ir::NativeType::Bytes => "PackedByteArray".to_owned(),
+            ir::NativeType::Array { element } => {
+                let inner = self.type_name(&element.native, current_pkg);
+                format!("Array[{}]", inner)
+            }
+            ir::NativeType::Map { .. } => "Dictionary".to_owned(),
+            ir::NativeType::Message { descriptor } | ir::NativeType::Enum { descriptor } => {
+                descriptor.split('.').last().unwrap_or(descriptor).to_owned()
+            }
         }
     }
 }
+
+pub fn gdscript<W: Writer>() -> impl Generator<W> {
+    GDScript::default()
+}
```

### 9. Add Rust Generator (`src/generate/lang/rust.rs`)

**New file - port from `generate2/codegen/rust.rs`:**

Create a complete Rust generator similar to GDScript but generating Rust syntax. Key methods:

- `file_path()` - Convert package path to `foo/bar.rs`
- `gen_begin/gen_end()` - No-ops
- `pkg_begin()` - File header, imports
- `pkg_end()` - No footer needed
- `gen_msg_begin()` - `#[derive(...)]` + `pub struct Name {`
- `gen_msg_end()` - Close struct, generate `impl` block with `new()`, stub `encode/decode`, `Default` impl
- `gen_enum_begin()` - `#[derive(...)]` + `pub enum Name {`
- `gen_enum_end()` - Close enum
- `gen_field()` - `pub name: Type,` with doc comments
- `gen_variant()` - Handle Unit and Field variants
- `type_name()` - Convert `ir::NativeType` to Rust type strings
- `default_value()` - Generate default value expressions

### 10. Update `src/generate/lang/mod.rs`

```diff
 mod gdscript;
+mod rust;

 pub use gdscript::*;
+pub use rust::*;
```

## Implementation Sequence

1. **Writer trait** - Simplify to `write()` + `finish()`
2. **FileWriter & StringWriter** - Update implementations
3. **Generator trait** - Adapt to IR types, rename methods
4. **Orchestrator** - Rewrite `generate()` for Schema-based traversal
5. **CodeWriter** - Add enhanced helpers, fix indentation
6. **Module exports** - Add `GeneratorOutput`, `GeneratorError`
7. **GDScript** - Update to use IR types
8. **Rust generator** - Port from `generate2`
9. **Test** - Verify with golden tests from `generate2`

## Critical Files to Modify

- `src/generate/write/mod.rs` - Writer trait
- `src/generate/write/file.rs` - FileWriter impl
- `src/generate/write/string.rs` - StringWriter impl
- `src/generate/generator.rs` - Generator trait + orchestrator
- `src/generate/code.rs` - CodeWriter enhancements
- `src/generate/mod.rs` - Module exports
- `src/generate/lang/gdscript.rs` - Update to IR types
- `src/generate/lang/rust.rs` - NEW - Rust generator
- `src/generate/lang/mod.rs` - Export rust module

## Success Criteria

- All `generate2` golden tests pass using `src/generate` architecture
- Original visitor pattern and style preserved
- No layering (single `Generator<W>` trait, not split into Generator + CodeGen)
- Minimal modifications to achieve IR type compatibility

## Additional Notes

### Cross-Package Type Resolution

When a type in one package references a type in another package, `gen_include()` must be called to generate the appropriate import/include statement. The `find_package_dependencies()` helper function scans all fields/variants in a package to find referenced packages.

**Example**: If `pkg.foo` has a message with a field of type `pkg.bar.Type`, then when generating `pkg.foo`, we must call `gen_include(schema, bar_package, writer)` to generate:
- Rust: `use crate::bar::Type;`
- GDScript: `const BAR := preload("bar.gd")`

### Integration Test Required

Create test case with two packages where one references the other:

**Package A** (`test.pkga`):
```protobuf
message A {
  uint32 id = 1;
}
```

**Package B** (`test.pkgb`):
```protobuf
import "pkga.proto";

message B {
  pkga.A ref = 1;
}
```

Expected generated code should include proper import/include statements and verify compilation succeeds.

## Summary of Changes from Original Plan

1. **Writer trait** - KEEP lifecycle methods (configured, open, write, close), just change `Module` → `ir::Package`
2. **configure_writer()** - KEEP in Generator trait, handles filepath determination
3. **gen_include()** - KEEP in Generator trait, needed for cross-package imports
4. **gen_pkg(), gen_msg(), gen_enum()** - ADD to Generator trait with default impls, allows language-specific nesting
5. **GDScript** - Update all method signatures to new trait, implement `gen_include()`, handle cross-package type names
6. **Integration test** - ADD test for cross-package type references

---

## FINAL UPDATE - Per User Feedback

### Writer Trait - Already Simplified!

The Writer trait has been simplified by the user - NO changes needed:

```rust
pub trait Writer: Default {
    fn close(&mut self) -> anyhow::Result<()>;
    fn open<T>(&mut self, path: T) -> anyhow::Result<()> where T: AsRef<Path>;
    fn write(&mut self, input: &str) -> anyhow::Result<()>;
}
```

- ✅ No `configured()` method - path passed to `open()` directly
- ✅ Simple lifecycle: `open()` → `write()` → `close()`
- ✅ FileWriter & StringWriter already implement this

### Naming Change

- `GeneratorOutput` → `Output`
- Update all references throughout the code

### Updated Orchestrator

With simplified Writer, the orchestrator becomes:

```rust
pub fn generate<P, W, G>(out_dir: P, schema: &ir::Schema, g: &mut G) -> Result<Output>
where
    P: AsRef<Path>,
    W: Writer,
    G: Generator<W>,
{
    let out_dir = out_dir.as_ref().to_path_buf();
    if !out_dir.is_dir() {
        std::fs::create_dir_all(&out_dir)?;
    }

    let mut writers = HashMap::<PathBuf, W>::default();

    // Create and open writers for each package
    for pkg in &schema.packages {
        let mut w = W::default();
        let path = g.configure_writer(&out_dir, pkg)?;  // Returns PathBuf now
        w.open(&path)?;
        writers.insert(path, w);
    }

    g.gen_begin(schema, writers.iter_mut().collect())?;

    for pkg in &schema.packages {
        let w = writers.iter_mut()
            .find(|(p, _)| /* match by path */)
            .map(|(_, w)| w)
            .ok_or(anyhow!("missing writer"))?;

        g.gen_pkg(schema, pkg, w)?;
    }

    g.gen_end(schema, writers.iter_mut().collect())?;

    let mut output = Output::new();
    for (path, mut w) in writers {
        w.close()?;
        // Collect generated content into Output
    }

    Ok(output)
}
```

### Updated Generator Trait Signature

```rust
pub trait Generator<W: Writer> {
    // Returns PathBuf for package (no longer returns Writer)
    fn configure_writer(&self, out_dir: &Path, pkg: &ir::Package) -> Result<PathBuf>;

    // ... rest of trait unchanged
}
```

