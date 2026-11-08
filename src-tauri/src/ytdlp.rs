use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager};
use tokio::process::Child;

use crate::process::{self, ProcessTree};
use crate::update;

#[cfg(target_os = "macos")]
pub const EXECUTABLE: &str = "yt-dlp_macos";
#[cfg(windows)]
pub const EXECUTABLE: &str = "yt-dlp.exe";

#[cfg(unix)]
const DENO: &str = "deno";
#[cfg(windows)]
const DENO: &str = "deno.exe";

const MP3_BITRATES: [u32; 4] = [320, 256, 192, 128];
const MAX_TITLE_BYTES: usize = 200;
const BYTES_PER_KILOBIT: f64 = 125.0;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Mp4,
    Webm,
    Mkv,
    M4a,
    Opus,
    Mp3,
}

impl Format {
    const ALL: [Format; 6] = [
        Format::Mp4,
        Format::Webm,
        Format::Mkv,
        Format::M4a,
        Format::Opus,
        Format::Mp3,
    ];

    pub fn is_video(self) -> bool {
        matches!(self, Format::Mp4 | Format::Webm | Format::Mkv)
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Selection {
    pub format: Format,
    pub quality: Option<u32>,
}

#[derive(Clone, Serialize)]
pub struct Quality {
    value: Option<u32>,
    codec: Option<&'static str>,
    size: Option<u64>,
}

#[derive(Serialize)]
pub struct VideoInfo {
    url: String,
    title: String,
    duration: Option<f64>,
    thumbnail: Option<String>,
    options: BTreeMap<Format, Vec<Quality>>,
}

#[derive(Serialize)]
pub struct PlaylistEntry {
    url: String,
    title: String,
    duration: Option<f64>,
    thumbnail: Option<String>,
}

#[derive(Serialize)]
pub struct PlaylistInfo {
    title: String,
    entries: Vec<PlaylistEntry>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Probe {
    Video(VideoInfo),
    Playlist(PlaylistInfo),
}

#[derive(Default)]
pub struct Probes(Mutex<HashMap<u64, ProcessTree>>);

impl Probes {
    pub fn cancel(&self, token: u64) {
        if let Some(tree) = self.0.lock().unwrap().remove(&token) {
            tree.kill();
        }
    }
}

#[derive(Deserialize)]
pub struct Transfer {
    pub status: String,
    pub downloaded_bytes: Option<f64>,
    pub total_bytes: Option<f64>,
    pub total_bytes_estimate: Option<f64>,
    pub speed: Option<f64>,
    pub eta: Option<f64>,
    pub filename: Option<String>,
    pub fragment_index: Option<f64>,
    pub fragment_count: Option<f64>,
}

pub enum ProgressLine {
    Transfer(Transfer),
    Processing,
}

pub fn parse_progress(line: &str) -> Option<ProgressLine> {
    if let Some(json) = line.strip_prefix("KADER ") {
        return serde_json::from_str(json).ok().map(ProgressLine::Transfer);
    }
    line.starts_with("KADERPP ")
        .then_some(ProgressLine::Processing)
}

pub fn last_error(stderr: &str) -> String {
    let lines = || stderr.lines().map(str::trim).filter(|line| !line.is_empty());
    lines()
        .filter_map(|line| line.strip_prefix("ERROR: "))
        .last()
        .or_else(|| lines().last())
        .unwrap_or("yt-dlp exited unexpectedly")
        .to_string()
}

struct Tools {
    yt_dlp: PathBuf,
    deno: PathBuf,
    ffmpeg_dir: PathBuf,
    cache_dir: PathBuf,
    updated: bool,
}

impl Tools {
    fn resolve(app: &AppHandle) -> Result<Self, String> {
        let resources = app
            .path()
            .resource_dir()
            .map_err(|error| error.to_string())?;
        let engines = dunce::simplified(&resources).join("engines");
        let active = update::active_executable(app);
        let updated = active.is_some();
        Ok(Self {
            yt_dlp: active.unwrap_or_else(|| engines.join("yt-dlp").join(EXECUTABLE)),
            deno: engines.join(DENO),
            ffmpeg_dir: engines.join("ffmpeg"),
            cache_dir: app
                .path()
                .app_cache_dir()
                .map_err(|error| error.to_string())?
                .join("yt-dlp"),
            updated,
        })
    }

    fn common_args(&self) -> Vec<OsString> {
        let mut js_runtime = OsString::from("deno:");
        js_runtime.push(&self.deno);
        vec![
            "--ignore-config".into(),
            "--no-playlist".into(),
            "--no-warnings".into(),
            "--no-js-runtimes".into(),
            "--js-runtimes".into(),
            js_runtime,
            "--ffmpeg-location".into(),
            self.ffmpeg_dir.clone().into(),
            "--cache-dir".into(),
            self.cache_dir.clone().into(),
            "--encoding".into(),
            "utf-8".into(),
        ]
    }
}

pub fn spawn(
    app: &AppHandle,
    args: impl Fn(Vec<OsString>) -> Vec<OsString>,
) -> Result<(Child, ProcessTree), String> {
    let tools = Tools::resolve(app)?;
    match process::spawn(&tools.yt_dlp, &args(tools.common_args())) {
        Ok(spawned) => Ok(spawned),
        Err(_) if tools.updated => {
            update::reject_active(app);
            let tools = Tools::resolve(app)?;
            process::spawn(&tools.yt_dlp, &args(tools.common_args()))
                .map_err(|error| error.to_string())
        }
        Err(error) => Err(error.to_string()),
    }
}

pub async fn probe(app: &AppHandle, token: u64, url: &str) -> Result<Probe, String> {
    let (child, tree) = spawn(app, |mut args| {
        args.extend(["-J".into(), "--flat-playlist".into(), "--".into(), url.into()]);
        args
    })?;
    let probes = app.state::<Probes>();
    probes.0.lock().unwrap().insert(token, tree);
    let output = child.wait_with_output().await;
    probes.0.lock().unwrap().remove(&token);
    let output = output.map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(last_error(&String::from_utf8_lossy(&output.stderr)));
    }
    let info: Value = serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
    Ok(parse_probe(url, &info))
}

pub fn download_args(
    mut args: Vec<OsString>,
    selection: Selection,
    url: &str,
    working_dir: &Path,
) -> Vec<OsString> {
    let output = format!("%(title).{}B.%(ext)s", title_limit(working_dir));
    args.extend(
        [
            "-q",
            "--progress",
            "--newline",
            "--progress-delta",
            "0.25",
            "--progress-template",
            "download:KADER %(progress)j",
            "--progress-template",
            "postprocess:KADERPP %(progress.status)s",
            "-o",
            &output,
        ]
        .map(OsString::from),
    );
    args.extend(["-P".into(), working_dir.into()]);
    args.extend(format_args(selection).into_iter().map(OsString::from));
    args.extend(["--".into(), url.into()]);
    args
}

fn format_args(selection: Selection) -> Vec<String> {
    let Selection { format, quality } = selection;
    let resolution = quality.map_or("res".to_string(), |height| format!("res:{height}"));
    let bitrate = |args: &mut Vec<String>| {
        if let Some(kbps) = quality {
            args.extend(["-S".to_string(), format!("abr:{kbps}")]);
        }
    };
    let extract = |codec: &str| ["-x", "--audio-format", codec].map(String::from).to_vec();
    match format {
        Format::Mp4 => strings(&[
            "-f",
            "bv[ext=mp4][vcodec~=?'^(avc1|av01)']+ba[ext=m4a]/b[ext=mp4]",
            "-S",
            &format!("{resolution},vcodec:h264"),
            "--merge-output-format",
            "mp4",
        ]),
        Format::Webm => {
            let mut args = strings(&[
                "-f",
                "bv[vcodec~='^(vp0?[89]|av01)']+ba[acodec=opus]/b[ext=webm]",
                "--merge-output-format",
                "webm",
            ]);
            args.extend(["-S".to_string(), resolution]);
            args
        }
        Format::Mkv => {
            let mut args = strings(&[
                "-f",
                "bv+ba/b",
                "--merge-output-format",
                "mkv",
                "--remux-video",
                "mkv",
            ]);
            args.extend(["-S".to_string(), resolution]);
            args
        }
        Format::M4a => {
            let mut args = strings(&["-f", "ba[acodec^=mp4a]/b[acodec^=mp4a]"]);
            args.extend(extract("m4a"));
            bitrate(&mut args);
            args
        }
        Format::Opus => {
            let mut args = strings(&["-f", "ba[acodec=opus]/b[acodec=opus]"]);
            args.extend(extract("opus"));
            bitrate(&mut args);
            args
        }
        Format::Mp3 => {
            let mut args = strings(&["-f", "ba/b"]);
            args.extend(extract("mp3"));
            let audio_quality = quality.map_or("0".to_string(), |kbps| format!("{kbps}K"));
            args.extend(["--audio-quality".to_string(), audio_quality]);
            args
        }
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[cfg(windows)]
fn title_limit(working_dir: &Path) -> usize {
    const MAX_PATH: usize = 260;
    const RESERVED: usize = 40;
    const MIN_TITLE_BYTES: usize = 32;
    let directory = working_dir.as_os_str().to_string_lossy().encode_utf16().count();
    MAX_PATH
        .saturating_sub(directory + RESERVED)
        .clamp(MIN_TITLE_BYTES, MAX_TITLE_BYTES)
}

#[cfg(not(windows))]
fn title_limit(_working_dir: &Path) -> usize {
    MAX_TITLE_BYTES
}

fn parse_probe(url: &str, info: &Value) -> Probe {
    let title = text(info, "title").unwrap_or(url).to_string();
    match info.get("entries").and_then(Value::as_array) {
        Some(entries) => Probe::Playlist(PlaylistInfo {
            title,
            entries: entries.iter().filter_map(parse_entry).collect(),
        }),
        None => Probe::Video(VideoInfo {
            url: url.to_string(),
            title,
            duration: number(info, "duration"),
            thumbnail: thumbnail(info),
            options: quality_options(info),
        }),
    }
}

fn parse_entry(entry: &Value) -> Option<PlaylistEntry> {
    let url = text(entry, "url").or_else(|| text(entry, "webpage_url"))?;
    Some(PlaylistEntry {
        url: url.to_string(),
        title: text(entry, "title").unwrap_or(url).to_string(),
        duration: number(entry, "duration"),
        thumbnail: thumbnail(entry),
    })
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn number(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn thumbnail(value: &Value) -> Option<String> {
    text(value, "thumbnail")
        .or_else(|| {
            value
                .get("thumbnails")
                .and_then(Value::as_array)
                .and_then(|thumbnails| thumbnails.last())
                .and_then(|last| text(last, "url"))
        })
        .map(str::to_string)
}

struct Stream<'a> {
    ext: &'a str,
    vcodec: Option<&'a str>,
    acodec: Option<&'a str>,
    resolution: Option<u32>,
    bitrate: Option<f64>,
    size: Option<u64>,
}

impl<'a> Stream<'a> {
    fn new(format: &'a Value, duration: Option<f64>) -> Self {
        let height = number(format, "height");
        let resolution = match (number(format, "width"), height) {
            (Some(width), Some(height)) => Some(width.min(height)),
            (None, height) => height,
            (Some(_), None) => None,
        };
        let size = number(format, "filesize")
            .or_else(|| number(format, "filesize_approx"))
            .or_else(|| Some(number(format, "tbr")? * duration? * BYTES_PER_KILOBIT));
        Stream {
            ext: text(format, "ext").unwrap_or_default(),
            vcodec: text(format, "vcodec"),
            acodec: text(format, "acodec"),
            resolution: resolution.map(|value| value as u32),
            bitrate: number(format, "abr").or_else(|| number(format, "tbr")),
            size: size.map(|value| value as u64),
        }
    }

    fn has_video(&self) -> bool {
        self.vcodec != Some("none") && self.resolution.is_some()
    }

    fn is_video_only(&self) -> bool {
        self.has_video() && self.acodec == Some("none")
    }

    fn is_combined(&self) -> bool {
        self.has_video() && self.acodec != Some("none")
    }

    fn is_audio_only(&self) -> bool {
        self.vcodec == Some("none") && self.acodec.is_some_and(|codec| codec != "none")
    }

    fn vcodec_starts_with(&self, prefixes: &[&str]) -> bool {
        self.vcodec
            .is_some_and(|codec| prefixes.iter().any(|prefix| codec.starts_with(prefix)))
    }

    fn fits_video_stream(&self, format: Format) -> bool {
        match format {
            Format::Mp4 => {
                self.ext == "mp4" && (self.vcodec.is_none() || self.vcodec_starts_with(&["avc1", "av01"]))
            }
            Format::Webm => self.vcodec_starts_with(&["vp8", "vp08", "vp9", "vp09", "av01"]),
            _ => true,
        }
    }

    fn fits_combined(&self, format: Format) -> bool {
        match format {
            Format::Mp4 => self.ext == "mp4",
            Format::Webm => self.ext == "webm",
            _ => true,
        }
    }

    fn fits_audio_track(&self, format: Format) -> bool {
        match format {
            Format::Mp4 => self.ext == "m4a",
            Format::Webm => self.acodec == Some("opus"),
            _ => true,
        }
    }

    fn fits_audio_format(&self, format: Format) -> bool {
        let Some(codec) = self.acodec else {
            return false;
        };
        match format {
            Format::M4a => codec.starts_with("mp4a"),
            Format::Opus => codec == "opus",
            _ => codec != "none",
        }
    }

    fn mp4_codec_label(&self) -> Option<&'static str> {
        if self.vcodec_starts_with(&["avc1"]) {
            Some("H.264")
        } else if self.vcodec_starts_with(&["av01"]) {
            Some("AV1")
        } else {
            None
        }
    }
}

fn quality_options(info: &Value) -> BTreeMap<Format, Vec<Quality>> {
    let duration = number(info, "duration");
    let streams: Vec<Stream> = info
        .get("formats")
        .and_then(Value::as_array)
        .map(|formats| formats.iter().map(|format| Stream::new(format, duration)).collect())
        .unwrap_or_default();
    let mut options: BTreeMap<Format, Vec<Quality>> = Format::ALL
        .into_iter()
        .map(|format| {
            let qualities = match format {
                Format::Mp4 | Format::Webm | Format::Mkv => video_qualities(format, &streams),
                Format::M4a | Format::Opus => audio_qualities(format, &streams),
                Format::Mp3 => mp3_qualities(&streams, duration),
            };
            (format, qualities)
        })
        .collect();
    let best = Quality {
        value: None,
        codec: None,
        size: None,
    };
    if options.values().all(Vec::is_empty) {
        options.values_mut().for_each(|qualities| qualities.push(best.clone()));
    }
    options.retain(|_, qualities| !qualities.is_empty());
    options
}

fn video_qualities(format: Format, streams: &[Stream]) -> Vec<Quality> {
    let audio_size = streams
        .iter()
        .filter(|stream| stream.is_audio_only() && stream.fits_audio_track(format))
        .max_by(|left, right| left.bitrate.unwrap_or(0.0).total_cmp(&right.bitrate.unwrap_or(0.0)))
        .map(|stream| stream.size.unwrap_or(0));
    let mut by_resolution: BTreeMap<u32, Quality> = BTreeMap::new();
    for stream in streams {
        let Some(resolution) = stream.resolution else {
            continue;
        };
        let size = if stream.is_combined() && stream.fits_combined(format) {
            stream.size
        } else if stream.is_video_only() && stream.fits_video_stream(format) {
            let Some(audio_size) = audio_size else {
                continue;
            };
            stream.size.map(|size| size + audio_size)
        } else {
            continue;
        };
        let codec = (format == Format::Mp4).then(|| stream.mp4_codec_label()).flatten();
        let candidate = Quality {
            value: Some(resolution),
            codec,
            size,
        };
        by_resolution
            .entry(resolution)
            .and_modify(|current| {
                if selection_rank(&candidate) > selection_rank(current) {
                    *current = candidate.clone();
                }
            })
            .or_insert_with(|| candidate.clone());
    }
    by_resolution.into_values().rev().collect()
}

fn selection_rank(quality: &Quality) -> (bool, u64) {
    (quality.codec == Some("H.264"), quality.size.unwrap_or(0))
}

fn audio_qualities(format: Format, streams: &[Stream]) -> Vec<Quality> {
    let matching = || {
        streams
            .iter()
            .filter(move |stream| stream.is_audio_only() && stream.fits_audio_format(format))
    };
    let mut by_bitrate: BTreeMap<u32, Option<u64>> = BTreeMap::new();
    for stream in matching() {
        if let Some(bitrate) = stream.bitrate {
            let size = by_bitrate.entry(bitrate.round() as u32).or_insert(stream.size);
            *size = (*size).max(stream.size);
        }
    }
    if by_bitrate.is_empty() && matching().next().is_some() {
        return vec![Quality {
            value: None,
            codec: None,
            size: None,
        }];
    }
    by_bitrate
        .into_iter()
        .rev()
        .map(|(bitrate, size)| Quality {
            value: Some(bitrate),
            codec: None,
            size,
        })
        .collect()
}

fn mp3_qualities(streams: &[Stream], duration: Option<f64>) -> Vec<Quality> {
    let has_audio = streams
        .iter()
        .any(|stream| (stream.is_audio_only() || stream.is_combined()) && stream.fits_audio_format(Format::Mp3));
    if !has_audio {
        return Vec::new();
    }
    MP3_BITRATES
        .into_iter()
        .map(|bitrate| Quality {
            value: Some(bitrate),
            codec: None,
            size: duration.map(|seconds| (seconds * f64::from(bitrate) * BYTES_PER_KILOBIT) as u64),
        })
        .collect()
}
