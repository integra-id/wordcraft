//! WordCraft in the browser.
//!
//! Runs the same [`wordcraft_ui_egui::WordApp`] as the desktop app through eframe's web runner
//! (wgpu: WebGPU where available, WebGL2 otherwise). Build with `trunk build --release` from this
//! directory (output in `dist/web`).
//!
//! Differences from the desktop app: no TCP control channel; Open and Insert › Pictures use the
//! browser file picker (bytes arrive through `Services::inbox`); Save and Export download.
//! Preferences (theme, interface size, editing options…) are kept in `localStorage` (`prefs`).
//! URL flags: `?webgl` forces WebGL2; `?sample` opens the sample document.
//! `?host=parent` (inside an `<iframe>`) talks to a same-origin parent page instead: it sends the
//! document to open and stores what WordCraft saves (`host`), and `&author=` names the user for
//! comments and tracked changes.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[cfg(any(target_arch = "wasm32", test))]
mod prefs;
#[cfg(any(target_arch = "wasm32", test))]
mod save_fallback;

#[cfg(target_arch = "wasm32")]
mod host;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(target_arch = "wasm32")]
fn main() {
    web::start();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("wordcraft-web only runs in the browser: build it with `trunk build --release` in apps/wordcraft-web");
}

#[cfg(test)]
mod tests {
    /// `index.html` preloads exactly the fonts the app fetches (`web::fetch_fonts`), and Trunk
    /// copies them next to the app.
    #[test]
    fn index_preloads_the_fetched_fonts() {
        let html = include_str!("../index.html");
        let preloaded: Vec<&str> =
            html.lines().filter(|l| l.contains("rel=\"preload\"")).filter_map(|l| l.split("href=\"fonts/").nth(1)?.split('"').next()).collect();
        assert_eq!(preloaded, wordcraft_fonts::FETCHED_FILES);
        assert!(html.contains(r#"<link data-trunk rel="copy-dir" href="../../assets/fonts" />"#));
    }
}
