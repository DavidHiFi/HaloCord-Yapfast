//! HaloCord's two presence plugins. Custom RPC shows an activity written in
//! the plugin's settings, beside the detected game and the music. Animated
//! Status turns the custom status text through a list of lines, in the
//! presence only: Discord's own settings are never saved, since quick saves
//! get rate limited. The window sets what the plugins hold with
//! [`set_custom_rpc`] and [`set_animated`]; each session keeps a [`Watch`],
//! which puts them in the presence it sends and says when to send again.
//! `docs/halocord.md` lists the hooks in the session.

use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};

use serde_json::{Value, json};
use tokio::{
    sync::watch,
    time::{Instant, Sleep},
};

use crate::api::Api;

/// The least Animated Status waits between two lines, in seconds.
pub const LEAST_EVERY: u32 = 30;
/// How long a change to a setting waits for the next one before it is sent,
/// so typing is one presence update and not one for each letter.
const SETTLE: Duration = Duration::from_millis(1500);
/// Discord cuts a status or an activity's lines here.
const LINE: usize = 128;
/// Discord cuts a button's label here.
const LABEL: usize = 32;

/// The activity types, as the settings keep them and the gateway sends them.
pub const PLAYING: u32 = 0;
pub const STREAMING: u32 = 1;
pub const LISTENING: u32 = 2;
pub const WATCHING: u32 = 3;
pub const COMPETING: u32 = 5;

/// What Started puts in the activity's timer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Started {
    #[default]
    Never,
    /// Since Yapfast started.
    Launch,
    /// Since the plugin was switched on.
    TurnedOn,
}

impl Started {
    pub fn from_number(number: u32) -> Self {
        match number {
            1 => Self::Launch,
            2 => Self::TurnedOn,
            _ => Self::Never,
        }
    }
}

/// What the Custom RPC plugin's settings hold.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rpc {
    pub application: String,
    pub kind: u32,
    pub name: String,
    pub details: String,
    pub state: String,
    pub large_image: String,
    pub large_text: String,
    pub small_image: String,
    pub small_text: String,
    pub started: Started,
    /// Each button's label and link.
    pub buttons: [(String, String); 2],
    pub stream_link: String,
}

/// What the Animated Status plugin's settings hold.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Animated {
    /// The lines, separated by `|`.
    pub lines: String,
    pub every_seconds: u32,
    pub emoji: String,
}

impl Animated {
    /// The lines worth showing: trimmed, the empty ones skipped.
    pub fn shown(&self) -> Vec<&str> {
        self.lines
            .split('|')
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect()
    }

    /// The wait between two lines, never under [`LEAST_EVERY`].
    pub fn every(&self) -> Duration {
        Duration::from_secs(u64::from(self.every_seconds.max(LEAST_EVERY)))
    }

    /// The line to show at `step`, going round the list; none when there
    /// are no lines.
    pub fn line(&self, step: usize) -> Option<&str> {
        let shown = self.shown();
        (!shown.is_empty()).then(|| shown[step % shown.len()])
    }

    /// The custom status activity that shows `line`.
    fn activity(&self, line: &str) -> Value {
        let mut activity = json!({"type": 4, "name": "Custom Status", "state": cut(line, LINE)});
        let emoji = self.emoji.trim();
        if !emoji.is_empty() {
            activity["emoji"] = json!({"name": emoji, "id": null, "animated": false});
        }
        activity
    }
}

/// What the two plugins hold now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    pub rpc: Option<Rpc>,
    /// When Custom RPC was switched on, in Unix milliseconds.
    pub rpc_since: u64,
    pub animated: Option<Animated>,
}

static CONFIG: LazyLock<watch::Sender<Config>> =
    LazyLock::new(|| watch::channel(Config::default()).0);

/// When this run began, for Started's "when Yapfast started".
static LAUNCHED: LazyLock<u64> = LazyLock::new(crate::service::now_ms);

/// Sets the activity Custom RPC shows, or none when the plugin is off. It
/// counts from the first time it is set while on.
pub fn set_custom_rpc(rpc: Option<Rpc>) {
    LazyLock::force(&LAUNCHED);
    CONFIG.send_if_modified(|config| {
        let since = if config.rpc.is_none() && rpc.is_some() {
            crate::service::now_ms()
        } else {
            config.rpc_since
        };
        if config.rpc == rpc && config.rpc_since == since {
            return false;
        }
        config.rpc = rpc;
        config.rpc_since = since;
        true
    });
}

