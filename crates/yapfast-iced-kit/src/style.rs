//! The small style helpers every view uses. The colours are the theme's
//! ([`crate::theme::Theme`]); the few drawn the same in every theme are
//! here.

use iced::{Background, Border, Color, border, widget::container};

use crate::theme::Theme;

/// What is drawn in the same colour whatever the theme:
/// Discord's grey for a member list's headings, its wash for a member row
/// under the pointer (`#97979f1f`) or whose card is open (`#97979f33`),
/// the chips of server tags, and the crown and the APP tag.
pub const HEADING: u32 = 0x81828a;
pub const MEMBER_HOVER: Color = grey(0x1f);
pub const MEMBER_OPEN: Color = grey(0x33);
pub const CHIP: Color = grey(0x33);
pub const CROWN: u32 = 0xf0b232;
pub const BLURPLE: u32 = 0x5865f2;
/// Black at 70%: the ground of marks and notes over pictures.
pub const SMOKE: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0xb3 as f32 / 255.0,
};

const fn grey(alpha: u8) -> Color {
    Color {
        r: 0x97 as f32 / 255.0,
        g: 0x97 as f32 / 255.0,
        b: 0x9f as f32 / 255.0,
        a: alpha as f32 / 255.0,
    }
}

/// A colour from 0xRRGGBB.
pub fn hex(rgb: u32) -> Color {
    Color::from_rgb8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// `color` with its alpha scaled by `alpha`.
pub fn faded(color: Color, alpha: f32) -> Color {
    Color {
        a: color.a * alpha,
        ..color
    }
}

/// `over` laid on the opaque colour `under`.
pub fn blend(under: Color, over: Color) -> Color {
    let mix = |a: f32, b: f32| a * (1.0 - over.a) + b * over.a;
    Color::from_rgb(
        mix(under.r, over.r),
        mix(under.g, over.g),
        mix(under.b, over.b),
    )
}

/// A container filled with one colour.
pub fn fill(color: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(color)),
        ..Default::default()
    }
}

/// How much HaloCord's corner setting scales the rounded corners, in
/// percent. 100 leaves them as drawn.
static CORNER_SCALE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(100);

/// Sets the corner scale, in percent, up to 200.
pub fn set_corner_scale(percent: u32) {
    CORNER_SCALE.store(percent.min(200), std::sync::atomic::Ordering::Relaxed);
}

/// The corners with the scale applied. Corners over 14 (pills, and circles
/// such as avatars) keep their size, since the scale is for the cards,
/// menus and buttons.
fn scaled(radius: border::Radius) -> border::Radius {
    let percent = CORNER_SCALE.load(std::sync::atomic::Ordering::Relaxed);
    if percent == 100 {
        return radius;
    }
    let grow = |corner: f32| {
        if corner > 14.0 {
            corner
        } else {
            corner * percent as f32 / 100.0
        }
    };
    border::Radius {
        top_left: grow(radius.top_left),
        top_right: grow(radius.top_right),
        bottom_right: grow(radius.bottom_right),
        bottom_left: grow(radius.bottom_left),
    }
}

/// The look every theme but Bonfire draws with: 2 px edges, a hover edge
/// halfway between the theme's two edge colours (blue and mauve on
/// MochaCord; iced draws a border in one colour, so the blend stands in
/// for a gradient), the second edge colour while pressed, and floating
/// surfaces on the theme's solid frost. Bonfire keeps Discord's measured
/// 1 px look.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rice {
    pub hover: Color,
    pub press: Color,
    pub rest: Color,
    pub ground: Color,
    pub shadow: Color,
    pub dim: Color,
}

thread_local! {
    /// The shown theme's [`Rice`], set where the window takes a theme.
    /// Per thread, since the window's update and draw share one and each
    /// test has its own.
    static RICE: std::cell::Cell<Option<Rice>> = const { std::cell::Cell::new(None) };
}

