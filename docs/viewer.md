# WordCraft Viewer

> **Branch:** `office-viewer` · **Last updated:** 2026-11-03

On this branch the `wordcraft` desktop app and the `wordcraft-web` browser app are **WordCraft Viewer**: it opens documents, shows
them read-only, prints them and exports or converts them to the formats WordCraft writes. Editing
and the application ribbon/toolbar are turned off. Everything else (the importers, the layout
engine, the PDF/PNG renderers and the exporters) is the same code the editor uses, so documents
look and convert exactly as they do in WordCraft.

## Build and run

Needs Rust 1.95 or newer (`rustup`), and on Linux the usual eframe/winit system libraries
(X11 or Wayland, xkbcommon, fontconfig).

```sh
cargo run --release -p wordcraft -- report.docx       # open a document in the viewer
cargo run --release -p wordcraft -- --sample          # the built-in sample document
cargo run --release -p wordcraft                      # empty window with an Open… button
cargo build --release -p wordcraft                    # target/release/wordcraft
```

Headless conversion needs no window: `wordcraft-cli convert IN OUT`
(`cargo run --release -p wordcraft-cli -- convert report.docx report.pdf`).

## What it does

| | |
|---|---|
| **Opens** | `.docx`, `.docm`, `.dotx`, `.dotm`, Word 97–2003 `.doc`/`.dot`, `.odt`, `.rtf`, `.html`/`.htm`/`.mht`, `.md`, `.tex`, `.txt`, `.xml` (Word XML), WordCraft `.json` — from the command line, File › Open, the toolbar's **Open**, drag and drop or the recent-files list |
| **Exports** | toolbar **PDF** (Save a copy as PDF) and **Export** / File › Export: PDF, DOCX, Word XML, Word 2003 XML, ODT, RTF, HTML, Markdown, LaTeX, plain text, PNG (one image per page). Exporting writes a new file and never changes the open document |
| **Prints** | toolbar **Print** / File › Print (print preview, page ranges, Save as PDF) |
| **Navigates** | scrolling, Page Up/Down, previous/next page buttons, Go To (Ctrl+G or the status bar's page number), the Navigation pane (headings, pages, search results), Find (Ctrl+F), zoom (toolbar, status bar slider, Ctrl+wheel, Page Width, One Page), Read Mode, Focus and Full Screen views, several documents in windows |
| **Selects and copies** | mouse and keyboard selection, Ctrl+A, Ctrl+C / context-menu Copy (text, HTML and RTF on the system clipboard) |

## What is removed or disabled

- **The ribbon, title bar and Quick Access Toolbar** are replaced by one slim viewer toolbar
  (`crates/ui-egui/src/viewer.rs`). Keytips (Alt badges) are off.
- **Every command that edits the document is refused** by the engine (`Session::viewer`,
  `crates/engine/src/viewer.rs`): every command whose `mutates` flag is set, plus the pure
  commands that only exist to edit (undo/redo, Save, New, AutoSave, Format Painter, Find and
  Replace, header/footer editing, ink pens, styles, macros, mail merge, …; see
  `viewer::EDITING`). The gate sits in `Session::run`, so the window, the control channel and
  MCP clients all get the same answer, and `Session::why_disabled` greys the commands out with
  the reason *"WordCraft Viewer only views documents (editing is turned off)"*.
- **Typing, input methods, paste, cut, Delete/Backspace and formatting shortcuts** do nothing on
  the page; shortcuts bound to editing commands are swallowed silently.
- **The mini toolbar and the editing context menu** are gone; right-click offers Copy, Select All
  and Find.
- **File page (Backstage)**: only Open, Print, Export, Close and Options. New, Info, Save,
  Save As, Recover and the Start screen are hidden; with nothing to open the viewer starts with an
  empty window and an **Open…** button.
- Interface-level editing is refused too (`viewer::UI_EDITING`: picture change/crop, text-box
  links, mail-merge recipients, Save as Template, ribbon commands, Zotero citations).

Because nothing edits, a document never becomes dirty, there is nothing to save, and closing
never asks. Export runs File › Save As's writers on a copy: the open document's path, undo
history and dirty flag are unchanged.

## How it is built

- Engine (`wordcraft-engine`): `Session::viewer: bool` (default `false`) and the `viewer`
  module (`REASON`, `EDITING`, `refuses`). Off by default, so the CLI and every
  existing test behave as before.
- UI (`wordcraft-ui-egui`): the `viewer` module (toolbar, empty window, `UI_EDITING`,
  `BACKSTAGE_PAGES`) and small checks of `session.viewer` in the frame layout, key handling,
  context menu, mini toolbar, keytips, Backstage and `WordApp::execute`.
- App (`apps/wordcraft`): sets `session.viewer = true`, titles the window *WordCraft Viewer*
  and starts empty (`WordApp::start_empty`) when given nothing to open.
- Web app (`apps/wordcraft-web`): sets `session.viewer = true` too and starts empty
  (`?sample` opens the sample). Open (File › Open, drag and drop, `?host=parent`) reads the
  file in the browser; Export, PDF and Print download the result. Build it small with
  `trunk build --release --cargo-profile web` (`packaging/web/README.md`).
- The `editor` feature of `wordcraft-ui-egui` (on for the desktop app, off for the web app):
  without it `viewer::EDITOR` is false, `WordApp::is_viewer` is always true and `WordApp::new`
  puts the session in viewer mode, so every editor-only branch is dead and the build leaves out
  the ribbon, keytips, the mini toolbar, the editing context menus and every dialog but the
  viewer's (`Dialog::open_viewing`: Find/Go To, Zoom, Word Count, command search, About; plus
  password, unsaved changes and recovery).
