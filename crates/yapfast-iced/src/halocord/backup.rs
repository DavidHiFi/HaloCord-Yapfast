//! Backup & Restore: HaloCord's setup in one JSON file. It holds the
//! plugins (`halocord` in `layout.json`), MochaCord's options (`mochacord`),
//! the person's own themes and the theme chosen. Save to file writes it,
//! Load from file reads one back, checks it and hands it to Settings to
//! apply. The file dialogs work as the theme Import and Export do.

use std::path::PathBuf;

use iced::Task;
use yapfast_core::theme::Custom;

use crate::{
    app::Message,
    halocord::Kept,
    settings::{Event as SettingsEvent, HaloEvent},
};

/// What a backup file says it is.
const KIND: &str = "halocord-backup";
/// The file's layout; a newer one is refused rather than half read.
const VERSION: u32 = 1;
/// A backup is a few kilobytes; anything far bigger isn't one.
const LARGEST: u64 = 1024 * 1024;

/// What Backup & Restore says after a file was saved or loaded, or wasn't.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Note {
    Saved,
    NotSaved,
    Loaded,
    NotABackup,
    NotRead,
}

impl Note {
    pub fn words(self) -> &'static str {
        match self {
            Self::Saved => "Saved your setup to a file.",
            Self::NotSaved => "Couldn't save the file there. Try another folder.",
            Self::Loaded => "Loaded. Your plugins, themes and look are as the file had them.",
            Self::NotABackup => "That file isn't a HaloCord backup.",
            Self::NotRead => "Couldn't read that file.",
        }
    }

    pub fn bad(self) -> bool {
        matches!(self, Self::NotSaved | Self::NotABackup | Self::NotRead)
    }
}

/// Everything a backup holds.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Backup {
    kind: String,
    version: u32,
    #[serde(default)]
    pub halocord: Kept,
    #[serde(default)]
    pub mochacord: crate::mochacord::Kept,
    #[serde(default)]
    pub themes: Vec<Custom>,
    /// The theme chosen: a built-in's id or one of `themes`.
    #[serde(default)]
    pub theme: Option<String>,
}

impl Backup {
    /// A backup of this setup.
    pub fn of(
        halocord: &Kept,
        mochacord: &crate::mochacord::Kept,
        themes: &[Custom],
        theme: Option<&str>,
    ) -> Self {
        Self {
            kind: KIND.to_owned(),
            version: VERSION,
            halocord: halocord.clone(),
            mochacord: mochacord.clone(),
            themes: themes.to_vec(),
            theme: theme.map(str::to_owned),
        }
    }

    /// The file's text.
    pub fn text(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// The backup in a file's text; none when it isn't one.
    pub fn read(text: &str) -> Option<Self> {
        let backup: Self = serde_json::from_str(text).ok()?;
        (backup.kind == KIND && backup.version <= VERSION).then_some(backup)
    }
}

/// The file name offered.
pub const FILE_NAME: &str = "halocord-backup.json";

fn message(event: HaloEvent) -> Message {
    crate::app::settings(SettingsEvent::HaloCord(event))
}

/// Asks where to save `backup`, then writes it there.
pub fn save(backup: &Backup) -> Task<Message> {
    let contents = backup.text();
    Task::perform(
        async move {
            let file = rfd::AsyncFileDialog::new()
                .set_title("Save HaloCord backup")
                .set_file_name(FILE_NAME)
                .add_filter("Backup", &["json"])
                .save_file()
                .await?;
            Some(std::fs::write(file.path(), contents).is_ok())
        },
        |written| {
            message(HaloEvent::BackupDone(
                written.map(|ok| if ok { Note::Saved } else { Note::NotSaved }),
            ))
        },
    )
}

/// Asks for a backup file, then reads it: the text, none when the window
/// was closed, and an empty text when the file couldn't be read.
pub fn open() -> Task<Message> {
    Task::perform(
        async {
            let file = rfd::AsyncFileDialog::new()
                .set_title("Load HaloCord backup")
                .add_filter("Backup", &["json"])
                .pick_file()
                .await?;
            let path: PathBuf = file.path().to_owned();
            let small = std::fs::metadata(&path).is_ok_and(|meta| meta.len() <= LARGEST);
            Some(if small {
                std::fs::read_to_string(&path).ok()
            } else {
                None
            })
        },
        |text| message(HaloEvent::BackupRead(text)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::halocord::{Change, PLUGINS, id, place};
    use yapfast_core::theme as palette;

    fn setup() -> Backup {
        let mut halocord = Kept::default();
        let stereo = place(id::STEREO).unwrap();
        halocord.change(Change::Switch {
            plugin: stereo,
            on: !PLUGINS[stereo].on_at_first,
        });
        let mochacord = crate::mochacord::Kept {
            glass_strength: 40,
            corners: 150,
            ..Default::default()
        };
        let base = palette::find("nord").unwrap();
        let themes = vec![Custom {
            id: "custom:1".into(),
            name: "Plum".into(),
            base: base.id,
            theme: base.theme,
        }];
        Backup::of(&halocord, &mochacord, &themes, Some("custom:1"))
    }

    #[test]
    fn a_saved_backup_reads_back_the_same() {
        let saved = setup();
        let loaded = Backup::read(&saved.text()).unwrap();
        assert_eq!(loaded.halocord, saved.halocord);
        assert_eq!(loaded.mochacord, saved.mochacord);
        assert_eq!(loaded.mochacord.glass_strength, 40);
        assert_eq!(loaded.themes.len(), 1);
        assert_eq!(loaded.themes[0].name, "Plum");
        assert_eq!(loaded.themes[0].theme, saved.themes[0].theme);
        assert_eq!(loaded.theme.as_deref(), Some("custom:1"));
    }

    #[test]
    fn a_file_with_parts_missing_takes_what_they_start_as() {
        let loaded = Backup::read(r#"{"kind":"halocord-backup","version":1}"#).unwrap();
        assert_eq!(loaded.halocord, Kept::default());
        assert_eq!(loaded.mochacord, crate::mochacord::Kept::default());
        assert!(loaded.themes.is_empty() && loaded.theme.is_none());
    }

    #[test]
    fn other_files_are_refused() {
        assert!(Backup::read("not json").is_none());
        assert!(Backup::read("{}").is_none());
        assert!(Backup::read(r#"{"name":"a theme","colours":{}}"#).is_none());
        assert!(Backup::read(r#"{"kind":"something-else","version":1}"#).is_none());
        assert!(Backup::read(r#"{"kind":"halocord-backup","version":2}"#).is_none());
        assert!(Backup::read(r#"{"kind":"halocord-backup","version":1,"halocord":5}"#).is_none());
    }

    #[test]
    fn the_notes_say_which_went_wrong() {
        assert!(!Note::Saved.bad() && !Note::Loaded.bad());
        assert!(Note::NotSaved.bad() && Note::NotABackup.bad() && Note::NotRead.bad());
    }
}
