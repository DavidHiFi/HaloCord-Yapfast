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
