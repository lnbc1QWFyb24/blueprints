ROLE
- Reviewer: derive a minimal, high-value set of test vectors to cover the Spec with MVP-first rigor.

INPUTS (read-only)
- Spec: ./blueprints/02-spec.md (authoritative)
- Requirements: ./blueprints/01-requirements.md (for R-id refs)
- Contracts: ./blueprints/03-contracts.md (handwritten Markdown with `C-###` items for Rust types, external APIs, links; may be missing or empty)
- Test Vectors: ./blueprints/04-test-vectors.md (may be missing)

DETERMINISM
- Stable order: process S-IDs in Spec file order (as they appear in ./blueprints/02-spec.md). Within each S-id context, reference R-IDs in ascending numeric order.
- No randomness or rephrasing; identical inputs -> identical plan.

PARSING (fast, exact)
- Requirements: lines matching `^R-[0-9]{3} - (.+)[.!?]$`; extract R-id and text.
- Spec: records-only S-lines matching `^S-([0-9]{3}(\.[0-9]+)?) \| R:(R-[0-9]{3}(,R-[0-9]{3})*) \| DO:(.+?) \| TITLE:([^|]+)( \| .+)?$` (optional IF/ER/LM/OB fields may follow); extract S-id and R-id list.
- Contracts (optional, Markdown):
  - Skim `C-###` items to identify: Rust type names and code blocks; external APIs (methods/paths), auth schemes; error mappings; and doc links.
  - When present, prefer adding Integration (`L:I`) vectors that exercise concrete external API interactions and type serialization/deserialization mandated by Spec clauses and described in the Contracts doc.
- TITLE is mandatory, must follow DO: immediately, and precede any optional IF/ER/LM/OB fields; keep TITLE concise and stable for reuse.

POLICY (80/20)
- Prefer one vector per core S-id; merge multiple S-ids when a single vector suffices.
- Cover happy paths and critical boundaries needed for a robust first release; defer exhaustive edge cases.
- Do not invent behaviors; include determinism only when essential.
- Propose actions only for existing S-ids and R-ids.
- Coverage guarantee: if any S-id in Spec has no referencing TV in ./blueprints/04-test-vectors.md, propose a minimal ADD line for that S-id (default `L:U`) to ensure baseline coverage.
- Contracts-aware vectors: when `./blueprints/03-contracts.md` lists concrete external APIs or types exercised by any Spec clause, include at least one vector per such clause to validate request/response shapes and error codes; mark `L:I` where IO/HTTP occurs.
- Merge constraints: only merge S-ids that share an identical R set; otherwise emit separate ADD lines per S-id.
- Referential subset for merges: the plan's R list must be a subset of every referenced S-id's R list to keep plans deterministic and valid.

PRECONDITIONS
- If Requirements or Spec are missing or empty, output exactly: ${ERROR_TOKEN}

COMPLETION
- If the current vectors satisfy this policy, output exactly: ${COMPLETED_TOKEN}

PLAN FORMAT (records-only)
- Output only lines between:
  ---PLAN START---
  ...
  ---PLAN END---
- Each action is one line, ordered by S-id in Spec file order:
  - ADD | R:... | S:... | L:<U|I|P[,U|I|P...]> | GIVEN:... | WHEN:... | THEN:...[ | ERR:...][ | DET:...][ | OB:...]
  - REPLACE TV-### | R:... | S:... | L:<U|I|P[,U|I|P...]> | GIVEN:... | WHEN:... | THEN:...[ | ...]
  - REMOVE TV-###
- Include both S and R ids (from Spec) for traceability. Keep lines concise and testable.
- L codes legend: U = Unit-level/isolated checks; I = Integration/cross-component/IO; P = Property-based behavior. These codes drive Delivery Plan integration and proptest flags.
- Merged S-ids rule: only merge S-ids that have identical R sets; the plan's R list must be a subset of each merged S-id's R list to keep plans deterministic and valid.

BEGIN REVIEW NOW.
