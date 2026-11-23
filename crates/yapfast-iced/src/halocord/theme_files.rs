//! Themes as files: Appearance's Save to file writes the theme open in the
//! editor as the JSON `layout.json` keeps a custom theme in (see
//! `docs/themes.md`), and Load from file reads one back as a new custom
//! theme, chosen at once. A file from someone else's copy works the same,
//! and a colour it doesn't name takes its base theme's.

use std::path::PathBuf;

use iced::Task;
use yapfast_core::theme::{self as palette, Custom};

use crate::app::Message;
use crate::settings::{Event as SettingsEvent, ThemeEvent as Event};

/// What Appearance says after a file was saved or loaded, or wasn't.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Note {
    Saved,
    NotSaved,
    Loaded,
    NotATheme,
}

impl Note {
    pub fn words(self) -> &'static str {
        match self {
            Self::Saved => "Saved the theme to a file.",
            Self::NotSaved => "Couldn't save the file there. Try another folder.",
            Self::Loaded => "Loaded the theme. It's with your own themes now.",
            Self::NotATheme => "That file isn't a theme Yapfast can read.",
        }
    }

    pub fn bad(self) -> bool {
        matches!(self, Self::NotSaved | Self::NotATheme)
    }
}

/// The text a theme file holds.
pub fn text(custom: &Custom) -> String {
    serde_json::to_string_pretty(custom).unwrap_or_default()
}

/// The theme in a file's text, as a new custom theme after `custom`; none
/// when the text isn't a theme.
pub fn read(text: &str, custom: &[Custom]) -> Option<Custom> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let colours = value.get("colours")?.as_object()?;
    // A file with none of the colours Yapfast knows is something else.
    if !colours.keys().any(|token| {
        palette::Theme::NAMES
            .iter()
            .any(|name| palette::token(name) == *token)
    }) {
        return None;
    }
    let mut theme: Custom = serde_json::from_value(value).ok()?;
    theme.id = palette::new_id(custom);
    theme.name = match theme.name.trim() {
        "" => "Loaded theme".to_owned(),
        name => name.chars().take(64).collect(),
    };
    Some(theme)
}

/// The file name offered for a theme called `name`.
pub fn file_name(name: &str) -> String {
    let kept: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    match kept.trim() {
        "" => "theme.json".to_owned(),
        kept => format!("{kept}.json"),
    }
}

fn message(event: Event) -> Message {
    Message::Settings(SettingsEvent::Theme(event))
}

/// Asks where to save `custom`, then writes it there.
pub fn save(custom: &Custom) -> Task<Message> {
    let (name, contents) = (file_name(&custom.name), text(custom));
    Task::perform(
        async move {
            let file = rfd::AsyncFileDialog::new()
                .set_title("Save theme")
                .set_file_name(name)
                .add_filter("Theme", &["json"])
                .save_file()
                .await?;
            Some(std::fs::write(file.path(), contents).is_ok())
        },
        |written| {
            message(Event::FileDone(
                written.map(|ok| if ok { Note::Saved } else { Note::NotSaved }),
            ))
        },
    )
}

/// Asks for a theme file, then reads it.
pub fn open() -> Task<Message> {
    Task::perform(
        async {
            let file = rfd::AsyncFileDialog::new()
                .set_title("Load theme")
                .add_filter("Theme", &["json"])
                .pick_file()
                .await?;
            let path: PathBuf = file.path().to_owned();
            // A theme file is a few kilobytes; anything far bigger isn't one.
            let size = std::fs::metadata(&path).ok()?.len();
            Some(if size > 256 * 1024 {
                String::new()
            } else {
                std::fs::read_to_string(&path).unwrap_or_default()
            })
        },
        |text| message(Event::FileRead(text)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plum() -> Custom {
        let base = palette::find("nord").unwrap();
        let mut theme = base.theme;
        theme.chat = palette::parse("#2a1a3a").unwrap();
        Custom {
            id: "custom:4".into(),
            name: "Plum".into(),
            base: base.id,
            theme,
        }
    }

    #[test]
    fn a_saved_theme_loads_back_the_same_as_a_new_one() {
        let saved = plum();
        let mine = vec![saved.clone()];
        let loaded = read(&text(&saved), &mine).unwrap();
        assert_eq!(loaded.id, "custom:5", "a new id after the ones there are");
        assert_eq!(loaded.name, "Plum");
        assert_eq!(loaded.base, "nord");
        assert_eq!(loaded.theme, saved.theme);
    }

    #[test]
    fn a_file_missing_colours_takes_its_bases_and_other_files_are_refused() {
        let loaded = read(
            r##"{"name":"Short","base":"dracula","colours":{"chat":"#102030"}}"##,
            &[],
        )
        .unwrap();
        let dracula = palette::find("dracula").unwrap().theme;
        assert_eq!(loaded.theme.chat, palette::parse("#102030").unwrap());
        assert_eq!(loaded.theme.rail, dracula.rail);
        assert_eq!(loaded.theme.focus, dracula.focus);
        assert!(read("not json", &[]).is_none());
        assert!(read(r#"{"name":"x"}"#, &[]).is_none());
        assert!(read(r##"{"colours":{"nothing":"#fff"}}"##, &[]).is_none());
    }

    #[test]
    fn file_names_keep_only_plain_characters() {
        assert_eq!(file_name("Plum"), "Plum.json");
        assert_eq!(file_name("a/b:c"), "a_b_c.json");
        assert_eq!(file_name("  "), "theme.json");
    }
}
