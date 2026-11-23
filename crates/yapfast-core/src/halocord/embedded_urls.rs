//! Embedded URLs: an Instagram, TikTok, X or Reddit post's link goes to
//! Discord with an embed fixer's host in place of its own (`fxtwitter.com`
//! and the others, as set), so its video or pictures show in chat. The
//! service's own tracking keys go with it; a link in `<angle brackets>`,
//! whose embed was asked away, stays as it is.

use std::{borrow::Cow, ops::Range, sync::RwLock};

/// The hosts each service's post links go to; an empty one leaves that
/// service's links alone.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fixers {
    pub instagram: String,
    pub tiktok: String,
    pub twitter: String,
    pub reddit: String,
}

static FIXERS: RwLock<Option<Fixers>> = RwLock::new(None);

impl Fixers {
    /// A site as written in the plugin's setting, as a host: without a
    /// scheme or a trailing `/`, so `https://fxtwitter.com/` works too.
    pub fn site(written: &str) -> String {
        let written = written.trim();
        let written = written
            .strip_prefix("https://")
            .or_else(|| written.strip_prefix("http://"))
            .unwrap_or(written);
        written.trim_end_matches('/').to_owned()
    }
}

/// The plugin's hosts while it's on, or `None` while it's off.
pub fn set(fixers: Option<Fixers>) {
    *FIXERS
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = fixers;
}

/// `text` with each post link sent to its fixer, while the plugin is on.
pub fn wire(text: &str) -> Cow<'_, str> {
    if !text.contains("http") {
        return Cow::Borrowed(text);
    }
    let fixers = FIXERS
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match fixers.as_ref() {
        Some(fixers) => rewrite(text, fixers),
        None => Cow::Borrowed(text),
    }
}

/// Each service: its hosts (without `www.`), whether a path is a post's,
/// and whether a query key is only tracking.
struct Service {
    hosts: &'static [&'static str],
    /// Whether a path, in parts split at `/`, is a post's, on this host.
    post: fn(&str, &[&str]) -> bool,
    tracking: fn(&str) -> bool,
}

/// Letters, digits and `_`, at least one.
fn word(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|char| char.is_alphanumeric() || char == '_')
}

fn word_or(text: &str, more: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|char| char.is_alphanumeric() || char == '_' || more.contains(char))
}

fn starts_with_digit(text: &str) -> bool {
    text.chars()
        .next()
        .is_some_and(|char| char.is_ascii_digit())
}

fn utm(key: &str) -> bool {
    key.strip_prefix("utm_").is_some_and(word)
}

const INSTAGRAM: Service = Service {
    hosts: &["instagram.com", "m.instagram.com"],
    // /p/…, /reel/…, /reels/…, /tv/…, /share/…, maybe after a user name.
    post: |_, parts| {
        let post = |at: usize| {
            parts
                .get(at)
                .is_some_and(|part| ["p", "reel", "reels", "tv", "share"].contains(part))
                && parts.len() > at + 1
        };
        post(1) || (parts.get(1).is_some_and(|user| word_or(user, ".")) && post(2))
    },
    tracking: |key| ["igsh", "igshid", "img_index"].contains(&key) || utm(key),
};

const TIKTOK: Service = Service {
    hosts: &[
        "tiktok.com",
        "m.tiktok.com",
        "vm.tiktok.com",
        "vt.tiktok.com",
    ],
    // /@user/video/123, /@user/photo/123, /t/code, or, on the short-link
    // hosts, a short code alone (www.tiktok.com/explore is no post).
    post: |host, parts| {
        let named = parts
            .get(1)
            .and_then(|part| part.strip_prefix('@'))
            .is_some_and(|user| word_or(user, ".-"))
            && parts
                .get(2)
                .is_some_and(|kind| ["video", "photo"].contains(kind))
            && parts.get(3).is_some_and(|id| starts_with_digit(id));
        let short = parts.get(1) == Some(&"t")
            && parts.get(2).is_some_and(|code| {
                code.chars()
                    .next()
                    .is_some_and(|char| char.is_alphanumeric() || char == '_')
            });
        let code = ["vm.tiktok.com", "vt.tiktok.com"].contains(&host)
            && parts
                .get(1)
                .is_some_and(|code| word(code) && code.chars().count() >= 6)
            && (parts.len() == 2 || (parts.len() == 3 && parts[2].is_empty()));
        named || short || code
    },
    tracking: |key| {
        ["_r", "_t", "is_from_webapp", "sender_device", "web_id"].contains(&key) || utm(key)
    },
};

