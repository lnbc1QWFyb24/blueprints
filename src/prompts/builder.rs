use crate::prompts::embedded;
use anyhow::{Result, anyhow};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const WORKSPACE_BLUEPRINTS_MD: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/BLUEPRINTS.md"));

/// Whitelisted prompt modules (slug equals filename without `.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Module {
    BlueprintsReference,
    InteractionStyle,
    Design,
    Update,
    ImplementationStandards,
    Review,
    ParsingRules,
    WorkspaceConstraints,
    DeliveryPlan,
    TokensOutputProtocol,
    ImplementBuilder,
    ImplementReviewer,
}

impl Module {
    pub const fn slug(self) -> &'static str {
        match self {
            Module::BlueprintsReference => "blueprints-reference",
            Module::InteractionStyle => "interaction-style",
            Module::Design => "design",
            Module::Update => "update",
            Module::ImplementationStandards => "implementation-standards",
            Module::Review => "review",
            Module::ParsingRules => "parsing-rules",
            Module::WorkspaceConstraints => "workspace-constraints",
            Module::DeliveryPlan => "delivery-plan",
            Module::TokensOutputProtocol => "tokens-output-protocol",
            Module::ImplementBuilder => "implement-builder",
            Module::ImplementReviewer => "implement-reviewer",
        }
    }
}

/// Builder-pattern API to compose a full prompt from modular sections.
///
/// Features:
/// - Add literal sections (inline markdown).
/// - Add module sections by slug or path (reads from `modules_dir`).
/// - Optional variable interpolation for `${VARS}` (simple replace).
/// - Idempotent deduplication by slug (same module only included once).
/// - Simple formatting normalization (collapse blank lines, ensure trailing newline).
#[derive(Debug, Clone)]
pub struct PromptBuilder {
    sections: Vec<Section>,
    seen_modules: BTreeSet<Module>,
    modules_dir: PathBuf,
    variables: BTreeMap<String, String>,
    inline_blueprints_md: bool,
}

#[derive(Debug, Clone)]
enum Section {
    Module { module: Module, path: PathBuf },
}

impl PromptBuilder {
    /// Create a new `PromptBuilder` with a base directory for modules
    /// (defaults to `src/prompts/modules`).
    pub fn new(modules_dir: impl AsRef<Path>) -> Self {
        Self {
            sections: Vec::new(),
            seen_modules: BTreeSet::new(),
            modules_dir: modules_dir.as_ref().to_path_buf(),
            variables: BTreeMap::new(),
            inline_blueprints_md: false,
        }
    }

    /// Convenience: use repo-default modules directory `src/prompts/modules`.
    pub fn with_default_modules_dir() -> Self {
        Self::new("src/prompts/modules")
    }

    /// Provide a `${KEY}` → `value` substitution applied after module composition.
    pub fn with_variable(mut self, key: impl AsRef<str>, value: impl Into<String>) -> Self {
        let key = key.as_ref().trim();
        if key.is_empty() {
            return self;
        }
        self.variables.insert(key.to_string(), value.into());
        self
    }

    /// Convenience helper for setting `${BLUEPRINTS_DIR}`.
    pub fn with_blueprints_dir(self, dir: impl Into<String>) -> Self {
        self.with_variable("BLUEPRINTS_DIR", dir.into())
    }

    /// Inline the workspace `BLUEPRINTS.md` contents when building the prompt.
    pub fn inline_blueprints(mut self) -> Self {
        self.inline_blueprints_md = true;
        self
    }

    /// Add a module (enum-backed slug) mapping to `<modules_dir>/<slug>.md`.
    /// Deduplicated by module (first wins).
    pub fn add_module(mut self, module: Module) -> Self {
        if self.seen_modules.contains(&module) {
            return self;
        }
        let slug = module.slug();
        let path = self.modules_dir.join(format!("{slug}.md"));
        self.sections.push(Section::Module { module, path });
        self.seen_modules.insert(module);
        self
    }

