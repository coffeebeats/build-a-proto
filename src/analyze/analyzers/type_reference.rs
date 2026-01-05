use crate::analyze::{Analyzer, Diagnostic};
use crate::ast;
use crate::compile::Symbols;
use crate::core::{Descriptor, DescriptorBuilder, PackageName};
use crate::visit::{Visitor, walk};

/* -------------------------------------------------------------------------- */
/*                       Struct: TypeReferenceResolver                        */
/* -------------------------------------------------------------------------- */

/// Analyzer that validates all type references resolve to existing types.
pub struct TypeReferenceResolver<'a> {
    symbols: &'a Symbols,
    package: Option<PackageName>,
    scope: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}

/* ----------------------- Impl: TypeReferenceResolver ---------------------- */

impl<'a> TypeReferenceResolver<'a> {
    /// Creates a new type reference resolver.
    pub fn new(symbols: &'a Symbols) -> Self {
        Self {
            symbols,
            package: None,
            scope: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Attempts to resolve a type reference.
    fn resolve_reference(&self, reference: &ast::Reference) -> Option<Descriptor> {
        let package = self.package.as_ref()?;

        // Build fully qualified name from reference components
        let ref_parts: Vec<String> = reference
            .components
            .iter()
            .map(|c| c.name.clone())
            .collect();

        // For absolute references (starting with .), search from root
        if reference.components.first().map(|c| c.name.as_str()) == Some("") {
            // Absolute reference - build descriptor from package root
            let descriptor = DescriptorBuilder::default()
                .package(package.clone())
                .path(ref_parts[1..].to_vec())
                .build()
                .ok()?;

            return if self.symbols.contains(&descriptor) {
                Some(descriptor)
            } else {
                None
            };
        }

        // Relative reference - search outward from current scope
        // Try current scope first, then progressively remove nesting
        for depth in (0..=self.scope.len()).rev() {
            let mut path = self.scope[..depth].to_vec();
            path.extend_from_slice(&ref_parts[..ref_parts.len().saturating_sub(1)]);

            let name = ref_parts.last().cloned();

            let descriptor = DescriptorBuilder::default()
                .package(package.clone())
                .path(path)
                .name(name.unwrap_or_default())
                .build()
                .ok()?;

            if self.symbols.contains(&descriptor) {
                return Some(descriptor);
            }
        }

        None
    }
}

/* ------------------------- Impl: Analyzer --------------------------------- */

impl Analyzer for TypeReferenceResolver<'_> {
    fn drain_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }
}

/* ------------------- Impl: Visitor<TypeReferenceResolver> ---------------- */

impl<'ast> Visitor<'ast> for TypeReferenceResolver<'_> {
    fn visit_package(&mut self, pkg: &'ast ast::Package) {
        // Extract and store package name
        if let Ok(package) = PackageName::try_from(pkg.clone()) {
            self.package = Some(package);
        }
    }

    fn visit_message(&mut self, msg: &'ast ast::Message) {
        // Push message name onto scope
        self.scope.push(msg.name.name.clone());

        // Continue walking
        walk::walk_message(self, msg);

        // Pop from scope
        self.scope.pop();
    }

    fn visit_enum(&mut self, enum_: &'ast ast::Enum) {
        // Push enum name onto scope
        self.scope.push(enum_.name.name.clone());

        // Continue walking
        walk::walk_enum(self, enum_);

        // Pop from scope
        self.scope.pop();
    }

    fn visit_reference(&mut self, reference: &'ast ast::Reference) {
        // Try to resolve the reference
        if self.resolve_reference(reference).is_none() {
            let ref_name = reference
                .components
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join(".");

            self.diagnostics.push(Diagnostic::error(
                reference.span.clone(),
                format!("unresolved type reference '{}'", ref_name),
            ));
        }
    }
}
