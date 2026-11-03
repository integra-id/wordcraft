//! Viewer features an embedder hides or turns off (`docs/viewer.md`, "Hiding or turning off
//! features"): `?hide=open,print&disable=pdf` in the web app, `--hide`/`--disable` on the
//! desktop app's command line.
//!
//! Each [`Feature`] is [`Access::Shown`] (the default), [`Access::Disabled`] (drawn greyed out
//! and inactive) or [`Access::Hidden`] (not drawn at all). Either way it is off: its buttons,
//! shortcuts, menu items and commands do nothing ([`refuses`]), and with `spelling` or
//! `grammar` off nothing of that kind is checked or marked.

use serde_json::Value;

use crate::WordApp;

/// Why a command a feature switched off doesn't run.
pub const REASON: &str = "this feature is turned off in this viewer";

/// How a feature appears: shown, drawn greyed out and inactive, or not drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Access {
    #[default]
    Shown,
    Disabled,
    Hidden,
}

/// A part of the viewer an embedder can hide or turn off. The names ([`Feature::name`]) are
/// stable: they are what URLs and command lines spell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    /// The toolbar's File button (the File page itself stays reachable from Export or Print).
    File,
    /// Opening a document: Open, Ctrl+O, the File › Open page, dropping a file on the window.
    Open,
    /// Export: Save As in any format but PDF (Word, ODT, RTF, HTML, Markdown, text, PNG…).
    Export,
    /// Saving as PDF (toolbar PDF, Export › PDF, Print › Save as PDF).
    Pdf,
    /// Printing (toolbar Print, Ctrl+P, the File › Print page).
    Print,
    /// Spelling squiggles and checking, and the spelling and grammar pane.
    Spelling,
    /// Grammar squiggles and checking, and the spelling and grammar pane.
    Grammar,
    /// Find (Ctrl+F) and the Navigation pane.
    Find,
    /// Page navigation: Next/Previous Page, Go To, the status bar's page number.
    PageNav,
    /// Zoom: the toolbar's One Page / Page Width / Zoom In / Zoom Out, the status bar's zoom,
    /// Ctrl+wheel.
    Zoom,
    /// The views: Read Mode, Print Layout, Web Layout, Focus, full screen.
    Views,
    /// Copying from the page (Ctrl+C, the context menu's Copy).
    Copy,
    /// The page's right-click menu.
    ContextMenu,
    /// File › Options.
    Options,
    /// File › Close and closing the document.
    Close,
    /// Word count (status bar, dialog).
    WordCount,
    /// About and the community link on the File page.
    About,
    /// The whole toolbar across the top.
    Toolbar,
    /// The whole status bar across the bottom.
    StatusBar,
}

impl Feature {
    /// Every feature, in the order the docs list them.
    pub const ALL: &[Feature] = &[
        Feature::File,
        Feature::Open,
        Feature::Export,
        Feature::Pdf,
        Feature::Print,
        Feature::Spelling,
        Feature::Grammar,
        Feature::Find,
        Feature::PageNav,
        Feature::Zoom,
        Feature::Views,
        Feature::Copy,
        Feature::ContextMenu,
        Feature::Options,
        Feature::Close,
        Feature::WordCount,
        Feature::About,
        Feature::Toolbar,
        Feature::StatusBar,
    ];

    /// The feature's stable name in URLs and on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Feature::File => "file",
            Feature::Open => "open",
            Feature::Export => "export",
            Feature::Pdf => "pdf",
            Feature::Print => "print",
            Feature::Spelling => "spelling",
            Feature::Grammar => "grammar",
            Feature::Find => "find",
            Feature::PageNav => "pagenav",
            Feature::Zoom => "zoom",
            Feature::Views => "views",
            Feature::Copy => "copy",
            Feature::ContextMenu => "contextmenu",
            Feature::Options => "options",
            Feature::Close => "close",
            Feature::WordCount => "wordcount",
            Feature::About => "about",
            Feature::Toolbar => "toolbar",
            Feature::StatusBar => "statusbar",
        }
    }

    fn index(self) -> usize {
        Feature::ALL.iter().position(|&f| f == self).unwrap_or(0)
    }
}

