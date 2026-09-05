/// CSS theme for PDF rendering, modeled after asciidoctor's default stylesheet.
/// Optimized for printpdf's azul-layout CSS subset.
pub fn default_css() -> &'static str {
    r#"
@page {
    size: A4;
    margin: 20mm 25mm;
}

body {
    font-family: Lato, "Noto Serif", serif;
    font-size: 10.5pt;
    line-height: 1.6;
    color: rgba(0,0,0,0.85);
    margin: 0;
    padding: 0;
}

/* === Headings (matches asciidoctor defaults) === */
h1, h2, h3, h4, h5, h6 {
    font-family: Lato, "Helvetica Neue", Helvetica, sans-serif;
    color: rgba(0,0,0,0.85);
    line-height: 1.2;
    margin-top: 1.2em;
    margin-bottom: 0.5em;
    page-break-after: avoid;
}

h1 {
    font-size: 2em;
    font-weight: 700;
    border-bottom: 2px solid #ddd;
    padding-bottom: 0.15em;
    margin-top: 0;
}

h2 {
    font-size: 1.5em;
    font-weight: 700;
    border-bottom: 1px solid #eee;
    padding-bottom: 0.1em;
}

h3 { font-size: 1.25em; font-weight: 600; }
h4 { font-size: 1.1em; font-weight: 600; }
h5 { font-size: 1em; font-weight: 600; font-style: italic; }
h6 { font-size: 0.9em; font-weight: 600; color: #6c757d; }

/* === Document header === */
#header {
    margin-bottom: 2em;
    padding-bottom: 1em;
    border-bottom: 3px solid #4682b4;
}

#header > h1 {
    font-size: 2.4em;
    border-bottom: none;
    margin-bottom: 0.15em;
    color: #2c3e50;
}

#header .details {
    font-size: 1em;
    color: #6c757d;
}

#header .details .author {
    font-weight: bold;
    color: #2c3e50;
}

#header .details .revdate {
    margin-left: 0.5em;
}

/* === Preamble / abstract === */
#preamble {
    margin-bottom: 1.5em;
}

.abstract {
    font-style: italic;
    border-left: 3px solid #4682b4;
    padding: 0.5em 1em;
    margin: 1em 0;
    background: #f8f9fa;
    color: #333;
}

/* === Paragraphs === */
.paragraph {
    margin-bottom: 0.8em;
}

.paragraph p {
    margin: 0;
    line-height: 1.6;
}

/* === Links === */
a {
    color: #2156a5;
    text-decoration: none;
}

/* === Inline formatting === */
strong, b { font-weight: bold; }
em, i { font-style: italic; }

code, kbd, pre {
    font-family: UbuntuMono, "Noto Sans Mono", "Droid Sans Mono", monospace;
    font-size: 0.85em;
}

code {
    background-color: #f5f5f5;
    padding: 1px 4px;
    border-radius: 3px;
    color: #c7254e;
}

kbd {
    background-color: #f7f7f7;
    border: 1px solid #ccc;
    border-radius: 3px;
    padding: 1px 4px;
}

/* === Sections === */
.sect1 {
    margin-top: 1.5em;
}

.sect2, .sect3, .sect4 {
    margin-top: 1em;
}

.sectionbody {
    margin-top: 0.3em;
}

/* === Lists === */
ul, ol {
    margin: 0 0 0.8em 0;
    padding-left: 2.5em;
}

li {
    margin-bottom: 0.15em;
    line-height: 1.5;
}

li > p {
    margin: 0;
}

dl {
    margin: 0 0 0.8em 0;
}

dt {
    font-weight: bold;
    margin-top: 0.6em;
}

dd {
    margin-left: 2em;
    margin-bottom: 0.2em;
}

/* === Code blocks === */
.listingblock, .literalblock {
    margin: 0.8em 0;
}

.listingblock pre, .literalblock pre, pre {
    background-color: #f6f8fa;
    border: 1px solid #e1e4e8;
    border-left: 4px solid #6c757d;
    padding: 0.6em 0.8em;
    font-size: 0.8em;
    line-height: 1.45;
    overflow-wrap: break-word;
    white-space: pre-wrap;
    margin: 0;
}

.listingblock pre code, .literalblock pre code, pre code {
    background: none;
    padding: 0;
    color: inherit;
    font-size: inherit;
    border-radius: 0;
}

/* Title on code blocks */
.listingblock .title, .literalblock .title {
    font-size: 0.85em;
    font-weight: bold;
    color: #555;
    margin-bottom: 0.2em;
}

/* Callout list */
.colist {
    margin-top: -0.3em;
    margin-bottom: 0.8em;
    padding-left: 1em;
    font-size: 0.85em;
    color: #555;
}

/* === Tables === */
table {
    border-collapse: collapse;
    width: 100%;
    margin: 0.8em 0;
    font-size: 0.95em;
}

th, td {
    border: 1px solid #b0b0b0;
    padding: 5px 8px;
    text-align: left;
    vertical-align: top;
}

th {
    font-weight: bold;
    background-color: #e9ecef;
    border-bottom: 2px solid #6c757d;
}

