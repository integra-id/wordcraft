//! `?hide=` and `?disable=`: viewer features the embedding page hides or turns off
//! (`wordcraft_ui_egui::features`, packaging/web/README.md). Comma-separated names, any case;
//! unknown names are ignored with a console warning. `wordcraft:config` (`host`) sends the
//! same two lists later.

use wordcraft_ui_egui::features::{Feature, GROUPS, ViewerFeatures};

/// The features `hide` and `disable` name, and a warning naming those it doesn't know.
pub fn parse(hide: Option<&str>, disable: Option<&str>) -> (ViewerFeatures, Option<String>) {
    let (features, unknown) = ViewerFeatures::parse(hide.unwrap_or(""), disable.unwrap_or(""));
    (features, warning(&unknown))
}

/// The console warning for unknown feature names, with the names that work.
fn warning(unknown: &[String]) -> Option<String> {
    if unknown.is_empty() {
        return None;
    }
    let known: Vec<&str> = Feature::ALL.iter().map(|f| f.name()).chain(GROUPS.iter().map(|(n, _)| *n)).collect();
    Some(format!("wordcraft: ignoring unknown feature name(s) {}; known: {}", unknown.join(", "), known.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wordcraft_ui_egui::features::Access;

    #[test]
    fn hide_and_disable_lists_are_parsed_and_unknown_names_warned_about() {
        let (f, warn) = parse(Some("open,export,print"), Some("PDF, nope"));
        assert_eq!(f.get(Feature::Open), Access::Hidden);
        assert_eq!(f.get(Feature::Print), Access::Hidden);
        assert_eq!(f.get(Feature::Pdf), Access::Disabled);
        let warn = warn.unwrap_or_default();
        assert!(warn.contains("nope") && warn.contains("spellcheck"), "{warn}");
        let (f, warn) = parse(None, None);
        assert!(f.is_default() && warn.is_none());
        let (f, warn) = parse(Some("spellcheck"), None);
        assert!(warn.is_none());
        assert!(!f.visible(Feature::Spelling) && !f.visible(Feature::Grammar));
    }
}