/// Names that stand for several features, or other spellings of one.
pub const GROUPS: &[(&str, &[Feature])] = &[
    ("spellcheck", &[Feature::Spelling, Feature::Grammar]),
    ("proofing", &[Feature::Spelling, Feature::Grammar]),
    ("save", &[Feature::Export, Feature::Pdf]),
    ("goto", &[Feature::PageNav]),
    ("search", &[Feature::Find]),
    ("help", &[Feature::About]),
];

/// The features a name stands for: a feature's own name or a group ([`GROUPS`]). Case and
/// spaces, `-` and `_` don't matter (`Spell-Check`, `page_nav`).
pub fn lookup(name: &str) -> Option<Vec<Feature>> {
    let key: String = name.chars().filter(|c| !matches!(c, ' ' | '-' | '_')).flat_map(char::to_lowercase).collect();
    if let Some(f) = Feature::ALL.iter().find(|f| f.name() == key) {
        return Some(vec![*f]);
    }
    GROUPS.iter().find(|(n, _)| *n == key).map(|(_, fs)| fs.to_vec())
}

/// What the viewer shows of each [`Feature`]. The default shows everything.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ViewerFeatures {
    access: [Access; Feature::ALL.len()],
}

impl ViewerFeatures {
    pub fn get(&self, f: Feature) -> Access {
        self.access[f.index()]
    }

    pub fn set(&mut self, f: Feature, a: Access) {
        self.access[f.index()] = a;
    }

    /// Whether the feature is drawn (shown or greyed out).
    pub fn visible(&self, f: Feature) -> bool {
        self.get(f) != Access::Hidden
    }

    /// Whether the feature works.
    pub fn enabled(&self, f: Feature) -> bool {
        self.get(f) == Access::Shown
    }

    /// Whether anything is hidden or turned off.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Hide or turn off the features a comma-separated list names (`"open, spellcheck"`).
    /// Hidden wins over disabled, whichever comes first. Returns the names it doesn't know,
    /// which it ignores.
    pub fn apply(&mut self, list: &str, a: Access) -> Vec<String> {
        let mut unknown = Vec::new();
        for name in list.split(',').map(str::trim).filter(|n| !n.is_empty()) {
            match lookup(name) {
                Some(fs) => {
                    for f in fs {
                        if self.get(f) != Access::Hidden {
                            self.set(f, a);
                        }
                    }
                }
                None => unknown.push(name.to_string()),
            }
        }
        unknown
    }

    /// From `hide` and `disable` lists (see [`Self::apply`]); also returns the unknown names.
    pub fn parse(hide: &str, disable: &str) -> (Self, Vec<String>) {
        let mut f = Self::default();
        let mut unknown = f.apply(hide, Access::Hidden);
        unknown.extend(f.apply(disable, Access::Disabled));
        (f, unknown)
    }

    /// What the engine should stop checking and marking.
    pub fn proofing_off(&self) -> wordcraft_engine::viewer::ProofingOff {
        wordcraft_engine::viewer::ProofingOff { spelling: !self.enabled(Feature::Spelling), grammar: !self.enabled(Feature::Grammar) }
    }

