//! HaloCord's Themes page: the themes as small windows drawn in their own
//! colours (a click shows one at once and keeps it), Load from file and New
//! theme, a pencil on each of the person's own opening the editor, and a cog
//! on the MochaCord tile opening its options. The editor's events are in
//! `appearance.rs`; its changes show at once (Save keeps them among the
//! person's own themes, Cancel puts the theme from before back). See
//! docs/themes.md.

use iced::{
    Alignment, Border, Color, Element, Length,
    widget::{Space, button, column, container, row, stack, text},
};
use yapfast_core::theme as palette;

use super::{HaloEvent, halo_message, mocha_found};
use crate::halocord::theme_files::Note;
use crate::{
    app::Message,
    controls::{Kind, action, lights, placed},
    fonts::{BOLD, MEDIUM},
    form::{self, switch},
    pictures::Icon,
    settings::{
        Drop, Event as SettingsEvent, Page,
        appearance::{Editor, Event, theme_message},
    },
    themes::color,
};

/// A theme's tile: at least this wide, this tall, this far apart.
const TILE: f32 = 150.0;
const TILE_HEIGHT: f32 = 124.0;
const TILE_GAP: f32 = 12.0;
/// The editor's colours: two columns from this width, each row this tall.
const TWO_COLUMNS: f32 = 520.0;
const COLOUR_GAP: f32 = 24.0;
/// The name field.
const NAME: &str = "theme-name";

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

    /// The Themes page.
    pub(in crate::settings) fn themes_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        if let Some(editor) = &self.s.editor {
            return self.editor_view(editor, self.s.open == Some(Drop::Start));
        }
        form::page([
            form::title(theme, "Themes"),
            form::group([
                form::beside(
                    form::heading(theme, "THEME"),
                    row![
                        action(
                            theme,
                            "Load from file",
                            Kind::Plain,
                            Some(theme_message(Event::LoadFile))
                        )
                        .small(),
                        action(
                            theme,
                            "New theme",
                            Kind::Plain,
                            Some(theme_message(Event::New))
                        )
                        .small()
                        .icon(self.cx.look.icon(
                            Icon::Plus,
                            16,
                            crate::theme::rgb(theme.text)
                        )),
                    ]
                    .spacing(8),
                ),
                file_note(theme, self.s.theme_file),
                self.theme_tiles(),
            ]),
        ])
    }

    /// The themes as tiles, as many 150 px or wider across as fit.
    fn theme_tiles(self) -> Element<'a, Message> {
        let query = self.s.query.to_lowercase();
        iced::widget::responsive(move |size| {
            let columns = ((size.width + TILE_GAP) / (TILE + TILE_GAP))
                .floor()
                .max(1.0) as usize;
            let width = (size.width - (columns - 1) as f32 * TILE_GAP) / columns as f32;
            let cards: Vec<(String, String, bool, palette::Theme)> =
                palette::BUILT_IN
                    .iter()
                    .map(|built_in| {
                        (
                            built_in.id.to_owned(),
                            built_in.name.to_owned(),
                            false,
                            built_in.theme,
                        )
                    })
                    .chain(
                        self.cx.themes.custom.iter().map(|custom| {
                            (custom.id.clone(), custom.name.clone(), true, custom.theme)
                        }),
                    )
                    .filter(|(id, name, ..)| {
                        query.is_empty()
                            || "themes".contains(&query)
                            || name.to_lowercase().contains(&query)
                            || (id == crate::mochacord::THEME && mocha_found(&query))
                    })
                    .collect();
            let mut rows = column![].spacing(TILE_GAP);
            for chunk in cards.chunks(columns) {
                // A row is as tall as its longest name needs, so a name that
                // wraps grows its row and the row's tiles stay level.
                let label = chunk
                    .iter()
                    .map(|(id, name, custom, _)| {
                        let current = self.cx.themes.chosen_id() == id;
                        name_height(
                            name,
                            current,
                            name_room(width, current, self.has_button(id, *custom)),
                        )
                    })
                    .fold(LABEL, f32::max);
                let mut line = row![].spacing(TILE_GAP);
                for (id, name, custom, palette) in chunk {
                    line = line.push(self.theme_tile(id, name, *custom, palette, width, label));
                }
                rows = rows.push(line);
            }
            rows.into()
        })
        .height(Length::Shrink)
        .into()
    }

    /// Whether the tile for `id` carries a button over its name's right end:
    /// MochaCord's cog or the pencil on the person's own.
    fn has_button(self, id: &str, custom: bool) -> bool {
        custom || id == crate::mochacord::THEME
    }

    /// A theme to choose: a small window in its colours, its name under it
    /// (on as many lines as it needs, `label` tall), an accent ring and a
    /// tick on the one shown, and a pencil on the person's own.
    fn theme_tile(
        self,
        id: &str,
        name: &str,
        custom: bool,
        palette: &palette::Theme,
        width: f32,
        label_height: f32,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let current = self.cx.themes.chosen_id() == id;
        let preview = preview(palette, width - 16.0);
        let label = tile_label(
            theme,
            self.cx.look.glyph(Icon::Check, 16, theme.accent).into(),
            name,
            current,
            self.has_button(id, custom),
            label_height,
        );
        let (accent, faint, line, hover) = (theme.accent, theme.faint, theme.line, theme.hover);
        let face = button(column![preview, label].spacing(6).padding(8))
            .padding(0)
            .width(width)
            .height(TILE_HEIGHT - LABEL + label_height)
            .on_press(theme_message(Event::Choose(id.to_owned())))
            .style(move |_, status| {
                let lit = lights(status);
                button::Style {
                    background: lit.then(|| hover.into()),
                    border: Border {
                        color: if current {
                            accent
                        } else if lit {
                            faint
                        } else {
                            line
                        },
                        width: if current { 2.0 } else { 1.0 },
                        radius: 10.0.into(),
                    },
                    ..Default::default()
                }
            });
        if id == crate::mochacord::THEME {
            let cog = self
                .cx
                .look
                .icon_button(
                    Icon::Settings,
                    "MochaCord options",
                    halo_message(HaloEvent::Mocha(true)),
                )
                .size(24.0, 14);
            return stack![face, placed(cog, width - 8.0 - 24.0, 8.0 + 78.0 + 6.0)].into();
        }
        if !custom {
            return face.into();
        }
        let pencil = self
            .cx
            .look
            .icon_button(
                Icon::Pencil,
                "Edit",
                theme_message(Event::Edit(id.to_owned())),
            )
            .size(24.0, 14);
        stack![face, placed(pencil, width - 8.0 - 24.0, 8.0 + 78.0 + 6.0)].into()
    }

    /// The editor: the name with Save and Cancel, what a new theme starts
    /// from, Dark, each colour's field, and Delete for a saved one.
    fn editor_view(self, editor: &'a Editor, starts_open: bool) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let mut parts = vec![
            form::title(theme, "Themes"),
            form::group([
                form::heading(
                    theme,
                    if editor.editing.is_some() {
                        "EDIT THEME"
                    } else {
                        "NEW THEME"
                    },
                ),
                row![
                    form::field(theme, NAME, "Name", &editor.name)
                        .on_input(|name| theme_message(Event::Name(name)))
                        .on_submit(theme_message(Event::Save)),
                    action(
                        theme,
                        "Save",
                        Kind::Primary,
                        editor.valid().then(|| theme_message(Event::Save))
                    ),
                    action(
                        theme,
                        "Cancel",
                        Kind::Plain,
                        Some(theme_message(Event::Cancel))
                    ),
                    action(
                        theme,
                        "Save to file",
                        Kind::Plain,
                        editor.valid().then(|| theme_message(Event::SaveFile))
                    )
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
                file_note(theme, self.s.theme_file),
            ]),
        ];
        if editor.editing.is_none() {
            let names: Vec<String> = palette::starts(&self.cx.themes.custom)
                .map(|start| start.name.to_owned())
                .collect();
            parts.push(form::group([
                form::heading(theme, "START FROM"),
                form::dropdown(
                    names,
                    Some(editor.start),
                    starts_open,
                    Message::Settings(SettingsEvent::Drop(Drop::Start)),
                    |index| theme_message(Event::Start(index)),
                )
                .view(self.cx.look),
            ]));
        }
        parts.push(
            switch(
                "Dark",
                editor.theme.dark,
                Some(theme_message(Event::Dark(!editor.theme.dark))),
            )
            .description("Drop-down lists and scroll bars draw dark to match.")
            .view(self.cx.look),
        );
        let colours = iced::widget::responsive(move |size| {
            let columns = if size.width >= TWO_COLUMNS { 2 } else { 1 };
            let per = editor.fields.len().div_ceil(columns);
            let mut line = row![].spacing(COLOUR_GAP);
            for column_index in 0..columns {
                let mut cells = column![];
                for index in column_index * per..((column_index + 1) * per).min(editor.fields.len())
                {
                    let (name, hex, wrong) = &editor.fields[index];
                    let shown = editor.theme.colour(index).map_or(Color::TRANSPARENT, color);
                    cells = cells.push(form::colour_row(
                        theme,
                        name,
                        shown,
                        hex,
                        *wrong,
                        move |text| theme_message(Event::Colour(index, text)),
                    ));
                }
                line = line.push(container(cells).width(Length::FillPortion(1)));
            }
            line.into()
        })
        .height(Length::Shrink);
        parts.push(form::group([
            form::heading(theme, "COLOURS"),
            form::note(
                theme,
                "Each as #rgb, #rrggbb or #rrggbbaa. The window shows a change as you type it.",
            ),
            colours.into(),
        ]));
        if editor.editing.is_some() {
            parts.push(form::buttons([action(
                theme,
                if editor.confirming {
                    "Delete it for good"
                } else {
                    "Delete theme"
                },
                Kind::Danger,
                Some(theme_message(Event::Delete)),
            )
            .small()
            .icon(
                self.cx
                    .look
                    .icon(Icon::Trash, 16, crate::theme::rgb(theme.on_danger)),
            )
            .into()]));
        }
        form::page(parts)
    }
}