- The `editor` feature of `wordcraft-engine` (on for the desktop app, the CLI and the MCP
  server; off for the web app): without it every command is still registered, but only the
  viewer's ([`viewer::VIEWER_COMMANDS`], checked against the registry by `tests::viewer`) keep
  their handlers. The others run `viewer::refused`, chosen at compile time by the `spec!` macro,
  so the editing handlers aren't linked.
- Web fonts (`wordcraft-fonts` feature `fetched-fonts`, on for the wasm build): the interface
  fonts and Source Sans 3 Regular are compiled in; the other eight bundled document faces
  (`wordcraft_fonts::FETCHED_FILES`) are fetched from `fonts/` without holding up start-up and
  join the font database in bundled order (`FontDb::add_fetched`), which relays documents out.
- egui's default faces (`wordcraft-ui-egui` feature `egui-fonts`, off for the web app): Hack,
  Ubuntu Light, egui's icons and the monochrome emoji are fetched from `egui-fonts/` after start-up
  (`EGUI_FACE_FILES`) and take egui's places in the interface fonts as they arrive
  (`theme::add_egui_default_face`). Until then, symbols and emoji only they have draw as boxes.
- Screenshots: `{"viewer": true}` on the first line of a `ui_shot` script
  (`cargo run -p wordcraft-ui-egui --example ui_shot -- script.jsonl`).

## Tests

- `crates/engine/src/tests/viewer.rs`: every refused command errors with the viewer reason, is
  greyed out and leaves the document, dirty flag and undo stack untouched; every allowed command
  leaves the document unchanged; select/copy/find work; a `.docx` opens and exports to PDF, DOCX,
  ODT, RTF, HTML, Markdown, LaTeX, text and PNG, and the exports reopen.
- `crates/ui-egui/src/viewer.rs` (`tests`): the window has the viewer toolbar and no ribbon;
  typing, paste, cut, Enter, Backspace, Delete and Ctrl+B on the page change nothing while
  Ctrl+A and copy work; interface editing is refused; the File page shows only the viewer's
  pages; the viewer starts empty, opens a `.docx` and exports it.
