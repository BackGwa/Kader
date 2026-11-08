use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};

use crate::process::quiet_command;
use crate::ytdlp::EXECUTABLE;

const RELEASE_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
const CURRENT_FILE: &str = "current";
const REJECTED_FILE: &str = "rejected";
const STAGING_DIR: &str = "staging";

#[cfg(target_os = "macos")]
const ASSET: &str = "yt-dlp_macos.zip";
#[cfg(windows)]
const ASSET: &str = "yt-dlp_win.zip";

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineState {
    Checking,
    Updating,
    Ready,
    Failed,
    Restored,
}

#[derive(Clone, Serialize)]
pub struct EngineStatus {
    state: EngineState,
    detail: Option<String>,
}

pub struct Engine(Mutex<EngineStatus>);

impl Default for Engine {
    fn default() -> Self {
        Self(Mutex::new(EngineStatus {
            state: EngineState::Checking,
            detail: None,
        }))
    }
}

impl Engine {
    pub fn status(&self) -> EngineStatus {
        self.0.lock().unwrap().clone()
    }
}

pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn_blocking(move || match install_latest(&app) {
        Ok(()) => publish(&app, EngineState::Ready, None),
        Err(error) => publish(&app, EngineState::Failed, Some(error)),
    });
}

pub fn active_executable(app: &AppHandle) -> Option<PathBuf> {
    let root = root_dir(app).ok()?;
    let hash = read_marker(&root, CURRENT_FILE)?;
    let executable = root.join(hash).join(EXECUTABLE);
    executable.is_file().then_some(executable)
}

pub fn reject_active(app: &AppHandle) {
    let Ok(root) = root_dir(app) else {
        return;
    };
    let detail = read_marker(&root, CURRENT_FILE)
        .and_then(|hash| fs::write(root.join(REJECTED_FILE), hash).err())
        .map(|error| error.to_string());
    let removed = fs::remove_file(root.join(CURRENT_FILE)).err();
    publish(
        app,
        EngineState::Restored,
        detail.or(removed.map(|error| error.to_string())),
    );
}

fn publish(app: &AppHandle, state: EngineState, detail: Option<String>) {
    let status = EngineStatus { state, detail };
    *app.state::<Engine>().0.lock().unwrap() = status.clone();
    let _ = app.emit("engine", status);
}

fn root_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|directory| directory.join("yt-dlp"))
        .map_err(|error| error.to_string())
}

fn read_marker(root: &Path, name: &str) -> Option<String> {
    fs::read_to_string(root.join(name))
        .ok()
        .map(|content| content.trim().to_string())
}

fn bundled_hash(app: &AppHandle) -> Result<String, String> {
    let resources = app
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?;
    let path = dunce::simplified(&resources).join("engines").join("yt-dlp.sha256");
    fs::read_to_string(path)
        .map(|content| content.trim().to_string())
        .map_err(|error| error.to_string())
}

fn install_latest(app: &AppHandle) -> Result<(), String> {
    let root = root_dir(app)?;
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let latest = fetch_latest_hash()?;
    let active = read_marker(&root, CURRENT_FILE);
    if latest == bundled_hash(app)? {
        if active.is_some() {
            fs::remove_file(root.join(CURRENT_FILE)).map_err(|error| error.to_string())?;
        }
        return remove_unused_versions(&root, None);
    }
    remove_unused_versions(&root, active.as_deref())?;
    let rejected = read_marker(&root, REJECTED_FILE);
    if active.as_deref() == Some(&latest) || rejected.as_deref() == Some(&latest) {
        return Ok(());
    }
    publish(app, EngineState::Updating, None);
    install_version(&root, &latest)
}

fn fetch_latest_hash() -> Result<String, String> {
    let sums = ureq::get(format!("{RELEASE_URL}/SHA2-256SUMS"))
        .call()
        .map_err(|error| error.to_string())?
        .into_body()
        .read_to_string()
        .map_err(|error| error.to_string())?;
    sums.lines()
        .find_map(|line| {
            let (hash, name) = line.split_once(char::is_whitespace)?;
            (name.trim() == ASSET).then(|| hash.to_ascii_lowercase())
        })
        .ok_or_else(|| format!("{ASSET} is missing from SHA2-256SUMS"))
}

fn remove_unused_versions(root: &Path, active: Option<&str>) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let is_directory = entry.file_type().map_err(|error| error.to_string())?.is_dir();
        if is_directory && active != entry.file_name().to_str() {
            fs::remove_dir_all(entry.path()).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn install_version(root: &Path, hash: &str) -> Result<(), String> {
    let staging = root.join(STAGING_DIR);
    fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
    let archive = staging.join(ASSET);
    download_verified(&archive, hash)?;
    let unpacked = staging.join("unpacked");
    fs::create_dir_all(&unpacked).map_err(|error| error.to_string())?;
    extract(&archive, &unpacked)?;
    confirm_runs(&unpacked.join(EXECUTABLE))?;
    let version = root.join(hash);
    fs::rename(&unpacked, &version).map_err(|error| error.to_string())?;
    let marker = root.join("current.next");
    fs::write(&marker, hash).map_err(|error| error.to_string())?;
    fs::rename(&marker, root.join(CURRENT_FILE)).map_err(|error| error.to_string())?;
    fs::remove_dir_all(&staging).map_err(|error| error.to_string())
}

fn download_verified(path: &Path, expected: &str) -> Result<(), String> {
    let response = ureq::get(format!("{RELEASE_URL}/{ASSET}"))
        .call()
        .map_err(|error| error.to_string())?;
    let mut reader = response.into_body().into_with_config().limit(u64::MAX).reader();
    let file = File::create(path).map_err(|error| error.to_string())?;
    let mut writer = HashingWriter {
        file,
        hasher: Sha256::new(),
    };
    io::copy(&mut reader as &mut dyn Read, &mut writer).map_err(|error| error.to_string())?;
    let actual = format!("{:x}", writer.hasher.finalize());
    if actual != expected {
        return Err(format!("Checksum mismatch for {ASSET}"));
    }
    Ok(())
}

struct HashingWriter {
    file: File,
    hasher: Sha256,
}

impl Write for HashingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = self.file.write(bytes)?;
        self.hasher.update(&bytes[..written]);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

fn archive_tool() -> PathBuf {
    #[cfg(windows)]
    {
        let system_root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        PathBuf::from(system_root).join("System32").join("tar.exe")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/usr/bin/tar")
    }
}

fn extract(archive: &Path, destination: &Path) -> Result<(), String> {
    let status = quiet_command(archive_tool())
        .arg("-xf")
        .arg(archive)
        .arg("-C")
        .arg(destination)
        .status()
        .map_err(|error| error.to_string())?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("Could not extract {ASSET}"))
}

fn confirm_runs(executable: &Path) -> Result<(), String> {
    let output = quiet_command(executable)
        .args(["--ignore-config", "--version"])
        .output()
        .map_err(|error| error.to_string())?;
    output
        .status
        .success()
        .then_some(())
        .ok_or_else(|| String::from_utf8_lossy(&output.stderr).trim().to_string())
}