/// A tile's name: wrapped inside the tile, never past its edge, with the
/// tick after it on the one shown, and room kept at its right end for a
/// pencil or cog laid over it. A long name ("Catppuccin Macchiato" in a
/// wide font) ran out past the tile's border on one line.
fn tile_label<'a>(
    theme: &crate::theme::Theme,
    tick: Element<'a, Message>,
    name: &str,
    current: bool,
    button: bool,
    height: f32,
) -> Element<'a, Message> {
    let mut label = row![
        text(name.to_owned())
            .size(NAME_SIZE)
            .font(name_font(current))
            .color(if current { theme.text } else { theme.secondary })
            .wrapping(text::Wrapping::WordOrGlyph)
            .width(Length::Fill)
    ]
    .spacing(TICK_GAP)
    .align_y(Alignment::Center)
    .height(height);
    if current {
        label = label.push(tick);
    }
    container(label)
        .padding(iced::Padding {
            left: 2.0,
            right: if button { BUTTON + TICK_GAP } else { 2.0 },
            ..iced::Padding::ZERO
        })
        .into()
}

/// The name's text size, and the one-line height its row keeps at least.
const NAME_SIZE: f32 = 14.0;
const LABEL: f32 = 24.0;
/// The tick's size and gap, and a pencil or cog's width.
const TICK: f32 = 16.0;
const TICK_GAP: f32 = 6.0;
const BUTTON: f32 = 24.0;

