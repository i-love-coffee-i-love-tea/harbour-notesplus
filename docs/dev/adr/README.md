# Architecture Decision Records

ADRs in this project are authored in AsciiDoc (`.adoc`) format following the template and structure defined in `000-index.adoc`.

## Naming Convention

```
NNN-descriptive-slug.adoc
```

- `NNN`: zero-padded 3-digit number (`001`, `002`, ...)
- `descriptive-slug`: lowercase, hyphens, no special characters

## AsciiDoc ADR Template

```asciidoc
= ADR-NNN: [Title]
:status: Proposed
:date: YYYY-MM-DD
:toc:
:icons: font

[NOTE]
.Status: [Proposed | Accepted | Deprecated | Superseded]
Summary of the proposal and current approval state.

== Context
Describe the architectural problem, requirements, operational constraints, and considerations.

== Decision
Specify the technical design, abstractions, and trade-offs chosen.

== Consequences

=== Positive / Utility Delivered
* icon:check[] Key benefit or capability unlocked.

=== Trade-offs & Mitigations
* icon:warning[] Constraint accepted or managed operational risk.
```

## Linting

The release pipeline validates all ADRs (`001-*.adoc` through `NNN-*.adoc`) against the AsciiDoc specification.
Run `./scripts/release.d/12-validate-adr-format.sh` or `./scripts/release.sh --dry-run X.Y.Z` to check.
