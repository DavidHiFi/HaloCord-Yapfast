//! Notification title: a reply's notification says whom it replies to,
//! "Sam replied to Lena", where it says "Sam".

#[cfg(not(test))]
static ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
thread_local! {
    // Each test its own.
    static ON: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Turns the plugin on or off.
pub fn set(on: bool) {
    #[cfg(not(test))]
    ON.store(on, std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    ON.with(|cell| cell.set(on));
}

fn on() -> bool {
    #[cfg(not(test))]
    return ON.load(std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    ON.with(std::cell::Cell::get)
}

/// Whom a reply answers: this account, or someone by name.
pub enum Replied {
    You,
    Named(String),
}

/// A message's author as its notification names them: while the plugin is
/// on and it replies to someone (`replied`, asked only then), "`author`
/// replied to them", or "to you" when it answers this account.
pub fn author(author: String, replied: impl FnOnce() -> Option<Replied>) -> String {
    if !on() {
        return author;
    }
    match replied() {
        Some(Replied::You) => format!("{author} replied to you"),
        Some(Replied::Named(replied)) if !replied.is_empty() => {
            format!("{author} replied to {replied}")
        }
        _ => author,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_replys_notification_says_whom_it_replies_to() {
        let lena = || Some(Replied::Named("Lena".to_owned()));
        assert_eq!(author("Sam".into(), lena), "Sam", "off");
        set(true);
        assert_eq!(author("Sam".into(), lena), "Sam replied to Lena");
        assert_eq!(
            author("Sam".into(), || Some(Replied::You)),
            "Sam replied to you",
            "to this account"
        );
        assert_eq!(author("Sam".into(), || None), "Sam", "not a reply");
        assert_eq!(
            author("Sam".into(), || Some(Replied::Named(String::new()))),
            "Sam"
        );
        set(false);
    }
}
