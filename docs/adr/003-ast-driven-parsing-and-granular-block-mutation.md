# ADR-003: AST-Based Block Parsing with In-Place Mutation Pipeline

#### Status
Accepted

#### Context
A note application requires fast interactive operations on specific document parts: toggling a checklist item, editing an individual paragraph or code block, appending a task to today's journal section, or inserting a section break. 

Using ad-hoc regular expressions or naive string replacements for partial document updates easily leads to corrupted formatting, broken nested blocks, or escaped delimiter errors.

#### Decision
Implement a strongly typed Abstract Syntax Tree (AST) in `notesplusplus-core`:
1. **Typed AST Representation**: Parse AsciiDoc documents into recursive `Block` nodes (Paragraph, Heading, List, Table, Code, Admonition, Quote, Sidebar, PageBreak) containing structured `InlineSpan` spans.
2. **Deterministic Re-Serialization**: Implement deterministic `ToAsciiDoc` serialization on AST blocks to round-trip modified ASTs back to valid AsciiDoc markup without formatting drift.
3. **Unified Mutation Pipeline (`mutate_page_blocks`)**: Channel all partial updates (checkbox toggling, block saves, section appends) through a single atomic pipeline:
   - Read source file from disk
   - Parse into AST
   - Apply block-level mutation closure
   - Serialize AST back to AsciiDoc
   - Write back to disk atomically
   - Update SQLite FTS5 search index

#### Consequences
##### Positive / Utility Delivered
- **Guaranteed Structural Integrity**: Partial edits and checkbox toggles cannot corrupt surrounding blocks, table rows, or code fences.
- **Single Source of Truth for Document Operations**: Eliminates duplicate file I/O, parsing, and re-indexing logic across the codebase.
- **Fine-Grained QML / Web Rendering**: UI delegates can render individual blocks natively (e.g. specialized Table, Code, and Checklist components).

##### Trade-offs / Mitigations
- Parsing large documents into an in-memory AST introduces slight CPU overhead; this is mitigated by efficient linear line scanners, pre-allocated vector buffers, and asynchronous worker thread execution.
