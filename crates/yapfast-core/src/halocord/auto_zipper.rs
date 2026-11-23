//! Auto zipper: a file whose extension is listed goes up as a ZIP holding it,
//! named after it with `.zip`. The ZIP stores the file as it is (no
//! compression), made in memory from the bytes being sent, so a file Discord
//! would warn about or mangle arrives as an archive it takes whole.

use std::sync::RwLock;

/// The listed extensions while the plugin is on, lowercase without a dot.
static LISTED: RwLock<Option<Vec<String>>> = RwLock::new(None);

/// The plugin's extensions while it's on, as a list like `.psd, .blend`, or
/// `None` while it's off.
pub fn set(extensions: Option<&str>) {
    let listed = extensions.map(|list| {
        list.split([',', ' '])
            .map(|extension| extension.trim().trim_start_matches('.').to_lowercase())
            .filter(|extension| !extension.is_empty())
            .collect()
    });
    *LISTED
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = listed;
}

/// The name and bytes a file goes up as: a ZIP of it while the plugin is on
/// and its extension is listed, else as it is. A file of 4 GiB or more (past
/// what a ZIP without ZIP64 holds) goes as it is.
pub fn zipped(name: String, data: Vec<u8>) -> (String, Vec<u8>) {
    let listed = {
        let listed = LISTED
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(listed) = listed.as_ref() else {
            return (name, data);
        };
        let lower = name.to_lowercase();
        lower.rsplit_once('.').is_some_and(|(stem, extension)| {
            !stem.is_empty() && listed.iter().any(|it| it == extension)
        })
    };
    if !listed {
        return (name, data);
    }
    match stored_zip(&name, &data, dos_now()) {
        Some(zip) => (format!("{name}.zip"), zip),
        None => (name, data),
    }
}

/// A ZIP holding `data` as `name`, stored, dated `(time, date)` in DOS form;
/// `None` when it's too big for a ZIP without ZIP64.
fn stored_zip(name: &str, data: &[u8], (time, date): (u16, u16)) -> Option<Vec<u8>> {
    let size = u32::try_from(data.len())
        .ok()
        .filter(|size| *size < u32::MAX)?;
    let name_len = u16::try_from(name.len()).ok()?;
    let crc = crc32(data);
    // What the local and central headers share, from "version needed" on.
    let mut shared = Vec::with_capacity(26);
    shared.extend_from_slice(&20_u16.to_le_bytes());
    // Bit 11: the name is UTF-8.
    shared.extend_from_slice(&0x0800_u16.to_le_bytes());
    // Stored.
    shared.extend_from_slice(&0_u16.to_le_bytes());
    shared.extend_from_slice(&time.to_le_bytes());
    shared.extend_from_slice(&date.to_le_bytes());
    shared.extend_from_slice(&crc.to_le_bytes());
    shared.extend_from_slice(&size.to_le_bytes());
    shared.extend_from_slice(&size.to_le_bytes());
    shared.extend_from_slice(&name_len.to_le_bytes());
    // No extra field.
    shared.extend_from_slice(&0_u16.to_le_bytes());

    let local_len = 30 + name.len();
    let central_at = u32::try_from(local_len + data.len()).ok()?;
    let central_len = 46 + name.len();
    let mut zip = Vec::with_capacity(local_len + data.len() + central_len + 22);
    zip.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
    zip.extend_from_slice(&shared);
    zip.extend_from_slice(name.as_bytes());
    zip.extend_from_slice(data);

    zip.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
    // Made by: version 2.0.
    zip.extend_from_slice(&20_u16.to_le_bytes());
    zip.extend_from_slice(&shared);
    // No comment, disk 0, no attributes, the local header at 0.
    zip.extend_from_slice(&[0; 14]);
    zip.extend_from_slice(name.as_bytes());

    zip.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    // This disk, the central directory's disk, one entry here, one in all.
    zip.extend_from_slice(&[0, 0, 0, 0, 1, 0, 1, 0]);
    zip.extend_from_slice(&u32::try_from(central_len).ok()?.to_le_bytes());
    zip.extend_from_slice(&central_at.to_le_bytes());
    // No comment.
    zip.extend_from_slice(&0_u16.to_le_bytes());
    Some(zip)
}

