//! HaloCord's group in Settings: the overview, the Plugins page (a filter, a
//! row of areas and the plugins as cards, each with a cog that opens its
//! settings in a dialog), the MochaCord dialog the Themes page opens, and
//! what Backup & Restore's buttons do. The plugins are
//! [`crate::halocord::PLUGINS`]; where each belongs is `halocord/areas.rs`.
//! The dialogs open and close as popups do (`widgets::motion`).

use std::{cell::Cell, time::Instant};

use iced::{
    Alignment, Element, Length, Task,
    widget::{Space, button, column, container, responsive, row, stack, text},
};

use super::{Pref, pref};
use crate::{
    app::Message,
    controls::{Kind as Look, action, choices, lights, slider},
    dialog::buttons,
    fonts::{BOLD, MEDIUM},
    form,
    halocord::{
        Area, Change, Kind, PLUGINS, Plugin, Setting, area,
        backup::{self, Backup, Note},
        found,
    },
    pictures::Icon,
    settings::{Event, Page, Section, Settings},
    style,
    widgets::motion,
};

/// The filter field.
const FILTER: &str = "halocord-filter";
/// A plugin's card: this tall, this far from the next, and two across from
/// this much room.
const CARD: f32 = 92.0;
const CARD_GAP: f32 = 12.0;
const TWO_ACROSS: f32 = 600.0;
/// How far the card's switch and cog sit in from its edge.
const CARD_PAD: f32 = 14.0;
/// The cog's button.
const COG: f32 = 28.0;
/// The dialogs' widths.
const DIALOG: f32 = 480.0;

/// Which plugins the Plugins page lists by their switch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Show {
    #[default]
    All,
    On,
    Off,
}

/// The HaloCord pages' state while Settings is open.
#[derive(Default)]
pub struct HaloState {
    /// The Plugins page's filter, the switch filter and the area chosen.
    pub filter: String,
    pub show: Show,
    pub area: Option<Area>,
    /// The plugin whose settings dialog is open, by its place in `PLUGINS`.
    pub plugin: Option<usize>,
    /// The MochaCord dialog is open.
    pub mocha: bool,
    /// What Backup & Restore says of the last file.
    pub note: Option<Note>,
    /// The dialog closed and still drawn going.
    pub leaving: Option<Dialog>,
    /// When the dialog shown started opening or closing (empty until it's
    /// first drawn so).
    pub moved: Cell<Option<Instant>>,
}

/// A dialog over Settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialog {
    /// A plugin's settings, by its place in `PLUGINS`.
    Plugin(usize),
    MochaCord,
}

impl HaloState {
    /// The dialog open, if one is: a plugin's over MochaCord's.
    pub fn dialog(&self) -> Option<Dialog> {
        self.plugin
            .map(Dialog::Plugin)
            .or(self.mocha.then_some(Dialog::MochaCord))
    }

    /// Opens or closes the dialogs to `plugin` and `mocha`: one opened
    /// opens as a popup does, and one closed is drawn going.
    pub fn show_dialog(&mut self, plugin: Option<usize>, mocha: bool) {
        let before = self.dialog();
        self.plugin = plugin.filter(|index| *index < PLUGINS.len());
        self.mocha = mocha;
        let now = Instant::now();
        match (before, self.dialog()) {
            (before, after) if before == after => {}
            (None, None) => {}
            (None, Some(_)) => {
                // Opened again as it closes, it grows on from there.
                let start = match self.leaving.take() {
                    Some(_) => match self.moved.get() {
                        Some(closing) => Some(motion::reopened(closing, now)),
                        None => now.checked_sub(motion::open_time()),
                    },
                    None => None,
                };
                self.moved.set(start);
            }
            (Some(was), None) => {
                if motion::still() {
                    self.leaving = None;
                    self.moved.set(None);
                    return;
                }
                // Closed as it opens, it turns round from there.
                let opened = self.moved.get();
                self.moved
                    .set(opened.and_then(|opened| motion::closed_from(opened, now)));
                self.leaving = Some(was);
            }
            (Some(_), Some(_)) => {
                self.leaving = None;
                self.moved.set(None);
            }
        }
    }

