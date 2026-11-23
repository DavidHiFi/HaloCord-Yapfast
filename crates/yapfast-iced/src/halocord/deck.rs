//! Stream Deck: HaloCord's own Stream Deck plugin presses keys here, since
//! Elgato's Discord plugin only talks to the official client. While the
//! plugin is on, a small server on this computer answers it: what the call
//! shows now, and each key's press.
//!
//! Only programs on this computer reach it, and only ones that send the
//! deck's header: a web page can't add that header, a page's address is
//! refused by the host check, and a request with an origin is refused.
//!
//! `GET /state` is what the keys draw; `GET /press/<key>` presses one:
//! `mute`, `deafen`, `camera`, `share`, `fake-mute`, `fake-deafen`,
//! `voice-mode`, `leave`, `screen-1` to `screen-4`, and the held keys
//! `talk-down`/`talk-up` and `mute-hold-down`/`mute-hold-up`.

use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

use iced::{
    Subscription, Task,
    futures::{Stream, stream},
};
use yapfast_core::{
    application::Action,
    push_to_talk,
    stream_worker::{self, ScreenPlace, Source},
    voice::{CameraPhase, Command},
};

use super::{Change, PLUGINS, Plugin, id};
use crate::app::{App, Message};

/// The port the deck's plugin asks on.
pub const PORT: u16 = 47830;
/// The header every request carries.
const HEADER: &str = "x-halocord-deck";
/// How long a press waits for the window to take it.
const ANSWER: Duration = Duration::from_secs(2);
/// A held key the deck stopped repeating is let go after this.
const LET_GO: Duration = Duration::from_millis(3500);
/// The most a request's head may be.
const MOST_HEAD: usize = 8 * 1024;

pub(super) const STREAM_DECK: Plugin = Plugin {
    id: id::STREAM_DECK,
    name: "Stream Deck",
    about: "Lets HaloCord's Stream Deck keys mute, deafen, film, share a screen or jump between screens, and flip the fake voice switches. Only programs on this computer can reach it.",
    on_at_first: true,
    settings: &[],
    apply: |_, _| {},
    hooks: None,
};

/// A key pressed on the deck, for the window to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    Mute,
    Deafen,
    Camera,
    Share,
    FakeMute,
    FakeDeafen,
    VoiceMode,
    Leave,
    /// Share this screen, switch the stream to it, or stop it there.
    Display(u64),
}

/// A press and where its answer goes.
#[derive(Clone)]
pub struct Asked {
    pub press: Press,
    answer: mpsc::Sender<Result<(), String>>,
}

/// What the keys draw, as the window last showed it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Shown {
    in_call: bool,
    ready: bool,
    mute: bool,
    deafen: bool,
    camera: bool,
    can_film: bool,
    sharing: bool,
    can_share: bool,
    fake_mute: bool,
    fake_deafen: bool,
    talk_mode: bool,
    shared_screen: Option<u64>,
}

static SHOWN: Mutex<Option<Shown>> = Mutex::new(None);

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The server while the plugin is on; it keeps what the keys draw current.
pub fn subscription(app: &App) -> Option<Subscription<Message>> {
    if !app.layout.halocord.is(id::STREAM_DECK) {
        *lock(&SHOWN) = None;
        return None;
    }
    let now = shown(app);
    *lock(&SHOWN) = Some(now);
    Some(Subscription::run(listen).map(Message::Deck))
}

fn shown(app: &App) -> Shown {
    let voice = &app.client.state().voice;
    let join = (
        app.layout.unmute_on_join.unwrap_or(true),
        app.layout.deafen_on_join,
    );
    let sharing = voice.sharing.is_some();
    Shown {
        in_call: app.account.in_call,
        ready: app.account.ready,
        mute: app.account.shown_muted(join),
        deafen: app.account.shown_deafened(join),
        camera: voice.camera_phase != CameraPhase::Off,
        can_film: voice.can_film,
        sharing,
        can_share: crate::call::stage::can_share(app.ctx()),
        fake_mute: app.layout.halocord.is(id::FAKE_MUTE),
        fake_deafen: app.layout.halocord.is(id::FAKE_DEAFEN),
        talk_mode: app.layout.voice.input_mode == crate::layout::InputMode::PushToTalk,
        shared_screen: match stream_worker::current_pick() {
            Source::Display(screen) if sharing => Some(screen),
            _ => None,
        },
    }
}

