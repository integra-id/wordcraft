//! Viewer mode: a session that shows documents and never edits them.
//!
//! The WordCraft Viewer (`apps/wordcraft` on the `office-viewer` branch, see
//! `docs/viewer.md`) sets [`crate::Session::viewer`]. In viewer mode [`crate::Session::run`]
//! refuses every command that changes a document ([`crate::CommandSpec::mutates`]) and the
//! pure commands that start editing, write the open file in place, or make a new document to
//! edit ([`EDITING`]). What is left is what a viewer needs: opening and closing documents,
//! exporting a copy in another format (Save As, Export PDF/PNG, Print to PDF), printing,
//! caret movement, selection, Copy, Find and Go To, and every view command (zoom, views, panes,
//! Read Mode). [`refuses`] also greys the commands out ([`crate::Session::why_disabled`]).
//!
//! `tests::viewer` runs every allowed command on a sample document and checks the document is
//! unchanged, so a pure command that edits after all can't slip through the list.

/// Why a refused command is unavailable in viewer mode.
pub const REASON: &str = "WordCraft Viewer only views documents (editing is turned off)";

/// Pure commands (`mutates == false`) that the viewer refuses all the same: they edit the
/// document on a later step (Undo, Repeat, Format Painter, the ink pens, Design Mode, header and
/// equation editing), write the open file in place (Save, AutoSave, Rename, Encrypt, a version
/// restore), make a new document to edit (New, templates), or customise editing.
pub const EDITING: &[&str] = &[
    // Undo history and Repeat: nothing to undo in a viewer.
    "edit.undo",
    "edit.undoTo",
    "edit.redo",
    "edit.repeat",
    // Formatting that applies on the next click.
    "edit.formatPainter",
    "edit.copyFormat",
    // Find and Replace opens on its Replace tab; Find (Mod+F) and Go To stay.
    "edit.findReplace",
    // Clipboard pane items are for pasting.
    "edit.clipboardPane",
    // Writing the open file, or a new one to edit.
    "file.new",
    "file.newFromTemplate",
    "file.save",
    "file.autosave",
    "file.rename",
    "file.encrypt",
    "file.versions",
    "file.recover",
    "file.saveTemplate",
    "file.setAuthor",
    "file.privacy",
    // Entering an editing mode.
    "insert.editHeader",
    "insert.editFooter",
    "equation.edit",
    "equation.inputFormat",
    "equation.normalText",
    "equation.structureActions",
    "developer.designMode",
    "developer.control",
    "text.overtype",
    "layout.manualHyphenation",
    "styles.apply",
    "styles.manage",
    "styles.organizer",
    "design.setDefault",
    // Ink.
    "draw.pen",
    "draw.pencil",
    "draw.highlighter",
    "draw.eraser",
    "draw.lasso",
    "draw.inkToShape",
    "draw.addPen",
    "draw.removePen",
    // Proofing that changes the document or the dictionary.
    "review.addToDictionary",
    "review.ignoreAll",
    "review.editor",
    // The default language of new blank documents (Options › Language › Set as default): the
    // viewer makes none.
    "review.language.preferred",
    // Installing and removing language packs (Options › Language): the viewer changes no
    // proofing setup.
    "proof.installPack",
    "proof.removePack",
    // Macros, AutoCorrect and customising the editing UI.
    "tools.recordMacro",
    "tools.macros",
    "tools.autocorrect",
    "tools.customizeKeyboard",
    "tools.customizeRibbon",
    "tools.templates",
    "smartArt.textPane",
    "arrange.selectionPane",
];

/// File › Options settings the viewer refuses (`file.options` itself stays, for the panes and
/// the view settings): the dictionaries folder and the custom dictionary's words.
pub const EDITING_OPTIONS: &[&str] = &["dictionariesFolder", "addWords", "removeWords"];

/// Command namespaces the viewer refuses whole: mail merge edits the document or makes new ones.
pub const EDITING_NAMESPACES: &[&str] = &["mailings."];

