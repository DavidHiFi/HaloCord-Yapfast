//! Fix file extensions: a photo whose name ends in a JPEG extension Discord
//! doesn't show in chat, such as `.jfif`, goes up as `.jpg` so it shows.

use std::{
    borrow::Cow,
    sync::atomic::{AtomicBool, Ordering},
};

static ON: AtomicBool = AtomicBool::new(false);

/// The JPEG extensions Discord doesn't show as a picture.
const JPEGS: [&str; 6] = ["jfif", "jfi", "jif", "jpe", "pjpeg", "pjp"];

/// Turns the plugin on or off.
pub fn set(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

/// The name a file goes up under, fixed while the plugin is on.
pub fn name(name: &str) -> Cow<'_, str> {
    if ON.load(Ordering::Relaxed) {
        fixed(name)
    } else {
        Cow::Borrowed(name)
    }
}

fn fixed(name: &str) -> Cow<'_, str> {
    match name.rsplit_once('.') {
        Some((stem, extension))
            if !stem.is_empty() && JPEGS.contains(&extension.to_ascii_lowercase().as_str()) =>
        {
            Cow::Owned(format!("{stem}.jpg"))
        }
        _ => Cow::Borrowed(name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jpeg_names_discord_does_not_show_go_up_as_jpg() {
        assert_eq!(fixed("cat.jfif"), "cat.jpg");
        assert_eq!(fixed("holiday.photo.JPE"), "holiday.photo.jpg");
        assert_eq!(fixed("scan.pjpeg"), "scan.jpg");
        for kept in ["cat.png", "cat.jpg", "notes.txt", ".jfif", "jfif"] {
            assert_eq!(fixed(kept), kept);
        }
        assert_eq!(name("cat.jfif"), "cat.jfif", "nothing while off");
    }
}
