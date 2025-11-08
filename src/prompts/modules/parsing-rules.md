# Parsing Rules (fast, exact)

- Requirements: lines matching `^R-[0-9]{3} - (.+)[.!?]$`; extract R-id and text; preserve numeric order; ASCII only.
- Spec: records-only S-lines matching `^S-([0-9]{3}(\\.[0-9]+)?) \\| R:(R-[0-9]{3}(,R-[0-9]{3})*) \\| DO:([^|]+?) \\| TITLE:([^|]+)( \\| IF:[^|]+)?( \\| ER:[^|]+)?( \\| LM:[^|]+)?( \\| OB:[^|]+)?$`.
  - Extract S-id, R-id list, DO text, and required TITLE text; optional fields may follow (`IF/ER/LM/OB`) in canonical order when present.
- Contracts: heading entries `### C-### — <Title>`; skim code blocks for Rust types; detect external endpoints (`<METHOD> <path>`), auth schemes, request/response shapes, error mapping, and doc links.
- Test Vectors: records-only TV lines matching `^TV-([0-9]{3}) \\| R:(R-[0-9]{3}(,R-[0-9]{3})*) \\| S:(S-[0-9]{3}(\\.[0-9]+)?(,S-[0-9]{3}(\\.[0-9]+)?)*) \\| L:([UIP](,[UIP])*) \\| GIVEN:.+ \\| WHEN:.+ \\| THEN:.+`.
  - Extract TV id, S-id list, R-id list, and Level set `L:` (U, I, P; one or more as CSV).
