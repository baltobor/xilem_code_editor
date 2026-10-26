# xilem_code_editor
A reusable code editor crate.

From a plain Markdown editor up to a full IDE editor (LSP, debugger gutter,
VCS changes, popups) for [Xilem](https://github.com/linebender/xilem).
The workspace is layered the way Xilem is:

| Crate | Role | Depends on |
|---|---|---|
| `code_editor_core` | Model: document, selection, cursor, edits, snippets, popup timing, provider traits | — |
| `code_editor_tree_sitter` | Syntax highlighting (`lang-*` features) | core |
| `masonry_code_editor` | Widget: layout, painting, gutter, popups | core, masonry |
| `xilem_code_editor` | View and builder for Xilem apps | masonry_code_editor, xilem |
| `xilem_rich_text` | Wrapping paragraphs with inline styles and links (Markdown rendering) | xilem |
| `xilem_markdown_editing` | Markdown formatting bar and Edit/Preview/Split toggles | xilem_code_editor |

Data sources (language servers, git, debuggers, word lists) attach from outside through the traits in `code_editor_core::provider`.

## License

Apache-2.0, see [LICENSE-APACHE](LICENSE-APACHE). Same as xilem.


### Credits

We stand on the shoulders of giants. And not everything here was invented from scratch. This editor is based on the ideas of the following open-source projects, parts of which have been ported to Xilem.

- **[Lapce](https://github.com/lapce/lapce)** (Apache-2.0) and
  **[Floem](https://github.com/lapce/floem)** (MIT, © 2023 Floem):
  the document trait, the selection model, phantom (inlay) text,
  the line layout manager and the visual-line coordinates (`VLine`/`RVLine`)
  are adapted from their design and code. Both licences permit use,
  modification and redistribution as a derived work, provided the licence
  texts and notices are kept: see [LICENSE-APACHE](LICENSE-APACHE) and
  [LICENSE-MIT-FLOEM](LICENSE-MIT-FLOEM). Adapted files say so in their header.
- **[Zed](https://github.com/zed-industries/zed)** (editor: GPL-3.0-or-later):
  only *concepts* are borrowed — popup timing values and behavior (hover
  delay and grace period, completion re-filtering, debounces) and the
  idea of attachable providers. **No Zed code is copied**, so this crate is
  not a derived work of Zed and its GPL does not apply; copying Zed code
  would require this crate to become GPL.