/// The window takes a press: what it did, told back to the deck.
pub fn asked(app: &mut App, asked: Asked) -> Task<Message> {
    let (answer, task) = match pressed(app, asked.press) {
        Ok(task) => (Ok(()), task),
        Err(why) => (Err(why.to_owned()), Task::none()),
    };
    // The deck may have stopped waiting.
    asked.answer.send(answer).ok();
    task
}

fn pressed(app: &mut App, press: Press) -> Result<Task<Message>, &'static str> {
    use crate::call::{Event, Popup};
    if !app.layout.halocord.is(id::STREAM_DECK) {
        return Err("Stream Deck is off in HaloCord's plugins");
    }
    let active = app.client.state().voice.active();
    Ok(match press {
        Press::Mute => crate::account::update(app, crate::account::Event::Mute),
        Press::Deafen => crate::account::update(app, crate::account::Event::Deafen),
        Press::Camera => {
            if !active {
                return Err("Join a call first");
            }
            if !app.client.state().voice.can_film {
                return Err("No camera to film with");
            }
            crate::call::update(app, Event::Camera)
        }
        Press::Share => match crate::keybinds::go_live_key(app) {
            Some(event @ Event::Open(_)) => {
                // Go Live's menu wants a pick, so the window comes up.
                let open = crate::call::update(app, event);
                Task::batch([open, app.open_window()])
            }
            Some(event) => crate::call::update(app, event),
            None if app.call.popup == Some(Popup::Share) => {
                crate::call::popups::close_popup(app);
                Task::none()
            }
            None if active => return Err("This call can't take a stream"),
            None => return Err("Join a call first"),
        },
        Press::FakeMute => switch(app, id::FAKE_MUTE)?,
        Press::FakeDeafen => switch(app, id::FAKE_DEAFEN)?,
        Press::VoiceMode => {
            crate::keybinds::toggled(app, yapfast_core::keybinds::Action::ToggleVoiceMode)
        }
        Press::Leave => {
            if !active {
                return Err("Not in a call");
            }
            crate::call::update(app, Event::Leave)
        }
        Press::Display(screen) => display(app, screen)?,
    })
}

/// Flips the plugin with `id`, as the user area's buttons do.
fn switch(app: &mut App, id: &str) -> Result<Task<Message>, &'static str> {
    let plugin = PLUGINS
        .iter()
        .position(|plugin| plugin.id == id)
        .ok_or("That plugin isn't in this HaloCord")?;
    let on = !app.layout.halocord.is(id);
    Ok(crate::account::update(
        app,
        crate::account::Event::Plugin(Change::Switch { plugin, on }),
    ))
}

/// A screen's key: in a call, it goes live on that screen; live on
/// another, the stream moves to it; live on it, the stream stops.
fn display(app: &mut App, screen: u64) -> Result<Task<Message>, &'static str> {
    let voice = &app.client.state().voice;
    if !voice.active() {
        return Err("Join a call first");
    }
    let wanted = Source::Display(screen);
    let command = if voice.sharing.is_some() {
        if stream_worker::current_pick() == wanted {
            return Ok(crate::call::update(app, crate::call::Event::StopSharing));
        }
        stream_worker::pick(wanted);
        Command::SwitchLive
    } else {
        let quality = voice.quality;
        if !crate::call::stage::can_share(app.ctx()) {
            return Err("This call can't take a stream");
        }
        stream_worker::pick(wanted);
        Command::GoLive(quality)
    };
    if app.call.popup == Some(crate::call::Popup::Share) {
        crate::call::popups::close_popup(app);
    }
    app.client.dispatch(Action::Voice(command));
    Ok(app.sync())
}

/// The server's life: it runs while this stream is kept, and the window
/// hears each press from it.
fn listen() -> impl Stream<Item = Asked> + Send + 'static {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let server = Server::start(sender);
    stream::unfold(
        (receiver, server),
        |(mut receiver, server)| async move {
            let asked = receiver.recv().await?;
            Some((asked, (receiver, server)))
        },
    )
}

fn address() -> SocketAddr {
    SocketAddr::from((Ipv4Addr::LOCALHOST, PORT))
}

/// The server's thread; dropped, it stops and lets the port go.
struct Server {
    stop: Arc<AtomicBool>,
    bound: Arc<AtomicBool>,
}