/// Colours the person picked for MochaCord's edges: the first and second
/// ends of the blue to mauve run, and the edge at rest. None keeps the
/// theme's own.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Accents {
    pub from: Option<Color>,
    pub to: Option<Color>,
    pub rest: Option<Color>,
}

static ACCENTS: std::sync::RwLock<Accents> = std::sync::RwLock::new(Accents {
    from: None,
    to: None,
    rest: None,
});

/// Draws MochaCord's edges in these colours from the next frame on.
pub fn set_accents(accents: Accents) {
    if let Ok(mut now) = ACCENTS.write() {
        *now = accents;
    }
}

fn accents() -> Accents {
    ACCENTS.read().map(|now| *now).unwrap_or_default()
}

/// `theme`'s [`Rice`]: none on Bonfire or a theme made from it.
pub fn rice_of(theme: &Theme) -> Option<Rice> {
    let mix = |a: f32, b: f32| (a + b) / 2.0;
    let picked = if mocha(theme) {
        accents()
    } else {
        Accents::default()
    };
    let from = picked.from.unwrap_or(theme.edge_from);
    let to = picked.to.unwrap_or(theme.edge_to);
    (!theme.bonfire).then(|| Rice {
        hover: Color::from_rgba(
            mix(from.r, to.r),
            mix(from.g, to.g),
            mix(from.b, to.b),
            mix(from.a, to.a),
        ),
        press: to,
        rest: picked.rest.unwrap_or(theme.edge),
        ground: theme.frost_solid,
        shadow: theme.shadow,
        dim: theme.dim,
    })
}

/// Draws with `theme`'s look from now on.
pub fn set_rice(theme: &Theme) {
    RICE.with(|rice| rice.set(rice_of(theme)));
}

/// The look the shown theme draws with, if it isn't Bonfire.
pub fn rice() -> Option<Rice> {
    RICE.with(std::cell::Cell::get)
}

/// An edge's width: 2 px with the rice look, else 1.
pub fn edge_width() -> f32 {
    if rice().is_some() { 2.0 } else { 1.0 }
}

/// A hover or pressed frame in the rice look, `radius` round; none on
/// Bonfire or at rest.
pub fn lit_edge(lit: bool, pressed: bool, radius: impl Into<border::Radius>) -> Border {
    match rice() {
        Some(rice) if lit || pressed => Border {
            color: if pressed { rice.press } else { rice.hover },
            width: 2.0,
            radius: scaled(radius.into()),
        },
        _ => round(radius),
    }
}

/// Whether `theme` is MochaCord, the theme the rice recipe's control fills
/// are drawn for.
pub fn mocha(theme: &Theme) -> bool {
    theme.id == "mochacord"
}

/// The rice recipe's look for something that can be picked (a tab, a
/// segment, a chip, a list row, an option): `ground` is its fill at rest
/// (none for a row on its dock), the theme's raised ground under the
/// pointer, that ground at three quarters and a lit 2 px edge once chosen,
/// and a 2 px edge at rest when `edged`.
#[derive(Clone, Copy, Debug)]
pub struct Pick {
    raised: Color,
    edge: Color,
    ground: Option<Color>,
    edged: bool,
    radius: f32,
}

impl Pick {
    pub fn new(theme: &Theme, ground: Option<Color>, edged: bool, radius: f32) -> Self {
        Self {
            raised: theme.raised,
            edge: rice().map_or(theme.edge, |rice| rice.rest),
            ground,
            edged,
            radius,
        }
    }

    /// Its fill and edge in the state given.
    pub fn face(self, lit: bool, pressed: bool, chosen: bool) -> (Option<Background>, Border) {
        let background = if chosen {
            Some(Color {
                a: 0.75,
                ..self.raised
            })
        } else if lit || pressed {
            Some(self.raised)
        } else {
            self.ground
        };
        let border = if chosen || lit || pressed {
            lit_edge(true, pressed, self.radius)
        } else if self.edged {
            edge(rice().map_or(self.edge, |rice| rice.rest), self.radius)
        } else {
            round(self.radius)
        };
        (background.map(Background::Color), border)
    }
}