/// What the plugins hold now.
pub fn config() -> Config {
    CONFIG.borrow().clone()
}

/// Sets the lines Animated Status turns through, or none when it is off.
pub fn set_animated(animated: Option<Animated>) {
    CONFIG.send_if_modified(|config| {
        if config.animated == animated {
            return false;
        }
        config.animated = animated;
        true
    });
}

/// Outside pictures' keys, by application and link. A link Discord is still
/// asked about, or couldn't give a key for, holds none.
static KEYS: LazyLock<Mutex<Keys>> = LazyLock::new(Default::default);

type Keys = HashMap<(String, String), Option<String>>;

/// The key Discord gave for an outside picture, once it has.
fn key_of(application: &str, link: &str) -> Option<String> {
    KEYS.lock()
        .ok()?
        .get(&(application.to_owned(), link.to_owned()))
        .cloned()
        .flatten()
}

fn is_link(text: &str) -> bool {
    text.starts_with("https://") || text.starts_with("http://")
}

/// Whether `text` is an application id: digits only, so it can sit in a path.
fn is_application(text: &str) -> bool {
    !text.is_empty() && text.len() <= 20 && text.bytes().all(|byte| byte.is_ascii_digit())
}

fn cut(text: &str, most: usize) -> String {
    text.trim().chars().take(most).collect()
}

/// A picture setting as the activity names it: an asset key as it is, or the
/// key `lookup` has for a link.
fn picture(
    application: &str,
    value: &str,
    lookup: &dyn Fn(&str, &str) -> Option<String>,
) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else if is_link(value) {
        lookup(application, value)
    } else {
        Some(value.to_owned())
    }
}

/// Whether `link` is one Discord takes for a stream: Twitch or YouTube.
fn is_stream_link(link: &str) -> bool {
    let Ok(link) = url::Url::parse(link.trim()) else {
        return false;
    };
    matches!(link.scheme(), "https" | "http")
        && link.host_str().is_some_and(|host| {
            let host = host.trim_start_matches("www.");
            ["twitch.tv", "youtube.com", "youtu.be"].contains(&host)
        })
}

fn is_web_link(link: &str) -> bool {
    url::Url::parse(link.trim()).is_ok_and(|link| matches!(link.scheme(), "https" | "http"))
}

impl Rpc {
    /// The activity for the gateway, as Discord's client sends one; none
    /// without a name. `launched` and `since` are Unix milliseconds, and
    /// `lookup` gives the key of an outside picture.
    pub fn activity(
        &self,
        launched: u64,
        since: u64,
        lookup: &dyn Fn(&str, &str) -> Option<String>,
    ) -> Option<Value> {
        let name = cut(&self.name, LINE);
        if name.is_empty() {
            return None;
        }
        let application = self.application.trim();
        let application = is_application(application).then_some(application);
        // Streaming wants a Twitch or YouTube link; without one it plays.
        let streaming = self.kind == STREAMING && is_stream_link(&self.stream_link);
        let kind = match self.kind {
            LISTENING | WATCHING | COMPETING => self.kind,
            _ if streaming => STREAMING,
            _ => PLAYING,
        };
        let mut activity = json!({"type": kind, "name": name});
        if streaming {
            activity["url"] = json!(self.stream_link.trim());
        }
        for (key, text) in [("details", &self.details), ("state", &self.state)] {
            let text = cut(text, LINE);
            if !text.is_empty() {
                activity[key] = json!(text);
            }
        }
        match self.started {
            Started::Never => {}
            Started::Launch => activity["timestamps"] = json!({"start": launched}),
            Started::TurnedOn => activity["timestamps"] = json!({"start": since}),
        }
        let Some(application) = application else {
            return Some(activity);
        };
        activity["application_id"] = json!(application);
        let mut assets = json!({});
        for (key, image, text_key, text) in [
            (
                "large_image",
                &self.large_image,
                "large_text",
                &self.large_text,
            ),
            (
                "small_image",
                &self.small_image,
                "small_text",
                &self.small_text,
            ),
        ] {
            let Some(image) = picture(application, image, lookup) else {
                continue;
            };
            assets[key] = json!(image);
            let text = cut(text, LINE);
            if !text.is_empty() {
                assets[text_key] = json!(text);
            }
        }
        if assets.as_object().is_some_and(|assets| !assets.is_empty()) {
            activity["assets"] = assets;
        }
        let buttons: Vec<(String, &str)> = self
            .buttons
            .iter()
            .filter(|(label, link)| !label.trim().is_empty() && is_web_link(link))
            .map(|(label, link)| (cut(label, LABEL), link.trim()))
            .collect();
        if !buttons.is_empty() {
            activity["buttons"] = json!(buttons.iter().map(|(label, _)| label).collect::<Vec<_>>());
            activity["metadata"] =
                json!({"button_urls": buttons.iter().map(|(_, link)| link).collect::<Vec<_>>()});
        }
        Some(activity)
    }

