//! The way a popup opens and closes: over [`open_time`], eased out, its
//! card rises [`RISE`] and grows from [`FROM`] of its size to all of it
//! about its centre ([`Grow`]) and the window's dim under it fades in
//! ([`dim`]); closing, over [`close_time`] and eased in, it goes the same
//! way back. A popup under a bar drops from it instead ([`drop_down`]). A
//! layer that opens is wrapped in [`Motion`], which says how far open it is
//! to what's drawn inside it while it draws, and asks for the window's
//! redraws until it's open; nothing sends a message or builds the view anew
//! for it. One closing ([`leave`]) is drawn and takes nothing, neither the
//! pointer nor the keys, and says once it's gone. Outside a `Motion` a card
//! is drawn as it is.
//!
//! How quick it all is comes from [`set_speed`]: a percent of the normal
//! times, and none for still, where a popup shows open at once and goes at
//! once.

use std::{
    cell::Cell,
    sync::atomic::{AtomicU32, Ordering},
    time::{Duration, Instant},
};

use iced::{
    Color, Element, Transformation,
    advanced::{Renderer as _, Shell, renderer, widget::Operation},
    mouse, window,
};

use super::{At, Hooks, Inner, Renderer, Stateless, Wrap};

/// How long a popup takes to open, at the normal speed.
pub const OPEN: Duration = Duration::from_millis(160);
/// How long one takes to close, at the normal speed.
pub const CLOSE: Duration = Duration::from_millis(120);
/// How large a card opens from.
pub const FROM: f32 = 0.96;
/// How far a card rises (or drops) as it opens, in px.
pub const RISE: f32 = 8.0;

/// The speed, in percent of the normal times: 0 is still.
static SPEED: AtomicU32 = AtomicU32::new(100);

/// Sets how quick popups open and close: `percent` of the normal times,
/// 0 for still.
pub fn set_speed(percent: u32) {
    SPEED.store(percent.min(400), Ordering::Relaxed);
}

/// The speed set, in percent; 0 is still.
pub fn speed() -> u32 {
    SPEED.load(Ordering::Relaxed)
}

/// Whether popups show and go at once.
pub fn still() -> bool {
    speed() == 0
}

/// How long opening takes at the speed set.
pub fn open_time() -> Duration {
    OPEN * speed() / 100
}

/// How long closing takes at the speed set.
pub fn close_time() -> Duration {
    CLOSE * speed() / 100
}

/// How far through `time` it is from `start` to `now`, 0 to 1; all the way
/// when there's no time at all.
fn through(start: Instant, now: Instant, time: Duration) -> f32 {
    if time.is_zero() {
        return 1.0;
    }
    (now.saturating_duration_since(start).as_secs_f32() / time.as_secs_f32()).min(1.0)
}

thread_local! {
    /// How far open the layer being drawn is, eased, from 0 to 1: set by
    /// the [`Motion`] drawing it, 1 outside one.
    static SHOWN: Cell<f32> = const { Cell::new(1.0) };
}

/// How far open a layer is that opened at `start`, at `now`, eased out
/// on a quadratic curve.
pub fn shown(start: Instant, now: Instant) -> f32 {
    let t = through(start, now, open_time());
    1.0 - (1.0 - t) * (1.0 - t)
}

/// How far open a layer still is that started closing at `start`, at
/// `now`, eased in on a quadratic curve: the open played backwards.
pub fn closing(start: Instant, now: Instant) -> f32 {
    let t = through(start, now, close_time());
    1.0 - t * t
}

/// For a layer closing since `start`, when it would have started opening
/// to be as far open at `now` as it is: a popup opened again as it closes
/// grows on from there.
pub fn reopened(start: Instant, now: Instant) -> Instant {
    let open = closing(start, now);
    // `shown` backwards: how far through opening it is at `open`.
    let t = 1.0 - (1.0 - open).max(0.0).sqrt();
    now - open_time().mul_f32(t)
}

