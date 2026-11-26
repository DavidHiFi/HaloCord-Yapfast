//! HaloCord's own updates, from DavidHiFi/HaloCord-Yapfast's releases rather
//! than Yapfast's: which release a channel follows, what number a build is,
//! and whether a release is newer. The root crate asks GitHub and swaps the
//! program (`src/update/halocord.rs`); these are pure functions of what it
//! read, so tests cover them without a network.
//!
//! - **Stable** follows the newest normal release, tagged
//!   `<Yapfast's tag>-halocord.N`.
//! - **Nightly** follows the one pre-release tagged `nightly`, which the
//!   HaloCord Nightly workflow rebuilds from the `halocord` branch.
//!
//! A build's number comes from the name of its zip,
//! `HaloCord-Yapfast-<version>-<N>-windows-x64.zip` for a release and
//! `HaloCord-Yapfast-<version>-<N>-nightly.<YYYYMMDD>.<run>-windows-x64.zip`
//! for a nightly, where N is the release the nightly follows. So release N
//! comes before every nightly made after it, and those before release N+1.

/// The releases' own address on GitHub's API.
const API: &str = "https://api.github.com/repos/DavidHiFi/HaloCord-Yapfast/releases";

/// Where a release's page is, for the browser: only this project's.
pub const PAGES: &str = "https://github.com/DavidHiFi/HaloCord-Yapfast/releases/";

/// Where a release's files download from: only this project's.
const DOWNLOADS: &str = "https://github.com/DavidHiFi/HaloCord-Yapfast/releases/download/";

/// The nightly pre-release's tag, which every nightly build replaces.
pub const NIGHTLY_TAG: &str = "nightly";

/// The sums every release publishes beside its files.
pub const SUMS: &str = "SHA256SUMS.txt";

/// The release line an update follows, which the Updates page chooses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Stable,
    Nightly,
}

impl Channel {
    pub const ALL: [Channel; 2] = [Channel::Stable, Channel::Nightly];

    /// The one request a look makes for this channel.
    pub fn url(self) -> String {
        match self {
            Channel::Stable => format!("{API}/latest"),
            Channel::Nightly => format!("{API}/tags/{NIGHTLY_TAG}"),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Channel::Stable => "Stable",
            Channel::Nightly => "Nightly",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Channel::Stable => "Numbered HaloCord releases.",
            Channel::Nightly => {
                "A new build of the latest HaloCord changes every day. Expect some bugs."
            }
        }
    }

    /// The place in [`Channel::ALL`], for the picker.
    pub fn index(self) -> usize {
        Channel::ALL
            .iter()
            .position(|channel| *channel == self)
            .unwrap_or(0)
    }
}

/// When a nightly was built: the day, as `YYYYMMDD`, and the workflow's run
/// that day or after, which orders two of the same day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Nightly {
    pub date: u32,
    pub run: u32,
}

impl Nightly {
    /// Reads `YYYYMMDD.run`, as the workflow stamps a build.
    pub fn parse(stamp: &str) -> Option<Self> {
        let (date, run) = stamp.trim().split_once('.')?;
        let digits = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
        if date.len() != 8 || !digits(date) || !digits(run) {
            return None;
        }
        Some(Self {
            date: date.parse().ok()?,
            run: run.parse().ok()?,
        })
    }

    /// The stamp [`Nightly::parse`] reads.
    pub fn stamp(self) -> String {
        format!("{}.{}", self.date, self.run)
    }
}

/// Which HaloCord a build is: the release number, and for a nightly when it
/// was made after that release. Ordered as they come out: release N, then
/// N's nightlies by date and run, then release N+1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Build {
    pub number: u32,
    pub nightly: Option<Nightly>,
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

