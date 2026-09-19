---
status: accepted
date: 2026-09-19
---

# Use MADR format for Architecture Decision Records

## Context and Problem Statement

The project needs a structured way to record architectural decisions so future contributors understand why things are built the way they are. Decisions should be versioned alongside the code.

## Decision Outcome

Use MADR (Markdown Any Decision Records) for all ADRs numbered 011 and above. ADRs 001–010 are grandfathered in whatever format they were written.

### Consequences

- Good, because MADR is a well-known standard with clear required sections
- Good, because the release pipeline can lint ADRs automatically
- Neutral, because existing ADRs (001–010) are exempt from the new format