fn name_font(current: bool) -> iced::Font {
    if current { BOLD } else { MEDIUM }
}

/// How wide a tile `width` wide leaves its name: inside the tile's 8 px
/// padding and the label's own, less the tick and a pencil or cog.
fn name_room(width: f32, current: bool, button: bool) -> f32 {
    let mut room = width - 16.0 - 2.0;
    room -= if button { BUTTON + TICK_GAP } else { 2.0 };
    if current {
        room -= TICK + TICK_GAP;
    }
    room.max(1.0)
}

/// How tall `name` draws wrapped to `room`, in the font the tile uses, so
/// a row can grow to its longest name. Measured with the fonts the window
/// draws with (a wide one like Fira Code takes two lines where a narrow one
/// takes one), and kept, as the tiles draw often.
fn name_height(name: &str, current: bool, room: f32) -> f32 {
    use iced::advanced::{
        graphics::text::Paragraph,
        text::{self, LineHeight, Paragraph as _, Shaping, Wrapping},
    };
    thread_local! {
        static MEASURED: std::cell::RefCell<std::collections::HashMap<(String, bool, u32), f32>> =
            std::cell::RefCell::default();
    }
    let key = (name.to_owned(), current, room.to_bits());
    if let Some(height) = MEASURED.with(|kept| kept.borrow().get(&key).copied()) {
        return height;
    }
    let height = Paragraph::with_text(text::Text {
        content: name,
        bounds: iced::Size::new(room, f32::INFINITY),
        size: NAME_SIZE.into(),
        line_height: LineHeight::default(),
        font: name_font(current),
        align_x: text::Alignment::Default,
        align_y: iced::alignment::Vertical::Top,
        shaping: Shaping::Advanced,
        wrapping: Wrapping::WordOrGlyph,
    })
    .min_bounds()
    .height
    .ceil();
    MEASURED.with(|kept| kept.borrow_mut().insert(key, height));
    height
}