impl Build {
    /// Reads a release's zip name (see the module's notes).
    pub fn from_zip(name: &str) -> Option<Self> {
        let stem = name
            .strip_prefix("HaloCord-Yapfast-")?
            .strip_suffix("-windows-x64.zip")?;
        let (rest, last) = stem.rsplit_once('-')?;
        let (number, nightly) = match last.strip_prefix("nightly.") {
            Some(stamp) => (rest.rsplit_once('-')?.1, Some(Nightly::parse(stamp)?)),
            None => (last, None),
        };
        Some(Self {
            number: number_of(number)?,
            nightly,
        })
    }

    /// What the Updates page and the notification call it, which also tells one build
    /// from another when one is skipped.
    pub fn name(self) -> String {
        match self.nightly {
            None => format!("HaloCord {}", self.number),
            Some(nightly) => {
                let (year, month, day) = (
                    nightly.date / 10_000,
                    (nightly.date / 100) % 100,
                    nightly.date % 100,
                );
                let month = MONTHS
                    .get((month as usize).wrapping_sub(1))
                    .copied()
                    .unwrap_or("?");
                format!(
                    "HaloCord nightly {day} {month} {year}, build {}",
                    nightly.run
                )
            }
        }
    }
}

/// A release number: digits only.
pub fn number_of(text: &str) -> Option<u32> {
    let text = text.trim();
    (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
}

/// A file on a release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

/// The release a channel's look found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub channel: Channel,
    pub build: Build,
    /// [`Build::name`].
    pub name: String,
    pub page: String,
    /// What's new, as plain lines.
    pub notes: String,
    /// The Windows zip and the sums that check it.
    pub zip: Asset,
    pub sums: Option<Asset>,
}

/// Reads GitHub's answer for one release (`GET .../releases/latest` or
/// `.../releases/tags/nightly`), with the reason in plain words when it
/// isn't one this channel can install.
pub fn parse_release(json: &[u8], channel: Channel) -> Result<Release, String> {
    #[derive(serde::Deserialize)]
    struct Listed {
        tag_name: String,
        #[serde(default)]
        draft: bool,
        #[serde(default)]
        html_url: String,
        #[serde(default)]
        body: Option<String>,
        #[serde(default)]
        assets: Vec<ListedAsset>,
    }
    #[derive(serde::Deserialize)]
    struct ListedAsset {
        name: String,
        browser_download_url: String,
        #[serde(default)]
        size: u64,
    }
    let listed: Listed = serde_json::from_slice(json)
        .map_err(|_| "GitHub's answer wasn't a release.".to_owned())?;
    if listed.draft {
        return Err("The release isn't published yet.".into());
    }
    let ours = |asset: &ListedAsset| asset.browser_download_url.starts_with(DOWNLOADS);
    let asset = |asset: &ListedAsset| Asset {
        name: asset.name.clone(),
        url: asset.browser_download_url.clone(),
        size: asset.size,
    };
    let (zip, build) = listed
        .assets
        .iter()
        .filter(|candidate| ours(candidate))
        .find_map(|candidate| Some((asset(candidate), Build::from_zip(&candidate.name)?)))
        .ok_or("This release has no Windows download yet.")?;
    if build.nightly.is_some() != (channel == Channel::Nightly) {
        return Err(match channel {
            Channel::Stable => "The newest release is a nightly, not a stable one.".into(),
            Channel::Nightly => "The nightly release holds no nightly build.".into(),
        });
    }
    let sums = listed
        .assets
        .iter()
        .find(|candidate| candidate.name == SUMS && ours(candidate))
        .map(asset);
    let page = if listed.html_url.starts_with(PAGES) {
        listed.html_url
    } else {
        format!("{PAGES}tag/{}", listed.tag_name)
    };
    Ok(Release {
        channel,
        build,
        name: build.name(),
        page,
        notes: listed.body.as_deref().map(notes_of).unwrap_or_default(),
        zip,
        sums,
    })
}

