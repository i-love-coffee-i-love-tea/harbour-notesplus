# Architecture Decision Records

ADRs in this project follow the [MADR](https://adr.github.io/madr/) format.

## Naming Convention

```
NNN-descriptive-slug.md
```

- `NNN`: zero-padded 3-digit number (001, 002, ...)
- `descriptive-slug`: lowercase, hyphens, no special characters

## MADR Template (required for ADRs 011+)

```markdown
---
status: proposed | accepted | deprecated | superseded by NNN
date: YYYY-MM-DD
---

# Short title

## Context and Problem Statement

Why is this decision needed?

## Decision Outcome

Chosen option: ..., because ...

### Consequences

- Good, because ...
- Bad, because ...
```

## Linting

The release pipeline validates ADRs 011+ against the MADR spec.
Run `./scripts/release.sh --dry-run X.Y.Z` to check.