    /// The dialog closed has been drawn going.
    fn dialog_gone(&mut self) {
        if self.dialog().is_none() {
            self.leaving = None;
            self.moved.set(None);
        }
    }
}

#[derive(Clone, Debug)]
pub enum HaloEvent {
    Filter(String),
    /// A choice in the All, On, Off row.
    Show(usize),
    /// An area's chip: the same one again lets it go.
    Area(Area),
    /// A plugin's dialog opened or closed.
    Plugin(Option<usize>),
    /// The MochaCord dialog opened or closed.
    Mocha(bool),
    /// The Themes page with the MochaCord dialog open.
    OpenMocha,
    /// The dialog closed has been drawn going.
    DialogGone,
    BackupSave,
    BackupLoad,
    /// The text of the file picked: none when the window was closed, an
    /// inner none when it couldn't be read.
    BackupRead(Option<Option<String>>),
    BackupDone(Option<Note>),
}

/// A HaloCord page's event as a message.
pub fn halo_message(event: HaloEvent) -> Message {
    Message::Settings(Event::HaloCord(event))
}

fn open(section: Section) -> Message {
    Message::Settings(Event::Open(section))
}

impl Settings {
    pub(in crate::settings) fn halocord_event(
        &mut self,
        event: HaloEvent,
        cx: &mut crate::ctx::Cx<'_>,
    ) -> Task<Message> {
        let halo = &mut self.halo;
        match event {
            HaloEvent::Filter(text) => halo.filter = text,
            HaloEvent::Show(index) => {
                halo.show = [Show::All, Show::On, Show::Off][index.min(2)];
            }
            HaloEvent::Area(chosen) => {
                halo.area = (halo.area != Some(chosen)).then_some(chosen);
            }
            HaloEvent::Plugin(plugin) => {
                let mocha = halo.mocha;
                halo.show_dialog(plugin, mocha);
            }
            HaloEvent::Mocha(open) => {
                let plugin = halo.plugin;
                halo.show_dialog(plugin, open);
                // Its speed list goes with it.
                if !open && self.open == Some(crate::settings::Drop::AnimationSpeed) {
                    self.open = None;
                }
            }
            HaloEvent::OpenMocha => {
                let task = self.show(Section::Themes, cx);
                self.halo.show_dialog(None, true);
                return task;
            }
            HaloEvent::DialogGone => halo.dialog_gone(),
            HaloEvent::BackupSave => {
                halo.note = None;
                let backup = Backup::of(
                    &cx.layout.halocord,
                    &cx.layout.mochacord,
                    &cx.themes.custom,
                    Some(cx.themes.chosen_id()),
                );
                return backup::save(&backup);
            }
            HaloEvent::BackupLoad => return backup::open(),
            HaloEvent::BackupRead(None) => {}
            HaloEvent::BackupRead(Some(None)) => halo.note = Some(Note::NotRead),
            HaloEvent::BackupRead(Some(Some(text))) => {
                halo.note = Some(match Backup::read(&text) {
                    Some(backup) => {
                        restore(backup, cx);
                        Note::Loaded
                    }
                    None => Note::NotABackup,
                });
            }
            HaloEvent::BackupDone(note) => halo.note = note,
        }
        Task::none()
    }
}

/// Takes a backup's setup: the plugins, MochaCord's options, the person's
/// own themes and the theme chosen, each kept and told to the core and
/// the window.
fn restore(backup: Backup, cx: &mut crate::ctx::Cx<'_>) {
    use crate::ctx::{Pref as Shown, Request};
    // A restored look never counts as the first start, which would
    // switch the theme again.
    let mut mochacord = backup.mochacord;
    mochacord.switched = true;
    cx.layout.halocord = backup.halocord;
    cx.layout.mochacord = mochacord;
    cx.layout.halocord.apply();
    cx.layout.mochacord.apply();
    cx.keep("halocord", &cx.layout.halocord);
    cx.keep("mochacord", &cx.layout.mochacord);
    let mut themes = backup.themes;
    yapfast_core::theme::tidy(&mut themes);
    cx.themes.custom = themes;
    cx.keep("custom_themes", &cx.themes.custom);
    if let Some(id) = backup.theme.filter(|id| cx.themes.find(id).is_some()) {
        cx.themes.choose(cx.client.root(), &id);
    }
    cx.request(Request::Changed(Shown::Theme));
    cx.touch();
}