/// HaloCord: what the last theme file did, or nothing.
fn file_note<'a>(theme: &crate::theme::Theme, note: Option<Note>) -> Element<'a, Message> {
    match note {
        Some(note) => text(note.words())
            .size(13)
            .color(if note.bad() {
                theme.danger
            } else {
                theme.secondary
            })
            .into(),
        None => Space::new().height(0).into(),
    }
}

/// A theme's small window, 78 px tall: the rail with three servers, the
/// sidebar's channels, two messages and the composer, in its colours.
fn preview<'a>(palette: &palette::Theme, width: f32) -> Element<'a, Message> {
    let c = |colour| color(colour);
    let (rail, sidebar, chat, raised, accent, ink, muted) = (
        c(palette.rail),
        c(palette.sidebar),
        c(palette.chat),
        c(palette.raised),
        c(palette.accent),
        c(palette.text),
        c(palette.muted),
    );
    let blob = |w: f32, h: f32, colour: Color, radius: f32| -> Element<'a, Message> {
        container(Space::new())
            .width(w)
            .height(h)
            .style(move |_| crate::style::rounded(colour, radius))
            .into()
    };
    let mut layers = stack![blob(width, 78.0, chat, 6.0)];
    layers = layers.push(blob(16.0, 78.0, rail, 0.0));
    for index in 0..3 {
        let (colour, radius) = if index == 0 {
            (accent, 3.0)
        } else {
            (raised, 5.0)
        };
        layers = layers.push(placed(
            blob(10.0, 10.0, colour, radius),
            3.0,
            6.0 + index as f32 * 13.0,
        ));
    }
    layers = layers.push(placed(blob(38.0, 78.0, sidebar, 0.0), 16.0, 0.0));
    for index in 0..4 {
        let w = if index == 1 {
            24.0
        } else {
            18.0 + (index % 2) as f32 * 4.0
        };
        let colour = if index == 1 { ink } else { muted };
        layers = layers.push(placed(
            blob(w, 4.0, colour, 2.0),
            22.0,
            8.0 + index as f32 * 11.0,
        ));
    }
    let room = width - 66.0;
    for index in 0..2 {
        let y = 8.0 + index as f32 * 22.0;
        let first = index == 0;
        layers = layers
            .push(placed(
                blob(10.0, 10.0, if first { accent } else { muted }, 5.0),
                60.0,
                y,
            ))
            .push(placed(
                blob(if first { 22.0 } else { 16.0 }, 4.0, ink, 2.0),
                74.0,
                y + 1.0,
            ))
            .push(placed(
                blob(
                    (room - 14.0).min(if first { 52.0 } else { 40.0 }),
                    3.0,
                    muted,
                    1.5,
                ),
                74.0,
                y + 8.0,
            ));
    }
    layers = layers
        .push(placed(blob(room, 12.0, raised, 4.0), 60.0, 78.0 - 18.0))
        .push(placed(
            blob(6.0, 6.0, accent, 3.0),
            60.0 + room - 10.0,
            78.0 - 18.0,
        ));
    container(layers.width(width).height(78))
        .clip(true)
        .style(|_| iced::widget::container::Style {
            border: crate::style::round(6),
            ..Default::default()
        })
        .into()
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
