//! HaloCord's own updates, from DavidHiFi/HaloCord-Yapfast's releases: the
//! pure rules are [`yapfast_iced::halocord_updates`], and this asks GitHub,
//! then downloads, checks, tests and puts in place a release the person can
//! update from.
//!
//! A HaloCord build is its release number, and for a nightly the day and run
//! it was made (`YAPFAST_BUILD_HALOCORD`, `YAPFAST_BUILD_HALOCORD_NIGHTLY`).
//! A build without either reads `HALOCORD.json` beside the exe, which the
//! build PC's updater and [`prepare`] write, and only when it names the exe
//! that's running. So a copied exe isn't taken for the one the file describes.
//!
//! An install replaces `yapfast.exe` itself: the new one is downloaded to the
//! temporary folder, checked against the release's `SHA256SUMS.txt`, unpacked,
//! started once with `--version`, and only then moved in. The running exe is
//! renamed aside (Windows allows that while it runs), the old one is copied to
//! `halocord-backups` first, and a new exe that doesn't answer `--version` is
//! rolled back. The running program keeps going from the file it has open;
//! the new one starts next time.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use yapfast_core::runtime::BackgroundRuntime;
use yapfast_iced::halocord_updates as hc;

use super::install::Progress;
#[cfg(windows)]
use super::install::Stage;

/// Looks at the channel's release.
pub async fn check(channel: hc::Channel) -> Result<hc::Release, String> {
    let client = super::client()?;
    hc::look_up(channel, |url| async move {
        let response = client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .timeout(std::time::Duration::from_secs(20))
            .send()
            .await
            .map_err(|error| format!("GitHub couldn't be reached: {error}"))?;
        let status = response.status().as_u16();
        let body = response.bytes().await.map_err(|error| error.to_string())?;
        Ok((status, body.to_vec()))
    })
    .await
}

/// This build's number and nightly stamp, when the build named them.
pub fn compiled() -> Option<hc::Build> {
    let number = option_env!("YAPFAST_BUILD_HALOCORD").and_then(hc::number_of)?;
    let nightly = option_env!("YAPFAST_BUILD_HALOCORD_NIGHTLY").and_then(hc::Nightly::parse);
    Some(hc::Build { number, nightly })
}

/// The HaloCord the build names, or the one `HALOCORD.json` beside the exe
/// says is in place, when that file's hash is the running exe's.
pub fn current() -> Option<hc::Build> {
    compiled().or_else(|| {
        let exe = std::env::current_exe().ok()?;
        let state: serde_json::Value =
            serde_json::from_slice(&std::fs::read(exe.with_file_name("HALOCORD.json")).ok()?)
                .ok()?;
        if state.get("exe_sha256")?.as_str()? != sha(&exe).ok()?.as_str() {
            return None;
        }
        let number = u32::try_from(state.get("build")?.as_u64()?).ok()?;
        let nightly = state
            .get("nightly")
            .and_then(serde_json::Value::as_str)
            .and_then(hc::Nightly::parse);
        Some(hc::Build { number, nightly })
    })
}

/// The folder the running exe is in.
fn own_folder() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
}