/// The soft shadow under a floating surface in the rice look.
pub fn soft_shadow(rice: Rice) -> iced::Shadow {
    iced::Shadow {
        color: rice.shadow,
        offset: iced::Vector::new(0.0, 6.0),
        blur_radius: 20.0,
    }
}

/// A floating surface in the rice look: a menu, a tooltip, a dialog. The
/// solid frost (opaque, as Yapfast has no blur behind a pop-up yet) in a
/// 2 px `edge`, `radius` round, over the soft shadow.
pub fn floating(rice: Rice, edge: Color, radius: f32) -> container::Style {
    container::Style {
        background: Some(rice.ground.into()),
        border: Border {
            color: edge,
            width: 2.0,
            radius: scaled(radius.into()),
        },
        shadow: soft_shadow(rice),
        ..Default::default()
    }
}

/// A container filled with `color`, its corners rounded by `radius`.
pub fn rounded(color: Color, radius: impl Into<border::Radius>) -> container::Style {
    container::Style {
        background: Some(color.into()),
        border: Border::default().rounded(scaled(radius.into())),
        ..Default::default()
    }
}

/// An outline in `color` ([`edge_width`] wide), its corners rounded by
/// `radius`.
pub fn edge(color: Color, radius: impl Into<border::Radius>) -> Border {
    Border {
        color,
        width: edge_width(),
        radius: scaled(radius.into()),
    }
}

/// No outline, its corners rounded by `radius`: the border of a button's,
/// a field's or a quad's style.
pub fn round(radius: impl Into<border::Radius>) -> Border {
    Border::default().rounded(scaled(radius.into()))
}

/// A container filled with `ground` in a 1 px `outline`, its corners
/// rounded by `radius`: a card, a bar, a field.
pub fn outlined(
    ground: Color,
    outline: Color,
    radius: impl Into<border::Radius>,
) -> container::Style {
    let radius = radius.into();
    rounded(ground, radius).border(edge(outline, radius))
}

/// Each corner its own radius: top left, top right, bottom right, bottom left.
pub fn corners([top_left, top_right, bottom_right, bottom_left]: [f32; 4]) -> border::Radius {
    border::Radius {
        top_left,
        top_right,
        bottom_right,
        bottom_left,
    }
}

/// A card's shadow, over the window or a dimmed one: the sign-in card, a
/// dialog.
pub fn card_shadow() -> iced::Shadow {
    iced::Shadow {
        color: Color {
            a: 0.4,
            ..Color::BLACK
        },
        offset: iced::Vector::new(0.0, 12.0),
        blur_radius: 32.0,
    }
}

/// The shadow of a window over a card: the Display Name Style's, over the
/// profile editor.
pub fn window_shadow() -> iced::Shadow {
    iced::Shadow {
        color: Color {
            a: 0.7,
            ..Color::BLACK
        },
        offset: iced::Vector::new(0.0, 16.0),
        blur_radius: 48.0,
    }
}

/// The raised panels' shadow: menus, the picker, suggestions; and the
/// settings' cards.
pub fn raised_shadow() -> iced::Shadow {
    iced::Shadow {
        color: Color {
            a: 0.5,
            ..Color::BLACK
        },
        offset: iced::Vector::new(0.0, 8.0),
        blur_radius: 24.0,
    }
}

/// A raised panel: `ground` in an `edge` with `radius` corners, over
/// the raised shadow (the soft one in the rice look), as menus,
/// suggestions, the picker and cards are.
pub fn panel(ground: Color, edge: Color, radius: f32) -> container::Style {
    container::Style {
        background: Some(ground.into()),
        border: self::edge(edge, radius),
        shadow: rice().map_or_else(raised_shadow, soft_shadow),
        ..Default::default()
    }
}

