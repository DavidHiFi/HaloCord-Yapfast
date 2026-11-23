//! Favorite anything: still pictures and attached videos in messages can be
//! favourites too, not only pictures that move.

use crate::chat::Gif;

#[cfg(not(test))]
static ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
thread_local! {
    // Each test its own, out of the way of the GIF tests beside it.
    static ON: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Turns the plugin on or off.
pub fn set(on: bool) {
    #[cfg(not(test))]
    ON.store(on, std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    ON.with(|cell| cell.set(on));
}

/// Whether the plugin is on.
pub fn is_on() -> bool {
    #[cfg(not(test))]
    return ON.load(std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    ON.with(std::cell::Cell::get)
}

/// The id a still picture's favourite carries on its message, which tells
/// it from a moving one's: a message's GIF has none. It stays on the
/// message: a favourite kept on Discord carries no id.
pub const STILL: &str = "halocord-still";

/// Whether a message's GIF is a still picture's, which Favorite anything
/// offered: it has no GIF corner.
pub fn still(gif: &Gif) -> bool {
    gif.id.as_deref() == Some(STILL)
}

/// Whether a message's attachment can be a favourite, and if so the id its
/// favourite carries: a picture that `moves`, as Yapfast offers now (no
/// id), and while the plugin is on, an attached video too (no id, kept as
/// Discord keeps a video GIF) and a still picture ([`STILL`]).
#[cfg(feature = "discord")]
pub(crate) fn offered(
    attachment: &yapfast_discord::message::Attachment,
    moves: bool,
) -> Option<Option<String>> {
    offered_with(attachment, moves, is_on())
}

#[cfg(feature = "discord")]
fn offered_with(
    attachment: &yapfast_discord::message::Attachment,
    moves: bool,
    anything: bool,
) -> Option<Option<String>> {
    if attachment.image && moves {
        return Some(None);
    }
    if !anything {
        return None;
    }
    if attachment.video_url.is_some()
        && attachment.shown == yapfast_discord::message::Shown::Message
    {
        return Some(None);
    }
    attachment.image.then(|| Some(STILL.to_owned()))
}

#[cfg(all(test, feature = "discord"))]
mod tests {
    use super::*;

    #[test]
    fn offers_stills_and_videos_while_on() {
        let message = yapfast_discord::message::parse(&serde_json::json!({
            "id": "10", "channel_id": "2", "content": "",
            "author": {"id": "3", "username": "sam"},
            "timestamp": "2024-01-01T00:00:00+00:00",
            "attachments": [
                {"id": "5", "filename": "still.png", "size": 1000, "width": 64, "height": 64,
                 "url": "https://cdn.discordapp.com/attachments/2/5/still.png",
                 "proxy_url": "https://media.discordapp.net/attachments/2/5/still.png",
                 "content_type": "image/png"},
                {"id": "6", "filename": "clip.mp4", "size": 9000, "width": 640, "height": 360,
                 "url": "https://cdn.discordapp.com/attachments/2/6/clip.mp4",
                 "proxy_url": "https://media.discordapp.net/attachments/2/6/clip.mp4",
                 "content_type": "video/mp4"},
                {"id": "7", "filename": "notes.pdf", "size": 9000,
                 "url": "https://cdn.discordapp.com/attachments/2/7/notes.pdf",
                 "proxy_url": "https://media.discordapp.net/attachments/2/7/notes.pdf",
                 "content_type": "application/pdf"},
            ],
        }))
        .unwrap();
        let [still, video, file] = &message.attachments[..] else {
            panic!("three attachments")
        };
        // What moves, as now, either way.
        assert_eq!(offered_with(still, true, false), Some(None));
        // Off: neither a still picture nor a video.
        assert_eq!(offered_with(still, false, false), None);
        assert_eq!(offered_with(video, false, false), None);
        // On: both, the still picture by its id.
        assert_eq!(
            offered_with(still, false, true),
            Some(Some(STILL.to_owned()))
        );
        assert_eq!(offered_with(video, false, true), Some(None));
        assert_eq!(
            offered_with(file, false, true),
            None,
            "nothing to show for a file"
        );
    }
}
