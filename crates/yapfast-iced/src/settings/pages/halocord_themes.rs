//! HaloCord's Themes bits: the name of the theme in use, for HaloCord's
//! page. The Themes page itself is upstream's (`theme_page.rs`), with the
//! MochaCord tile's cog added in `appearance.rs`. See docs/themes.md.

use yapfast_core::theme as palette;

use crate::settings::Page;

impl<'a> Page<'a> {
    /// The name of the theme in use.
    pub(in crate::settings) fn theme_name(self) -> String {
        let id = self.cx.themes.chosen_id();
        palette::BUILT_IN
            .iter()
            .find(|built_in| built_in.id == id)
            .map(|built_in| built_in.name.to_owned())
            .or_else(|| {
                self.cx
                    .themes
                    .custom
                    .iter()
                    .find(|custom| custom.id == id)
                    .map(|custom| custom.name.clone())
            })
            .unwrap_or_else(|| id.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::Space;
    use iced_test::{Simulator, selector::Candidate};

    /// A long name in a narrow tile with a pencil wraps inside the tile, short
    /// of the pencil, and its row grows to hold both lines.
    #[test]
    fn a_long_theme_name_wraps_inside_its_tile() {
        let name = "Catppuccin Macchiato";
        let room = super::name_room(150.0, false, true);
        let height = super::name_height(name, false, room);
        assert!(height > super::LABEL, "two lines, the row grows: {height}");
        let built_in = &yapfast_core::theme::BUILT_IN[0];
        let theme = crate::themes::theme(built_in.id, &built_in.theme);
        let inside = 150.0 - 16.0;
        let label = super::tile_label(&theme, Space::new().into(), name, false, true, height);
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(inside, 200.0),
            label,
        );
        let mut words = None;
        let _ = ui.find(|candidate: Candidate<'_>| {
            if let Candidate::Text {
                content, bounds, ..
            } = candidate
                && content == name
            {
                words = Some(bounds);
            }
            None::<()>
        });
        let words = words.expect("the name");
        assert!(
            words.x + words.width <= inside - super::BUTTON,
            "short of the pencil and inside the tile: {words:?}"
        );
        assert!(words.height <= height + 0.5, "inside its row: {words:?}");
    }

    #[test]
    fn a_short_theme_name_keeps_one_line() {
        let room = super::name_room(150.0, true, false);
        assert!(super::name_height("Nord", true, room) <= super::LABEL);
    }
}