/// Deletes exes an earlier install renamed aside and nothing runs now.
pub fn clear_old() {
    let Some(Ok(entries)) = own_folder().map(std::fs::read_dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with("yapfast.exe.halocord-old-")
        {
            // Fails while that exe still runs; the next look tries again.
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Whether this copy can put `release` in place itself, and the line under
/// the button: its size, or why it can't. A package manager's copy says so
/// elsewhere on the page.
pub fn install_offer(release: &hc::Release) -> (bool, String) {
    if !cfg!(windows) {
        return (
            false,
            "This copy can't update itself. Get the newest one from the releases page.".into(),
        );
    }
    if super::source().manager().is_some() {
        return (false, String::new());
    }
    let Some(folder) = own_folder() else {
        return (false, "This copy can't find its own folder.".into());
    };
    if !super::install::writable(&folder) {
        return (
            false,
            "Yapfast's folder can't be written to, so it can't update itself here.".into(),
        );
    }
    if release.sums.is_none() {
        return (
            false,
            "This release has no sums to check it with, so it can't be installed from here.".into(),
        );
    }
    let megabytes = release.zip.size.div_ceil(1_000_000);
    (
        true,
        format!("{megabytes} MB. Yapfast restarts when it's done."),
    )
}

/// Where the download of `release` goes, which a cancelled one removes.
pub fn download_path(release: &hc::Release) -> PathBuf {
    std::env::temp_dir()
        .join("halocord-update")
        .join(&release.zip.name)
}

/// Downloads `release`, checks it against the release's sums, tests the new
/// exe and puts it in place for the next start; gives the exe to start.
pub async fn prepare(
    runtime: &BackgroundRuntime,
    client: &reqwest::Client,
    release: &hc::Release,
    progress: Arc<Progress>,
) -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        prepare_in(runtime, client, release, progress).await
    }
    #[cfg(not(windows))]
    {
        let _ = (runtime, client, release, progress);
        Err("This copy can't update itself. Get the newest one from the releases page.".into())
    }
}

#[cfg(windows)]
async fn prepare_in(
    runtime: &BackgroundRuntime,
    client: &reqwest::Client,
    release: &hc::Release,
    progress: Arc<Progress>,
) -> Result<PathBuf, String> {
    progress.stage(Stage::Starting);
    let sums = release
        .sums
        .as_ref()
        .ok_or("This release doesn't publish SHA256SUMS.txt.")?;
    let sums = super::install::fetch_small(client, &sums.url, 1 << 20).await?;
    let expected = hc::sum_for(&sums, &release.zip.name).ok_or_else(|| {
        format!(
            "The download of {} couldn't be verified, so it wasn't installed.",
            release.zip.name
        )
    })?;

    let zip = download_path(release);
    let folder = zip.parent().ok_or("The temporary folder has no name.")?;
    std::fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    let unpacked = folder.join(format!("unpacked-{}", stamp()));
    progress.stage(Stage::Downloading);
    let actual =
        super::install::download(client, &release.zip.url, &zip, release.zip.size, &progress)
            .await?;
    if actual != expected {
        let _ = std::fs::remove_file(&zip);
        return Err(format!(
            "{} doesn't match the sum the release published.",
            release.zip.name
        ));
    }

    progress.stage(Stage::Testing);
    let program = std::env::current_exe().map_err(|error| error.to_string())?;
    let (release, placed) = (release.clone(), program.clone());
    let finished = runtime
        .spawn_blocking(move || {
            let result = unpack(&zip, &unpacked).and_then(|()| {
                let new = find_exe(&unpacked)?;
                version(&new)?;
                progress.stage(Stage::Installing);
                swap(&placed, &new, &release)
            });
            let _ = std::fs::remove_dir_all(&unpacked);
            let _ = std::fs::remove_file(&zip);
            result
        })
        .await
        .map_err(|_| "The update stopped partway.".to_owned())?;
    finished.map(|()| program)
}

/// The release's `yapfast.exe`: at the zip's top, or in the one folder the
/// zip holds, as the release's package puts it.
#[cfg(windows)]
fn find_exe(unpacked: &Path) -> Result<PathBuf, String> {
    let top = unpacked.join("yapfast.exe");
    if top.is_file() {
        return Ok(top);
    }
    let folders: Vec<PathBuf> = std::fs::read_dir(unpacked)
        .map_err(|error| error.to_string())?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    match folders.as_slice() {
        [folder] if folder.join("yapfast.exe").is_file() => Ok(folder.join("yapfast.exe")),
        _ => Err("The download has no yapfast.exe in it.".into()),
    }
}

/// The local time as `YYYYMMDD-HHMMSS`, for a file's name.
#[cfg(windows)]
fn stamp() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

/// Unpacks the zip with Windows' own `tar`, which reads zips, hidden.
#[cfg(windows)]
fn unpack(zip: &Path, to: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::fs::create_dir_all(to).map_err(|error| error.to_string())?;
    let tar = std::env::var_os("SystemRoot")
        .map(|root| PathBuf::from(root).join(r"System32\tar.exe"))
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows\System32\tar.exe"));
    let output = std::process::Command::new(tar)
        .args(["-xf"])
        .arg(zip)
        .arg("-C")
        .arg(to)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| format!("The download couldn't be unpacked: {error}."))?;
    if output.status.success() {
        Ok(())
    } else {
        Err("The download couldn't be unpacked.".into())
    }
}

