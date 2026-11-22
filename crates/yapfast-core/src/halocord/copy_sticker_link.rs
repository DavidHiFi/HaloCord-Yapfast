//! Copy sticker link: a row in a sticker's right-click menu that copies the
//! sticker's web address.

use super::{Hooks, Picked};
use crate::chat::{ChatMessage, Placement};

pub struct CopyStickerLink;

/// The address of attachment `at`, when it is a sticker with one.
fn link(message: &ChatMessage, at: usize) -> Option<&str> {
    let attachment = message.attachments.get(at)?;
    (attachment.placement == Placement::Sticker)
        .then_some(attachment.url.as_deref())
        .flatten()
}

impl Hooks for CopyStickerLink {
    fn id(&self) -> &'static str {
        "copy-sticker-links"
    }

    fn attachment_menu(
        &self,
        message: &ChatMessage,
        at: usize,
    ) -> Vec<(&'static str, &'static str)> {
        link(message, at).map_or_else(Vec::new, |_| vec![("copy", "Copy sticker link")])
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

    fn attachment(placement: Placement, url: Option<&str>) -> Attachment {
        Attachment {
            name: "wave".into(),
            size: None,
            image: true,
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
                attachment(Placement::Sticker, Some("https://cdn.example/wave.png")),
                attachment(Placement::Message, Some("https://cdn.example/cat.png")),
                attachment(Placement::Sticker, None),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn a_sticker_copies_its_address() {
        let message = message();
        assert!(attachment_menu(&message, 0).is_empty());
        plug(&CopyStickerLink, true);
        assert_eq!(
            attachment_menu(&message, 0),
            [("copy-sticker-links/copy".to_owned(), "Copy sticker link")]
        );
        assert_eq!(
            attachment_pick("copy-sticker-links/copy", &message, 0)
                .copy
                .as_deref(),
            Some("https://cdn.example/wave.png")
        );
        plug(&CopyStickerLink, false);
        assert!(
            attachment_pick("copy-sticker-links/copy", &message, 0)
                .copy
                .is_none()
        );
    }

    #[test]
    fn a_plain_picture_or_a_sticker_without_an_address_gets_no_row() {
        let message = message();
        plug(&CopyStickerLink, true);
        assert!(attachment_menu(&message, 1).is_empty());
        assert!(attachment_menu(&message, 2).is_empty());
        plug(&CopyStickerLink, false);
    }
}