impl Server {
    fn start(asks: tokio::sync::mpsc::UnboundedSender<Asked>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let bound = Arc::new(AtomicBool::new(false));
        let (stopped, binds) = (stop.clone(), bound.clone());
        let started = std::thread::Builder::new()
            .name("stream deck".into())
            .spawn(move || serve(&stopped, &binds, &asks));
        if let Err(error) = started {
            tracing::warn!(%error, "Stream Deck: no thread for the server");
        }
        Self { stop, bound }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if self.bound.load(Ordering::SeqCst) {
            // Wakes the waiting accept so the thread sees the stop.
            TcpStream::connect_timeout(&address(), Duration::from_millis(300)).ok();
        }
        TALK.let_go();
        MUTE_HOLD.let_go();
    }
}

fn serve(stop: &AtomicBool, bound: &AtomicBool, asks: &tokio::sync::mpsc::UnboundedSender<Asked>) {
    let mut warned = false;
    let listener = loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        match TcpListener::bind(address()) {
            Ok(listener) => break listener,
            Err(error) => {
                if !warned {
                    tracing::warn!(%error, port = PORT, "Stream Deck: the port is taken; trying again every 5 seconds");
                    warned = true;
                }
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    };
    bound.store(true, Ordering::SeqCst);
    tracing::info!(port = PORT, "Stream Deck: listening");
    for connection in listener.incoming() {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let Ok(connection) = connection else {
            continue;
        };
        if let Err(error) = answer(connection, asks) {
            tracing::debug!(%error, "Stream Deck: a request went wrong");
        }
    }
}

/// A request's head: its method, path and the headers that matter.
#[derive(Debug, Default, PartialEq, Eq)]
struct Head {
    method: String,
    path: String,
    host: Option<String>,
    deck: bool,
    origin: bool,
}

fn read_head(text: &str) -> Option<Head> {
    let mut lines = text.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let mut head = Head {
        method: first.next()?.to_owned(),
        path: first.next()?.to_owned(),
        ..Head::default()
    };
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match name.trim().to_ascii_lowercase().as_str() {
            "host" => head.host = Some(value.to_ascii_lowercase()),
            HEADER => head.deck = value == "1",
            "origin" => head.origin = true,
            _ => {}
        }
    }
    Some(head)
}

/// Whether a request is the deck's own: a GET on this computer's address,
/// with the deck's header and no web page behind it.
fn allowed(head: &Head) -> Result<(), (u16, &'static str)> {
    if head.method != "GET" {
        return Err((405, "only GET"));
    }
    let local = [format!("127.0.0.1:{PORT}"), format!("localhost:{PORT}")];
    if head.origin
        || !head.deck
        || !head.host.as_ref().is_some_and(|host| local.contains(host))
    {
        return Err((403, "not the deck"));
    }
    Ok(())
}

fn answer(
    mut connection: TcpStream,
    asks: &tokio::sync::mpsc::UnboundedSender<Asked>,
) -> std::io::Result<()> {
    connection.set_read_timeout(Some(ANSWER))?;
    connection.set_write_timeout(Some(ANSWER))?;
    let mut read = Vec::new();
    let mut buffer = [0; 1024];
    while !read.windows(4).any(|end| end == b"\r\n\r\n") {
        let count = connection.read(&mut buffer)?;
        if count == 0 || read.len() + count > MOST_HEAD {
            return Ok(());
        }
        read.extend_from_slice(&buffer[..count]);
    }
    let text = String::from_utf8_lossy(&read);
    let (status, body) = match read_head(&text) {
        None => (400, error("not a request")),
        Some(head) => match allowed(&head) {
            Err((status, why)) => (status, error(why)),
            Ok(()) => route(&head.path, asks),
        },
    };
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Method Not Allowed",
    };
    let reply = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    connection.write_all(reply.as_bytes())?;
    connection.flush()
}

fn error(why: &str) -> String {
    serde_json::json!({ "ok": false, "error": why }).to_string()
}

