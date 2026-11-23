//! Copy sticker link: a row in a sticker's right-click menu that copies the
//! sticker's web address, and Open sticker link beside it, which opens the
//! sticker large in the browser.

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

/// A sticker's picture large: 512 px, and lossless where the media server
/// takes it (`lossless` only means something on media.discordapp.net; a
/// sticker from cdn.discordapp.com comes at 512 px as it is). None for an
/// animated Lottie sticker (`.json`), which a browser
/// would only show as text.
fn large(link: &str) -> Option<String> {
    let base = link.split_once('?').map_or(link, |(base, _)| base);
    (!base.ends_with(".json")).then(|| format!("{base}?size=512&lossless=true"))
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
        let Some(link) = link(message, at) else {
            return Vec::new();
        };
        let mut rows = vec![("copy", "Copy sticker link")];
        if large(link).is_some() {
            rows.push(("open", "Open sticker link"));
        }
        rows
    }

    fn attachment_pick(&self, action: &str, message: &ChatMessage, at: usize) -> Picked {
        let Some(link) = link(message, at) else {
            return Picked::default();
        };
        match action {
            "copy" => Picked {
                copy: Some(link.to_owned()),
                ..Picked::default()
            },
            "open" => Picked {
                open: large(link),
                ..Picked::default()
            },
            _ => Picked::default(),
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
            [
                ("copy-sticker-links/copy".to_owned(), "Copy sticker link"),
                ("copy-sticker-links/open".to_owned(), "Open sticker link"),
            ]
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

    #[test]
    fn open_sticker_link_opens_it_at_512_lossless() {
        let mut message = message();
        message.attachments[0].url =
            Some("https://media.discordapp.net/stickers/8.png?size=160".into());
        plug(&CopyStickerLink, true);
        assert_eq!(
            attachment_pick("copy-sticker-links/open", &message, 0)
                .open
                .as_deref(),
            Some("https://media.discordapp.net/stickers/8.png?size=512&lossless=true")
        );
        // An animated Lottie sticker gets Copy alone.
        message.attachments[0].url = Some("https://media.discordapp.net/stickers/9.json".into());
        assert_eq!(
            attachment_menu(&message, 0),
            [("copy-sticker-links/copy".to_owned(), "Copy sticker link")]
        );
        plug(&CopyStickerLink, false);
        assert!(
            attachment_pick("copy-sticker-links/open", &message, 0)
                .open
                .is_none()
        );
    }
}
