//! HaloCord's themes and colour roles. The themes join Yapfast's own in
//! `theme::BUILT_IN`; the roles are the colours HaloCord added to every theme
//! (a focus ring, the hover gradient's ends, the frosted fill for menus and
//! pop-ups with its solid fallback, the dim behind a dialog, a shadow, the
//! success, warning and info tones, a code block's ground and the loading
//! shimmer's low end). A theme that leaves a role clear gets it worked out
//! from its own colours ([`roles`]); `docs/halocord.md` lists the hooks.

use crate::theme::{Colour, MOCHACORD, Theme};

const fn rgb(hex: u32) -> Colour {
    Colour::rgb(hex)
}

const fn rgba(hex: u32) -> Colour {
    Colour::rgba(hex)
}

/// `colour` at `alpha`.
const fn alpha(colour: Colour, alpha: u32) -> Colour {
    Colour::from_argb((alpha << 24) | colour.rgb_part())
}

/// A role left clear, for [`roles`] to work out.
const CLEAR: Colour = Colour::from_argb(0);

const fn clear(colour: Colour) -> bool {
    colour.argb() == 0
}

/// Each role, worked out from the theme's own colours where the theme left
/// it clear.
pub const fn roles(mut theme: Theme) -> Theme {
    let dark = theme.dark;
    if clear(theme.focus) {
        theme.focus = theme.accent;
    }
    if clear(theme.edge_from) {
        theme.edge_from = theme.link;
    }
    if clear(theme.edge_to) {
        theme.edge_to = theme.accent;
    }
    if clear(theme.frost) {
        theme.frost = alpha(theme.sidebar, if dark { 0x99 } else { 0xb3 });
    }
    if clear(theme.frost_solid) {
        theme.frost_solid = alpha(theme.sidebar, 0xff);
    }
    if clear(theme.dim) {
        theme.dim = rgba(if dark { 0x00000099 } else { 0x00000066 });
    }
    if clear(theme.shadow) {
        theme.shadow = rgba(if dark { 0x00000073 } else { 0x0000002e });
    }
    if clear(theme.success) {
        theme.success = theme.live;
    }
    if clear(theme.warning) {
        theme.warning = theme.loud;
    }
    if clear(theme.info) {
        theme.info = theme.link;
    }
    if clear(theme.code) {
        theme.code = theme.panel;
    }
    if clear(theme.shimmer_low) {
        theme.shimmer_low = alpha(theme.shimmer, theme.shimmer.alpha() as u32 / 2);
    }
    theme
}

/// Every role clear: the end of a theme written before the roles, for
/// [`roles`] to fill.
pub const UNSET: Theme = Theme {
    dark: true,
    rail: CLEAR,
    sidebar: CLEAR,
    panel: CLEAR,
    chat: CLEAR,
    stage: CLEAR,
    raised: CLEAR,
    field: CLEAR,
    sunken: CLEAR,
    hover: CLEAR,
    active: CLEAR,
    row_hover: CLEAR,
    line: CLEAR,
    edge: CLEAR,
    outline: CLEAR,
    outline_chat: CLEAR,
    outline_field: CLEAR,
    text: CLEAR,
    secondary: CLEAR,
    muted: CLEAR,
    faint: CLEAR,
    accent: CLEAR,
    accent_hover: CLEAR,
    on_accent: CLEAR,
    accent_soft: CLEAR,
    danger: CLEAR,
    danger_hover: CLEAR,
    on_danger: CLEAR,
    live: CLEAR,
    loud: CLEAR,
    link: CLEAR,
    button_default: CLEAR,
    button_default_hover: CLEAR,
    overlay: CLEAR,
    shimmer: CLEAR,
    reply_spine: CLEAR,
    tooltip: CLEAR,
    avatar: CLEAR,
    focus: CLEAR,
    edge_from: CLEAR,
    edge_to: CLEAR,
    frost: CLEAR,
    frost_solid: CLEAR,
    dim: CLEAR,
    shadow: CLEAR,
    success: CLEAR,
    warning: CLEAR,
    info: CLEAR,
    code: CLEAR,
    shimmer_low: CLEAR,
};