/// For a layer opening since `start`, when it would have started closing
/// to be as far open at `now` as it is: a popup closed as it opens turns
/// round from there. None when it's all the way open, to start closing as
/// it's next drawn.
pub fn closed_from(start: Instant, now: Instant) -> Option<Instant> {
    let open = shown(start, now);
    if open >= 1.0 {
        return None;
    }
    // `closing` backwards: how far through closing it is at `open`.
    let t = (1.0 - open).max(0.0).sqrt();
    Some(now - close_time().mul_f32(t))
}

/// A layer opening or closing: `start` is when it was first drawn so,
/// kept by the one who keeps what's open, as the widget tree's state
/// would go with a layer opened or closed under it.
pub struct Motion<'a, M> {
    start: &'a Cell<Option<Instant>>,
    /// While it closes, what it says once it's closed.
    closed: Option<M>,
}

impl<M> Motion<'_, M> {
    /// How far open it is at `now`, its start set if it had none.
    fn shown(&self, now: Instant) -> f32 {
        let start = *self.start.get().get_or_insert(now);
        self.start.set(Some(start));
        if self.closed.is_some() {
            closing(start, now)
        } else {
            shown(start, now)
        }
    }
}

/// `layer`, opening since it was first drawn (`start`, empty until then).
pub fn motion<'a, M: Clone + 'a>(
    layer: impl Into<Element<'a, M>>,
    start: &'a Cell<Option<Instant>>,
) -> Element<'a, M> {
    Wrap::new(
        layer,
        Motion {
            start,
            closed: None,
        },
    )
    .into()
}

/// `layer`, closed and closing since it was first drawn so (`start`):
/// drawn, it takes nothing, and says `closed` as it's gone.
pub fn leave<'a, M: Clone + 'a>(
    layer: impl Into<Element<'a, M>>,
    start: &'a Cell<Option<Instant>>,
    closed: M,
) -> Element<'a, M> {
    Wrap::new(
        layer,
        Motion {
            start,
            closed: Some(closed),
        },
    )
    .into()
}

impl<M: Clone> Hooks<M> for Motion<'_, M> {
    type State = Stateless;

    fn update(
        &mut self,
        _state: &mut Stateless,
        event: &iced::Event,
        _at: At,
        shell: &mut Shell<'_, M>,
        child: &mut Inner<'_, '_, M>,
    ) {
        if let iced::Event::Window(window::Event::RedrawRequested(now)) = event {
            let start = *self.start.get().get_or_insert(*now);
            self.start.set(Some(start));
            let time = if self.closed.is_some() {
                close_time()
            } else {
                open_time()
            };
            if now.saturating_duration_since(start) < time {
                shell.request_redraw();
            } else if let Some(closed) = &self.closed {
                shell.publish(closed.clone());
            }
        }
        if self.closed.is_none() {
            child.update(shell);
        }
    }

    fn interaction(
        &self,
        _state: &Stateless,
        _at: At,
        child: mouse::Interaction,
    ) -> mouse::Interaction {
        // Closing, the pointer goes to what's under it.
        if self.closed.is_some() {
            mouse::Interaction::None
        } else {
            child
        }
    }

    fn draw(
        &self,
        _state: &Stateless,
        renderer: &mut Renderer,
        _at: At,
        child: &mut dyn FnMut(&mut Renderer),
    ) {
        let shown = self.shown(Instant::now());
        if shown <= 0.0 {
            return;
        }
        let outer = SHOWN.replace(shown);
        child(renderer);
        SHOWN.set(outer);
    }

    fn overlays(&self) -> bool {
        self.closed.is_none()
    }

    fn operate(
        &mut self,
        _state: &mut Stateless,
        operation: &mut dyn Operation,
        child: &mut dyn FnMut(&mut dyn Operation),
    ) {
        // Closing, no focus or scroll finds what's in it.
        if self.closed.is_none() {
            child(operation);
        }
    }
}

/// A popup's card, risen [`RISE`] and grown from [`FROM`] of its size about
/// its centre as the [`Motion`] it's drawn in opens: `rise` below where it
/// ends, or above it for one that drops from a bar.
pub struct Grow {
    rise: f32,
}

