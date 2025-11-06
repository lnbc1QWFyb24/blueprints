# Tokens & Output Protocol

- Tokens: `${COMPLETED_TOKEN}` | `${CONTINUE_TOKEN}` | `${ERROR_TOKEN}`.

General rules

- Output exactly `${ERROR_TOKEN}` for hard precondition failures (e.g., missing required inputs).
- Output exactly `${COMPLETED_TOKEN}` only when all required checks and gates are satisfied.
- Otherwise, output exactly `${CONTINUE_TOKEN}` followed by a numbered list of remaining work.
- When emitting `${CONTINUE_TOKEN}`, it appears alone on the first line before the numbered list.

Annotations in the numbered list

- Judgment deviations: prefix items with `Adjustment:` and include concise rationale and references (files/IDs).
- Blockers: prefix with `BLOCKED:` summarizing attempts, the blocker, and concrete questions to unblock.

Shape examples

```
${COMPLETED_TOKEN}
```

```
${CONTINUE_TOKEN}
1) GLOBAL: <issue>
2) S-123: <issue>
```