/// Why viewer mode refuses command `id` (`mutates` is its [`crate::CommandSpec::mutates`]), or
/// `None` when the viewer runs it.
pub fn refuses(id: &str, mutates: bool) -> Option<&'static str> {
    (mutates || EDITING.contains(&id) || EDITING_NAMESPACES.iter().any(|ns| id.starts_with(ns))).then_some(REASON)
}

/// Whether this build registers the editing commands' handlers (the `editor` feature, on by
/// default). Without it every command outside [`VIEWER_COMMANDS`] is still registered, so menus,
/// shortcuts and [`refuses`] see it, but runs [`refused`]: the linker then leaves the editing
/// handlers out (the web viewer's download).
pub const EDITOR: bool = cfg!(feature = "editor");

/// The commands viewer mode runs: every pure command not in [`EDITING`] or
/// [`EDITING_NAMESPACES`] (`tests::viewer` checks the list against the registry). A build
/// without the `editor` feature keeps only these commands' handlers.
pub const VIEWER_COMMANDS: &[&str] = &[
    // cmd::caret
    "caret.left",
    "caret.right",
    "caret.up",
    "caret.down",
    "caret.wordLeft",
    "caret.wordRight",
    "caret.home",
    "caret.end",
    "caret.paraUp",
    "caret.paraDown",
    "caret.docStart",
    "caret.docEnd",
    "caret.pageUp",
    "caret.pageDown",
    "caret.set",
    "select.all",
    "select.range",
    "select.word",
    "select.sentence",
    "select.paragraph",
    "select.dragUnit",
    "select.line",
    "select.text",
    "select.collapse",
    // cmd::column
    "select.column",
    // cmd::edit
    "edit.undoHistory",
    "edit.copy",
    "edit.clipboardItems",
    "edit.deleteClipboardItem",
    "edit.clearClipboard",
    "edit.find",
    "edit.advancedFind",
    "edit.findNext",
    "edit.findPrevious",
    "edit.goto",
    "edit.goBack",
    // cmd::format
    "format.fontDialog",
    "format.state",
    // cmd::font
    "font.add",
    "font.list",
    "font.missing",
    "font.remove",
    "font.loadSystem",
    // cmd::para
    "para.dialog",
    "styles.list",
    "styles.pane",
    // cmd::lists
    "list.get",
    // cmd::inspector
    "styles.inspector",
    "styles.inspect",
    // cmd::view
    "view.marks",
    "view.printLayout",
    "view.webLayout",
    "view.draft",
    "view.outline",
    "view.readMode",
    "view.readColumnWidth",
    "view.readTextSize",
    "view.readLineSpacing",
    "view.readPageColor",
    "view.hideWhiteSpace",
    "view.focus",
    "view.fullScreenView",
    "view.ruler",
    "view.gridlines",
    "table.viewGridlines",
    "view.navigationPane",
    "view.zoom",
    "view.zoom100",
    "view.zoomIn",
    "view.zoomOut",
    "view.onePage",
    "view.multiplePages",
    "view.pageWidth",
    "view.darkMode",
    "view.interfaceTheme",
    "view.stylesPane",
    "view.stylesPaneDock",
    "view.commentsPane",
    "view.accessibilityPane",
    "view.altTextPane",
    "view.formatShapePane",
    "view.newWindow",
    "view.switchWindows",
    "view.split",
    "view.fullScreen",
    "view.state",
    "view.numeral",
    // cmd::outline
    "outline.expand",
    "outline.collapse",
    "outline.toggle",
    "outline.selectSubtree",
    "outline.showLevel",
    "outline.showLevel1",
    "outline.showLevel2",
    "outline.showLevel3",
    "outline.showLevel4",
    "outline.showLevel5",
    "outline.showLevel6",
    "outline.showLevel7",
    "outline.showLevel8",
    "outline.showLevel9",
    "outline.showAll",
    "outline.firstLineOnly",
    "outline.showFormatting",
    "outline.close",
    "outline.state",
    // cmd::insert
    "link.targets",
    "insert.closeHeader",
    // cmd::equation
    "equation.exit",
    "equation.move",
    "equation.get",
    "equation.gallery",
    // cmd::page
    "layout.section",
    // cmd::table
    "table.selectTable",
    "table.selectRow",
    "table.selectColumn",
    "table.selectCell",
    // cmd::table_mouse
    "table.selectAt",
    // cmd::review
    "review.nextComment",
    "review.previousComment",
    "review.comments",
    "review.nextChange",
    "review.previousChange",
    "review.markup",
    "review.trackingOptions",
    "review.wordCount",
    "review.changes",
    "review.spelling",
    "review.issues",
    "review.suggestions",
    "review.proofing",
    "review.thesaurus",
    // cmd::file
    "file.open",
    "file.saveAs",
    "file.exportPdf",
    "file.exportPng",
    "file.printMarkup",
    "file.printPdf",
    "file.print",
    "file.close",
    "file.closeDocument",
    "file.info",
    "document.inspect",
    "document.text",
    "document.paragraph",
    "document.selection",
    "document.layout",
    // cmd::options
    "file.options",
    // cmd::packs
    "proof.packs",
    // cmd::design
    "design.themes",
    // cmd::references
    "references.nextFootnote",
    "references.notes",
    // cmd::fields
    "view.fieldCodes",
    "fields.next",
    "fields.previous",
    "fields.selected",
    // cmd::citations
    "references.researcher",
    // cmd::objects
    "select.addObject",
    "select.member",
    "select.objects",
    // cmd::draw
    "draw.select",
    "draw.replay",
    "review.hideInk",
    // cmd::tools
    "tools.search",
    "file.accessibility",
    "file.compatibility",
    "file.share",
    "hf.next",
    "hf.previous",
    "review.showMarkup",
    "select.similar",
    "select.extend",
    "view.immersive",
    "view.vertical",
    "view.sideToSide",
    // cmd::user_templates
    "file.listTemplates",
    // cmd::speech
    "review.readAloud",
    "readAloud.playPause",
    "readAloud.next",
    "readAloud.previous",
    "readAloud.stop",
    "readAloud.speed",
    "readAloud.skipCitations",
    "readAloud.status",
    "caret.pastCitation",
];

