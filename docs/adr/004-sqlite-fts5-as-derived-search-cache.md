# ADR-004: SQLite FTS5 Index as an Asynchronous, Disposable Search Cache

#### Status
Accepted

#### Context
Searching across dozens or hundreds of plain-text `.adoc` files on resource-constrained mobile hardware with slow flash storage leads to noticeable UI freezes and battery drain if done via naive filesystem grep on every keystroke. 

Conversely, treating a database as the primary source of truth violates our plain-text portability principles ([ADR-002](002-asciidoc-as-canonical-storage-format.md)).

#### Decision
Implement a local SQLite database (`index.db`) utilizing the **FTS5 (Full-Text Search 5)** extension strictly as an ephemeral, derived search cache.

1. **Derived Cache Contract**: The database is never the authoritative source of truth. If missing, corrupted, or deleted, it is automatically recreated and populated by scanning the `.adoc` files in the notes directory.
2. **Asynchronous Indexing**: Document indexing occurs asynchronously in worker threads upon document creation, modification, or on-demand background sync.
3. **FTS5 Features**: Leverage BM25 relevance ranking, Porter stemming, prefix matching, and snippet extraction to deliver instant search results with highlighted matching excerpts.

#### Consequences
##### Positive / Utility Delivered
- **Sub-Millisecond Search Queries**: Instant search-as-you-type response times across the entire note collection on mobile hardware.
- **Zero Risk of Data Loss**: The SQLite database can be deleted at any time without losing any note content or structure.
- **Rich Snippets & Ranking**: Provides relevant text snippets with highlighted query tokens in search result lists.

##### Trade-offs / Mitigations
- Periodic disk synchronization is required to keep the FTS index aligned with external filesystem changes (e.g., changes made via Syncthing); this is mitigated by quick incremental mtime checks on startup.
