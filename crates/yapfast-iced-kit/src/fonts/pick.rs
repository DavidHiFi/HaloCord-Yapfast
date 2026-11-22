//! HaloCord's font choice: the window's text in a font the person picked,
//! at a weight they picked, with Yapfast's own font behind it.
//!
//! The window names its text "DM Sans" in about five hundred places, at four
//! weights. Rather than change each, [`add`] loads the chosen family's files
//! and gives their faces that name, one per weight the window uses, with
//! Yapfast's own font renamed [`OWN`] and left as the fallback. A family that
//! can't be found leaves everything as it was.

use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use iced::advanced::graphics::text::cosmic_text::fontdb::{self, Database, Weight};

/// What Yapfast's own font is called while another font takes "DM Sans".
pub const OWN: &str = "Yapfast Sans";
/// The name the window draws with.
const WINDOW_NAME: &str = "DM Sans";
/// The weights the window draws with: regular, medium, semibold, bold.
const SLOTS: [u16; 4] = [400, 500, 600, 700];

/// The choice made before the window opens.
static CHOICE: Mutex<Option<Choice>> = Mutex::new(None);

#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    /// The family's name, as in the font's own list, such as "FiraCode Nerd
    /// Font". Empty keeps Yapfast's own.
    pub family: String,
    /// The weight the regular text takes, 100 to 900; the others follow.
    pub weight: u16,
}

/// Chooses the font for the window about to open.
pub fn choose(choice: Option<Choice>) {
    *CHOICE.lock().expect("font choice") = choice;
}

/// Whether the chosen font took over the window's name, after [`add`].
static TOOK: Mutex<Option<String>> = Mutex::new(None);

/// The family that took over, if one did.
pub fn taken() -> Option<String> {
    TOOK.lock().expect("font taken").clone()
}

/// Gives `db` the chosen font under the window's name, and Yapfast's own
/// font (already in `db`) the name [`OWN`]. Returns the name the fallback
/// list should start with.
pub fn add(db: &mut Database) -> &'static str {
    let choice = CHOICE.lock().expect("font choice").clone();
    *TOOK.lock().expect("font taken") = None;
    let Some(choice) = choice.filter(|choice| !choice.family.trim().is_empty()) else {
        return WINDOW_NAME;
    };
    let mut found = Database::new();
    for file in candidate_files(&choice.family) {
        let _ = found.load_font_file(file);
    }
    let faces: Vec<fontdb::FaceInfo> = found
        .faces()
        .filter(|face| face.style == fontdb::Style::Normal && has_family(face, &choice.family))
        .cloned()
        .collect();
    if faces.is_empty() {
        tracing::warn!(family = %choice.family, "the chosen font was not found; keeping Yapfast's own");
        return WINDOW_NAME;
    }
    rename_own(db);
    for slot in SLOTS {
        let wanted = i32::from(slot) + i32::from(choice.weight) - 400;
        if let Some(face) = nearest(&faces, wanted) {
            let mut face = face.clone();
            face.families = vec![(WINDOW_NAME.into(), fontdb::Language::English_UnitedStates)];
            face.weight = Weight(slot);
            db.push_face_info(face);
        }
    }
    *TOOK.lock().expect("font taken") = Some(choice.family);
    OWN
}

fn has_family(face: &fontdb::FaceInfo, family: &str) -> bool {
    face.families
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case(family))
}

/// The face whose weight is closest to `wanted`.
fn nearest(faces: &[fontdb::FaceInfo], wanted: i32) -> Option<&fontdb::FaceInfo> {
    faces
        .iter()
        .min_by_key(|face| (i32::from(face.weight.0) - wanted).abs())
}

/// Renames every face of Yapfast's own font to [`OWN`].
fn rename_own(db: &mut Database) {
    let own: Vec<fontdb::FaceInfo> = db
        .faces()
        .filter(|face| has_family(face, WINDOW_NAME))
        .cloned()
        .collect();
    for face in own {
        db.remove_face(face.id);
        let mut face = face;
        face.families = vec![(OWN.into(), fontdb::Language::English_UnitedStates)];
        db.push_face_info(face);
    }
}

/// The font files whose names look like `family`: "FiraCode Nerd Font" finds
/// "FiraCodeNerdFont-Medium.ttf". Only the folders fonts install to are
/// looked at, so the whole system's list is never read.
fn candidate_files(family: &str) -> Vec<PathBuf> {
    let squashed = squash(family);
    let mut files = Vec::new();
    for folder in font_folders() {
        let Ok(entries) = std::fs::read_dir(folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if is_font(&path) && squash(&stem(&path)).starts_with(&squashed) {
                files.push(path);
            }
        }
    }
    files
}

fn is_font(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("ttf" | "otf" | "ttc")
    )
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Lower case letters and digits only.
fn squash(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|letter| letter.to_ascii_lowercase())
        .collect()
}

#[cfg(windows)]
fn font_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();
    if let Ok(windows) = std::env::var("WINDIR") {
        folders.push(PathBuf::from(windows).join("Fonts"));
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        folders.push(PathBuf::from(local).join("Microsoft/Windows/Fonts"));
    }
    folders
}

#[cfg(not(windows))]
fn font_folders() -> Vec<PathBuf> {
    let mut folders = vec![
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        folders.push(PathBuf::from(home).join(".local/share/fonts"));
    }
    folders
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_family_name_matches_its_files_without_spaces_or_case() {
        assert_eq!(squash("FiraCode Nerd Font"), "firacodenerdfont");
        assert!(squash("FiraCodeNerdFont-Medium").starts_with(&squash("FiraCode Nerd Font")));
        assert!(!squash("DMSans").starts_with(&squash("FiraCode Nerd Font")));
    }

    #[test]
    fn without_a_choice_the_window_keeps_its_own_font() {
        choose(None);
        let mut db = Database::new();
        assert_eq!(add(&mut db), "DM Sans");
        assert!(taken().is_none());
    }

    #[test]
    fn a_family_that_is_missing_keeps_the_own_font() {
        choose(Some(Choice {
            family: "No Such Family Anywhere".into(),
            weight: 500,
        }));
        let mut db = Database::new();
        assert_eq!(add(&mut db), "DM Sans");
        choose(None);
    }
}