    /// The feature command `id` belongs to, if any. `params` refines it: Open with a `path` or
    /// `data` is a document the host or a script hands over (`?url=`, the embedding page),
    /// not the person picking one, and Save As in PDF format is `pdf`, not `export`.
    pub fn command_features(id: &str, params: Option<&Value>) -> &'static [Feature] {
        let param = |k: &str| params.and_then(|p| p.get(k)).and_then(Value::as_str);
        match id {
            "file.open" if param("path").is_some() || param("data").is_some() => &[],
            "file.open" | "ui.openFileDialog" | "ui.fileBrowser" => &[Feature::Open],
            "file.saveAs" if param("format").is_some_and(|f| f.eq_ignore_ascii_case("pdf")) => &[Feature::Pdf],
            "file.saveAs" if param("path").is_some_and(|p| p.to_ascii_lowercase().ends_with(".pdf")) => &[Feature::Pdf],
            "file.saveAs" | "file.exportPng" => &[Feature::Export],
            "file.exportPdf" | "file.printPdf" => &[Feature::Pdf],
            "file.print" | "file.printMarkup" | "ui.print" | "ui.printSubmit" | "ui.printSettings" => &[Feature::Print],
            "review.spelling" | "review.issues" | "review.suggestions" | "review.proofing" | "review.applySuggestion" => {
                &[Feature::Spelling, Feature::Grammar]
            }
            "proof.packs" | "proof.installPack" | "proof.removePack" => &[Feature::Spelling, Feature::Grammar],
            "edit.find" | "edit.advancedFind" | "edit.findNext" | "edit.findPrevious" | "view.navigationPane" => &[Feature::Find],
            "edit.goto" | "edit.goBack" => &[Feature::PageNav],
            "view.zoom" | "view.zoom100" | "view.zoomIn" | "view.zoomOut" | "view.onePage" | "view.multiplePages" | "view.pageWidth"
            | "view.readTextSize" => &[Feature::Zoom],
            "view.readMode"
            | "view.webLayout"
            | "view.draft"
            | "view.outline"
            | "view.focus"
            | "view.fullScreen"
            | "view.fullScreenView"
            | "view.immersive"
            | "view.sideToSide"
            | "view.vertical" => &[Feature::Views],
            "edit.copy" => &[Feature::Copy],
            "ui.contextMenu" => &[Feature::ContextMenu],
            "file.options" => &[Feature::Options],
            "file.close" | "file.closeDocument" => &[Feature::Close],
            "review.wordCount" => &[Feature::WordCount],
            "ui.discord" => &[Feature::About],
            "ui.dialog" => param("name").map_or(&[], Self::dialog_features),
            "ui.backstage" => param("page").map_or(&[], Self::page_features),
            _ => &[],
        }
    }

    /// The features behind dialog `name` (`Dialog::open`).
    pub fn dialog_features(name: &str) -> &'static [Feature] {
        match name {
            "find" | "replace" => &[Feature::Find],
            "goto" => &[Feature::PageNav],
            "zoom" => &[Feature::Zoom],
            "wordCount" => &[Feature::WordCount],
            "about" => &[Feature::About],
            "spelling" | "proofing" => &[Feature::Spelling, Feature::Grammar],
            _ => &[],
        }
    }

    /// The features behind File page `page`.
    pub fn page_features(page: &str) -> &'static [Feature] {
        match page {
            "open" => &[Feature::Open],
            "print" => &[Feature::Print],
            "export" => &[Feature::Export],
            "close" => &[Feature::Close],
            "options" => &[Feature::Options],
            _ => &[],
        }
    }

    /// Why command `id` doesn't run here, or `None` when every feature it belongs to is on.
    pub fn refuses(&self, id: &str, params: Option<&Value>) -> Option<&'static str> {
        Self::command_features(id, params).iter().any(|&f| !self.enabled(f)).then_some(REASON)
    }

    /// Whether File page `page` is drawn, and whether it works.
    pub fn page(&self, page: &str) -> (bool, bool) {
        let fs = Self::page_features(page);
        (fs.iter().all(|&f| self.visible(f)), fs.iter().all(|&f| self.enabled(f)))
    }
}

impl WordApp {
    /// Hide or turn off viewer features (see the module docs). Spelling and grammar checking
    /// stop or start again at once.
    pub fn set_features(&mut self, features: ViewerFeatures) {
        self.features = features;
        self.session.set_proofing_off(features.proofing_off());
    }

