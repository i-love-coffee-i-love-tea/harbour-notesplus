# ADR-002: Plain-Text AsciiDoc Files on the Filesystem as Canonical Storage

#### Status
Accepted

#### Context
Note-taking systems frequently store user notes in monolithic SQLite databases, proprietary binary schemas, or vendor-locked cloud backends. When storage schemas change or applications are abandoned, users face data lock-in and high migration friction. Furthermore, mobile device file synchronization (via Syncthing, Nextcloud, or Git) requires granular file-level conflict resolution.

#### Decision
Store all user notes as standard, human-readable AsciiDoc (`.adoc`) files directly on the local filesystem in the user's document directory (`~/Documents/Notes++` or XDG document paths).

AsciiDoc was chosen over Markdown due to its standard specification and native support for:
- Admonition blocks (`NOTE`, `TIP`, `IMPORTANT`, `WARNING`, `CAUTION`)
- Multi-column tables with alignment, cell spans, and multipliers (`[cols="2,1"]`, `[cols="3*"]`)
- Delimited blocks (source code, sidebars, quotes, verse, open blocks)
- Document attributes and title metadata (`:toc:`, `:icons: font`)
- Inline formatting, keyboard/button/menu macros, and cross-references

#### Consequences
##### Positive / Utility Delivered
- **Zero Vendor Lock-In**: Notes remain completely portable plain-text files that can be viewed, edited, or backed up with any text editor or CLI tool.
- **Transparent Multi-Device Sync**: Works out of the box with file synchronization engines (Syncthing, Nextcloud, Git, rsync) without database merge conflicts.
- **Rich Semantic Markup**: Provides structured technical documentation capabilities beyond standard Markdown without external plugins.

##### Trade-offs / Mitigations
- Full-text search cannot rely on scanning every plain-text file on demand without mobile I/O latency; this is resolved by maintaining an asynchronous derived SQLite FTS5 search index (see [ADR-004](004-sqlite-fts5-as-derived-search-cache.md)).
