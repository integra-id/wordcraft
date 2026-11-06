//! Host modes: a page that runs WordCraft (as an `<iframe>`, or in its own document) opens
//! documents in it and stores what it saves instead of the browser downloading. The messages and
//! the two transports are `craft-web-host`'s (storytold/craft-libs), the same contract as the
//! other Craft apps' bridges, with the `wordcraft:` prefix. This module is WordCraft's part of it.
//!
//! - `?host=parent` (an `<iframe>`, such as a Nextcloud app) talks to the same-origin parent over
//!   `postMessage`; `data-host="page"` on the canvas (the page's own document, no iframe) uses
//!   DOM `CustomEvent`s on the canvas instead. `?host=parent` wins when both are present. With no
//!   parent, or a parent on another origin, there is no bridge: the normal app with downloads.
//! - `wordcraft:ready` once the app is running and listening; send a file now. WordCraft reads
//!   bytes, so it says `file: false` and a host sends `bytes`.
//! - `wordcraft:open` (from the host): `{ name, bytes }`. It opens like a document picked with
//!   File › Open or dropped on the window; a file over [`MAX_OPEN_BYTES`] is refused before it is
//!   copied.
//! - `wordcraft:save` (to the host): `{ name, saveAs, bytes }` for every file WordCraft would
//!   otherwise download. The host stores it and reports failures itself. `saveAs` says whether
//!   the file becomes the document's file:
//!   - Save: `false`. It writes the opened file under its own name and format, as the desktop
//!     app does. A name the parent never sent (an untitled document, an imported .doc saved as
//!     .docx) is a new file: `true`, as for Save As.
//!   - Save As, and File › Export to another document format (which runs Save As): `true` for a
//!     format that keeps everything (.docx and the other Word packages, .odt, .rtf). The document
//!     is now that file; the next Save sends its name with `false`.
//!   - Save As or Export to a format that doesn't (PDF, Markdown, plain text), Export as PDF and
//!     Export Page as PNG: `false`. They are copies; the document's file stays the same.
//! - `wordcraft:config` (from the host; not part of craft-web-host): `{ hide, disable }`, each a
//!   comma-separated string or an array of feature names, as `?hide=` and `?disable=` take them
//!   (`viewer_features`). It replaces the lists the URL gave; send it any time, e.g. after
//!   `wordcraft:ready`. On `?host=parent` it arrives as a `postMessage`; on `data-host="page"` as
//!   a `wordcraft:config` CustomEvent on the canvas (`detail` holds the fields). URL `?hide=` /
//!   `?disable=` apply first; a later `wordcraft:config` replaces them.
//! - `wordcraft:dirty` whenever unsaved changes appear or go away, `wordcraft:failed` when the
//!   app couldn't start.
//!
//! In both host modes the host sends the document to edit, so no sample opens, saves are handed
//! to the host rather than downloaded, the host guards leaving the page, and `?author=` names the
//! user for comments and tracked changes.

use std::cell::RefCell;
use std::rc::Rc;

use craft_web_host::web::{FileData, Host, Message};
use wasm_bindgen::JsCast as _;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use wordcraft_ui_egui::{Inbox, Services, WordApp};

/// The app id in the host messages (`wordcraft:ready`).
const APP: &str = "wordcraft";

/// The name a document gets when the host sent none (the crate says `document` for a blank name).
const DEFAULT_NAME: &str = "Document.docx";

/// The largest document taken from the host page, refused before it is copied.
const MAX_OPEN_BYTES: u64 = 512 << 20;

/// The longest user name taken from `?author=`.
const MAX_AUTHOR_CHARS: usize = 200;

/// The `hide` and `disable` lists of the last `wordcraft:config`, until the app applies them.
pub type PendingConfig = Rc<RefCell<Option<(String, String)>>>;

#[derive(Clone)]
pub struct Bridge {
    host: Rc<Host>,
}

impl Bridge {
    /// The bridge for this page, or `None` when WordCraft isn't hosted: no `?host=parent` and no
    /// `data-host="page"` on the canvas, an unframed `?host=parent` page (it stays the normal app
    /// with downloads), or a parent on another origin.
    pub fn detect(canvas: &web_sys::HtmlCanvasElement) -> Option<Self> {
        Some(Self { host: Rc::new(Host::connect(APP, canvas)?) })
    }

    /// Hand saved and exported files to the host instead of downloading them, report unsaved
    /// changes to it, hear the files it sends, and hear `wordcraft:config` for feature lists.
    pub fn connect(&self, services: &mut Services, inbox: Inbox, config: PendingConfig, ctx: egui::Context) {
        self.listen(inbox, ctx.clone());
        self.listen_config(config, ctx);
        let bridge = self.clone();
        services.host_save = Some(Box::new(move |name: &str, bytes: &[u8], save_as: bool| bridge.post_save(name, bytes, save_as)));
        // Anything else that would download goes to the host too.
        let bridge = self.clone();
        services.download = Some(Box::new(move |name: &str, bytes: &[u8]| bridge.post_save(name, bytes, false)));
        // The browser's save picker is never used, so no save finishes later.
        services.async_saves = None;
        // The crate does not repeat a state it already reported.
        let host = self.host.clone();
        services.on_dirty = Some(Box::new(move |dirty: bool| host.post_dirty(dirty)));
    }