fn route(path: &str, asks: &tokio::sync::mpsc::UnboundedSender<Asked>) -> (u16, String) {
    if path == "/state" {
        return (200, state());
    }
    let Some(key) = path.strip_prefix("/press/") else {
        return (404, error("no such page"));
    };
    let press = match key {
        "talk-down" | "talk-up" => return held(&TALK, key == "talk-down"),
        "mute-hold-down" | "mute-hold-up" => return held(&MUTE_HOLD, key == "mute-hold-down"),
        "mute" => Press::Mute,
        "deafen" => Press::Deafen,
        "camera" => Press::Camera,
        "share" => Press::Share,
        "fake-mute" => Press::FakeMute,
        "fake-deafen" => Press::FakeDeafen,
        "voice-mode" => Press::VoiceMode,
        "leave" => Press::Leave,
        _ => match key.strip_prefix("screen-").and_then(|slot| slot.parse::<usize>().ok()) {
            Some(slot @ 1..=4) => match slots(&stream_worker::screen_places())[slot - 1] {
                Some(screen) => Press::Display(screen),
                None => return (200, error("No screen there")),
            },
            _ => return (400, error("no such key")),
        },
    };
    if lock(&SHOWN).is_none() {
        return (200, error("HaloCord is starting"));
    }
    let (answer, answered) = mpsc::channel();
    if asks.send(Asked { press, answer }).is_err() {
        return (200, error("HaloCord is closing"));
    }
    match answered.recv_timeout(ANSWER) {
        Ok(Ok(())) => (200, serde_json::json!({ "ok": true }).to_string()),
        Ok(Err(why)) => (200, error(&why)),
        Err(_) => (200, error("HaloCord didn't answer")),
    }
}

/// What the keys draw: the call's state and which screens are there.
fn state() -> String {
    let Some(shown) = *lock(&SHOWN) else {
        return error("HaloCord is starting");
    };
    let slots = slots(&stream_worker::screen_places());
    let screens: Vec<_> = slots
        .iter()
        .enumerate()
        .map(|(place, screen)| {
            serde_json::json!({
                "slot": place + 1,
                "present": screen.is_some(),
                "active": screen.is_some() && *screen == shown.shared_screen,
            })
        })
        .collect();
    serde_json::json!({
        "ok": true,
        "inCall": shown.in_call,
        "ready": shown.ready,
        "mute": shown.mute,
        "deafen": shown.deafen,
        "camera": shown.camera,
        "canFilm": shown.can_film,
        "sharing": shown.sharing,
        "canShare": shown.can_share,
        "fakeMute": shown.fake_mute,
        "fakeDeafen": shown.fake_deafen,
        "pttMode": shown.talk_mode,
        "talking": push_to_talk::held(),
        "muteHeld": push_to_talk::muting(),
        "screens": screens,
    })
    .to_string()
}

/// The four screen keys' screens: 2 is the main screen, 1 the nearest to
/// its left, 3 to its right and 4 above it; any others fill the empty keys
/// from left to right.
fn slots(places: &[ScreenPlace]) -> [Option<u64>; 4] {
    let mut slots = [None; 4];
    let Some(main) = places
        .iter()
        .find(|place| place.primary)
        .or_else(|| places.first())
    else {
        return slots;
    };
    slots[1] = Some(main.id);
    let centre = |place: &ScreenPlace| {
        (
            i64::from(place.x) + i64::from(place.width) / 2,
            i64::from(place.y) + i64::from(place.height) / 2,
        )
    };
    let (main_x, main_y) = centre(main);
    let mut nearest: [Option<(i64, &ScreenPlace)>; 4] = [None; 4];
    let mut rest = Vec::new();
    for place in places.iter().filter(|place| place.id != main.id) {
        let (x, y) = centre(place);
        let (across, down) = (x - main_x, y - main_y);
        let slot = if across.abs() >= down.abs() {
            Some(if across < 0 { 0 } else { 2 })
        } else if down < 0 {
            Some(3)
        } else {
            None
        };
        let distance = across * across + down * down;
        match slot {
            Some(slot) => match nearest[slot] {
                Some((was, _)) if was <= distance => rest.push(place),
                Some((_, farther)) => {
                    rest.push(farther);
                    nearest[slot] = Some((distance, place));
                }
                None => nearest[slot] = Some((distance, place)),
            },
            None => rest.push(place),
        }
    }
    rest.sort_by_key(|place| (place.x, place.y));
    let mut rest = rest.into_iter();
    for slot in [0, 2, 3] {
        slots[slot] = nearest[slot]
            .map(|(_, place)| place.id)
            .or_else(|| rest.next().map(|place| place.id));
    }
    slots
}

/// A key the deck holds down, which it repeats while held.
struct Hold {
    since: Mutex<Option<Instant>>,
    set: fn(bool),
}

static TALK: Hold = Hold {
    since: Mutex::new(None),
    set: push_to_talk::hold,
};
static MUTE_HOLD: Hold = Hold {
    since: Mutex::new(None),
    set: push_to_talk::hold_mute,
};