/// Starts an exe with `--version` and says whether it answered with its own
/// version, giving up after 30 seconds.
#[cfg(windows)]
fn version(exe: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut child = std::process::Command::new(exe)
        .arg("--version")
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|error| format!("The new version wouldn't start: {error}."))?;
    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                let mut text = String::new();
                if let Some(mut out) = child.stdout.take() {
                    use std::io::Read;
                    let _ = out.read_to_string(&mut text);
                }
                return (text.starts_with("Yapfast v"))
                    .then_some(())
                    .ok_or_else(|| "The new version didn't say it was Yapfast.".to_owned());
            }
            Ok(Some(_)) => return Err("The new version didn't start.".into()),
            Ok(None) if started.elapsed() > std::time::Duration::from_secs(30) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("The new version took too long to start.".into());
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
            Err(error) => return Err(error.to_string()),
        }
    }
}

/// Puts `new` in place of the running `program`, keeping a backup and rolling
/// back when the new exe doesn't answer `--version`. The running exe is
/// renamed aside, which Windows allows while it runs.
#[cfg(windows)]
fn swap(program: &Path, new: &Path, release: &hc::Release) -> Result<(), String> {
    let folder = program
        .parent()
        .ok_or("This copy can't find its own folder.")?;
    let backups = folder.join("halocord-backups");
    std::fs::create_dir_all(&backups).map_err(|error| error.to_string())?;
    let now = stamp();
    let backup = backups.join(format!("yapfast.exe.bak-{now}"));
    std::fs::copy(program, &backup).map_err(|error| error.to_string())?;
    let aside = folder.join(format!("yapfast.exe.halocord-old-{now}"));
    std::fs::rename(program, &aside).map_err(|error| error.to_string())?;
    let placed = std::fs::copy(new, program)
        .map_err(|error| error.to_string())
        .and_then(|_| version(program));
    if let Err(error) = placed {
        let _ = std::fs::remove_file(program);
        std::fs::rename(&aside, program)
            .or_else(|_| std::fs::copy(&backup, program).map(|_| ()))
            .map_err(|rollback| {
                format!("{error}; and the old version couldn't be put back: {rollback}")
            })?;
        return Err(error);
    }
    write_state(program, release);
    prune(&backups);
    Ok(())
}

/// Writes `HALOCORD.json` beside the exe, keeping any fields another updater
/// put there.
#[cfg(windows)]
fn write_state(program: &Path, release: &hc::Release) {
    let path = program.with_file_name("HALOCORD.json");
    let mut state: serde_json::Value = std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let Some(state) = state.as_object_mut() else {
        return;
    };
    if let Ok(hash) = sha(program) {
        state.insert("exe_sha256".into(), hash.into());
    }
    state.insert("swapped".into(), chrono::Local::now().to_rfc3339().into());
    // The build PC's updater reads these back, and needs them there.
    for field in ["base", "commit"] {
        state.entry(field).or_insert_with(|| "".into());
    }
    state.insert("build".into(), release.build.number.into());
    match release.build.nightly {
        Some(nightly) => state.insert("nightly".into(), nightly.stamp().into()),
        None => state.remove("nightly"),
    };
    let _ = std::fs::write(path, serde_json::to_vec_pretty(&state).unwrap_or_default());
}

/// Keeps the three newest backups.
#[cfg(windows)]
fn prune(folder: &Path) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    let mut backups: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    backups.sort();
    let old = backups.len().saturating_sub(3);
    for old in backups.into_iter().take(old) {
        let _ = std::fs::remove_file(old);
    }
}

/// A file's SHA-256 as lower-case hex.
pub fn sha(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
