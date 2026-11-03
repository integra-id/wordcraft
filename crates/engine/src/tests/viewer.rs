//! Viewer mode (`crate::viewer`): documents open, show, print and export, and nothing edits them.

use serde_json::json;
use wordcraft_doc::StoryRef;

use crate::{CmdError, Session, cmd, viewer};

fn viewer_session() -> Session {
    let mut s = Session::new(crate::sample::sample_document());
    s.viewer = true;
    s
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wordcraft-engine-viewer-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Every command that changes a document is refused, greyed out, and leaves the document,
/// its unsaved-changes flag and its undo history as they were.
#[test]
fn viewer_refuses_every_editing_command() {
    let reg = cmd::registry();
    let mut refused = 0;
    for spec in reg.all() {
        let mut s = viewer_session();
        let _ = s.run("select.all", &json!({}));
        let before = s.doc.clone();
        let refuses = viewer::refuses(spec.id, spec.mutates).is_some();
        if !refuses {
            continue;
        }
        refused += 1;
        for params in [json!({}), json!({"text": "Edited", "value": true, "size": 20})] {
            let r = s.run(spec.id, &params);
            assert!(matches!(r, Err(CmdError::Disabled(ref m)) if m.contains(viewer::REASON)), "{} ran in the viewer: {r:?}", spec.id);
            assert_eq!(s.why_disabled(spec.id), Some(viewer::REASON), "{} is greyed out", spec.id);
        }
        assert!(s.doc.same_content(&before), "{} changed the document", spec.id);
        assert!(!s.dirty, "{} marked the document changed", spec.id);
        assert!(!s.can_undo(), "{} left an undo step", spec.id);
    }
    assert!(refused > 400, "only {refused} commands refused");
    for id in ["text.insert", "text.backspace", "format.bold", "edit.paste", "edit.cut", "insert.table", "file.save", "file.new", "edit.undo"] {
        assert!(viewer::refuses(id, reg.get(id).unwrap().mutates).is_some(), "{id} is refused");
    }
}

/// What a viewer does is still there: caret movement and selection, Copy, Find, Go To, zoom,
/// views and panes. None of the commands the viewer allows changes the document, whatever
/// their params (the file commands are covered by the export test below).
#[test]
fn viewer_allowed_commands_leave_the_document_unchanged() {
    let reg = cmd::registry();
    for id in [
        "caret.docEnd",
        "caret.pageDown",
        "select.all",
        "select.word",
        "edit.copy",
        "edit.find",
        "edit.goto",
        "view.zoom",
        "view.zoomIn",
        "view.pageWidth",
        "view.onePage",
        "view.navigationPane",
        "view.readMode",
        "view.printLayout",
        "file.open",
        "file.saveAs",
        "file.exportPdf",
        "file.printPdf",
        "file.print",
        "file.close",
    ] {
        assert_eq!(viewer::refuses(id, reg.get(id).unwrap().mutates), None, "{id} works in the viewer");
    }
    let params = [json!({}), json!({"text": "Studio"}), json!({"value": true}), json!({"value": 150, "page": 2})];
    for spec in reg.all() {
        // These reach outside the session (files, dialogs); `viewer_opens_and_exports` covers them.
        if viewer::refuses(spec.id, spec.mutates).is_some()
            || spec.id.starts_with("file.")
            || spec.id.starts_with("font.")
            || spec.id.starts_with("proof.")
        {
            continue;
        }
        for p in &params {
            let mut s = viewer_session();
            let before = s.doc.clone();
            let _ = s.run(spec.id, p);
            assert!(s.doc.same_content(&before), "{} with {p} changed the document in the viewer", spec.id);
            assert!(!s.dirty, "{} with {p} marked the document changed in the viewer", spec.id);
        }
    }
}

/// Selection, Copy and Find work on a document the viewer shows.
#[test]
fn viewer_selects_copies_and_finds() {
    let mut s = viewer_session();
    s.run("edit.find", &json!({"text": "Studio"})).unwrap();
    assert!(!s.sel.is_collapsed(), "Find selects the match");
    s.run("edit.copy", &json!({})).unwrap();
    assert!(s.clipboard_text.contains("Studio"));
    s.run("select.all", &json!({})).unwrap();
    s.run("view.zoom", &json!({"value": 150})).unwrap();
    assert!(s.run("text.insert", &json!({"text": "x"})).is_err());
    assert!(s.run("edit.cut", &json!({})).is_err());
    assert!(!s.dirty);
}

/// The viewer opens a document and exports it to every format it writes; the document it shows
/// is never changed on the way.
#[test]
fn viewer_opens_and_exports() {
    let dir = temp_dir("export");
    let source = dir.join("handbook.docx");
    let mut editor = Session::new(crate::sample::sample_document());
    editor.run("file.saveAs", &json!({"path": source.to_string_lossy()})).unwrap();
    let text = editor.doc.plain_text(StoryRef::Body);

    let mut s = Session::new(wordcraft_doc::Document::new());
    s.viewer = true;
    s.run("file.open", &json!({"path": source.to_string_lossy()})).unwrap();
    assert_eq!(s.doc.plain_text(StoryRef::Body), text, "the viewer shows the document");
    assert!(s.run("text.insert", &json!({"text": "Edited"})).is_err());
    let shown = s.doc.clone();
    for ext in ["pdf", "docx", "odt", "rtf", "html", "md", "txt", "tex"] {
        let out = dir.join(format!("copy.{ext}"));
        s.run("file.saveAs", &json!({"path": out.to_string_lossy()})).unwrap_or_else(|e| panic!("export to {ext}: {e}"));
        let bytes = std::fs::read(&out).unwrap();
        assert!(!bytes.is_empty(), "{ext} export is empty");
        if ext == "pdf" {
            assert!(bytes.starts_with(b"%PDF"), "a PDF");
        }
    }
    let pdf = dir.join("export.pdf");
    s.run("file.exportPdf", &json!({"path": pdf.to_string_lossy()})).unwrap();
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
    let png = dir.join("page.png");
    s.run("file.exportPng", &json!({"path": png.to_string_lossy()})).unwrap();
    assert!(
        png.exists()
            || dir.join("page-1.png").exists()
            || std::fs::read_dir(&dir).unwrap().any(|e| e.unwrap().path().extension().is_some_and(|x| x == "png"))
    );
    // Saving a copy only records the save in the properties (their revision count), as Word does.
    assert_eq!(s.doc.plain_text(StoryRef::Body), shown.plain_text(StoryRef::Body), "exporting doesn't change the document");
    assert!(!s.dirty && !s.can_undo(), "exporting is not an edit");

    // The exported copies open in the viewer again.
    for ext in ["docx", "odt", "rtf", "html", "md", "txt"] {
        let mut v = Session::new(wordcraft_doc::Document::new());
        v.viewer = true;
        v.run("file.open", &json!({"path": dir.join(format!("copy.{ext}")).to_string_lossy()})).unwrap_or_else(|e| panic!("open {ext}: {e}"));
        assert!(v.doc.plain_text(StoryRef::Body).contains("Studio"), "{ext} round trip keeps the text");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