const TWITTER: Service = Service {
    hosts: &["twitter.com", "x.com", "mobile.twitter.com", "mobile.x.com"],
    // /user/status/123
    post: |_, parts| {
        parts.get(1).is_some_and(|user| word(user))
            && parts.get(2) == Some(&"status")
            && parts.get(3).is_some_and(|id| starts_with_digit(id))
    },
    tracking: |key| ["s", "t", "ref_src"].contains(&key) || utm(key),
};

const REDDIT: Service = Service {
    hosts: &[
        "reddit.com",
        "old.reddit.com",
        "new.reddit.com",
        "m.reddit.com",
    ],
    // /r/name/comments/… or /r/name/s/…
    post: |_, parts| {
        parts.get(1) == Some(&"r")
            && parts.get(2).is_some_and(|name| word(name))
            && parts
                .get(3)
                .is_some_and(|kind| ["comments", "s"].contains(kind))
            && parts.len() > 4
    },
    // `context` (how many parent comments show) is the link's, not tracking.
    tracking: |key| ["share_id", "rdt"].contains(&key) || utm(key),
};

/// The link with its fixer's host in place of its own, or `None` to leave
/// it.
fn fix(link: &str, fixers: &Fixers) -> Option<String> {
    let rest = link
        .strip_prefix("https://")
        .or_else(|| link.strip_prefix("http://"))?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, rest) = rest.split_at(end);
    let host = authority
        .rsplit('@')
        .next()?
        .split(':')
        .next()?
        .to_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    let (rest, hash) = rest.split_at(rest.find('#').unwrap_or(rest.len()));
    let (path, query) = match rest.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (rest, None),
    };
    let path = if path.is_empty() { "/" } else { path };
    let parts: Vec<&str> = path.split('/').collect();
    for (target, service) in [
        (&fixers.instagram, INSTAGRAM),
        (&fixers.tiktok, TIKTOK),
        (&fixers.twitter, TWITTER),
        (&fixers.reddit, REDDIT),
    ] {
        let target = target.trim();
        if target.is_empty() || !service.hosts.contains(&host) || !(service.post)(host, &parts) {
            continue;
        }
        let kept: Vec<&str> = query
            .into_iter()
            .flat_map(|query| query.split('&'))
            .filter(|pair| {
                !pair.is_empty() && !(service.tracking)(pair.split('=').next().unwrap_or_default())
            })
            .collect();
        let search = if kept.is_empty() {
            String::new()
        } else {
            format!("?{}", kept.join("&"))
        };
        return Some(format!("https://{target}{path}{search}{hash}"));
    }
    None
}

/// Where `text` has code, a link already made (a masked link, or one in
/// `<angle brackets>`) or HTML: links there stay as typed, as
/// `links::linkify` leaves them.
fn skipped(text: &str) -> Vec<Range<usize>> {
    use pulldown_cmark::{Event, Options, Parser, Tag};
    Parser::new_ext(text, Options::ENABLE_STRIKETHROUGH)
        .into_offset_iter()
        .filter_map(|(event, range)| {
            matches!(
                event,
                Event::Start(
                    Tag::Link { .. } | Tag::Image { .. } | Tag::CodeBlock(_) | Tag::HtmlBlock
                ) | Event::Code(_)
                    | Event::Html(_)
                    | Event::InlineHtml(_)
            )
            .then_some(range)
        })
        .collect()
}

