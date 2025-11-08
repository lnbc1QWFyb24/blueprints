// Embedded copies of prompt modules for portability when running outside the repo.
// Lookup by slug (filename without .md).

#[allow(clippy::too_many_lines)]
pub fn get(slug: &str) -> Option<&'static str> {
    match slug {
        // Consolidated modules
        "blueprints-reference" => Some(include_str!("modules/blueprints-reference.md")),
        "interaction-style" => Some(include_str!("modules/interaction-style.md")),
        "implementation-standards" => Some(include_str!("modules/implementation-standards.md")),
        "delivery-plan" => Some(include_str!("modules/delivery-plan.md")),
        "design" => Some(include_str!("modules/design.md")),
        "update" => Some(include_str!("modules/update.md")),
        "review" => Some(include_str!("modules/review.md")),
        "parsing-rules" => Some(include_str!("modules/parsing-rules.md")),
        "implement-builder" => Some(include_str!("modules/implement-builder.md")),
        "implement-reviewer" => Some(include_str!("modules/implement-reviewer.md")),

        // Standalone canonical modules
        "workspace-constraints" => Some(include_str!("modules/workspace-constraints.md")),
        "tokens-output-protocol" => Some(include_str!("modules/tokens-output-protocol.md")),

        // Builders for design-time flows
        "requirements-builder" => Some(include_str!("modules/requirements-builder.md")),
        "spec-builder" => Some(include_str!("modules/spec-builder.md")),
        "test-vectors-builder" => Some(include_str!("modules/test-vectors-builder.md")),
        "delivery-plan-builder" => Some(include_str!("modules/delivery-plan-builder.md")),

        _ => None,
    }
}