/// `card`, rising and grown as its layer opens: a card in the middle.
pub fn grow<'a, M: 'a>(card: impl Into<Element<'a, M>>) -> Element<'a, M> {
    Wrap::new(card, Grow { rise: RISE }).into()
}

/// `card`, dropping from the bar above it and grown as its layer opens.
pub fn drop_down<'a, M: 'a>(card: impl Into<Element<'a, M>>) -> Element<'a, M> {
    Wrap::new(card, Grow { rise: -RISE }).into()
}

impl<M> Hooks<M> for Grow {
    type State = Stateless;

    fn draw(
        &self,
        _state: &Stateless,
        renderer: &mut Renderer,
        at: At,
        child: &mut dyn FnMut(&mut Renderer),
    ) {
        let shown = SHOWN.get();
        if shown >= 1.0 {
            child(renderer);
            return;
        }
        let scale = FROM + (1.0 - FROM) * shown;
        let centre = at.child.center();
        let off = self.rise * (1.0 - shown);
        let transformation = Transformation::translate(centre.x, centre.y + off)
            * Transformation::scale(scale)
            * Transformation::translate(-centre.x, -centre.y);
        renderer.with_transformation(transformation, |renderer| child(renderer));
    }
}

/// The window dimmed by `color` under a popup, fading in as its layer
/// opens: `layer` (taking the clicks, holding the id) is drawn over it.
pub fn dim<'a, M: 'a>(layer: impl Into<Element<'a, M>>, color: Color) -> Element<'a, M> {
    Wrap::new(layer, Dim { color }).into()
}

/// What [`dim`] paints.
pub struct Dim {
    color: Color,
}

impl<M> Hooks<M> for Dim {
    type State = Stateless;

    fn draw(
        &self,
        _state: &Stateless,
        renderer: &mut Renderer,
        at: At,
        child: &mut dyn FnMut(&mut Renderer),
    ) {
        let color = Color {
            a: self.color.a * SHOWN.get(),
            ..self.color
        };
        renderer.fill_quad(
            renderer::Quad {
                bounds: at.bounds,
                ..Default::default()
            },
            color,
        );
        child(renderer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // One test sets the speed, so no two race on it.
    #[test]
    fn a_popup_opens_eased_out_and_closes_eased_in_at_the_speed_set() {
        set_speed(100);
        let start = Instant::now();
        assert_eq!(shown(start, start), 0.0);
        let half = shown(start, start + OPEN / 2);
        // Eased out: past half way at half the time.
        assert!((half - 0.75).abs() < 1e-4, "{half}");
        assert_eq!(shown(start, start + OPEN), 1.0);
        assert_eq!(shown(start, start + OPEN * 3), 1.0);
        // Eased in: still mostly there at half the closing time.
        let half = closing(start, start + CLOSE / 2);
        assert!((half - 0.75).abs() < 1e-4, "{half}");
        assert_eq!(closing(start, start + CLOSE), 0.0);

        // Opened again as it closes, it grows on from where it was.
        let now = start + CLOSE / 4;
        let open = closing(start, now);
        let again = reopened(start, now);
        assert!((shown(again, now) - open).abs() < 1e-3);
        // Closed all the way, it opens from the start.
        let gone = start + CLOSE;
        assert!(shown(reopened(start, gone), gone) < 1e-3);
        // Closed as it opens, it turns round from where it was.
        let now = start + OPEN / 3;
        let from = closed_from(start, now).expect("still opening");
        assert!((closing(from, now) - shown(start, now)).abs() < 1e-3);
        // All the way open, it starts closing as it's next drawn.
        assert_eq!(closed_from(start, start + OPEN), None);

        // Slow doubles the times.
        set_speed(200);
        assert_eq!(open_time(), OPEN * 2);
        assert!((shown(start, start + OPEN) - 0.75).abs() < 1e-4);

        // Still: open at once, gone at once.
        set_speed(0);
        assert!(still());
        assert_eq!(shown(start, start), 1.0);
        assert_eq!(closing(start, start), 0.0);
        set_speed(100);
    }
}
