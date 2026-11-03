//! The WordCraft Viewer's window (`docs/viewer.md`): a slim toolbar in place of the title bar,
//! ribbon and Quick Access Toolbar, over read-only pages.
//!
//! Viewer mode is the engine's [`wordcraft_engine::Session::viewer`]: the engine refuses every
//! command that edits ([`wordcraft_engine::viewer`]), and this module refuses the interface
//! commands that only make sense for editing ([`UI_EDITING`]). What the window offers is
//! opening, exporting (Save As in any format, PDF), printing, Find, Go To, zoom, the views and
//! the Navigation pane; the page itself still scrolls, selects and copies.

use egui::{Align2, Rect, Sense, Ui, pos2, vec2};
use serde_json::json;

use crate::theme::{Tokens, TypeRung, medium, paint_text, text_width};
use crate::{WordApp, icons};

/// The app's name in the viewer's toolbar and window title.
pub const NAME: &str = "WordCraft Viewer";

/// Height of the viewer's toolbar.
pub const BAR_HEIGHT: f32 = 40.0;

/// Interface commands (`ui.*`) the viewer refuses: the ribbon and its keytips, the Quick Access
/// Toolbar, and the editing tools that start from the interface (pictures, cropping, text box
/// links, mail merge recipients, saving a template).
pub const UI_EDITING: &[&str] = &[
    "ui.collapseRibbon",
    "ui.focusRibbon",
    "ui.quickAccessToolbar",
    "ui.keytips",
    "ui.changePicture",
    "ui.crop",
    "ui.linkTextBox",
    "ui.openRecipientList",
    "ui.saveAsTemplate",
];

/// The File page's (Backstage's) pages the viewer shows: no New, Info, Save or Recover.
pub const BACKSTAGE_PAGES: &[&str] = &["open", "print", "export", "close", "options"];

/// The File page the viewer shows in place of one it doesn't have.
pub const BACKSTAGE_START: &str = "open";

/// Why the viewer refuses command `id` (engine, interface or Zotero), or `None` when it runs it
/// or the app isn't the viewer.
pub fn refuses(app: &WordApp, id: &str) -> Option<&'static str> {
    if !app.session.viewer {
        return None;
    }
    if UI_EDITING.contains(&id) || id.starts_with("zotero.") {
        return Some(wordcraft_engine::viewer::REASON);
    }
    let spec = app.session.registry.get(id)?;
    wordcraft_engine::viewer::refuses(id, spec.mutates)
}

/// Whether the File page shows page `id` in this app.
pub fn shows_backstage_page(app: &WordApp, id: &str) -> bool {
    !app.session.viewer || BACKSTAGE_PAGES.contains(&id)
}

/// A toolbar button: an icon, with the label beside it when `label` is set. Returns whether it
/// was clicked (never while disabled).
fn button(ui: &mut Ui, icon: &str, tip: &str, label: Option<&str>, enabled: bool, checked: bool) -> bool {
    let t = Tokens::get(ui.ctx());
    let font = TypeRung::Control.regular();
    let track = TypeRung::Control.tracking();
    let label_w = label.map_or(0.0, |l| text_width(ui.painter(), l, &font, track, t.text) + 6.0);
    let (r, resp) = ui.allocate_exact_size(vec2(30.0 + label_w, 28.0), if enabled { Sense::click() } else { Sense::hover() });
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, tip));
    if checked {
        ui.painter().rect_filled(r, 4.0, t.checked);
    } else if enabled && resp.hovered() {
        ui.painter().rect_filled(r, 4.0, t.hover);
    }
    let c = if enabled { t.icon } else { t.text_disabled };
    icons::paint(
        ui.painter(),
        Rect::from_center_size(pos2(r.min.x + 15.0, r.center().y), vec2(18.0, 18.0)),
        icon,
        c,
        if enabled { t.accent } else { c },
    );
    if let Some(l) = label {
        let ink = if enabled { t.text } else { t.text_disabled };
        paint_text(ui.painter(), pos2(r.min.x + 28.0, r.center().y - TypeRung::Control.line_height() / 2.0), Align2::LEFT_TOP, l, &font, track, ink);
    }
    enabled && resp.on_hover_text(tip).clicked()
}

fn separator(ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let (r, _) = ui.allocate_exact_size(vec2(9.0, 22.0), Sense::hover());
    ui.painter().vline(r.center().x, r.y_range(), egui::Stroke::new(1.0, t.border));
}

