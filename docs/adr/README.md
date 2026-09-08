# Architecture Decision Records (ADRs)

This directory documents the key architectural decisions made in **Notes++** (`harbour-notesplusplus`).

## Index of Decisions

| ID | Title | Status |
| :--- | :--- | :--- |
| [ADR-001](001-hybrid-core-and-gui-architecture.md) | Separation of Pure Rust Core Engine and Sailfish Silica UI Bridge | Accepted |
| [ADR-002](002-asciidoc-as-canonical-storage-format.md) | Plain-Text AsciiDoc Files on the Filesystem as Canonical Storage | Accepted |
| [ADR-003](003-ast-driven-parsing-and-granular-block-mutation.md) | AST-Based Block Parsing with In-Place Mutation Pipeline | Accepted |
| [ADR-004](004-sqlite-fts5-as-derived-search-cache.md) | SQLite FTS5 Index as an Asynchronous, Disposable Search Cache | Accepted |
| [ADR-005](005-embedded-zero-dependency-web-server.md) | Lightweight Embedded HTTP Server with Server-Sent Events (SSE) | Accepted |
| [ADR-006](006-public-network-rejection-and-local-isolation.md) | Socket-Level Public IP Rejection and Session Persistence Strategy | Accepted |
| [ADR-007](007-offline-vendored-web-assets.md) | Zero External CDN Dependency and Vendored Vue 3 ESM Assets | Accepted |
| [ADR-008](008-provider-agnostic-llm-adapter-architecture.md) | Extensible LLM Client Adapter for Local and Cloud AI Backends | Accepted |
| [ADR-009](009-presentation-mode-document-segmentation.md) | Dynamic Slide Deck Partitioning via Headings and Page Breaks | Accepted |

## ADR Template

New records should follow the structure below:

```markdown
### ADR-XXX: [Title]

#### Status
[Proposed | Accepted | Deprecated | Superseded]

#### Context
[Describe the problem, requirements, operational constraints, and considerations.]

#### Decision
[Specify the architectural design, abstraction, or trade-off chosen.]

#### Consequences
##### Positive / Utility Delivered
- [Key benefit or capability unlocked]

##### Trade-offs / Mitigations
- [Constraint accepted or managed risk]
```