    /// The outside pictures this activity would use that Discord has not
    /// been asked about, claimed so that nothing asks twice.
    fn wanted(&self) -> Vec<(String, String)> {
        let application = self.application.trim();
        if !is_application(application) {
            return Vec::new();
        }
        let Ok(mut keys) = KEYS.lock() else {
            return Vec::new();
        };
        let mut wanted = Vec::new();
        for image in [&self.large_image, &self.small_image] {
            let image = image.trim();
            let key = (application.to_owned(), image.to_owned());
            if is_link(image) && !keys.contains_key(&key) {
                keys.insert(key.clone(), None);
                wanted.push(key);
            }
        }
        wanted
    }
}

/// Asks Discord for the keys of outside pictures, then wakes the sessions to
/// send the activity again. A picture it can't get a key for stays out.
fn ask(api: Arc<Api>, wanted: Vec<(String, String)>) {
    if wanted.is_empty() || tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    tokio::spawn(async move {
        let application = wanted[0].0.clone();
        let links: Vec<String> = wanted.iter().map(|(_, link)| link.clone()).collect();
        match api.external_assets(&application, &links).await {
            Ok(answer) => {
                let found: Vec<(String, String)> = answer
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|asset| {
                        Some((
                            asset["url"].as_str()?.to_owned(),
                            asset["external_asset_path"].as_str()?.to_owned(),
                        ))
                    })
                    .collect();
                if let Ok(mut keys) = KEYS.lock() {
                    for (link, path) in found {
                        keys.insert((application.clone(), link), Some(format!("mp:{path}")));
                    }
                }
                CONFIG.send_modify(|_| {});
            }
            Err(error) => tracing::warn!(%error, "could not get a key for the picture"),
        }
    });
}

/// Puts what the plugins show in `presence`: the status line Animated Status
/// is on, and the activity Custom RPC writes after the game and the music.
/// An invisible account shares no activity.
fn put(
    presence: &mut Value,
    config: &Config,
    step: usize,
    launched: u64,
    lookup: &dyn Fn(&str, &str) -> Option<String>,
) {
    let invisible = presence["status"] == "invisible";
    let Some(activities) = presence["activities"].as_array_mut() else {
        return;
    };
    if let Some(animated) = &config.animated
        && let Some(line) = animated.line(step)
    {
        activities.retain(|activity| activity["type"] != 4);
        activities.insert(0, animated.activity(line));
    }
    if !invisible
        && let Some(rpc) = &config.rpc
        && let Some(activity) = rpc.activity(launched, config.rpc_since, lookup)
    {
        activities.push(activity);
    }
}

/// [`put`] for the presence a session identifies with, on its first line.
pub(crate) fn identify(presence: &mut Value) {
    let config = CONFIG.borrow().clone();
    put(presence, &config, 0, *LAUNCHED, &key_of);
}

/// Which plugins are on; a change of this is sent at once, and a change of
/// what they say waits for the typing to stop.
fn shape(config: &Config) -> (bool, bool) {
    (config.rpc.is_some(), config.animated.is_some())
}

/// What a session keeps for the two plugins: the settings as they change,
/// the line it is on, and the one timer that sends the next.
pub(crate) struct Watch {
    config: watch::Receiver<Config>,
    api: Option<Arc<Api>>,
    shape: (bool, bool),
    step: usize,
    settle: Pin<Box<Sleep>>,
    settling: bool,
    tick: Pin<Box<Sleep>>,
    ticking: bool,
}

impl Watch {
    pub(crate) fn new(api: Arc<Api>) -> Self {
        Self::with(CONFIG.subscribe(), Some(api))
    }