/// Asks for the channel's one release through `fetch`, which gets the
/// address and answers with GitHub's status and body. The root crate passes
/// its web client; tests pass a fake answer.
pub async fn look_up<F, Answer>(channel: Channel, fetch: F) -> Result<Release, String>
where
    F: FnOnce(String) -> Answer,
    Answer: std::future::Future<Output = Result<(u16, Vec<u8>), String>>,
{
    let (status, body) = fetch(channel.url()).await?;
    match status {
        200..=299 => parse_release(&body, channel),
        404 => Err(match channel {
            Channel::Nightly => "There's no nightly build yet.".into(),
            Channel::Stable => "There's no HaloCord release yet.".into(),
        }),
        status => Err(format!("GitHub answered {status}.")),
    }
}

/// The longest notes kept, in characters; the release's page has the rest.
const NOTES_LIMIT: usize = 4000;

/// A release's Markdown as plain lines: headings and list marks dropped,
/// blank runs closed up, and cut short.
pub fn notes_of(body: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        let line = line.trim_start_matches('#').trim_start();
        let line = match line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            Some(item) => format!("• {}", item.trim()),
            None => line.replace("**", ""),
        };
        if line.is_empty() && lines.last().is_none_or(String::is_empty) {
            continue;
        }
        lines.push(line);
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    let notes = lines.join("\n");
    match notes.char_indices().nth(NOTES_LIMIT) {
        Some((cut, _)) => format!("{}…", &notes[..cut]),
        None => notes,
    }
}

/// What a look found, against this build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    UpToDate,
    Available,
    /// Newer, but the person skipped it.
    Skipped,
    /// A build from source is never offered a release.
    FromSource,
}

/// Compares `release` with this build: `current` is its number when it
/// knows it (one it doesn't know takes any release), and `released` whether
/// it came from a release at all. `skipped` is the name the person skipped.
pub fn decide(
    release: &Release,
    current: Option<Build>,
    released: bool,
    skipped: Option<&str>,
) -> Outcome {
    if !released {
        return Outcome::FromSource;
    }
    if current.is_some_and(|current| release.build <= current) {
        return Outcome::UpToDate;
    }
    if skipped == Some(release.name.as_str()) {
        Outcome::Skipped
    } else {
        Outcome::Available
    }
}

