# ADR-009: Dynamic Slide Deck Partitioning via Headings and Page Breaks

#### Status
Accepted

#### Context
Users frequently use notes for presentations, meeting agendas, and technical briefs. Embedding dedicated third-party presentation frameworks (such as Reveal.js, Marp, or impress.js) would add significant bundle weight, external dependencies, and disparate styling rules.

#### Decision
Implement a native **Presentation Mode** within the web companion UI that dynamically partitions standard AsciiDoc documents into presentation slide decks:
1. **Dynamic Slide Segmentation**: Partition parsed AST blocks into slides based on standard AsciiDoc structural conventions:
   - Top-level headings (`= Document Title`, `== Section Heading`) create new slide boundaries.
   - Explicit page breaks (`<<<`) trigger immediate slide splits without requiring a new heading.
2. **Reusable HTML Rendering**: Render slide content through the existing document HTML rendering pipeline, ensuring identical typography, code syntax highlighting, callouts, and checklists across document and slide views.
3. **Presentation Controls**: Provide comprehensive presentation capabilities:
   - Keyboard navigation (`Arrow keys`, `Space`, `PageUp/Down`, `Home/End`, `Esc`, `F` for fullscreen, `O` for slide overview)
   - Touch swipe gestures for mobile and tablet browsers
   - Fullscreen viewport synchronization (`requestFullscreen`)
   - Interactive slide progress bar and visual thumbnail overview grid

#### Consequences
##### Positive / Utility Delivered
- **Zero Syntax Overhead**: Any standard AsciiDoc note can be presented immediately without special slide macros or proprietary frontmatter.
- **Zero External Runtime Dependencies**: Built entirely with vendored Vue 3 and CSS, preserving the offline architecture ([ADR-007](007-offline-vendored-web-assets.md)).
- **Seamless Document to Presentation Workflow**: Edits made in the note editor update the live presentation deck in real time.

##### Trade-offs / Mitigations
- Long sections without subheadings or `<<<` page breaks may overflow slide boundaries on smaller displays; this is mitigated by responsive viewport scaling and the overview drawer.