/// Every post link in `text` sent to its fixer, outside code and links.
fn rewrite<'a>(text: &'a str, fixers: &Fixers) -> Cow<'a, str> {
    let skipped = skipped(text);
    let mut out = String::with_capacity(text.len());
    let mut changed = false;
    let mut at = 0;
    while let Some(found) = text[at..].find("http") {
        let start = at + found;
        let before = text[..start].chars().next_back();
        let scheme = ["https://", "http://"]
            .into_iter()
            .find(|scheme| text[start..].starts_with(scheme));
        let boundary = before.is_none_or(|char| !(char.is_alphanumeric() || char == '_'));
        let Some(scheme) = scheme.filter(|_| boundary) else {
            out.push_str(&text[at..start + 4]);
            at = start + 4;
            continue;
        };
        let run = text[start..]
            .find(|char: char| char.is_whitespace() || char == '<' || char == '>')
            .map_or(text.len(), |end| start + end);
        // Punctuation after a link is the sentence's, not the link's.
        let mut end = run;
        while end > start + scheme.len()
            && text[..end]
                .chars()
                .next_back()
                .is_some_and(|char| ".,;:!?'\")]".contains(char))
        {
            end -= 1;
        }
        out.push_str(&text[at..start]);
        let link = &text[start..end];
        let left = before == Some('<')
            || skipped
                .iter()
                .any(|skip| skip.start < end && start < skip.end);
        match (end > start + scheme.len() + 1 && !left)
            .then(|| fix(link, fixers))
            .flatten()
        {
            Some(fixed) => {
                out.push_str(&fixed);
                changed = true;
            }
            None => out.push_str(link),
        }
        at = end;
    }
    if !changed {
        return Cow::Borrowed(text);
    }
    out.push_str(&text[at..]);
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixers() -> Fixers {
        Fixers {
            instagram: "zzinstagram.com".into(),
            tiktok: "tnktok.com".into(),
            twitter: "fxtwitter.com".into(),
            reddit: "vxreddit.com".into(),
        }
    }

    #[test]
    fn post_links_go_to_their_fixers() {
        let fixers = fixers();
        for (sent, goes) in [
            (
                "look https://x.com/someone/status/123?s=20&t=abc!",
                "look https://fxtwitter.com/someone/status/123!",
            ),
            (
                "https://www.instagram.com/reel/Cx1/?igsh=zz&keep=1",
                "https://zzinstagram.com/reel/Cx1/?keep=1",
            ),
            (
                "(https://www.tiktok.com/@a.b/video/7312?is_from_webapp=1)",
                "(https://tnktok.com/@a.b/video/7312)",
            ),
            (
                "https://vm.tiktok.com/ZMabcdef/",
                "https://tnktok.com/ZMabcdef/",
            ),
            (
                "https://old.reddit.com/r/rust/comments/1x/y/?utm_source=share#top",
                "https://vxreddit.com/r/rust/comments/1x/y/#top",
            ),
            // Reddit's `context` stays: it's the link's.
            (
                "https://www.reddit.com/r/rust/comments/1x/y/z/?context=3&share_id=a",
                "https://vxreddit.com/r/rust/comments/1x/y/z/?context=3",
            ),
            // Outside the code beside it.
            (
                "`code` then https://x.com/a/status/1",
                "`code` then https://fxtwitter.com/a/status/1",
            ),
        ] {
            assert_eq!(rewrite(sent, &fixers), goes, "{sent}");
        }
    }

    #[test]
    fn other_links_stay_as_they_are() {
        let fixers = fixers();
        for kept in [
            // A profile, not a post.
            "https://x.com/someone",
            // An embed asked away.
            "<https://x.com/a/status/1>",
            // Another site with the same path.
            "https://example.com/a/status/1",
            // Not a link's start.
            "nohttps://x.com/a/status/1",
            // In code, inline or fenced.
            "see `https://x.com/a/status/1` here",
            "```\nhttps://x.com/a/status/1\n```",
            // A masked link, and one masked with its own address.
            "[a clip](https://x.com/a/status/1)",
            "[https://x.com/a/status/1](https://x.com/a/status/1)",
            // Not a post: TikTok's own pages, short codes only on its short hosts.
            "https://www.tiktok.com/explore",
            "no links at all",
        ] {
            assert!(matches!(rewrite(kept, &fixers), Cow::Borrowed(_)), "{kept}");
        }
        let mut no_twitter = fixers;
        no_twitter.twitter.clear();
        let link = "https://x.com/a/status/1";
        assert_eq!(
            rewrite(link, &no_twitter),
            link,
            "an empty host leaves its service"
        );
    }

    #[test]
    fn a_site_is_written_as_a_host() {
        for written in [
            "fxtwitter.com",
            " https://fxtwitter.com/ ",
            "http://fxtwitter.com",
        ] {
            assert_eq!(Fixers::site(written), "fxtwitter.com", "{written}");
        }
        assert_eq!(Fixers::site(""), "");
    }
}