    fn with(config: watch::Receiver<Config>, api: Option<Arc<Api>>) -> Self {
        let shape = shape(&config.borrow());
        Self {
            config,
            api,
            shape,
            step: 0,
            settle: Box::pin(tokio::time::sleep(Duration::ZERO)),
            settling: false,
            tick: Box::pin(tokio::time::sleep(Duration::ZERO)),
            ticking: false,
        }
    }

    /// Waits until the presence should be sent again: a setting changed and
    /// settled, or the time came for the next line. Idle while neither
    /// plugin has anything to say.
    pub(crate) async fn due(&mut self) {
        loop {
            tokio::select! {
                changed = self.config.changed() => {
                    if changed.is_err() {
                        std::future::pending::<()>().await;
                    }
                    let shape = shape(&self.config.borrow());
                    let wait = if shape == self.shape { SETTLE } else { Duration::ZERO };
                    self.shape = shape;
                    // A new list starts from its first line.
                    self.step = 0;
                    self.ticking = false;
                    self.settle.as_mut().reset(Instant::now() + wait);
                    self.settling = true;
                }
                () = self.settle.as_mut(), if self.settling => {
                    self.settling = false;
                    return;
                }
                () = self.tick.as_mut(), if self.ticking => {
                    self.ticking = false;
                    self.step += 1;
                    return;
                }
            }
        }
    }

