# ADR-010: Note Storage Directory Convention and Asset Directories

#### Status
Accepted

#### Context
Notes++ stores user notes as plain-text `.adoc` files on the filesystem (see [ADR-002](002-asciidoc-as-canonical-storage-format.md)). Users organize notes into groups by creating subdirectories. The UI displays these groups in a hierarchical tree.

However, some documents reference images, themes, or other binary assets. Without a convention for where these assets live, users face a dilemma: place everything in the notes root (cluttering the directory) or create a group directory (which appears in the UI sidebar even when the user only wants a place to store images).

Additionally, the `examples/` directory — shipped as a source tree and installed to `/usr/share/harbour-notesplusplus/examples/` — must follow the same structure as a user's notes directory so that it can be copied recursively with a single `cp -r` and handled by the same runtime logic.

#### Decision

**Groups vs. Asset Directories:**

1. **Regular groups** are subdirectories that do NOT have a `.adoc` file with the same name in their parent. They appear in the UI group list and can contain notes and nested subgroups. Created explicitly by the user or auto-discovered during directory sync.

2. **Asset directories** are subdirectories where a `.adoc` file with the same name exists in the parent directory. Convention: `my-note.adoc` + `my-note/` = an asset directory for that note. Asset directories are hidden from the UI group list.

**Detection rule** (`is_asset_dir`):
```
A directory `foo/` is an asset directory if `foo.adoc` exists alongside it in the same parent directory.
```

**Behavioral differences:**

| Behavior | Regular group | Asset directory |
|---|---|---|
| Registered in `groups` table | Yes | No |
| Appears in UI sidebar | Yes | No (hidden) |
| `.adoc` files inside | Registered with this group's `group_path` | Skipped (duplicates of parent) |
| Non-`.adoc` assets inside | Available to notes in this group | Copied to notes root for image resolution |
| Treated as a group for subdirectories | Yes | Children inherit the parent's group |

**Examples directory convention:**
The `examples/` source tree follows the exact same structure. Documents without assets (e.g., `invoice.adoc`) sit flat at the top level. Documents with assets (e.g., `chronicles.adoc`) have a same-named subdirectory containing only the assets. The spec file copies the tree recursively; the runtime `copy_dir_recursive` and `sync_dir_recursive` handle asset directories transparently.

#### Consequences
##### Positive / Utility Delivered
- **Flat default**: Users who only write text notes see no directories unless they create them. No forced hierarchy.
- **Progressive complexity**: Adding images to a note requires only creating a same-named subdirectory and placing files in it. No configuration or special syntax needed.
- **Standard convention**: Matches the pattern used by Logseq, Obsidian, and other note-taking tools where a same-name directory holds page assets.
- **Single source tree**: The `examples/` directory and the user's notes directory use identical structure, eliminating special-case packaging logic.
- **No DB schema changes**: Asset directories are distinguished at the filesystem level by a simple naming convention, not by database flags or metadata.

##### Trade-offs / Mitigations
- A user who creates a directory with the same name as an existing `.adoc` file (e.g., `Work/Work.adoc`) will have it treated as an asset directory, hiding it from the group list. This is intentional and consistent with the convention. If the `.adoc` file is later deleted, the directory becomes a regular group on the next sync.
- Asset files are copied to the notes root for image resolution, which uses additional disk space for duplicates. This is acceptable given the typical size of note assets (images, themes).
