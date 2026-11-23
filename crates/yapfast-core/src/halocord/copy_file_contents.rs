//! Copy file text: a row in a text file's right-click menu that copies what
//! the file says. The file downloads the way Copy Image downloads a picture;
//! the window then puts it on the clipboard as text ([`take_wanted`]). Its
//! tests share [`WANTED`], so one test touches it.

use std::sync::Mutex;

use super::{Hooks, Picked};
use crate::{
    application::Action,
    chat::{ChatMessage, Placement},
};

pub struct CopyFileContents;

/// Files bigger than this aren't offered: the clipboard is no place for them.
pub const MOST_BYTES: u64 = 2 * 1024 * 1024;

/// File endings that hold plain text.
const TEXT: &[&str] = &[
    "txt",
    "md",
    "log",
    "json",
    "jsonc",
    "csv",
    "tsv",
    "ini",
    "cfg",
    "conf",
    "toml",
    "yaml",
    "yml",
    "xml",
    "html",
    "htm",
    "css",
    "scss",
    "js",
    "mjs",
    "cjs",
    "ts",
    "tsx",
    "jsx",
    "rs",
    "py",
    "lua",
    "c",
    "h",
    "cpp",
    "hpp",
    "cs",
    "java",
    "kt",
    "go",
    "rb",
    "php",
    "sh",
    "bash",
    "ps1",
    "psm1",
    "bat",
    "cmd",
    "sql",
    "gsc",
    "csc",
    "diff",
    "patch",
    "srt",
    "vtt",
    "nfo",
    "env",
    "properties",
    "gradle",
    "svg",
];

/// The copy asked for and not yet back: the message's item and the file's place.
static WANTED: Mutex<Option<(String, usize)>> = Mutex::new(None);

/// Whether attachment `at` of `message` is a text file small enough to copy.
fn text_file(message: &ChatMessage, at: usize) -> bool {
    let Some(file) = message.attachments.get(at) else {
        return false;
    };
    let ending = file
        .name
        .rsplit_once('.')
        .map(|(_, ending)| ending.to_ascii_lowercase());
    file.placement != Placement::Sticker
        && !file.image
        && !file.video
        && file.url.is_some()
        && file.size.is_none_or(|size| size <= MOST_BYTES)
        && ending.is_some_and(|ending| TEXT.contains(&ending.as_str()))
}

/// Whether the picture Copy Image just downloaded is the text this plugin
/// asked for, which the window then copies as text. Asking again forgets it.
pub fn take_wanted() -> bool {
    WANTED
        .lock()
        .ok()
        .and_then(|mut wanted| wanted.take())
        .is_some()
}

/// The downloaded file as text to copy: invalid bytes read as replacement
/// marks, and a leading byte order mark dropped.
pub fn as_text(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.strip_prefix('\u{feff}').unwrap_or(&text).to_owned()
}

impl Hooks for CopyFileContents {
    fn id(&self) -> &'static str {
        "copy-file-contents"
    }

    /// Any other copy forgets the one waiting, so a picture copied before
    /// the text comes back stays a picture.
    fn action(&self, action: &Action) -> bool {
        if let Action::CopyAttachment { item, index } = action
            && let Ok(mut wanted) = WANTED.lock()
            && wanted
                .as_ref()
                .is_some_and(|(want, at)| want != item || at != index)
        {
            *wanted = None;
        }
        true
    }

    fn attachment_menu(
        &self,
        message: &ChatMessage,
        at: usize,
    ) -> Vec<(&'static str, &'static str)> {
        if text_file(message, at) {
            vec![("copy", "Copy file text")]
        } else {
            Vec::new()
        }
    }

    fn attachment_pick(&self, action: &str, message: &ChatMessage, at: usize) -> Picked {
        if action != "copy" || !text_file(message, at) {
            return Picked::default();
        }
        if let Ok(mut wanted) = WANTED.lock() {
            *wanted = Some((message.item.clone(), at));
        }
        Picked {
            actions: vec![Action::CopyAttachment {
                item: message.item.clone(),
                index: at,
            }],
            ..Picked::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::Attachment;
    use crate::halocord::{attachment_menu, attachment_pick, keeps_action, plug};

    fn file(name: &str, size: Option<u64>, image: bool) -> Attachment {
        Attachment {
            name: name.into(),
            size,
            image,
            dimensions: None,
            preview: None,
            preview_failed: false,
            placement: Placement::Message,
            spoiler: false,
            video: false,
            gif: None,
            url: Some(format!("https://cdn.example/{name}").into()),
            description: None,
        }
    }

    fn message() -> ChatMessage {
        ChatMessage {
            item: "e:1".into(),
            attachments: vec![
                file("notes.TXT", Some(120), false),
                file("cat.png", Some(120), true),
                file("huge.log", Some(MOST_BYTES + 1), false),
                file("song.mp3", Some(120), false),
                file("config.json", None, false),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn a_text_file_gets_the_row_and_its_copy_is_asked_for() {
        let message = message();
        assert!(attachment_menu(&message, 0).is_empty());
        plug(&CopyFileContents, true);
        assert_eq!(
            attachment_menu(&message, 0),
            [("copy-file-contents/copy".to_owned(), "Copy file text")]
        );
        assert!(!attachment_menu(&message, 4).is_empty());
        let picked = attachment_pick("copy-file-contents/copy", &message, 0);
        assert!(picked.copy.is_none());
        assert!(matches!(
            picked.actions.as_slice(),
            [Action::CopyAttachment { item, index: 0 }] if item == "e:1"
        ));
        assert!(take_wanted());
        assert!(!take_wanted());

        // Another copy forgets the text waiting; the same one keeps it.
        let _ = attachment_pick("copy-file-contents/copy", &message, 0);
        assert!(keeps_action(&Action::CopyAttachment {
            item: "e:1".into(),
            index: 0,
        }));
        assert!(take_wanted());
        let _ = attachment_pick("copy-file-contents/copy", &message, 0);
        assert!(keeps_action(&Action::CopyAttachment {
            item: "e:2".into(),
            index: 0,
        }));
        assert!(!take_wanted());
        plug(&CopyFileContents, false);
    }

    #[test]
    fn pictures_big_files_and_other_kinds_get_no_row() {
        let message = message();
        plug(&CopyFileContents, true);
        for at in [1, 2, 3, 9] {
            assert!(attachment_menu(&message, at).is_empty(), "file {at}");
        }
        assert!(
            attachment_pick("copy-file-contents/copy", &message, 1)
                .actions
                .is_empty()
        );
        plug(&CopyFileContents, false);
    }

    #[test]
    fn text_drops_a_byte_order_mark_and_survives_bad_bytes() {
        assert_eq!(as_text(b"\xef\xbb\xbfhello"), "hello");
        assert_eq!(as_text(b"a\xffb"), "a\u{fffd}b");
    }
}