impl<'a> Page<'a> {
    /// The HaloCord page: which build this is, how many plugins are on, the
    /// theme in use, and the way to the rest.
    pub(in crate::settings) fn halocord_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let kept = &self.cx.layout.halocord;
        let on = PLUGINS.iter().filter(|plugin| kept.on(plugin)).count();
        let summary = if self.cx.build.summary.is_empty() {
            "HaloCord for Yapfast".to_owned()
        } else {
            self.cx.build.summary.clone()
        };
        let name = self.theme_name();
        let tiles = responsive(move |size| {
            let stat = |big: String, small: &'static str| -> Element<'a, Message> {
                form::card(
                    theme,
                    column![
                        text(big)
                            .size(22)
                            .font(BOLD)
                            .color(theme.text)
                            .wrapping(text::Wrapping::None),
                        form::note(theme, small),
                    ]
                    .spacing(4),
                )
            };
            let plugins = stat(format!("{on} of {}", PLUGINS.len()), "plugins are on");
            let shown = stat(name.clone(), "theme in use");
            if size.width >= 420.0 {
                row![plugins, shown].spacing(form::GAP).into()
            } else {
                column![plugins, shown].spacing(form::GAP).into()
            }
        })
        .height(Length::Shrink);
        let go = |label: &'static str, kind: Look, section: Section| {
            action(theme, label, kind, Some(open(section))).into()
        };
        let jumps: Vec<Element<'a, Message>> = vec![
            go("Plugins", Look::Primary, Section::Plugins),
            go("Themes", Look::Plain, Section::Themes),
            go("Backup & Restore", Look::Plain, Section::Backup),
        ];
        // HaloCord's own links: where to help, the owner's server and his
        // Ko-fi, as the other clients put on their About pages.
        const REPO: &str = "https://github.com/DavidHiFi/HaloCord-Yapfast";
        const LOCKER: &str = "https://discord.gg/PETWgTfS9x";
        const KOFI: &str = "https://ko-fi.com/davidhifi";
        let open_link = |label: &'static str, icon: crate::pictures::Icon, page: &'static str| {
            action(
                theme,
                label,
                Look::Plain,
                Some(super::account_message(super::AccountEvent::Open(page))),
            )
            .icon(self.cx.look.icon(icon, 18, crate::theme::rgb(theme.text)))
            .into()
        };
        let links: Vec<Element<'a, Message>> = vec![
            open_link("Contribute on GitHub", crate::pictures::Icon::GitHub, REPO),
            open_link(
                "Join Davey Jones' Locker",
                crate::pictures::Icon::Discord,
                LOCKER,
            ),
            open_link("Donate on Ko-fi", crate::pictures::Icon::Star, KOFI),
        ];
        form::page([
            form::title(theme, "HaloCord"),
            form::card(
                theme,
                column![
                    form::heading(theme, "THIS BUILD"),
                    text(summary).size(15).color(theme.text),
                    form::note(
                        theme,
                        "HaloCord adds its plugins and its look to Yapfast. Both stay when HaloCord brings Yapfast up to date.",
                    ),
                ]
                .spacing(6),
            ),
            tiles.into(),
            row(jumps).spacing(8).wrap().vertical_spacing(8).into(),
            form::heading(theme, "LINKS"),
            row(links).spacing(8).wrap().vertical_spacing(8).into(),
            form::heading(theme, "WINDOW"),
            self.title_bar_switch(),
            form::switch(
                "Acrylic behind the window",
                self.cx.layout.mochacord.glass,
                Some(pref(Pref::MochaCord(crate::mochacord::Change::Glass(
                    !self.cx.layout.mochacord.glass,
                )))),
            )
            .description("Windows blurs the desktop behind Yapfast, as Vencord's Acrylic material. The strength is with the other look options, on the Themes page.")
            .fresh(self.fresh("halocord-glass"))
            .view(self.cx.look),
            form::switch(
                "Disable the minimum window size",
                self.cx.layout.no_min_size.unwrap_or(false),
                Some(pref(Pref::NoMinSize(
                    !self.cx.layout.no_min_size.unwrap_or(false),
                ))),
            )
            .description("The window may shrink smaller than Yapfast normally allows. It shows from the next time Yapfast opens.")
            .fresh(self.fresh("halocord-no-min-size"))
            .view(self.cx.look),
        ])
    }

    /// The Plugins page.
    pub(in crate::settings) fn plugins_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let halo = &self.s.halo;
        let kept = &self.cx.layout.halocord;
        // Settings' own search narrows the list too.
        let query = self.s.query.to_lowercase();
        let by_search = query.is_empty() || "plugins".contains(&query);
        let filter = halo.filter.trim().to_lowercase();
        let shown: Vec<usize> = PLUGINS
            .iter()
            .enumerate()
            .filter(|(_, plugin)| {
                (by_search || found(plugin, &query))
                    && (filter.is_empty() || found(plugin, &filter))
                    && match halo.show {
                        Show::All => true,
                        Show::On => kept.on(plugin),
                        Show::Off => !kept.on(plugin),
                    }
                    && halo.area.is_none_or(|chosen| area(plugin.id) == chosen)
            })
            .map(|(index, _)| index)
            .collect();
        let on = shown
            .iter()
            .filter(|&&index| kept.on(&PLUGINS[index]))
            .count();
        let count = format!(
            "{} plugin{}, {on} on",
            shown.len(),
            if shown.len() == 1 { "" } else { "s" }
        );
        let show = match halo.show {
            Show::All => 0,
            Show::On => 1,
            Show::Off => 2,
        };
        let chips: Vec<Element<'a, Message>> = Area::ALL
            .iter()
            .map(|&each| self.chip(each.name(), halo.area == Some(each), HaloEvent::Area(each)))
            .collect();
        let mut page = vec![
            form::title(theme, "Plugins"),
            form::note(
                theme,
                "A switch turns a plugin on. A cog opens its settings. Both stay when HaloCord brings Yapfast up to date.",
            ),
            form::field(theme, FILTER, "Filter plugins", &halo.filter)
                .on_input(|text| halo_message(HaloEvent::Filter(text)))
                .into(),
            choices(theme, &["All", "On", "Off"], show, |index| {
                halo_message(HaloEvent::Show(index))
            }),
            row(chips).spacing(8).wrap().vertical_spacing(8).into(),
            form::note(theme, count),
        ];
        if shown.is_empty() {
            page.push(form::note(
                theme,
                "No plugin matches. Clear the filter or pick another area.",
            ));
        } else {
            page.push(self.plugin_grid(shown));
        }
        form::page(page)
    }

    /// An area's chip: lit while chosen.
    fn chip(self, label: &'static str, chosen: bool, event: HaloEvent) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let (rest, hover, line, faint) = (theme.sidebar, theme.hover, theme.line, theme.faint);
        let (accent, on_accent, ink) = (theme.accent, theme.on_accent, theme.secondary);
        button(
            text(label)
                .size(13)
                .font(MEDIUM)
                .wrapping(text::Wrapping::None),
        )
        .padding([6, 14])
        .height(32)
        .on_press(halo_message(event))
        .style(move |_, status| {
            let lit = lights(status);
            button::Style {
                background: Some(
                    if chosen {
                        accent
                    } else if lit {
                        hover
                    } else {
                        rest
                    }
                    .into(),
                ),
                text_color: if chosen { on_accent } else { ink },
                border: if style::rice().is_some() {
                    style::lit_edge(lit, status == button::Status::Pressed, 16.0)
                } else {
                    style::edge(if lit { faint } else { line }, 16.0)
                },
                ..Default::default()
            }
        })
        .into()
    }

    /// The plugins as cards, two across when there's room, else one.
    fn plugin_grid(self, shown: Vec<usize>) -> Element<'a, Message> {
        responsive(move |size| {
            let columns = if size.width >= TWO_ACROSS { 2 } else { 1 };
            let width = (size.width - (columns - 1) as f32 * CARD_GAP) / columns as f32;
            let mut rows = column![].spacing(CARD_GAP);
            for chunk in shown.chunks(columns) {
                let mut line = row![].spacing(CARD_GAP);
                for &index in chunk {
                    line = line.push(self.plugin_card(index, width));
                }
                if chunk.len() < columns {
                    line = line.push(Space::new().width(width));
                }
                rows = rows.push(line);
            }
            rows.into()
        })
        .height(Length::Shrink)
        .into()
    }

    /// A plugin's card: its name and what it does, the switch at the right
    /// and a cog beside it. The card itself flips
    /// the switch too.
    fn plugin_card(self, index: usize, width: f32) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let plugin = &PLUGINS[index];
        let on = self.cx.layout.halocord.on(plugin);
        let flip = pref(Pref::HaloCord(Change::Switch {
            plugin: index,
            on: !on,
        }));
        let room = CARD_PAD + 48.0 + 12.0 + 8.0 + COG;
        let words = column![
            text(plugin.name)
                .size(15)
                .font(BOLD)
                .color(theme.text)
                .wrapping(text::Wrapping::None),
            container(text(plugin.about).size(13).color(theme.muted))
                .height(36)
                .clip(true),
        ]
        .spacing(4);
        let (rest, hover, line) = (theme.sidebar, theme.hover, theme.line);
        let face = button(container(words).padding(iced::Padding {
            top: CARD_PAD,
            bottom: CARD_PAD,
            left: CARD_PAD,
            right: room,
        }))
        .padding(0)
        .width(width)
        .height(CARD)
        .on_press(flip.clone())
        .style(move |_, status| {
            let lit = lights(status);
            button::Style {
                background: Some(if lit { hover } else { rest }.into()),
                border: if style::rice().is_some() {
                    style::lit_edge(lit, status == button::Status::Pressed, 8.0)
                } else {
                    style::edge(line, 8.0)
                },
                ..Default::default()
            }
        });
        let mut controls = row![Space::new().width(Length::Fill)]
            .spacing(8)
            .align_y(Alignment::Center);
        controls = controls.push(
            self.cx
                .look
                .icon_button(
                    Icon::Settings,
                    "Settings",
                    halo_message(HaloEvent::Plugin(Some(index))),
                )
                .size(COG, 16),
        );
        controls = controls.push(form::toggle(theme, on, Some(flip)));
        stack![
            face,
            container(controls)
                .padding([0.0, CARD_PAD])
                .width(width)
                .height(CARD)
                .align_y(Alignment::Center)
        ]
        .width(width)
        .height(CARD)
        .into()
    }

    /// Backup & Restore: the setup in one file, and back from one.
    pub(in crate::settings) fn backup_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let mut parts = vec![
            form::title(theme, "Backup & Restore"),
            form::note(
                theme,
                "Keeps your plugins, the MochaCord options, your own themes and the theme you use in one file.",
            ),
            form::note(
                theme,
                "Loading a file replaces what you have now with what is in it.",
            ),
            row![
                action(
                    theme,
                    "Save to file",
                    Look::Primary,
                    Some(halo_message(HaloEvent::BackupSave)),
                ),
                action(
                    theme,
                    "Load from file",
                    Look::Plain,
                    Some(halo_message(HaloEvent::BackupLoad)),
                ),
            ]
            .spacing(8)
            .wrap()
            .vertical_spacing(8)
            .into(),
        ];
        if let Some(note) = self.s.halo.note {
            parts.push(
                text(note.words())
                    .size(13)
                    .color(if note.bad() {
                        theme.danger
                    } else {
                        theme.secondary
                    })
                    .into(),
            );
        }
        form::page(parts)
    }

    /// The dialog that's open over Settings, if one is.
    pub(in crate::settings) fn halo_dialog(self) -> Option<Element<'a, Message>> {
        let halo = &self.s.halo;
        let theme = self.cx.look.theme;
        let look = self.cx.look;
        let (shown, closing) = match halo.dialog() {
            Some(open) => (open, false),
            None => (halo.leaving?, true),
        };
        let (title, intro, mut body, close) = match shown {
            Dialog::Plugin(index) => {
                let plugin = PLUGINS.get(index)?;
                (
                    plugin.name,
                    plugin.about,
                    self.plugin_settings(index, plugin),
                    halo_message(HaloEvent::Plugin(None)),
                )
            }
            Dialog::MochaCord => (
                "MochaCord",
                "How this copy of Yapfast looks. These options work with any theme.",
                self.mochacord_options(),
                halo_message(HaloEvent::Mocha(false)),
            ),
        };
        body.insert(0, look.dialog_head(title, close.clone()));
        body.insert(1, form::note(theme, intro));
        body.push(buttons([action(
            theme,
            "Close",
            Look::Primary,
            Some(close.clone()),
        )
        .into()]));
        let framed = look.framed(body, DIALOG, close);
        Some(if closing {
            motion::leave(framed, &halo.moved, halo_message(HaloEvent::DialogGone))
        } else {
            motion::motion(framed, &halo.moved)
        })
    }

    /// A plugin's settings, each changing live: a choice, a switch, a slider
    /// or a line of text.
    fn plugin_settings(self, index: usize, plugin: &'static Plugin) -> Vec<Element<'a, Message>> {
        let theme = self.cx.look.theme;
        let kept = &self.cx.layout.halocord;
        let mut parts = Vec::new();
        for (place, setting) in plugin.settings.iter().enumerate() {
            let change = move |change| pref(Pref::HaloCord(change));
            self.setting(&mut parts, index, place, plugin, setting, change);
        }
        let _ = (theme, kept);
        parts
    }

    fn setting(
        self,
        parts: &mut Vec<Element<'a, Message>>,
        index: usize,
        place: usize,
        plugin: &'static Plugin,
        setting: &'static Setting,
        change: impl Fn(Change) -> Message + Copy + 'a,
    ) {
        let theme = self.cx.look.theme;
        let kept = &self.cx.layout.halocord;
        match &setting.kind {
            Kind::Choice { names, .. } => {
                parts.push(form::heading(theme, setting.heading));
                parts.push(choices(
                    theme,
                    names,
                    kept.choice(plugin, setting),
                    move |choice| {
                        change(Change::Choose {
                            plugin: index,
                            setting: place,
                            choice,
                        })
                    },
                ));
                parts.push(form::note(theme, setting.note));
            }
            Kind::Switch { .. } => {
                let set = kept.flag(plugin, setting);
                parts.push(
                    form::switch(
                        setting.heading,
                        set,
                        Some(change(Change::Flag {
                            plugin: index,
                            setting: place,
                            on: !set,
                        })),
                    )
                    .description(setting.note)
                    .view(self.cx.look),
                );
            }
            Kind::Slider {
                most, step, unit, ..
            } => {
                let value = kept.value(plugin, setting);
                parts.push(
                    slider(setting.heading, value, *most, move |value| {
                        change(Change::Slide {
                            plugin: index,
                            setting: place,
                            value,
                        })
                    })
                    .step(*step)
                    .shown(format!("{value}{unit}"))
                    .mark(false)
                    .view(theme),
                );
                parts.push(form::note(theme, setting.note));
            }
            Kind::Text { placeholder, .. } => {
                parts.push(form::heading(theme, setting.heading));
                parts.push(
                    form::field(theme, setting.key, placeholder, kept.text(plugin, setting))
                        .on_input(move |text| {
                            change(Change::Type {
                                plugin: index,
                                setting: place,
                                text,
                            })
                        })
                        .into(),
                );
                parts.push(form::note(theme, setting.note));
            }
        }
    }
}

/// Whether a search (lowercase) is about MochaCord's options.
pub fn mocha_found(query: &str) -> bool {
    [
        "mochacord",
        "glass",
        "strength",
        "accent",
        "gradient",
        "corner",
        "round",
        "font",
        "weight",
    ]
    .iter()
    .any(|word| word.contains(query))
}