    /// Build the final prompt contents.
    pub fn build(self) -> Result<String> {
        let mut out = String::new();

        for s in self.sections {
            match s {
                Section::Module { module, path } => match fs::read_to_string(&path) {
                    Ok(content) => {
                        out.push_str(&content);
                        if !content.ends_with('\n') {
                            out.push('\n');
                        }
                        out.push('\n');
                    }
                    Err(_) => {
                        if let Some(content) = embedded::get(module.slug()) {
                            out.push_str(content);
                            if !content.ends_with('\n') {
                                out.push('\n');
                            }
                            out.push('\n');
                        } else {
                            return Err(anyhow!(
                                "module '{}' not found at {} and no embedded copy present",
                                module.slug(),
                                path.display()
                            ));
                        }
                    }
                },
            }
        }

        normalize_markdown(&mut out);

        let mut rendered = out;
        for (key, value) in self.variables {
            let token = format!("${{{key}}}");
            rendered = rendered.replace(&token, &value);
        }

        if self.inline_blueprints_md {
            rendered = inline_blueprints_md(&rendered);
        }

        Ok(rendered)
    }
}

fn normalize_markdown(s: &mut String) {
    // Collapse runs of >2 newlines to 2
    let mut out = String::with_capacity(s.len());
    let mut newline_count = 0u8;
    for ch in s.chars() {
        if ch == '\n' {
            newline_count = newline_count.saturating_add(1);
            if newline_count <= 2 {
                out.push('\n');
            }
        } else {
            newline_count = 0;
            out.push(ch);
        }
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    *s = out;
}

fn inline_blueprints_md(template: &str) -> String {
    let content = WORKSPACE_BLUEPRINTS_MD;

    let mut combined = String::with_capacity(template.len() + content.len() + 128);
    combined.push_str(template);
    if !template.ends_with('\n') {
        combined.push('\n');
    }
    combined.push_str("\n### BLUEPRINTS.md (inline)\n\n");
    combined.push_str(content);
    combined
}

/// Convenience presets for composing full prompts by role/profile.
/// You can adapt or extend the sequences here without changing callers.
pub enum Profile {
    Design,
    Update,
    Review,
    ImplementBuilder,
    ImplementReviewer,
}

impl Profile {
    /// Return a builder preloaded with a sensible section order for the profile.
    /// Callers may still append or insert more sections before building.
    pub fn compose(self) -> PromptBuilder {
        use Module::{
            BlueprintsReference, DeliveryPlan, Design, ImplementBuilder, ImplementReviewer,
            ImplementationStandards, InteractionStyle, ParsingRules, Review, TokensOutputProtocol,
            Update, WorkspaceConstraints,
        };

        let builder = PromptBuilder::with_default_modules_dir();
        match self {
            Profile::Design => builder
                .add_module(BlueprintsReference)
                .add_module(ParsingRules)
                .add_module(InteractionStyle)
                .add_module(DeliveryPlan)
                .add_module(Design),
            Profile::Update => builder
                .add_module(BlueprintsReference)
                .add_module(ParsingRules)
                .add_module(InteractionStyle)
                .add_module(Update)
                .add_module(ImplementationStandards),
            Profile::Review => builder
                .add_module(BlueprintsReference)
                .add_module(ParsingRules)
                .add_module(ImplementationStandards)
                .add_module(InteractionStyle)
                .add_module(Review),
            Profile::ImplementBuilder => builder
                .add_module(WorkspaceConstraints)
                .add_module(ImplementationStandards)
                .add_module(DeliveryPlan)
                .add_module(TokensOutputProtocol)
                .add_module(InteractionStyle)
                .add_module(ImplementBuilder),
            Profile::ImplementReviewer => builder
                .add_module(WorkspaceConstraints)
                .add_module(BlueprintsReference)
                .add_module(ParsingRules)
                .add_module(ImplementationStandards)
                .add_module(DeliveryPlan)
                .add_module(TokensOutputProtocol)
                .add_module(InteractionStyle)
                .add_module(ImplementReviewer),
        }
    }
}

// Composition is defined in code via Profile presets and handlers.

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn appends_blueprints_for_reference_module() -> Result<()> {
        let prompt = PromptBuilder::with_default_modules_dir()
            .add_module(Module::BlueprintsReference)
            .inline_blueprints()
            .build()?;

        assert!(prompt.contains("### BLUEPRINTS.md (inline)"));
        assert!(prompt.contains("Blueprints describe everything"));
        Ok(())
    }

    #[test]
    fn appends_blueprints_when_placeholder_missing() {
        let template = "# Stub\n";
        let inlined = super::inline_blueprints_md(template);

        assert!(inlined.starts_with("# Stub"));
        assert!(inlined.contains("### BLUEPRINTS.md (inline)"));
        assert!(inlined.contains("Blueprints describe everything"));
    }
}
