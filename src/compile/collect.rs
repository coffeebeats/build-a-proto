use crate::analyze::Diagnostic;
use crate::ast;
use crate::compile::symbol::Symbols;
use crate::core::{Descriptor, DescriptorBuilder, ImportRoot, PackageName, SchemaImport};
use crate::ir::lower::TypeKind;
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
    symbols: &'a mut Symbols<TypeKind>,
}

/* -------------------------- Impl: TypeCollector --------------------------- */

impl<'a> TypeCollector<'a> {
    /// Registers all types and module metadata from a schema.
    ///
    /// This function visits the AST and registers types directly into the
    /// symbol table as they're discovered.
    ///
    /// Returns a vector of diagnostics (empty on success).
    pub fn register(
        ast: &ast::Schema,
        _import: &SchemaImport,
        symbols: &'a mut Symbols<TypeKind>,
        import_roots: &[ImportRoot],
    ) -> Vec<Diagnostic> {
        let mut collector = Self::new(symbols);
        ast.visit(&mut collector);

        if !collector.diagnostics.is_empty() {
            return collector.diagnostics;
        }

        let _package_name = match collector.package {
            Some(pkg) => pkg,
            None => {
                return vec![Diagnostic::error(
                    Span::default(),
                    "schema missing package declaration",
                )];
            }
        };

        let _deps = ast
            .iter_includes()
            .filter_map(|inc| {
                // Try each import root in order.
                for root in import_roots {
                    if let Ok(schema) = root.resolve_schema_import(&inc.path) {
                        return Some(schema);
                    }
                }
                None
            })
            .collect::<Vec<_>>();

        Vec::new()
    }

    fn new(symbols: &'a mut Symbols<TypeKind>) -> Self {
        Self {
            symbols,
            package: None,
            path: Vec::new(),
            descriptors: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn descriptor(&self, name: &str) -> Option<Descriptor> {
        let package = self.package.as_ref()?;

        let mut path = self.path.clone();
        path.push(name.to_owned());

        DescriptorBuilder::default()
            .package(package.clone())
            .path(path)
            .build()
            .ok()
    }

    fn register_type(&mut self, name: &str, kind: TypeKind) {
        if let Some(descriptor) = self.descriptor(name) {
            self.symbols.insert(descriptor.clone(), kind);

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

    fn visit_include(&mut self, node: &'ast ast::Include) {
        if self.package.is_none() {
            self.diagnostics.push(Diagnostic::error(
                node.span.clone(),
                "'include' statement cannot come before 'package' declaration",
            ));
        }
    }

    fn visit_message(&mut self, msg: &'ast ast::Message) {
        if self.package.is_none() {
            self.diagnostics.push(Diagnostic::error(
                msg.span.clone(),
                "'message' definition cannot come before 'package' declaration",
            ));
        }

        self.register_type(&msg.name.name, TypeKind::Message);

        self.path.push(msg.name.name.clone());

        walk::walk_message(self, msg);

        self.path.pop();
    }

    fn visit_enum(&mut self, enm: &'ast ast::Enum) {
        if self.package.is_none() {
            self.diagnostics.push(Diagnostic::error(
                enm.span.clone(),
                "'enum' definition cannot come before 'package' declaration",
            ));
        }

        self.register_type(&enm.name.name, TypeKind::Enum);

        self.path.push(enm.name.name.clone());

        walk::walk_enum(self, enm);

        self.path.pop();
    }
}