/// Every role clear on `theme`, so [`roles`] works them out again from its
/// colours.
pub const fn without_roles(mut theme: Theme) -> Theme {
    theme.focus = CLEAR;
    theme.edge_from = CLEAR;
    theme.edge_to = CLEAR;
    theme.frost = CLEAR;
    theme.frost_solid = CLEAR;
    theme.dim = CLEAR;
    theme.shadow = CLEAR;
    theme.success = CLEAR;
    theme.warning = CLEAR;
    theme.info = CLEAR;
    theme.code = CLEAR;
    theme.shimmer_low = CLEAR;
    theme
}

/// The roles MochaCord sets itself: the rice's frosted fill (Mocha mantle at
/// 55%, the opaque crust behind it), blue to mauve edges and Mocha's tones.
const MOCHA_ROLES: Theme = Theme {
    focus: rgb(0xb4befe),
    edge_from: rgb(0x89b4fa),
    edge_to: rgb(0xcba6f7),
    frost: rgba(0x1818258c),
    frost_solid: rgb(0x1e1e2e),
    dim: rgba(0x11111b99),
    shadow: rgba(0x11111b80),
    success: rgb(0xa6e3a1),
    warning: rgb(0xf9e2af),
    info: rgb(0x89dceb),
    code: rgb(0x11111b),
    shimmer_low: rgba(0xffffff08),
    ..MOCHACORD
};

/// MochaCord with Mocha blue for the accent.
pub const MOCHACORD_BLUE: Theme = Theme {
    accent: rgb(0x89b4fa),
    accent_hover: rgb(0x74c7ec),
    accent_soft: rgba(0x89b4fa26),
    link: rgb(0xb4befe),
    focus: rgb(0x89b4fa),
    ..MOCHA_ROLES
};

/// MochaCord with HaloCord's green (Mocha green) for the accent.
pub const MOCHACORD_GREEN: Theme = Theme {
    accent: rgb(0xa6e3a1),
    accent_hover: rgb(0x94e2d5),
    accent_soft: rgba(0xa6e3a126),
    focus: rgb(0xa6e3a1),
    edge_from: rgb(0x94e2d5),
    edge_to: rgb(0xa6e3a1),
    ..MOCHA_ROLES
};

/// MochaCord with Mocha pink for the accent.
pub const MOCHACORD_PINK: Theme = Theme {
    accent: rgb(0xf5c2e7),
    accent_hover: rgb(0xf2cdcd),
    accent_soft: rgba(0xf5c2e726),
    focus: rgb(0xf5c2e7),
    edge_from: rgb(0xcba6f7),
    edge_to: rgb(0xf5c2e7),
    ..MOCHA_ROLES
};

/// MochaCord itself, with its roles.
pub const MOCHACORD_ROLES: Theme = MOCHA_ROLES;

/// Catppuccin Frappé, with mauve for the accent.
pub const CATPPUCCIN_FRAPPE: Theme = Theme {
    dark: true,
    rail: rgb(0x232634),
    sidebar: rgb(0x292c3c),
    panel: rgb(0x232634),
    chat: rgb(0x303446),
    stage: rgb(0x303446),
    raised: rgb(0x414559),
    field: rgb(0x414559),
    sunken: rgb(0x232634),
    hover: rgb(0x3a3e51),
    active: rgb(0x51576d),
    row_hover: rgb(0x2c3041),
    line: rgb(0x414559),
    edge: rgb(0x232634),
    outline: rgb(0x414559),
    outline_chat: rgb(0x414559),
    outline_field: rgb(0x414559),
    text: rgb(0xc6d0f5),
    secondary: rgb(0xb5bfe2),
    muted: rgb(0x949cbb),
    faint: rgb(0x737994),
    accent: rgb(0xca9ee6),
    accent_hover: rgb(0xbabbf1),
    on_accent: rgb(0x232634),
    accent_soft: rgba(0xca9ee626),
    danger: rgb(0xe78284),
    danger_hover: rgb(0xea999c),
    on_danger: rgb(0x232634),
    live: rgb(0xa6d189),
    loud: rgb(0xef9f76),
    link: rgb(0x8caaee),
    button_default: rgb(0x51576d),
    button_default_hover: rgb(0x626880),
    overlay: rgb(0xffffff),
    shimmer: rgba(0xffffff10),
    reply_spine: rgb(0x626880),
    tooltip: rgb(0x232634),
    avatar: rgb(0x737994),
    warning: rgb(0xe5c890),
    info: rgb(0x99d1db),
    ..UNSET
};