    /// Whether feature `f` is drawn.
    pub fn shows(&self, f: Feature) -> bool {
        self.features.visible(f)
    }

    /// Whether feature `f` works.
    pub fn allows(&self, f: Feature) -> bool {
        self.features.enabled(f)
    }
}

/// Why command `id` (with `params`, when known) doesn't run because a feature is off.
pub fn refuses(app: &WordApp, id: &str, params: Option<&Value>) -> Option<&'static str> {
    app.features.refuses(id, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn names_are_unique_and_round_trip() {
        for f in Feature::ALL {
            assert_eq!(lookup(f.name()), Some(vec![*f]));
            assert_eq!(Feature::ALL.iter().filter(|g| g.name() == f.name()).count(), 1);
        }
        for (g, _) in GROUPS {
            assert!(Feature::ALL.iter().all(|f| f.name() != *g), "group {g} shadows a feature");
        }
    }

    #[test]
    fn parsing_is_case_and_space_insensitive_and_reports_unknown_names() {
        let (f, unknown) = ViewerFeatures::parse(" Open,EXPORT ,, print", "pdf,Page-Nav,bogus");
        assert_eq!(unknown, vec!["bogus".to_string()]);
        assert_eq!(f.get(Feature::Open), Access::Hidden);
        assert_eq!(f.get(Feature::Export), Access::Hidden);
        assert_eq!(f.get(Feature::Print), Access::Hidden);
        assert_eq!(f.get(Feature::Pdf), Access::Disabled);
        assert_eq!(f.get(Feature::PageNav), Access::Disabled);
        assert_eq!(f.get(Feature::Zoom), Access::Shown);
        assert!(ViewerFeatures::parse("", "").0.is_default());
    }

    #[test]
    fn groups_cover_several_features_and_hidden_wins() {
        let (f, unknown) = ViewerFeatures::parse("grammar", "spellcheck");
        assert!(unknown.is_empty());
        assert_eq!(f.get(Feature::Spelling), Access::Disabled);
        assert_eq!(f.get(Feature::Grammar), Access::Hidden, "hidden wins over disabled");
        let off = f.proofing_off();
        assert!(off.spelling && off.grammar);
        let (f, _) = ViewerFeatures::parse("", "save");
        assert!(!f.enabled(Feature::Export) && !f.enabled(Feature::Pdf) && f.enabled(Feature::Print));
    }

    #[test]
    fn commands_map_to_their_features() {
        let (f, _) = ViewerFeatures::parse("open,print", "pdf");
        assert!(f.refuses("file.open", None).is_some());
        assert!(f.refuses("file.open", Some(&json!({}))).is_some());
        // A document the host hands over still opens.
        assert!(f.refuses("file.open", Some(&json!({"path": "a.docx", "data": "AA=="}))).is_none());
        assert!(f.refuses("ui.print", None).is_some());
        assert!(f.refuses("file.exportPdf", None).is_some());
        assert!(f.refuses("file.saveAs", Some(&json!({"path": "a.PDF"}))).is_some());
        assert!(f.refuses("file.saveAs", Some(&json!({"path": "a.odt", "format": "odt"}))).is_none());
        assert!(f.refuses("ui.backstage", Some(&json!({"page": "print"}))).is_some());
        assert!(f.refuses("ui.backstage", Some(&json!({"page": "export"}))).is_none());
        assert!(f.refuses("edit.copy", None).is_none());
        assert_eq!(f.page("open"), (false, false));
        assert_eq!(f.page("export"), (true, true));
        let (f, _) = ViewerFeatures::parse("", "export");
        assert!(f.refuses("file.saveAs", None).is_some());
        assert!(f.refuses("file.saveAs", Some(&json!({"format": "pdf"}))).is_none());
        assert_eq!(f.page("export"), (true, false));
    }
}