/// The tooltip for command `id`: its name, with its shortcut when it has one.
fn tip(app: &WordApp, label: &str, id: &str) -> String {
    let label = tl!(label);
    match crate::widgets::shortcut_text(app, id) {
        k if k.is_empty() => label.to_string(),
        k => format!("{label} ({k})"),
    }
}

/// The viewer's toolbar across the top of the window: File, Open, Export, Print on the left;
/// Find, page and zoom controls and the views on the right; the document's name between.
pub fn bar(app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let mac = app.integrated_titlebar;
    let left = if mac { app.mac_titlebar.content_left as i8 } else { 8 };
    egui::Panel::top("viewer_bar")
        .exact_size(BAR_HEIGHT)
        .frame(
            egui::Frame::NONE
                .fill(t.title_bar)
                .inner_margin(egui::Margin { left, right: 10, top: 0, bottom: 0 })
                .stroke(egui::Stroke::new(1.0, t.border)),
        )
        .show(ui, |ui| {
            let full = ui.max_rect();
            // The bar drags the window, as the title bar it replaces did.
            let drag = ui.interact(full, ui.scope_id().with("viewer_drag"), Sense::click_and_drag());
            if drag.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
            }
            let has = app.has_document();
            let mut left_end = full.min.x;
            let mut right_start = full.max.x;
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                if button(ui, "file", tl!("File: open, print and export"), Some(tl!("File")), true, app.ui.backstage) {
                    let _ = app.run("ui.backstage", json!({"value": true, "page": BACKSTAGE_START}));
                }
                separator(ui);
                if button(ui, "open", &tip(app, "Open", "file.open"), Some(tl!("Open")), true, false) {
                    let _ = app.run("file.open", json!({}));
                }
                if button(ui, "export", tl!("Export a copy as PDF"), Some(tl!("PDF")), has, false) {
                    app.export_dialog("pdf");
                }
                if button(
                    ui,
                    "saveAs",
                    tl!("Export a copy in another format (DOCX, ODT, RTF, HTML, Markdown, LaTeX, text, PNG)"),
                    Some(tl!("Export")),
                    has,
                    false,
                ) {
                    let _ = app.run("ui.backstage", json!({"value": true, "page": "export"}));
                }
                if button(ui, "print", &tip(app, "Print", "file.print"), None, has, false) {
                    let _ = app.run("file.print", json!({}));
                }
                left_end = ui.min_rect().max.x;
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                    if !has {
                        return;
                    }
                    let view = &app.session.view;
                    let (read, nav) = (view.read_mode, view.nav_pane);
                    if button(ui, "readMode", tl!("Read Mode"), None, true, read) {
                        let _ = app.run(if read { "view.printLayout" } else { "view.readMode" }, json!({}));
                    }
                    separator(ui);
                    if button(ui, "onePage", tl!("One Page"), None, !read, false) {
                        let _ = app.run("view.onePage", json!({}));
                    }
                    if button(ui, "pageWidth", tl!("Page Width"), None, !read, false) {
                        let _ = app.run("view.pageWidth", json!({}));
                    }
                    if button(ui, "zoomIn", &tip(app, "Zoom In", "view.zoomIn"), None, true, false) {
                        let _ = app.run("view.zoomIn", json!({}));
                    }
                    if button(ui, "zoomOut", &tip(app, "Zoom Out", "view.zoomOut"), None, true, false) {
                        let _ = app.run("view.zoomOut", json!({}));
                    }
                    separator(ui);
                    if button(ui, "chevronRight", tl!("Next Page"), None, true, false) {
                        page_step(app, 1);
                    }
                    if button(ui, "chevronLeft", tl!("Previous Page"), None, true, false) {
                        page_step(app, -1);
                    }
                    separator(ui);
                    if button(ui, "navPane", &tip(app, "Find and Navigation pane", "edit.find"), None, true, nav) {
                        let _ = app.run("view.navigationPane", json!({}));
                    }
                    right_start = ui.min_rect().min.x;
                });
            });
            // The document's name, centred in the room between the two groups.
            let name = if app.has_document() { format!("{} — {}", app.title_stem(), tl!("Read only")) } else { NAME.to_string() };
            let font = TypeRung::Chrome.semibold();
            let track = TypeRung::Chrome.tracking();
            let room = right_start - left_end - 20.0;
            if room > 40.0 {
                let shown = crate::theme::fit_text(ui.painter(), &name, &font, track, t.text, room);
                let w = text_width(ui.painter(), &shown, &font, track, t.text);
                let cx = (left_end + right_start) / 2.0;
                paint_text(
                    ui.painter(),
                    pos2(cx - w / 2.0, full.center().y - TypeRung::Chrome.line_height() / 2.0),
                    Align2::LEFT_TOP,
                    &shown,
                    &font,
                    track,
                    t.text,
                );
            }
        });
}