/// Catppuccin Macchiato, with mauve for the accent.
pub const CATPPUCCIN_MACCHIATO: Theme = Theme {
    dark: true,
    rail: rgb(0x181926),
    sidebar: rgb(0x1e2030),
    panel: rgb(0x181926),
    chat: rgb(0x24273a),
    stage: rgb(0x24273a),
    raised: rgb(0x363a4f),
    field: rgb(0x363a4f),
    sunken: rgb(0x181926),
    hover: rgb(0x2e3247),
    active: rgb(0x494d64),
    row_hover: rgb(0x212435),
    line: rgb(0x363a4f),
    edge: rgb(0x181926),
    outline: rgb(0x363a4f),
    outline_chat: rgb(0x363a4f),
    outline_field: rgb(0x363a4f),
    text: rgb(0xcad3f5),
    secondary: rgb(0xb8c0e0),
    muted: rgb(0x939ab7),
    faint: rgb(0x6e738d),
    accent: rgb(0xc6a0f6),
    accent_hover: rgb(0xb7bdf8),
    on_accent: rgb(0x181926),
    accent_soft: rgba(0xc6a0f626),
    danger: rgb(0xed8796),
    danger_hover: rgb(0xee99a0),
    on_danger: rgb(0x181926),
    live: rgb(0xa6da95),
    loud: rgb(0xf5a97f),
    link: rgb(0x8aadf4),
    button_default: rgb(0x494d64),
    button_default_hover: rgb(0x5b6078),
    overlay: rgb(0xffffff),
    shimmer: rgba(0xffffff10),
    reply_spine: rgb(0x5b6078),
    tooltip: rgb(0x181926),
    avatar: rgb(0x6e738d),
    warning: rgb(0xeed49f),
    info: rgb(0x91d7e3),
    ..UNSET
};

/// Rosé Pine's main dark variant, with iris for the accent.
pub const ROSE_PINE: Theme = Theme {
    dark: true,
    rail: rgb(0x16141f),
    sidebar: rgb(0x1f1d2e),
    panel: rgb(0x16141f),
    chat: rgb(0x191724),
    stage: rgb(0x191724),
    raised: rgb(0x26233a),
    field: rgb(0x26233a),
    sunken: rgb(0x16141f),
    hover: rgb(0x21202e),
    active: rgb(0x403d52),
    row_hover: rgb(0x1c1a29),
    line: rgb(0x26233a),
    edge: rgb(0x16141f),
    outline: rgb(0x26233a),
    outline_chat: rgb(0x26233a),
    outline_field: rgb(0x26233a),
    text: rgb(0xe0def4),
    secondary: rgb(0xc5c2dc),
    muted: rgb(0x908caa),
    faint: rgb(0x6e6a86),
    accent: rgb(0xc4a7e7),
    accent_hover: rgb(0xebbcba),
    on_accent: rgb(0x191724),
    accent_soft: rgba(0xc4a7e726),
    danger: rgb(0xeb6f92),
    danger_hover: rgb(0xf08aa8),
    on_danger: rgb(0x191724),
    live: rgb(0x9ccfd8),
    loud: rgb(0xf6c177),
    link: rgb(0x9ccfd8),
    button_default: rgb(0x403d52),
    button_default_hover: rgb(0x524f67),
    overlay: rgb(0xffffff),
    shimmer: rgba(0xffffff10),
    reply_spine: rgb(0x524f67),
    tooltip: rgb(0x16141f),
    avatar: rgb(0x6e6a86),
    success: rgb(0x31748f),
    info: rgb(0x9ccfd8),
    ..UNSET
};