    /// `?author=`: the host's name for the user signs comments and tracked changes.
    pub fn set_author(app: &mut WordApp, author: &str) {
        let author: String = author.trim().chars().take(MAX_AUTHOR_CHARS).collect();
        if !author.is_empty() {
            app.session.author = author;
        }
    }

    pub fn post_ready(&self) {
        let _ = self.host.post_ready(env!("CARGO_PKG_VERSION"), false);
    }

    /// The app couldn't start (no WebGPU or WebGL2, for example): let the host say so.
    pub fn post_failed(&self, error: &str) {
        let _ = self.host.post_failed(error, None);
    }

    fn post_save(&self, path: &str, bytes: &[u8], save_as: bool) -> Result<(), String> {
        let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.to_string());
        self.host.post_save(&name, bytes, None, save_as)
    }

    /// Hear `wordcraft:open` and queue the file like a dropped one.
    fn listen(&self, inbox: Inbox, ctx: egui::Context) {
        self.host.listen(move |message| {
            if let Message::Open { name, data } = message {
                deliver_open(&inbox, &ctx, &name, &data);
            }
        });
    }

    /// Hear `wordcraft:config` on the same transport as open/save. craft-web-host does not carry
    /// this message type, so a second listener watches for it.
    fn listen_config(&self, config: PendingConfig, ctx: egui::Context) {
        match self.host.mode() {
            craft_web_host::Mode::Parent => {
                let Some(window) = web_sys::window() else { return };
                let Ok(Some(parent)) = window.parent() else { return };
                let Ok(origin) = window.location().origin() else { return };
                let on_message = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
                    if event.origin() != origin || !event.source().is_some_and(|s| js_sys::Object::is(&s, &parent)) {
                        return;
                    }
                    let data = event.data();
                    if string(&data, "type").as_deref() != Some("wordcraft:config") {
                        return;
                    }
                    *config.borrow_mut() = Some((list(&data, "hide"), list(&data, "disable")));
                    ctx.request_repaint();
                });
                if window.add_event_listener_with_callback("message", on_message.as_ref().unchecked_ref()).is_ok() {
                    on_message.forget();
                }
            }
            craft_web_host::Mode::Page => {
                // Same canvas id the shell mounts on (`web::CANVAS_ID`).
                let Some(target): Option<web_sys::EventTarget> = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id("wordcraft_canvas"))
                    .map(Into::into)
                else {
                    return;
                };
                let on_event = Closure::<dyn FnMut(web_sys::CustomEvent)>::new(move |event: web_sys::CustomEvent| {
                    let data = event.detail();
                    *config.borrow_mut() = Some((list(&data, "hide"), list(&data, "disable")));
                    ctx.request_repaint();
                });
                if target.add_event_listener_with_callback("wordcraft:config", on_event.as_ref().unchecked_ref()).is_ok() {
                    on_event.forget();
                }
            }
        }
    }
}

/// The file an open message carries, pushed onto the inbox for the next frame, like a dropped
/// file. A host's file is untrusted input: an oversized one is refused before it is copied into
/// the app's memory (the data's own length, not the host's claim). The crate has cleaned the name
/// (and cut it to 255 characters); a blank one comes as `document` and becomes `Document.docx`.
fn deliver_open(inbox: &Inbox, ctx: &egui::Context, name: &str, data: &FileData) {
    let name = if name == "document" { DEFAULT_NAME } else { name }.to_string();
    let FileData::Bytes(array) = data else {
        log::warn!("host file {name} came as a File; WordCraft reads bytes only; not opened");
        return;
    };
    if data.size() > MAX_OPEN_BYTES as f64 {
        log::warn!("host file is {} bytes, over the {} MiB host open limit; not opened", data.size(), MAX_OPEN_BYTES >> 20);
        return;
    }
    inbox.lock().unwrap_or_else(|e| e.into_inner()).push((name, array.to_vec()));
    ctx.request_repaint();
}

fn string(data: &JsValue, key: &str) -> Option<String> {
    js_sys::Reflect::get(data, &JsValue::from_str(key)).ok()?.as_string()
}

/// A feature list: a comma-separated string, or an array of names (joined with commas).
fn list(data: &JsValue, key: &str) -> String {
    let Ok(value) = js_sys::Reflect::get(data, &JsValue::from_str(key)) else { return String::new() };
    if let Some(s) = value.as_string() {
        return s;
    }
    if js_sys::Array::is_array(&value) {
        let names: Vec<String> = js_sys::Array::from(&value).iter().filter_map(|v| v.as_string()).collect();
        return names.join(",");
    }
    String::new()
}