/// [`panel`] through an installed theme's named surface tokens.
pub fn surface_panel(
    theme: &Theme,
    surface: &str,
    ground: Color,
    edge: Color,
    radius: f32,
) -> container::Style {
    container::Style {
        background: Some(ground.into()),
        border: theme.surface_border(surface, radius, 1.0, edge),
        shadow: theme.surface_shadow(surface, raised_shadow()),
        ..Default::default()
    }
}

/// The shadow of the pill floating over the timeline's end: Jump to present.
pub fn pill_shadow() -> iced::Shadow {
    iced::Shadow {
        color: Color::from_rgba8(0, 0, 0, 0x40 as f32 / 255.0),
        offset: iced::Vector::new(0.0, 2.0),
        blur_radius: 8.0,
    }
}

/// The shadow of a message's hover bar, over its row's top edge.
pub fn hover_bar_shadow() -> iced::Shadow {
    iced::Shadow {
        color: Color::from_rgba8(0, 0, 0, 0x59 as f32 / 255.0),
        offset: iced::Vector::new(0.0, 2.0),
        blur_radius: 10.0,
    }
}

/// A list's scroll bar's inks: faint on a dark theme, darker on a light
/// one, and plainer under the pointer.
pub fn scrollbar(theme: &Theme) -> crate::widgets::Scroller {
    // A theme's scrollbar paint, when it's one colour, inks the bar.
    if let Some(Background::Color(ink)) = theme.package_paint("scrollbar") {
        return crate::widgets::Scroller {
            rest: ink,
            pointed: ink,
        };
    }
    if theme.dark {
        crate::widgets::Scroller::DARK
    } else {
        crate::widgets::Scroller::LIGHT
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use iced::{Background, Color};

    use super::*;
    use crate::theme::Metrics;

    #[test]
    fn bonfire_keeps_its_look_and_other_themes_get_the_rice() {
        let mut bonfire = Theme::new("bonfire", true);
        bonfire.bonfire = true;
        set_rice(&bonfire);
        assert_eq!(rice(), None);
        assert_eq!(edge(Color::WHITE, 4).width, 1.0);
        assert_eq!(lit_edge(true, false, 4).width, 0.0);
        let mut mocha = Theme::new("mochacord", true);
        mocha.edge_from = hex(0x89b4fa);
        mocha.edge_to = hex(0xcba6f7);
        set_rice(&mocha);
        let rice = rice().expect("the rice look");
        assert_eq!(rice.press, hex(0xcba6f7));
        let [r, g, b, _] = rice.hover.into_rgba8();
        assert_eq!((r, g), (0xaa, 0xad), "halfway from blue to mauve");
        assert!((0xf8..=0xf9).contains(&b));
        assert_eq!(edge(Color::WHITE, 4).width, 2.0);
        assert_eq!(lit_edge(false, false, 4).width, 0.0, "nothing at rest");
        assert_eq!(lit_edge(true, false, 4).color, rice.hover);
        assert_eq!(lit_edge(true, true, 4).color, rice.press);
        assert_eq!(floating(rice, Color::WHITE, 8.0).border.width, 2.0);
        set_rice(&bonfire);
    }

    #[test]
    fn a_themes_scrollbar_paint_inks_the_bar() {
        let scroller = Color::from_rgb8(0x12, 0x34, 0x56);
        let theme = Theme::new("kit-scrollbar-owner", true)
            .worked_out()
            .with_metrics(std::sync::Arc::new(Metrics {
                paints: HashMap::from([("scrollbar".to_owned(), Background::Color(scroller))]),
                ..Metrics::default()
            }));
        let bar = scrollbar(&theme);
        assert_eq!((bar.rest, bar.pointed), (scroller, scroller));
        let stock = scrollbar(&Theme::new("kit-scrollbar-stock", true).worked_out());
        assert_eq!(stock.rest, crate::widgets::Scroller::DARK.rest);
    }
}
