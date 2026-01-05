use crate::analyze::Diagnostic;
use crate::ast;
use crate::compile::symbol::{ModuleMetadata, Symbols, TypeKind};
use crate::core::{Descriptor, DescriptorBuilder, ImportRoot, PackageName, SchemaImport};
use crate::lex::Span;
use crate::visit::{Visitable, Visitor, walk};

/* -------------------------------------------------------------------------- */
/*                           Struct: TypeCollector                            */
/* -------------------------------------------------------------------------- */

/// Visitor that registers types directly into the symbol table during traversal.
///
/// This visitor extracts the package declaration and registers all type
/// definitions (messages and enums) immediately as they're discovered. It
/// tracks type descriptors for module metadata and collects any diagnostics.
pub struct TypeCollector<'a> {
    descriptors: Vec<Descriptor>,
    diagnostics: Vec<Diagnostic>,
    package: Option<PackageName>,
    path: Vec<String>,
    symbols: &'a mut Symbols,
}

/* -------------------------- Impl: TypeCollector --------------------------- */

impl<'a> TypeCollector<'a> {
    /// Registers all types and module metadata from a schema.
    ///
    /// This function visits the AST, registers types directly into the symbol
    /// table as they're discovered, and creates module metadata with dependency
    /// information.
    ///
    /// Returns a vector of diagnostics (empty on success).
    pub fn register(
        ast: &ast::Schema,
        import: &SchemaImport,
        symbols: &'a mut Symbols,
        import_roots: &[ImportRoot],
    ) -> Vec<Diagnostic> {
        // Visit AST and register types during traversal
        let mut collector = Self::new(symbols);
        ast.visit(&mut collector);

        // Check for errors during traversal
        if !collector.diagnostics.is_empty() {
            return collector.diagnostics;
        }

        // Ensure package was declared
        let package = match collector.package {
            Some(pkg) => pkg,
            None => {
                return vec![Diagnostic::error(
                    Span::default(),
                    "schema missing package declaration".to_string(),
                )];
            }
        };

        // Resolve dependencies
        let deps = ast
            .iter_includes()
            .filter_map(|inc| {
                // Try each import root in order
                for root in import_roots {
                    if let Ok(schema) = root.resolve_schema_import(&inc.path) {
                        return Some(schema);
                    }
                }
                None
            })
            .collect();

        // Register module metadata
        let metadata = ModuleMetadata {
            package,
            deps,
            types: collector.descriptors,
        };

        collector.symbols.insert_module(import.clone(), metadata);

        Vec::new() // Success
    }

    /// Creates a new type collector that registers into the given symbol table.
    fn new(symbols: &'a mut Symbols) -> Self {
        Self {
            symbols,
            package: None,
            path: Vec::new(),
            descriptors: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Builds a descriptor for the current type being visited.
    fn build_descriptor(&self, name: &str) -> Option<Descriptor> {
        let package = self.package.as_ref()?;

        DescriptorBuilder::default()
            .package(package.clone())
            .path(self.path.clone())
            .name(name.to_string())
            .build()
            .ok()
    }

    /// Registers a type immediately in the symbol table.
    fn register_type(&mut self, name: &str, kind: TypeKind) {
        // Can only register if we've seen the package declaration
        if let Some(descriptor) = self.build_descriptor(name) {
            // Register immediately in symbol table!
            self.symbols.insert_type(descriptor.clone(), kind);

            // Track for module metadata
            self.descriptors.push(descriptor);
        }
    }
}

/* ---------------------- Impl: Visitor<TypeCollector> --------------------- */

impl<'ast> Visitor<'ast> for TypeCollector<'_> {
    fn visit_package(&mut self, pkg: &'ast ast::Package) {
        match PackageName::try_from(pkg.clone()) {
            Ok(name) => self.package = Some(name),
            Err(e) => self.diagnostics.push(Diagnostic::error(
                pkg.span.clone(),
                format!("invalid package name: {}", e),
            )),
        }
    }

    fn visit_message(&mut self, msg: &'ast ast::Message) {
        // Register this message
        self.register_type(&msg.name.name, TypeKind::Message);

        // Push onto path for nested types
        self.path.push(msg.name.name.clone());

        // Visit nested types
        walk::walk_message(self, msg);

        // Pop from path
        self.path.pop();
    }

    fn visit_enum(&mut self, enum_: &'ast ast::Enum) {
        // Register this enum
        self.register_type(&enum_.name.name, TypeKind::Enum);

        // Push onto path for nested types
        self.path.push(enum_.name.name.clone());

        // Visit nested types
        walk::walk_enum(self, enum_);

        // Pop from path
        self.path.pop();
    }
}