fn held(hold: &'static Hold, down: bool) -> (u16, String) {
    if lock(&SHOWN).is_none() {
        return (200, error("HaloCord is starting"));
    }
    if down {
        hold.down();
    } else {
        hold.let_go();
    }
    (200, serde_json::json!({ "ok": true }).to_string())
}

impl Hold {
    fn down(&'static self) {
        let first = lock(&self.since).replace(Instant::now()).is_none();
        (self.set)(true);
        if first {
            let watched = std::thread::Builder::new()
                .name("stream deck hold".into())
                .spawn(|| self.watch());
            if watched.is_err() {
                self.let_go();
            }
        }
    }

    /// Lets the key go, if the deck holds it.
    fn let_go(&self) {
        if lock(&self.since).take().is_some() {
            (self.set)(false);
        }
    }

    /// Lets go once the deck stops repeating, as when it was unplugged.
    fn watch(&self) {
        loop {
            std::thread::sleep(Duration::from_millis(500));
            let mut since = lock(&self.since);
            match *since {
                None => return,
                Some(at) if at.elapsed() > LET_GO => {
                    *since = None;
                    drop(since);
                    (self.set)(false);
                    return;
                }
                Some(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(id: u64, x: i32, y: i32, primary: bool) -> ScreenPlace {
        ScreenPlace {
            id,
            x,
            y,
            width: 1920,
            height: 1080,
            primary,
        }
    }

    #[test]
    fn the_main_screen_is_key_two_and_the_others_go_by_side() {
        let places = [
            screen(0, 0, 0, true),
            screen(1, -1920, 0, false),
            screen(2, 1920, 0, false),
            screen(3, 0, -1080, false),
        ];
        assert_eq!(slots(&places), [Some(1), Some(0), Some(2), Some(3)]);
    }

    #[test]
    fn screens_with_no_side_of_their_own_fill_the_empty_keys() {
        let places = [
            screen(5, 0, 0, true),
            screen(6, 1920, 0, false),
            screen(7, 3840, 0, false),
        ];
        assert_eq!(slots(&places), [Some(7), Some(5), Some(6), None]);
        assert_eq!(slots(&[]), [None; 4]);
        assert_eq!(slots(&[screen(9, 0, 0, false)]), [None, Some(9), None, None]);
    }

    #[test]
    fn only_the_decks_own_requests_get_in() {
        let head = |text: &str| read_head(text).unwrap();
        let deck = "GET /state HTTP/1.1\r\nHost: 127.0.0.1:47830\r\nX-HaloCord-Deck: 1\r\n\r\n";
        assert_eq!(allowed(&head(deck)), Ok(()));
        let page = "GET /press/mute HTTP/1.1\r\nHost: 127.0.0.1:47830\r\nX-HaloCord-Deck: 1\r\nOrigin: https://example.com\r\n\r\n";
        assert!(allowed(&head(page)).is_err());
        let bare = "GET /press/mute HTTP/1.1\r\nHost: localhost:47830\r\n\r\n";
        assert!(allowed(&head(bare)).is_err());
        let renamed = "GET /state HTTP/1.1\r\nHost: deck.example:47830\r\nX-HaloCord-Deck: 1\r\n\r\n";
        assert!(allowed(&head(renamed)).is_err());
        let posted = "POST /press/mute HTTP/1.1\r\nHost: localhost:47830\r\nX-HaloCord-Deck: 1\r\n\r\n";
        assert_eq!(allowed(&head(posted)), Err((405, "only GET")));
    }

    #[test]
    fn a_held_key_the_deck_stops_repeating_is_let_go() {
        use std::sync::atomic::AtomicU8;
        static LAST: AtomicU8 = AtomicU8::new(2);
        static PRESSED: Hold = Hold {
            since: Mutex::new(None),
            set: |down| LAST.store(u8::from(down), Ordering::SeqCst),
        };
        PRESSED.down();
        assert_eq!(LAST.load(Ordering::SeqCst), 1);
        PRESSED.let_go();
        assert_eq!(LAST.load(Ordering::SeqCst), 0);
        PRESSED.down();
        *lock(&PRESSED.since) = Some(Instant::now() - LET_GO - Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(1200));
        assert_eq!(LAST.load(Ordering::SeqCst), 0);
        assert!(lock(&PRESSED.since).is_none());
    }
}