/// Kanagawa's Wave style, with crystal blue for the accent.
pub const KANAGAWA: Theme = Theme {
    dark: true,
    rail: rgb(0x16161d),
    sidebar: rgb(0x1a1a22),
    panel: rgb(0x16161d),
    chat: rgb(0x1f1f28),
    stage: rgb(0x1f1f28),
    raised: rgb(0x2a2a37),
    field: rgb(0x2a2a37),
    sunken: rgb(0x16161d),
    hover: rgb(0x252530),
    active: rgb(0x363646),
    row_hover: rgb(0x1c1c25),
    line: rgb(0x2a2a37),
    edge: rgb(0x16161d),
    outline: rgb(0x2a2a37),
    outline_chat: rgb(0x2a2a37),
    outline_field: rgb(0x2a2a37),
    text: rgb(0xdcd7ba),
    secondary: rgb(0xc8c093),
    muted: rgb(0x938aa9),
    faint: rgb(0x727169),
    accent: rgb(0x7e9cd8),
    accent_hover: rgb(0x7fb4ca),
    on_accent: rgb(0x16161d),
    accent_soft: rgba(0x7e9cd826),
    danger: rgb(0xe46876),
    danger_hover: rgb(0xff5d62),
    on_danger: rgb(0x16161d),
    live: rgb(0x98bb6c),
    loud: rgb(0xffa066),
    link: rgb(0x7fb4ca),
    button_default: rgb(0x363646),
    button_default_hover: rgb(0x54546d),
    overlay: rgb(0xffffff),
    shimmer: rgba(0xffffff10),
    reply_spine: rgb(0x54546d),
    tooltip: rgb(0x16161d),
    avatar: rgb(0x727169),
    edge_from: rgb(0x7fb4ca),
    edge_to: rgb(0x957fb8),
    warning: rgb(0xe6c384),
    ..UNSET
};

/// Everforest's dark palette at medium contrast, with green for the accent.
pub const EVERFOREST: Theme = Theme {
    dark: true,
    rail: rgb(0x232a2e),
    sidebar: rgb(0x272f33),
    panel: rgb(0x232a2e),
    chat: rgb(0x2d353b),
    stage: rgb(0x2d353b),
    raised: rgb(0x343f44),
    field: rgb(0x343f44),
    sunken: rgb(0x232a2e),
    hover: rgb(0x313a3f),
    active: rgb(0x3d484d),
    row_hover: rgb(0x293136),
    line: rgb(0x3d484d),
    edge: rgb(0x1e2326),
    outline: rgb(0x3d484d),
    outline_chat: rgb(0x3d484d),
    outline_field: rgb(0x3d484d),
    text: rgb(0xd3c6aa),
    secondary: rgb(0xc5b99d),
    muted: rgb(0x9da9a0),
    faint: rgb(0x7a8478),
    accent: rgb(0xa7c080),
    accent_hover: rgb(0x83c092),
    on_accent: rgb(0x232a2e),
    accent_soft: rgba(0xa7c08026),
    danger: rgb(0xe67e80),
    danger_hover: rgb(0xe98f90),
    on_danger: rgb(0x232a2e),
    live: rgb(0xa7c080),
    loud: rgb(0xe69875),
    link: rgb(0x7fbbb3),
    button_default: rgb(0x475258),
    button_default_hover: rgb(0x4f585e),
    overlay: rgb(0xffffff),
    shimmer: rgba(0xffffff10),
    reply_spine: rgb(0x4f585e),
    tooltip: rgb(0x1e2326),
    avatar: rgb(0x7a8478),
    warning: rgb(0xdbbc7f),
    info: rgb(0x7fbbb3),
    ..UNSET
};