/// Now as a ZIP's DOS time and date, in local time.
fn dos_now() -> (u16, u16) {
    use chrono::{Datelike, Timelike};
    let now = chrono::Local::now();
    let time = (now.hour() << 11) | (now.minute() << 5) | (now.second() / 2);
    let year = u32::try_from(now.year() - 1980).unwrap_or(0);
    let date = (year << 9) | (now.month() << 5) | now.day();
    (time as u16, date as u16)
}

/// CRC-32 as ZIP takes it (IEEE, reflected).
fn crc32(bytes: &[u8]) -> u32 {
    static TABLE: std::sync::LazyLock<[u32; 256]> = std::sync::LazyLock::new(|| {
        let mut table = [0; 256];
        for (at, entry) in table.iter_mut().enumerate() {
            let mut value = at as u32;
            for _ in 0..8 {
                value = if value & 1 == 1 {
                    0xEDB8_8320 ^ (value >> 1)
                } else {
                    value >> 1
                };
            }
            *entry = value;
        }
        table
    });
    !bytes.iter().fold(!0_u32, |crc, byte| {
        TABLE[((crc ^ u32::from(*byte)) & 0xFF) as usize] ^ (crc >> 8)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOX: &[u8] = b"The quick brown fox jumps over the lazy dog";

    #[test]
    fn crc32_matches_the_known_value() {
        assert_eq!(crc32(FOX), 0x414F_A339);
        assert_eq!(crc32(b""), 0);
    }

    /// The ZIP read back by hand, field by field, as an unzipper reads it:
    /// from the end record to the central header to the local one.
    #[test]
    fn a_stored_zip_holds_the_file_as_it_is() {
        let zip = stored_zip("scene.blend", FOX, (0x6000, 0x5B77)).unwrap();
        let u16_at = |at: usize| u16::from_le_bytes([zip[at], zip[at + 1]]);
        let u32_at = |at: usize| u32::from_le_bytes(zip[at..at + 4].try_into().unwrap());
        let end = zip.len() - 22;
        assert_eq!(u32_at(end), 0x0605_4b50, "the end record");
        assert_eq!(u16_at(end + 10), 1, "one entry");
        let central = u32_at(end + 16) as usize;
        assert_eq!(u32_at(central), 0x0201_4b50, "the central header");
        assert_eq!(u16_at(central + 10), 0, "stored");
        assert_eq!(u32_at(central + 16), crc32(FOX));
        assert_eq!(u32_at(central + 20) as usize, FOX.len(), "packed size");
        assert_eq!(u32_at(central + 24) as usize, FOX.len(), "unpacked size");
        let name_len = u16_at(central + 28) as usize;
        assert_eq!(&zip[central + 46..central + 46 + name_len], b"scene.blend");
        let local = u32_at(central + 42) as usize;
        assert_eq!(u32_at(local), 0x0403_4b50, "the local header");
        let data_at = local + 30 + u16_at(local + 26) as usize + u16_at(local + 28) as usize;
        assert_eq!(&zip[data_at..data_at + FOX.len()], FOX);
        assert_eq!(
            data_at + FOX.len(),
            central,
            "nothing between the file and the directory"
        );
    }

    #[test]
    fn only_listed_files_go_as_zips_while_on() {
        let send = |name: &str| zipped(name.to_owned(), FOX.to_vec());
        set(None);
        assert_eq!(send("scene.blend").0, "scene.blend", "off");
        set(Some(".blend, PSD,exe"));
        let (name, data) = send("Scene.BLEND");
        assert_eq!(name, "Scene.BLEND.zip");
        assert_eq!(&data[..4], b"PK\x03\x04");
        assert_eq!(send("art.psd").0, "art.psd.zip");
        assert_eq!(send("setup.exe").0, "setup.exe.zip");
        assert_eq!(
            send("photo.png"),
            ("photo.png".to_owned(), FOX.to_vec()),
            "not listed"
        );
        assert_eq!(
            send(".blend").0,
            ".blend",
            "a name that is only an extension"
        );
        set(None);
    }
}
