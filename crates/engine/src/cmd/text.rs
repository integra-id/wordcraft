//! Typing and deleting.

use serde_json::Value;
use wordcraft_doc::para::{COLUMN_BREAK, NB_HYPHEN, NBSP, OBJ, PAGE_BREAK, SOFT_HYPHEN};
use wordcraft_doc::props::{Border, BorderStyle, NumRef};
use wordcraft_doc::{Block, ListKind, Path, Pos, StoryRef, Table};

use super::{delete_selection, delete_selection_effect, non_mac, sel_result, split_para, type_text, type_text_effect};
use crate::{CmdError, CmdResult, CommandSpec, Selection, Session, p};

mod autoformat;
mod delete;
mod enter_tab;
mod unicode;

use autoformat::insert;
use delete::{backspace, delete, delete_word_back, delete_word_fwd};
pub use enter_tab::MAX_TYPED_PARAGRAPHS;
use enter_tab::{back_tab, new_paragraph, tab};
pub use unicode::{alt_code_char, oem_code_page, system_code_pages, system_oem_code_page, windows_code_pages};
use unicode::{ins_char, insert_code, toggle_unicode};

/// The text-entry commands: typing, Enter, breaks, Tab, deleting and typed symbols.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!("text.insert", "Type Text", "Editing", insert).params(r#"{"text": string, "raw"?: bool, "paragraphs"?: bool} (paragraphs: each line a paragraph, one undo step, at most 10000 lines)"#),
        spec!("text.newParagraph", "New Paragraph", "Editing", new_paragraph).key("Enter"),
        spec!("text.lineBreak", "Line Break", "Editing", |s, _| ins_char(s, '\n')).key("Shift+Enter"),
        spec!("text.pageBreak", "Page Break", "Insert › Pages", |s, _| ins_char(s, PAGE_BREAK)).key("Mod+Enter"),
        spec!("text.columnBreak", "Column Break", "Layout › Breaks", |s, _| ins_char(s, COLUMN_BREAK)).key("Mod+Shift+Enter"),
        spec!("text.tab", "Tab", "Editing", tab).key("Tab"),
        spec!("text.backTab", "Shift+Tab", "Editing", back_tab).key("Shift+Tab"),
        // A tab character even in a table cell, where Tab moves to the next cell (#3137); ⌥Tab on macOS.
        spec!("text.insertTab", "Tab Character", "Editing", |s, _| ins_char(s, '\t'))
            .key(if cfg!(target_os = "macos") { "Alt+Tab" } else { "Mod+Tab" }),
        spec!("text.backspace", "Backspace", "Editing", backspace).key("Backspace"),
        spec!("text.delete", "Delete", "Editing", delete).key("Delete"),
        spec!("text.deleteWordBack", "Delete Previous Word", "Editing", delete_word_back).key("Mod+Backspace / Alt+Backspace"),
        spec!("text.deleteWordForward", "Delete Next Word", "Editing", delete_word_fwd).key("Mod+Delete / Alt+Delete"),
        spec!("text.nbsp", "Nonbreaking Space", "Insert › Symbols", |s, _| ins_char(s, NBSP)).key("Mod+Shift+Space"),
        spec!("text.nbHyphen", "Nonbreaking Hyphen", "Insert › Symbols", |s, _| ins_char(s, NB_HYPHEN)).key("Mod+Shift+-"),
        spec!("text.optionalHyphen", "Optional Hyphen", "Insert › Symbols", |s, _| ins_char(s, SOFT_HYPHEN)).key("Mod+-"),
        // Alt+X on Windows and Linux; macOS has no default (Option+X types a character there).
        spec!("text.toggleUnicode", "Toggle Unicode Character", "Editing", toggle_unicode).key(if cfg!(target_os = "macos") {
            ""
        } else {
            "Alt+X"
        }),
        // Alt+numpad codes: the shell collects the keypad digits while Alt is held (#961).
        spec!("text.typeCode", "Type Character Code", "Editing", insert_code)
            .params(r#"{"code": string (decimal digits as typed on the keypad, e.g. "0233"), "ansi"?: 874 | 1250–1258 (the leading-0 code page, default 1252), "oem"?: 437 | 737 | 850 | 852 | 857 | 866 (default from the locale)}"#),
        spec!("text.emDash", "Em Dash", "Insert › Symbols", |s, _| ins_char(s, '—')).key("Mod+Alt+-"),
        // On macOS Option+G / R / 2 / ; type these directly, so Word for Mac has no Cmd shortcut.
        spec!("text.copyright", "Copyright Sign", "Insert › Symbols", |s, _| ins_char(s, '©')).key(non_mac("Mod+Alt+C")),
        spec!("text.registered", "Registered Sign", "Insert › Symbols", |s, _| ins_char(s, '®')).key(non_mac("Mod+Alt+R")),
        spec!("text.trademark", "Trademark Sign", "Insert › Symbols", |s, _| ins_char(s, '™')).key(non_mac("Mod+Alt+T")),
        spec!("text.ellipsis", "Ellipsis", "Insert › Symbols", |s, _| ins_char(s, '…')).key(non_mac("Mod+Alt+.")),
    ]
}