/// Previous or next page: in Read Mode a screen of columns, elsewhere the caret goes to the
/// page before or after the one in view, and the view follows it.
pub fn page_step(app: &mut WordApp, by: i64) {
    if app.session.view.read_mode {
        crate::canvas::turn_read_page(app, by as f32);
        return;
    }
    let pages = app.session.layout().pages.len();
    let current = app.canvas.viewed_page.unwrap_or(app.session.page_hint);
    let target = (current as i64 + by).clamp(0, pages.saturating_sub(1) as i64);
    let _ = app.run("edit.goto", json!({"page": target + 1}));
}

/// The window with no document open: a prompt to open one.
pub fn empty(app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.canvas)).show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() / 2.0 - 60.0).max(20.0));
            ui.label(egui::RichText::new(NAME).font(medium(20.0)).color(t.text));
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(tl!("Open a document to view, print or export it: .docx, .doc, .odt, .rtf, .html, .md, .tex or .txt"))
                    .font(medium(13.0))
                    .color(t.text_dim),
            );
            ui.add_space(14.0);
            if ui.add(egui::Button::new(egui::RichText::new(tl!("Open…")).font(medium(14.0))).min_size(vec2(140.0, 32.0))).clicked() {
                let _ = app.run("file.open", json!({}));
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use egui_kittest::kittest::Queryable;
    use serde_json::json;
    use wordcraft_engine::Session;

    use crate::{Services, WordApp};

    fn viewer(text: &str) -> WordApp {
        let mut session = Session::new(wordcraft_doc::Document::from_text(text));
        session.viewer = true;
        WordApp::new(session, Services::default())
    }

    fn harness(a: WordApp) -> egui_kittest::Harness<'static, WordApp> {
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1440.0, 900.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            a,
        );
        h.run_steps(4);
        h
    }

    fn key(h: &mut egui_kittest::Harness<'static, WordApp>, key: egui::Key, modifiers: egui::Modifiers) {
        h.event(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers });
        h.event(egui::Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers });
        h.run_steps(2);
    }

    /// The viewer's window has its toolbar, not the title bar, ribbon or Quick Access Toolbar.
    #[test]
    fn the_window_has_the_viewer_bar_and_no_ribbon() {
        let h = harness(viewer("Hello world"));
        assert!(h.query_by_label("Insert").is_none(), "no ribbon tabs");
        assert!(h.query_by_label("Search commands and help").is_none(), "no title bar search");
        for label in ["Export a copy as PDF", "Read Mode", "Zoom In", "Next Page", "Page Width"] {
            assert!(h.query_all_by_label(label).next().is_some(), "the bar offers {label}");
        }
        assert_eq!(h.state().window_title(), format!("{} - {}", h.state().title_stem(), super::NAME));
    }

    /// The editor's window, for comparison: the ribbon is there and the viewer bar isn't.
    #[test]
    fn the_editor_keeps_its_ribbon() {
        let h = harness(WordApp::new(Session::new(wordcraft_doc::Document::from_text("Hello")), Services::default()));
        assert!(h.query_by_label("Insert").is_some());
        assert!(h.query_by_label("Export a copy as PDF").is_none());
    }

    /// Typing, Enter, Backspace, Delete, Ctrl+B, paste and cut on the page change nothing; the
    /// page still selects and copies.
    #[test]
    fn keys_on_the_page_never_edit() {
        let mut h = harness(viewer("Hello world"));
        h.state_mut().canvas.want_focus = true;
        h.run_steps(3);
        assert!(h.state().canvas.focused, "the page has the keyboard");
        let before = h.state().session.doc.plain_text(wordcraft_doc::StoryRef::Body);
        h.event(egui::Event::Text("typed".into()));
        h.event(egui::Event::Paste("pasted".into()));
        h.run_steps(2);
        for k in [egui::Key::Enter, egui::Key::Backspace, egui::Key::Delete, egui::Key::Tab] {
            key(&mut h, k, egui::Modifiers::NONE);
        }
        key(&mut h, egui::Key::B, egui::Modifiers::COMMAND);
        key(&mut h, egui::Key::A, egui::Modifiers::COMMAND);
        assert!(!h.state().session.sel.is_collapsed(), "Ctrl+A still selects");
        h.event(egui::Event::Cut);
        h.event(egui::Event::Copy);
        h.run_steps(2);
        key(&mut h, egui::Key::Backspace, egui::Modifiers::NONE);
        let app = h.state();
        assert_eq!(app.session.doc.plain_text(wordcraft_doc::StoryRef::Body), before);
        assert!(!app.session.dirty && !app.session.can_undo());
        assert!(!app.session.sel.is_collapsed(), "the selection survives");
    }

    /// Editing that starts in the interface (AutoSave, the ribbon, pictures, Zotero) is refused
    /// with the viewer's reason; viewing commands run.
    #[test]
    fn interface_editing_commands_are_refused() {
        let mut app = viewer("Hello");
        for id in ["file.autosave", "ui.collapseRibbon", "ui.crop", "ui.changePicture", "ui.saveAsTemplate", "text.insert", "zotero.addCitation"] {
            let e = app.execute(id, json!({"value": true, "text": "x"})).expect_err(id);
            assert!(e.contains(wordcraft_engine::viewer::REASON), "{id}: {e}");
        }
        for id in ["view.zoomIn", "view.navigationPane", "select.all", "edit.copy", "view.readMode", "view.printLayout"] {
            assert!(app.execute(id, json!({})).is_ok(), "{id} runs");
        }
        assert_eq!(app.session.doc.plain_text(wordcraft_doc::StoryRef::Body), "Hello");
        assert!(super::refuses(&viewer("x"), "ui.backstage").is_none());
        let editor = WordApp::new(Session::new(wordcraft_doc::Document::new()), Services::default());
        assert!(super::refuses(&editor, "text.insert").is_none(), "only the viewer refuses");
    }

    /// The File page offers only Open, Print, Export, Close and Options; New, Info, Save and Save
    /// As don't open.
    #[test]
    fn the_file_page_has_only_the_viewer_pages() {
        let mut app = viewer("Hello");
        app.run("ui.backstage", json!({"value": true, "page": "home"})).ok();
        let mut h = harness(app);
        assert_eq!(h.state().ui.backstage_page, super::BACKSTAGE_START, "Home gives way to Open");
        for page in ["new", "info", "save", "saveAs", "recover"] {
            crate::backstage::go_to_page(h.state_mut(), page);
            assert_eq!(h.state().ui.backstage_page, super::BACKSTAGE_START, "{page} doesn't open");
        }
        crate::backstage::go_to_page(h.state_mut(), "export");
        h.run_steps(2);
        assert_eq!(h.state().ui.backstage_page, "export");
        assert!(h.query_by_label("Info").is_none() && h.query_by_label("New").is_none());
        assert!(h.query_by_label("Open").is_some() && h.query_by_label("Options").is_some());
    }

    /// With nothing to open the viewer starts empty, asking for a document; opening one shows it
    /// read-only, and exporting it writes the copy without touching the document.
    #[test]
    fn starts_empty_then_opens_and_exports() {
        let dir = std::env::temp_dir().join(format!("wordcraft-viewer-ui-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("letter.docx");
        let mut editor = Session::new(wordcraft_doc::Document::from_text("Dear reader"));
        editor.run("file.saveAs", &json!({"path": src.to_string_lossy()})).unwrap();

        let mut app = viewer("");
        app.start_empty();
        let mut h = harness(app);
        assert!(!h.state().has_document());
        assert!(!h.state().ui.backstage, "no Start screen");
        assert!(h.query_by_label("Open…").is_some(), "the empty window asks for a document");
        assert_eq!(h.state().window_title(), super::NAME);

        h.state_mut().run("file.open", json!({"path": src.to_string_lossy()})).unwrap();
        h.run_steps(3);
        assert!(h.state().has_document());
        assert_eq!(h.state().session.doc.plain_text(wordcraft_doc::StoryRef::Body), "Dear reader");
        for ext in ["pdf", "odt", "html", "md", "txt"] {
            let out = dir.join(format!("letter.{ext}"));
            h.state_mut().run("file.saveAs", json!({"path": out.to_string_lossy()})).unwrap();
            assert!(std::fs::metadata(&out).map(|m| m.len() > 0).unwrap_or(false), "{ext} written");
        }
        let app = h.state();
        assert_eq!(app.session.doc.plain_text(wordcraft_doc::StoryRef::Body), "Dear reader");
        assert!(!app.session.dirty && !app.session.can_undo());
        std::fs::remove_dir_all(&dir).ok();
    }
}
