use thiserror::Error;

use crate::core::Registry;

/* -------------------------------------------------------------------------- */
/*                             Enum: CompileError                             */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Error)]
pub enum CompileError {
    // TODO: Add variants for type validation errors
}

/* ------------------------------- Fn: compile ------------------------------ */

/// Validates types and computes binary layouts for serialization.
///
/// This function is called after the link phase has validated all module
/// dependencies and detected any cycles.
#[allow(unused_variables)]
pub fn compile(registry: &mut Registry) -> Result<(), CompileError> {
    // TODO: Validate type references (fields reference existing types)
    // TODO: Compute binary layouts for serialization

    Ok(())
}
