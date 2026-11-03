//! `?url=` / `?src=` in the browser: resolve the address against the page, fetch it and queue the
//! bytes in the inbox as a dropped file (`crate::url_open` names it and checks the scheme). While
//! it loads, and if it fails, the empty viewer says so ([`UrlOpen::show`]); failures also go to
//! the status bar and the console.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::JsCast as _;
use wordcraft_ui_egui::{Inbox, WordApp, i18n};

/// Largest document fetched from an address (as a dropped file, the bytes are held in memory).
const MAX_BYTES: f64 = (512u32 << 20) as f64;

enum Phase {
    Fetching,
    /// In the inbox; the app opens it in its next frame.
    Queued,
    /// The message shown on the empty viewer, and the error still to put on the status bar
    /// (none when the app already did, as for a file it couldn't read).
    Failed {
        message: String,
        unreported: Option<String>,
    },
    Done,
}

struct State {
    name: String,
    phase: Phase,
}

/// A document being opened from an address.
#[derive(Clone)]
pub struct UrlOpen(Rc<RefCell<State>>);

impl UrlOpen {
    /// Start fetching `address`; `name` is `?name=`.
    pub fn start(address: &str, name: Option<&str>, inbox: Inbox, ctx: egui::Context) -> Self {
        let resolved = resolve(address);
        let doc_name = crate::url_open::doc_name(name, resolved.as_deref().unwrap_or(address));
        let this = Self(Rc::new(RefCell::new(State { name: doc_name.clone(), phase: Phase::Fetching })));
        let href = match resolved {
            Ok(href) => href,
            Err(e) => {
                this.fail(e);
                return this;
            }
        };
        log::info!("opening {doc_name} from {href}");
        let state = this.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match fetch(&href).await {
                Ok(bytes) => {
                    inbox.lock().unwrap_or_else(|e| e.into_inner()).push((doc_name, bytes));
                    state.0.borrow_mut().phase = Phase::Queued;
                }
                Err(e) => state.fail(e),
            }
            ctx.request_repaint();
        });
        this
    }

    fn fail(&self, error: String) {
        let mut s = self.0.borrow_mut();
        log::error!("couldn't open {} from the address: {error}", s.name);
        let message = i18n::fmt(i18n::t("Couldn't open {path}: {error}"), &[("path", &s.name), ("error", &error)]);
        s.phase = Phase::Failed { message, unreported: Some(error) };
    }

    /// After the app's frame logic: a queued document either opened, or the app refused it (an
    /// unsupported format, say), which is reported like a failed download. Reports a new failure
    /// on the status bar.
    pub fn after_logic(&self, app: &mut WordApp) {
        let mut s = self.0.borrow_mut();
        if matches!(s.phase, Phase::Queued) {
            let inbox_empty = app.services.inbox.as_ref().is_none_or(|i| i.lock().map(|q| q.is_empty()).unwrap_or(true));
            if !inbox_empty || app.dialog.is_some() {
                return;
            }
            if app.has_document() {
                s.phase = Phase::Done;
                return;
            }
            // The app has reported why (already formatted as "Couldn't open <name>: …").
            let message = app.status_msg.as_ref().map(|(m, _)| m.clone()).unwrap_or_else(|| {
                let why = i18n::t("WordCraft can't open this kind of file.");
                i18n::fmt(i18n::t("Couldn't open {path}: {error}"), &[("path", &s.name), ("error", why)])
            });
            log::error!("{message}");
            s.phase = Phase::Failed { message, unreported: None };
        }
        if let Phase::Failed { unreported, .. } = &mut s.phase
            && let Some(error) = unreported.take()
            && !app.has_document()
        {
            let name = s.name.clone();
            drop(s);
            app.report_open_failure(&name, &error);
        }
    }

    /// On the empty viewer: "Opening <name>…", or why it couldn't be opened.
    pub fn show(&self, app: &WordApp, ctx: &egui::Context) {
        if app.has_document() {
            return;
        }
        let s = self.0.borrow();
        let (text, error) = match &s.phase {
            Phase::Fetching | Phase::Queued => (i18n::fmt(i18n::t("Opening {name}…"), &[("name", &s.name)]), false),
            Phase::Failed { message, .. } => (message.clone(), true),
            Phase::Done => return,
        };
        egui::Area::new(egui::Id::unique("wordcraft_url_open")).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 96.0)).interactable(false).show(
            ctx,
            |ui| {
                ui.set_max_width((ctx.content_rect().width() - 48.0).clamp(200.0, 720.0));
                let color = if error { ui.visuals().error_fg_color } else { ui.visuals().weak_text_color() };
                ui.vertical_centered(|ui| ui.label(egui::RichText::new(text).size(14.0).color(color)));
            },
        );
    }
}

/// The absolute address, resolved against the page, if its scheme may be fetched.
fn resolve(address: &str) -> Result<String, String> {
    let base = web_sys::window().and_then(|w| w.location().href().ok()).unwrap_or_default();
    let url = web_sys::Url::new_with_base(address, &base).map_err(|_| i18n::t("the address isn't valid").to_string())?;
    crate::url_open::check_scheme(&url.protocol())?;
    Ok(url.href())
}

async fn fetch(href: &str) -> Result<Vec<u8>, String> {
    let window = web_sys::window().ok_or("no window")?;
    let init = web_sys::RequestInit::new();
    init.set_method("GET");
    // Cookies for the page's own server only, never for another site's.
    init.set_credentials(web_sys::RequestCredentials::SameOrigin);
    let response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str_and_init(href, &init)).await.map_err(|e| {
        log::error!("fetch {href}: {e:?}");
        i18n::t("the download failed (no network, or the server doesn't let this site read the file: it must send Access-Control-Allow-Origin)")
            .to_string()
    })?;
    let response: web_sys::Response = response.dyn_into().map_err(|_| "not a response".to_string())?;
    if !response.ok() {
        let text = response.status_text();
        return Err(if text.is_empty() { format!("HTTP {}", response.status()) } else { format!("HTTP {} {text}", response.status()) });
    }
    if let Some(len) = response.headers().get("content-length").ok().flatten().and_then(|l| l.trim().parse::<f64>().ok())
        && len > MAX_BYTES
    {
        return Err(i18n::t("the file is larger than 512 MB").to_string());
    }
    let buffer = wasm_bindgen_futures::JsFuture::from(response.array_buffer().map_err(|e| format!("{e:?}"))?).await.map_err(|e| format!("{e:?}"))?;
    let bytes = js_sys::Uint8Array::new(&buffer);
    if f64::from(bytes.length()) > MAX_BYTES {
        return Err(i18n::t("the file is larger than 512 MB").to_string());
    }
    Ok(bytes.to_vec())
}
