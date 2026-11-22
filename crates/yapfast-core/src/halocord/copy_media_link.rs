//! Copy file link: a row in a file's right-click menu that copies the file's
//! web address. Pictures and videos already have Copy Link in their menu, and
//! stickers have their own plugin.

use super::{Hooks, Picked};
use crate::chat::{ChatMessage, Placement};

pub struct CopyMediaLink;

/// The address of attachment `at`, when it is a file with one.
fn link(message: &ChatMessage, at: usize) -> Option<&str> {
    let attachment = message.attachments.get(at)?;
    (attachment.placement != Placement::Sticker)
        .then_some(attachment.url.as_deref())
        .flatten()
}

impl Hooks for CopyMediaLink {
    fn id(&self) -> &'static str {
        "copy-media-link"
    }

    fn attachment_menu(
        &self,
        message: &ChatMessage,
        at: usize,
    ) -> Vec<(&'static str, &'static str)> {
        link(message, at).map_or_else(Vec::new, |_| vec![("copy", "Copy file link")])
    }

    fn attachment_pick(&self, action: &str, message: &ChatMessage, at: usize) -> Picked {
        Picked {
            copy: link(message, at)
                .filter(|_| action == "copy")
                .map(str::to_owned),
            ..Picked::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::Attachment;
    use crate::halocord::{attachment_menu, attachment_pick, plug};

    fn file(placement: Placement, url: Option<&str>) -> Attachment {
        Attachment {
            name: "notes.txt".into(),
            size: None,
            image: false,
            dimensions: None,
            preview: None,
            preview_failed: false,
            placement,
            spoiler: false,
            video: false,
            gif: None,
            url: url.map(Into::into),
            description: None,
        }
    }

    fn message() -> ChatMessage {
        ChatMessage {
            attachments: vec![
                file(Placement::Message, Some("https://cdn.example/notes.txt")),
                file(Placement::Message, None),
                file(Placement::Sticker, Some("https://cdn.example/s.png")),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn a_file_with_an_address_copies_it() {
        let message = message();
        assert!(attachment_menu(&message, 0).is_empty());
        plug(&CopyMediaLink, true);
        assert_eq!(
            attachment_menu(&message, 0),
            [("copy-media-link/copy".to_owned(), "Copy file link")]
        );
        assert_eq!(
            attachment_pick("copy-media-link/copy", &message, 0)
                .copy
                .as_deref(),
            Some("https://cdn.example/notes.txt")
        );
        plug(&CopyMediaLink, false);
        assert!(
            attachment_pick("copy-media-link/copy", &message, 0)
                .copy
                .is_none()
        );
    }

    #[test]
    fn no_address_a_sticker_or_a_missing_file_gets_no_row() {
        let message = message();
        plug(&CopyMediaLink, true);
        assert!(attachment_menu(&message, 1).is_empty());
        assert!(attachment_menu(&message, 2).is_empty());
        assert!(attachment_menu(&message, 9).is_empty());
        plug(&CopyMediaLink, false);
    }
}
