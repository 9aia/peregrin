//! Pass metadata — declares what a pass consumes and produces.
//!
//! Used for:
//! - Ordering validation (`Pipeline::validate`)
//! - Tooling (pipeline visualization, pass selection)
//! - Documentation

/// Category of a pass — used for tooling and filtering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PassCategory {
    /// Reads IR, doesn't transform (e.g., lint, typecheck).
    #[default]
    Analysis,
    /// Modifies IR (e.g., optimization, dead-code elimination).
    Transform,
    /// Produces final output (e.g., code generation).
    Generate,
    /// User-defined category.
    Custom,
}

/// Metadata describing a pass's inputs, outputs, and category.
#[derive(Clone, Debug, Default)]
pub struct PassMetadata {
    /// Human-readable name (e.g., `"dead-field-elim"`).
    pub name: &'static str,
    /// IR stage names this pass reads (e.g., `["extracted"]`).
    pub consumes: &'static [&'static str],
    /// IR stage names this pass writes (e.g., `["extracted"]`).
    pub produces: &'static [&'static str],
    /// What kind of pass this is.
    pub category: PassCategory,
}

impl PassMetadata {
    pub fn new(name: &'static str) -> Self {
        PassMetadata {
            name,
            ..Default::default()
        }
    }

    pub fn with_consumes(mut self, consumes: &'static [&'static str]) -> Self {
        self.consumes = consumes;
        self
    }

    pub fn with_produces(mut self, produces: &'static [&'static str]) -> Self {
        self.produces = produces;
        self
    }

    pub fn with_category(mut self, category: PassCategory) -> Self {
        self.category = category;
        self
    }
}