/// Whether this build keeps command `id`'s handler: every command with the `editor` feature,
/// only [`VIEWER_COMMANDS`] without it. `const` so that [`spec!`](crate::spec) decides at compile
/// time and an editing handler is never referenced in a viewer build.
pub const fn builds(id: &str) -> bool {
    if EDITOR {
        return true;
    }
    let mut i = 0;
    while i < VIEWER_COMMANDS.len() {
        if str_eq(VIEWER_COMMANDS[i], id) {
            return true;
        }
        i += 1;
    }
    false
}

const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// The handler [`spec!`](crate::spec) registers: `run` when this build keeps it ([`builds`]),
/// [`refused`] otherwise. Always inlined, so a handler that isn't kept is never referenced.
#[inline(always)]
pub const fn handler<const KEEP: bool>(
    run: fn(&mut crate::Session, &serde_json::Value) -> crate::CmdResult,
) -> fn(&mut crate::Session, &serde_json::Value) -> crate::CmdResult {
    if KEEP { run } else { refused }
}

/// The handler of an editing command in a build without the `editor` feature ([`builds`]).
pub fn refused(_: &mut crate::Session, _: &serde_json::Value) -> crate::CmdResult {
    Err(crate::CmdError::Disabled(REASON.into()))
}

/// Why viewer mode refuses running `id` with `params` though it runs `id` itself: a
/// `file.options` that changes the dictionaries ([`EDITING_OPTIONS`]).
pub fn refuses_params(id: &str, params: &serde_json::Value) -> Option<&'static str> {
    (id == "file.options" && EDITING_OPTIONS.iter().any(|k| params.get(k).is_some())).then_some(REASON)
}