    /// [`put`] for the presence about to be sent, and the timer for the next
    /// line when there are two or more.
    pub(crate) fn apply(&mut self, presence: &mut Value) {
        let config = self.config.borrow().clone();
        if let (Some(rpc), Some(api)) = (&config.rpc, &self.api) {
            ask(api.clone(), rpc.wanted());
        }
        put(presence, &config, self.step, *LAUNCHED, &key_of);
        let every = config
            .animated
            .as_ref()
            .filter(|animated| animated.shown().len() > 1)
            .map(Animated::every);
        match every {
            Some(every) if !self.ticking => {
                self.tick.as_mut().reset(Instant::now() + every);
                self.ticking = true;
            }
            Some(_) => {}
            None => self.ticking = false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str, _: &str) -> Option<String> {
        None
    }

    fn rpc() -> Rpc {
        Rpc {
            name: "Halo".into(),
            ..Rpc::default()
        }
    }

    fn make(rpc: &Rpc) -> Option<Value> {
        rpc.activity(1_000, 2_000, &none)
    }

    #[test]
    fn each_type_has_its_number() {
        for (kind, number) in [(PLAYING, 0), (LISTENING, 2), (WATCHING, 3), (COMPETING, 5)] {
            let activity = make(&Rpc { kind, ..rpc() }).unwrap();
            assert_eq!(activity["type"], number);
            assert_eq!(activity["name"], "Halo");
        }
        let stream = Rpc {
            kind: STREAMING,
            stream_link: "https://www.twitch.tv/halo".into(),
            ..rpc()
        };
        let activity = make(&stream).unwrap();
        assert_eq!(activity["type"], 1);
        assert_eq!(activity["url"], "https://www.twitch.tv/halo");
    }

    #[test]
    fn streaming_without_a_twitch_or_youtube_link_plays() {
        for link in [
            "",
            "twitch",
            "https://example.com/live",
            "ftp://twitch.tv/x",
        ] {
            let stream = Rpc {
                kind: STREAMING,
                stream_link: link.into(),
                ..rpc()
            };
            let activity = make(&stream).unwrap();
            assert_eq!(activity["type"], 0, "{link}");
            assert!(activity.get("url").is_none(), "{link}");
        }
        for link in ["https://youtu.be/abc", "https://youtube.com/watch?v=1"] {
            let stream = Rpc {
                kind: STREAMING,
                stream_link: link.into(),
                ..rpc()
            };
            assert_eq!(make(&stream).unwrap()["type"], 1, "{link}");
        }
    }

    #[test]
    fn empty_fields_are_left_out_and_a_nameless_activity_is_none() {
        let activity = make(&rpc()).unwrap();
        assert_eq!(activity, json!({"type": 0, "name": "Halo"}));
        assert!(make(&Rpc::default()).is_none());
        assert!(
            make(&Rpc {
                name: "  ".into(),
                ..rpc()
            })
            .is_none()
        );
        let full = make(&Rpc {
            details: " In a match ".into(),
            state: "Round 3".into(),
            ..rpc()
        })
        .unwrap();
        assert_eq!(full["details"], "In a match");
        assert_eq!(full["state"], "Round 3");
    }

    #[test]
    fn the_timer_starts_when_asked() {
        let started = |started| make(&Rpc { started, ..rpc() }).unwrap();
        assert!(started(Started::Never).get("timestamps").is_none());
        assert_eq!(started(Started::Launch)["timestamps"]["start"], 1_000);
        assert_eq!(started(Started::TurnedOn)["timestamps"]["start"], 2_000);
    }

    #[test]
    fn pictures_and_buttons_need_an_application_id() {
        let full = Rpc {
            large_image: "big".into(),
            large_text: "Big one".into(),
            small_image: "mp:external/abc/small.png".into(),
            small_text: "Small".into(),
            buttons: [
                ("Watch".into(), "https://example.com/a".into()),
                ("Join".into(), "https://example.com/b".into()),
            ],
            ..rpc()
        };
        let without = make(&full).unwrap();
        assert!(without.get("assets").is_none());
        assert!(without.get("buttons").is_none());
        assert!(without.get("application_id").is_none());
        let with = make(&Rpc {
            application: "123456789".into(),
            ..full.clone()
        })
        .unwrap();
        assert_eq!(with["application_id"], "123456789");
        assert_eq!(with["assets"]["large_image"], "big");
        assert_eq!(with["assets"]["large_text"], "Big one");
        assert_eq!(with["assets"]["small_image"], "mp:external/abc/small.png");
        assert_eq!(with["assets"]["small_text"], "Small");
        assert_eq!(with["buttons"], json!(["Watch", "Join"]));
        assert_eq!(
            with["metadata"]["button_urls"],
            json!(["https://example.com/a", "https://example.com/b"])
        );
        // Not a number: left out, so it can't reach a path.
        let odd = make(&Rpc {
            application: "12/../3".into(),
            ..full
        })
        .unwrap();
        assert!(odd.get("application_id").is_none());
    }

    #[test]
    fn a_button_needs_a_label_and_a_web_link() {
        let activity = make(&Rpc {
            application: "1".into(),
            buttons: [
                ("".into(), "https://example.com/a".into()),
                ("Join".into(), "not a link".into()),
            ],
            ..rpc()
        })
        .unwrap();
        assert!(activity.get("buttons").is_none());
        assert!(activity.get("metadata").is_none());
        let one = make(&Rpc {
            application: "1".into(),
            buttons: [
                ("".into(), "".into()),
                ("Join".into(), "https://example.com/b".into()),
            ],
            ..rpc()
        })
        .unwrap();
        assert_eq!(one["buttons"], json!(["Join"]));
        assert_eq!(
            one["metadata"]["button_urls"],
            json!(["https://example.com/b"])
        );
    }

    #[test]
    fn an_outside_link_shows_by_the_key_discord_gave_and_waits_without_one() {
        let rpc = Rpc {
            application: "5".into(),
            large_image: "https://example.com/p.png".into(),
            ..rpc()
        };
        assert!(make(&rpc).unwrap().get("assets").is_none());
        let got = |application: &str, link: &str| {
            assert_eq!((application, link), ("5", "https://example.com/p.png"));
            Some("mp:external/k/p.png".to_owned())
        };
        let activity = rpc.activity(1, 2, &got).unwrap();
        assert_eq!(activity["assets"]["large_image"], "mp:external/k/p.png");
    }

    fn presence(status: &str) -> Value {
        json!({
            "status": status,
            "since": 0,
            "afk": false,
            "activities": [
                {"type": 4, "name": "Custom Status", "state": "Real"},
                {"type": 0, "name": "Game"},
                {"type": 2, "name": "Spotify"},
            ],
        })
    }

    fn names(presence: &Value) -> Vec<String> {
        presence["activities"]
            .as_array()
            .unwrap()
            .iter()
            .map(|activity| activity["name"].as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn the_activity_shows_beside_the_game_and_the_music() {
        let config = Config {
            rpc: Some(rpc()),
            ..Config::default()
        };
        let mut shown = presence("online");
        put(&mut shown, &config, 0, 1, &none);
        assert_eq!(names(&shown), ["Custom Status", "Game", "Spotify", "Halo"]);
    }

    #[test]
    fn an_invisible_account_shares_no_activity() {
        let config = Config {
            rpc: Some(rpc()),
            ..Config::default()
        };
        let mut shown = presence("invisible");
        put(&mut shown, &config, 0, 1, &none);
        assert_eq!(names(&shown), ["Custom Status", "Game", "Spotify"]);
    }

    fn animated(lines: &str, every_seconds: u32) -> Animated {
        Animated {
            lines: lines.into(),
            every_seconds,
            emoji: String::new(),
        }
    }

    #[test]
    fn the_next_line_skips_empty_ones_and_wraps() {
        let lines = animated(" one | |two||  three ", 60);
        assert_eq!(lines.shown(), ["one", "two", "three"]);
        let at = |step| lines.line(step).unwrap();
        assert_eq!(
            [at(0), at(1), at(2), at(3), at(4)],
            ["one", "two", "three", "one", "two"]
        );
        assert_eq!(animated("| |", 60).line(0), None);
        assert_eq!(animated("", 60).line(7), None);
    }

    #[test]
    fn the_wait_is_never_under_thirty_seconds() {
        assert_eq!(animated("a", 0).every(), Duration::from_secs(30));
        assert_eq!(animated("a", 12).every(), Duration::from_secs(30));
        assert_eq!(animated("a", 30).every(), Duration::from_secs(30));
        assert_eq!(animated("a", 90).every(), Duration::from_secs(90));
    }

    #[test]
    fn the_line_takes_the_custom_status_place_and_the_real_one_returns_when_off() {
        let mut lines = animated("one|two", 60);
        lines.emoji = " X ".into();
        let config = Config {
            animated: Some(lines),
            ..Config::default()
        };
        let state = |presence: &Value| presence["activities"][0]["state"].clone();
        let mut shown = presence("online");
        put(&mut shown, &config, 1, 1, &none);
        assert_eq!(state(&shown), "two");
        assert_eq!(shown["activities"][0]["type"], 4);
        assert_eq!(shown["activities"][0]["emoji"]["name"], "X");
        assert_eq!(names(&shown), ["Custom Status", "Game", "Spotify"]);
        // Off: the presence is as the real status made it.
        let mut real = presence("online");
        put(&mut real, &Config::default(), 1, 1, &none);
        assert_eq!(state(&real), "Real");
        assert!(real["activities"][0].get("emoji").is_none());
        // With no custom status at all, the line still shows.
        let mut bare = json!({"status": "online", "activities": []});
        put(&mut bare, &config, 0, 1, &none);
        assert_eq!(state(&bare), "one");
    }

    #[test]
    fn a_session_steps_through_the_lines_with_one_timer() {
        let (sender, receiver) = watch::channel(Config {
            animated: Some(animated("a|b", 5)),
            ..Config::default()
        });
        let sent = |watch: &mut Watch| {
            let mut shown = presence("online");
            watch.apply(&mut shown);
            shown["activities"][0]["state"].as_str().unwrap().to_owned()
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut watch = Watch::with(receiver, None);
            assert_eq!(sent(&mut watch), "a");
            assert!(watch.ticking);
            // Sending for another reason leaves the timer where it was.
            assert_eq!(sent(&mut watch), "a");
            watch.step += 1;
            assert_eq!(sent(&mut watch), "b");
            watch.step += 1;
            assert_eq!(sent(&mut watch), "a");
            // One line has nothing to turn to, and none puts the real one back.
            sender.send_replace(Config {
                animated: Some(animated("only", 60)),
                ..Config::default()
            });
            assert_eq!(sent(&mut watch), "only");
            assert!(!watch.ticking);
            sender.send_replace(Config::default());
            assert_eq!(sent(&mut watch), "Real");
            assert!(!watch.ticking);
        });
    }

    #[tokio::test]
    async fn switching_a_plugin_on_or_off_is_due_at_once() {
        let (sender, receiver) = watch::channel(Config::default());
        let mut watch = Watch::with(receiver, None);
        sender.send_replace(Config {
            animated: Some(animated("a|b", 60)),
            ..Config::default()
        });
        tokio::time::timeout(Duration::from_millis(500), watch.due())
            .await
            .expect("due at once");
        sender.send_replace(Config::default());
        tokio::time::timeout(Duration::from_millis(500), watch.due())
            .await
            .expect("due at once when off");
        // Idle with nothing changing: not due.
        assert!(
            tokio::time::timeout(Duration::from_millis(100), watch.due())
                .await
                .is_err()
        );
    }
}