/// Atom's One Dark, with blue for the accent.
pub const ONE_DARK: Theme = Theme {
    dark: true,
    rail: rgb(0x1e2227),
    sidebar: rgb(0x21252b),
    panel: rgb(0x1e2227),
    chat: rgb(0x282c34),
    stage: rgb(0x282c34),
    raised: rgb(0x2c313a),
    field: rgb(0x2c313a),
    sunken: rgb(0x1e2227),
    hover: rgb(0x2c313c),
    active: rgb(0x3e4451),
    row_hover: rgb(0x252930),
    line: rgb(0x3e4451),
    edge: rgb(0x181a1f),
    outline: rgb(0x3e4451),
    outline_chat: rgb(0x3e4451),
    outline_field: rgb(0x3e4451),
    text: rgb(0xd7dae0),
    secondary: rgb(0xabb2bf),
    muted: rgb(0x8b929e),
    faint: rgb(0x5c6370),
    accent: rgb(0x61afef),
    accent_hover: rgb(0x7dbdf2),
    on_accent: rgb(0x1e2227),
    accent_soft: rgba(0x61afef26),
    danger: rgb(0xe06c75),
    danger_hover: rgb(0xe5858c),
    on_danger: rgb(0x1e2227),
    live: rgb(0x98c379),
    loud: rgb(0xd19a66),
    link: rgb(0x56b6c2),
    button_default: rgb(0x3e4451),
    button_default_hover: rgb(0x4b5263),
    overlay: rgb(0xffffff),
    shimmer: rgba(0xffffff10),
    reply_spine: rgb(0x4b5263),
    tooltip: rgb(0x181a1f),
    avatar: rgb(0x5c6370),
    edge_to: rgb(0xc678dd),
    warning: rgb(0xe5c07b),
    ..UNSET
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{BUILT_IN, find};

    #[test]
    fn every_built_in_theme_has_every_role() {
        for built_in in &BUILT_IN {
            for (index, name) in Theme::NAMES.iter().enumerate() {
                let colour = built_in.theme.colour(index).unwrap();
                assert!(!clear(colour), "{} has no {name}", built_in.id);
            }
        }
    }

    #[test]
    fn a_theme_without_roles_works_them_out_from_its_colours() {
        let yap = find("yap").unwrap().theme;
        assert_eq!(yap.focus, yap.accent);
        assert_eq!(yap.success, yap.live);
        assert_eq!(yap.code, yap.panel);
        assert_eq!(yap.frost.rgb_part(), yap.sidebar.rgb_part());
        assert!(yap.frost.alpha() < 0xff, "frosted, not solid");
        assert_eq!(yap.frost_solid.alpha(), 0xff, "the fallback is solid");
        assert!(yap.shimmer_low.alpha() < yap.shimmer.alpha());
    }

    #[test]
    fn the_mochacord_accents_keep_mocha_and_change_the_accent() {
        let mocha = find("mochacord").unwrap().theme;
        for (id, accent) in [
            ("mochacord-blue", 0x89b4fa),
            ("mochacord-green", 0xa6e3a1),
            ("mochacord-pink", 0xf5c2e7),
        ] {
            let theme = find(id).unwrap().theme;
            assert_eq!(theme.accent, Colour::rgb(accent), "{id}");
            assert_eq!(theme.chat, mocha.chat, "{id}");
            assert_eq!(theme.frost, mocha.frost, "{id}");
        }
        // The rice's frosted glass: a light fill, never near solid.
        let frost = f32::from(mocha.frost.alpha()) / 255.0;
        assert!((0.40..=0.65).contains(&frost), "{frost}");
    }
}