/* Table title */
.title {
    font-weight: bold;
    font-size: 0.9em;
    color: #555;
    margin-bottom: 0.3em;
}

/* Frame variants */
.frame-all th, .frame-all td { border: 1px solid #b0b0b0; }
.frame-ends th, .frame-ends td { border-left: none; border-right: none; }
.frame-ends thead th { border-bottom: 2px solid #6c757d; }
.frame-ends tfoot td { border-top: 2px solid #6c757d; }
.frame-none th, .frame-none td { border: none; }
.grid-all th, .grid-all td { border: 1px solid #b0b0b0; }
.grid-rows th, .grid-rows td { border-left: none; border-right: none; }
.grid-cols th, .grid-cols td { border-top: none; border-bottom: none; }
.grid-none th, .grid-none td { border: none; }

/* Cell alignment */
.halign-left { text-align: left; }
.halign-center { text-align: center; }
.halign-right { text-align: right; }
.valign-top { vertical-align: top; }
.valign-middle { vertical-align: middle; }
.valign-bottom { vertical-align: bottom; }

/* Table footer */
tfoot td {
    font-weight: bold;
    border-top: 2px solid #6c757d;
    background-color: #f8f9fa;
}

/* === Blockquotes === */
blockquote {
    border-left: 4px solid #6c757d;
    margin: 0.8em 0;
    padding: 0.3em 1em;
    background-color: #f8f9fa;
    color: #333;
}

blockquote p {
    margin: 0;
}

blockquote .attribution {
    font-size: 0.85em;
    color: #6c757d;
    margin-top: 0.3em;
}

/* Verse block */
.verseblock {
    border-left: 4px solid #6c757d;
    margin: 0.8em 0;
    padding: 0.4em 1em;
    font-style: italic;
    white-space: pre-line;
    color: #495057;
}

.verseblock .attribution {
    font-style: normal;
    font-size: 0.85em;
    margin-top: 0.4em;
}

/* Quote block */
.quoteblock {
    border-left: 4px solid #6c757d;
    margin: 0.8em 0;
    padding: 0.3em 1em;
    background-color: #f8f9fa;
}

/* === Example block === */
.openblock, .exampleblock {
    border: 1px solid #dee2e6;
    border-left: 4px solid #6c757d;
    padding: 0.6em 1em;
    margin: 0.8em 0;
    background-color: #fafbfc;
}

/* === Admonition blocks === */
.admonitionblock {
    margin: 1em 0;
    border: 1px solid;
    border-left-width: 6px;
    padding: 0.5em 0.8em;
    page-break-inside: avoid;
}

.admonitionblock td.icon {
    font-weight: bold;
    text-transform: uppercase;
    font-size: 0.8em;
    padding-right: 0.6em;
    vertical-align: top;
    white-space: nowrap;
}

.admonitionblock.note {
    border-color: #428bca;
    background-color: #eef4ff;
}
.admonitionblock.note td.icon { color: #428bca; }

.admonitionblock.tip {
    border-color: #f0ad4e;
    background-color: #fff5eb;
}
.admonitionblock.tip td.icon { color: #f0ad4e; }

.admonitionblock.warning {
    border-color: #d9534f;
    background-color: #fdeef0;
}
.admonitionblock.warning td.icon { color: #d9534f; }

.admonitionblock td.content {
    padding: 0.3em 0;
}

.admonitionblock td.content .title {
    font-weight: bold;
    margin-bottom: 0.2em;
}

/* === Sidebar === */
.sidebarblock {
    border: 1px solid #dee2e6;
    border-left: 4px solid #6c757d;
    background-color: #f8f9fa;
    padding: 0.6em 1em;
    margin: 1em 0;
}

.sidebarblock .title {
    font-weight: bold;
    margin-bottom: 0.4em;
    color: #2c3e50;
}

/* === Horizontal rule === */
hr {
    border: none;
    border-top: 1px solid #dee2e6;
    margin: 1.5em 0;
}

/* === Images === */
.imageblock {
    margin: 1em 0;
    text-align: center;
}

.imageblock img {
    max-width: 100%;
}

.imageblock .title {
    font-size: 0.85em;
    font-style: italic;
    color: #6c757d;
    margin-top: 0.2em;
}

.thumb {
    float: left;
    margin: 0 1em 0.5em 0;
    max-width: 50%;
}

/* === Footnotes === */
#footnotes {
    margin-top: 2em;
    padding-top: 0.5em;
    border-top: 1px solid #dee2e6;
    font-size: 0.85em;
    color: #555;
}

#footnotes .footnote {
    margin-bottom: 0.2em;
}

sup.footnote, sup.footnoteref {
    font-size: 0.75em;
    vertical-align: super;
}

/* === TOC === */
#toc {
    margin-bottom: 1.5em;
    padding: 0.8em 1em;
    background-color: #f8f9fa;
    border: 1px solid #dee2e6;
}

#toc ul {
    list-style: none;
    padding-left: 1em;
    margin: 0;
}

#toc > ul {
    padding-left: 0;
}

#toc li {
    margin-bottom: 0.15em;
}

/* === Page break === */
.page-break {
    page-break-before: always;
}
"#
}