/// Reads `sha256sum`'s output into the sum for `file`.
pub fn sum_for(sums: &str, file: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, name) = line.split_once(char::is_whitespace)?;
        let name = name.trim().trim_start_matches('*');
        (name == file && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const STABLE_ZIP: &str = "HaloCord-Yapfast-0.5.0-nightly.20261120.19-18-windows-x64.zip";
    const NIGHTLY_ZIP: &str =
        "HaloCord-Yapfast-0.5.0-nightly.20261120.19-18-nightly.20261126.57-windows-x64.zip";

    fn stable(number: u32) -> Build {
        Build {
            number,
            nightly: None,
        }
    }

    fn nightly(number: u32, date: u32, run: u32) -> Build {
        Build {
            number,
            nightly: Some(Nightly { date, run }),
        }
    }

    fn listing(tag: &str, zip: &str) -> Vec<u8> {
        serde_json::json!({
            "tag_name": tag,
            "html_url": format!("{PAGES}tag/{tag}"),
            "body": "## What changed\n\n- **Updates** have a channel\n- Fixes\n\n\n",
            "assets": [
                {"name": zip, "browser_download_url": format!("{DOWNLOADS}{tag}/{zip}"), "size": 9},
                {"name": "HaloCord-Installer.exe", "browser_download_url": format!("{DOWNLOADS}{tag}/HaloCord-Installer.exe"), "size": 5},
                {"name": SUMS, "browser_download_url": format!("{DOWNLOADS}{tag}/{SUMS}"), "size": 100},
            ],
        })
        .to_string()
        .into_bytes()
    }

    #[test]
    fn each_channel_asks_for_its_own_release() {
        assert_eq!(
            Channel::Stable.url(),
            "https://api.github.com/repos/DavidHiFi/HaloCord-Yapfast/releases/latest"
        );
        assert_eq!(
            Channel::Nightly.url(),
            "https://api.github.com/repos/DavidHiFi/HaloCord-Yapfast/releases/tags/nightly"
        );
        assert_eq!(Channel::default(), Channel::Stable);
        assert_eq!(
            serde_json::to_string(&Channel::Nightly).expect("json"),
            "\"nightly\""
        );
        for channel in Channel::ALL {
            assert_eq!(Channel::ALL[channel.index()], channel);
        }
    }

    #[test]
    fn a_zip_names_its_build() {
        assert_eq!(Build::from_zip(STABLE_ZIP), Some(stable(18)));
        assert_eq!(Build::from_zip(NIGHTLY_ZIP), Some(nightly(18, 20261126, 57)));
        for odd in [
            "HaloCord-Installer.exe",
            "HaloCord-Yapfast-0.5.0-x-windows-x64.zip",
            "HaloCord-Yapfast-0.5.0-+18-windows-x64.zip",
            "HaloCord-Yapfast-0.5.0-18-nightly.2026112.5-windows-x64.zip",
            "HaloCord-Yapfast-0.5.0-18-linux-x64.zip",
        ] {
            assert_eq!(Build::from_zip(odd), None, "{odd}");
        }
    }

    #[test]
    fn nightlies_come_between_the_releases_around_them() {
        let mut builds = vec![
            stable(19),
            nightly(18, 20261127, 1),
            nightly(18, 20261126, 57),
            stable(18),
            nightly(18, 20261126, 3),
            nightly(17, 20261201, 9),
        ];
        builds.sort();
        assert_eq!(
            builds,
            [
                nightly(17, 20261201, 9),
                stable(18),
                nightly(18, 20261126, 3),
                nightly(18, 20261126, 57),
                nightly(18, 20261127, 1),
                stable(19),
            ]
        );
    }

    #[test]
    fn a_release_reads_with_its_files_and_notes() {
        let tag = "v0.5.0-nightly.20261120.19-halocord.18";
        let release = parse_release(&listing(tag, STABLE_ZIP), Channel::Stable).expect("release");
        assert_eq!(release.build, stable(18));
        assert_eq!(release.name, "HaloCord 18");
        assert_eq!(release.page, format!("{PAGES}tag/{tag}"));
        assert_eq!(release.zip.name, STABLE_ZIP);
        assert_eq!(release.sums.expect("sums").name, SUMS);
        assert_eq!(release.notes, "What changed\n\n• Updates have a channel\n• Fixes");
        let nightly = parse_release(&listing(NIGHTLY_TAG, NIGHTLY_ZIP), Channel::Nightly)
            .expect("nightly");
        assert_eq!(nightly.name, "HaloCord nightly 26 Nov 2026, build 57");
    }

    #[test]
    fn a_release_of_the_wrong_kind_or_elsewhere_is_refused() {
        assert!(parse_release(&listing(NIGHTLY_TAG, NIGHTLY_ZIP), Channel::Stable).is_err());
        assert!(parse_release(&listing("v1-halocord.18", STABLE_ZIP), Channel::Nightly).is_err());
        let elsewhere = listing("v1-halocord.18", STABLE_ZIP);
        let elsewhere = String::from_utf8(elsewhere)
            .expect("text")
            .replace("DavidHiFi/HaloCord-Yapfast/releases/download", "someone/else");
        assert!(parse_release(elsewhere.as_bytes(), Channel::Stable).is_err());
        assert!(parse_release(b"{\"message\":\"Not Found\"}", Channel::Nightly).is_err());
    }

    #[test]
    fn only_a_newer_build_is_offered() {
        let release =
            parse_release(&listing(NIGHTLY_TAG, NIGHTLY_ZIP), Channel::Nightly).expect("nightly");
        let name = release.name.clone();
        assert_eq!(decide(&release, Some(stable(18)), true, None), Outcome::Available);
        assert_eq!(
            decide(&release, Some(nightly(18, 20261126, 57)), true, None),
            Outcome::UpToDate
        );
        assert_eq!(decide(&release, Some(stable(19)), true, None), Outcome::UpToDate);
        assert_eq!(decide(&release, None, true, None), Outcome::Available);
        assert_eq!(
            decide(&release, Some(stable(18)), true, Some(&name)),
            Outcome::Skipped
        );
        assert_eq!(
            decide(&release, Some(stable(18)), false, None),
            Outcome::FromSource
        );
        // A nightly moving back to Stable waits for the next release.
        let stable_release =
            parse_release(&listing("v-halocord.18", STABLE_ZIP), Channel::Stable).expect("stable");
        assert_eq!(
            decide(&stable_release, Some(nightly(18, 20261126, 57)), true, None),
            Outcome::UpToDate
        );
    }

    #[test]
    fn sums_find_their_file() {
        let sum = "a".repeat(64);
        let sums = format!("{}  other.zip\n{sum} *{STABLE_ZIP}\n", "b".repeat(64));
        assert_eq!(sum_for(&sums, STABLE_ZIP), Some(sum));
        assert_eq!(sum_for(&sums, "missing.zip"), None);
        assert_eq!(sum_for("short  file.zip", "file.zip"), None);
    }

    /// Runs a look whose fake answer is ready at once, and says what
    /// address it asked for.
    fn look_with(channel: Channel, status: u16, body: Vec<u8>) -> (String, Result<Release, String>) {
        let asked = std::cell::RefCell::new(String::new());
        let look = look_up(channel, |url| {
            *asked.borrow_mut() = url;
            std::future::ready(Ok((status, body)))
        });
        let mut look = std::pin::pin!(look);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        let std::task::Poll::Ready(found) = look.as_mut().poll(&mut context) else {
            panic!("a ready answer finishes the look");
        };
        (asked.into_inner(), found)
    }

    #[test]
    fn a_look_asks_its_channel_and_reads_the_answer() {
        let tag = "v0.5.0-nightly.20261120.19-halocord.18";
        let (asked, found) = look_with(Channel::Stable, 200, listing(tag, STABLE_ZIP));
        assert_eq!(asked, Channel::Stable.url());
        assert_eq!(found.expect("stable").build, stable(18));
        let (asked, found) = look_with(Channel::Nightly, 200, listing(NIGHTLY_TAG, NIGHTLY_ZIP));
        assert!(asked.ends_with("/releases/tags/nightly"), "{asked}");
        let found = found.expect("nightly");
        assert_eq!(found.channel, Channel::Nightly);
        assert_eq!(found.build, nightly(18, 20261126, 57));
    }

    #[test]
    fn a_look_says_why_it_found_nothing() {
        let missing = b"{\"message\":\"Not Found\"}".to_vec();
        assert_eq!(
            look_with(Channel::Nightly, 404, missing.clone()).1.err().as_deref(),
            Some("There's no nightly build yet.")
        );
        assert_eq!(
            look_with(Channel::Stable, 404, missing).1.err().as_deref(),
            Some("There's no HaloCord release yet.")
        );
        assert_eq!(
            look_with(Channel::Stable, 500, Vec::new()).1.err().as_deref(),
            Some("GitHub answered 500.")
        );
        assert_eq!(
            look_with(Channel::Stable, 200, b"<html>".to_vec()).1.err().as_deref(),
            Some("GitHub's answer wasn't a release.")
        );
    }

    #[test]
    fn a_nightly_stamp_reads_back() {
        let stamp = Nightly::parse("20261126.57").expect("stamp");
        assert_eq!(stamp.stamp(), "20261126.57");
        assert_eq!(Nightly::parse("2026-11-26.57"), None);
        assert_eq!(number_of("18"), Some(18));
        assert_eq!(number_of(""), None);
    }
}
