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

/// Why viewer mode refuses running `id` with `params` though it runs `id` itself: a
/// `file.options` that changes the dictionaries ([`EDITING_OPTIONS`]).
pub fn refuses_params(id: &str, params: &serde_json::Value) -> Option<&'static str> {
    (id == "file.options" && EDITING_OPTIONS.iter().any(|k| params.get(k).is_some())).then_some(REASON)
}
