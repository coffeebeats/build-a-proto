use derive_more::Display;
use std::path::PathBuf;

use crate::core::PackageName;
use crate::core::PackageNameError;
use crate::lex::Span;

/* -------------------------------------------------------------------------- */
/*                               Struct: Package                              */
/* -------------------------------------------------------------------------- */

/// `Package` defines a package declaration within a schema file.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("{}", itertools::join(components, ","))]
pub struct Package {
    pub comment: Option<super::CommentBlock>,
    pub components: Vec<super::Ident>,
    pub span: Span,
}

/* ----------------------- Impl: TryInto<PackageName> ----------------------- */

impl TryFrom<Package> for PackageName {
    type Error = PackageNameError;

    fn try_from(value: Package) -> Result<Self, Self::Error> {
        PackageName::try_from(
            value
                .components
                .iter()
                .map(|id| &id.name)
                .collect::<Vec<_>>(),
        )
    }
}

/* -------------------------------------------------------------------------- */
/*                               Struct: Include                              */
/* -------------------------------------------------------------------------- */

/// `Include` represents an include statement for importing other schema files.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("{}", path.display())]
pub struct Include {
    pub path: PathBuf,
    pub span: Span,
}
