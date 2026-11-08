use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

use crate::process::ProcessTree;
use crate::settings::Settings;
use crate::ytdlp::{self, ProgressLine, Selection, Transfer};

const FIRST_STREAM_SHARE: f64 = 0.9;
const TREE_EXIT_GRACE: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Queued,
    Fetching,
    Downloading,
    Processing,
    Cancelling,
    Done,
    Failed,
    Cancelled,
}

impl Status {
    fn is_running(self) -> bool {
        matches!(
            self,
            Status::Fetching | Status::Downloading | Status::Processing | Status::Cancelling
        )
    }

    fn is_finished(self) -> bool {
        matches!(self, Status::Done | Status::Failed | Status::Cancelled)
    }
}

#[derive(Deserialize)]
pub struct NewJob {
    url: String,
    title: String,
    thumbnail: Option<String>,
    selection: Selection,
    label: String,
}

#[derive(Clone, Serialize)]
pub struct Job {
    id: u64,
    url: String,
    title: String,
    thumbnail: Option<String>,
    selection: Selection,
    label: String,
    status: Status,
    progress: f64,
    speed: Option<f64>,
    eta: Option<f64>,
    size: Option<u64>,
    error: Option<String>,
    #[serde(skip)]
    directory: PathBuf,
}

impl Job {
    fn working_dir(&self) -> PathBuf {
        self.directory.join(format!(".kader-{}", self.id))
    }
}

struct Inner {
    jobs: Vec<Job>,
    trees: HashMap<u64, ProcessTree>,
    next_id: u64,
}

pub struct Queue(Mutex<Inner>);

impl Default for Queue {
    fn default() -> Self {
        let next_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(1, |elapsed| elapsed.as_millis() as u64);
        Self(Mutex::new(Inner {
            jobs: Vec::new(),
            trees: HashMap::new(),
            next_id,
        }))
    }
}

impl Queue {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.0.lock().unwrap()
    }

    pub fn snapshot(&self) -> Vec<Job> {
        self.lock().jobs.clone()
    }

    pub fn unfinished_count(&self) -> usize {
        self.lock()
            .jobs
            .iter()
            .filter(|job| !job.status.is_finished())
            .count()
    }

    pub fn shutdown(&self) {
        let directories: Vec<PathBuf> = {
            let mut inner = self.lock();
            inner.trees.values().for_each(ProcessTree::kill);
            inner
                .jobs
                .iter_mut()
                .filter(|job| job.status != Status::Done)
                .map(|job| {
                    job.status = Status::Cancelled;
                    job.working_dir()
                })
                .collect()
        };
        thread::sleep(TREE_EXIT_GRACE);
        for directory in directories {
            let _ = fs::remove_dir_all(directory);
        }
    }
}

struct Tracker {
    stream_count: usize,
    stream_index: usize,
    filename: Option<String>,
    finished_bytes: f64,
    current_bytes: f64,
}

impl Tracker {
    fn new(selection: Selection) -> Self {
        Self {
            stream_count: if selection.format.is_video() { 2 } else { 1 },
            stream_index: 0,
            filename: None,
            finished_bytes: 0.0,
            current_bytes: 0.0,
        }
    }

    fn apply(&mut self, transfer: &Transfer) -> (f64, Option<u64>) {
        if self.filename.is_some() && self.filename != transfer.filename {
            self.stream_index += 1;
            self.finished_bytes += self.current_bytes;
        }
        self.filename = transfer.filename.clone();
        let total = transfer.total_bytes.or(transfer.total_bytes_estimate);
        self.current_bytes = total.unwrap_or(self.current_bytes);
        let fraction = stream_fraction(transfer, total).clamp(0.0, 1.0);
        let known = self.finished_bytes + self.current_bytes;
        let size = (known > 0.0).then_some(known as u64);
        (self.overall(fraction), size)
    }

    fn overall(&self, fraction: f64) -> f64 {
        match (self.stream_count, self.stream_index) {
            (1, _) => fraction,
            (_, 0) => fraction * FIRST_STREAM_SHARE,
            _ => FIRST_STREAM_SHARE + fraction * (1.0 - FIRST_STREAM_SHARE),
        }
    }
}

fn stream_fraction(transfer: &Transfer, total: Option<f64>) -> f64 {
    if transfer.status == "finished" {
        return 1.0;
    }
    match (transfer.downloaded_bytes, total) {
        (Some(downloaded), Some(total)) if total > 0.0 => downloaded / total,
        _ => match (transfer.fragment_index, transfer.fragment_count) {
            (Some(index), Some(count)) if count > 0.0 => index / count,
            _ => 0.0,
        },
    }
}

pub fn enqueue(app: &AppHandle, items: Vec<NewJob>) {
    let directory = app.state::<Settings>().values().save_dir;
    let queue = app.state::<Queue>();
    {
        let mut inner = queue.lock();
        for item in items {
            let id = inner.next_id;
            inner.next_id += 1;
            inner.jobs.push(Job {
                id,
                url: item.url,
                title: item.title,
                thumbnail: item.thumbnail,
                selection: item.selection,
                label: item.label,
                status: Status::Queued,
                progress: 0.0,
                speed: None,
                eta: None,
                size: None,
                error: None,
                directory: directory.clone(),
            });
        }
    }
    emit_all(app);
    pump(app);
}

pub fn cancel(app: &AppHandle, id: u64) {
    let queue = app.state::<Queue>();
    let mut inner = queue.lock();
    let tree = inner.trees.get(&id).cloned();
    let Some(job) = inner.jobs.iter_mut().find(|job| job.id == id) else {
        return;
    };
    match job.status {
        Status::Queued => job.status = Status::Cancelled,
        status if status.is_running() => {
            job.status = Status::Cancelling;
            if let Some(tree) = tree {
                tree.kill();
            }
        }
        _ => return,
    }
    let job = job.clone();
    drop(inner);
    emit_job(app, &job);
}

pub fn retry(app: &AppHandle, id: u64) {
    let queue = app.state::<Queue>();
    {
        let mut inner = queue.lock();
        let position = inner.jobs.iter().position(|job| {
            job.id == id && matches!(job.status, Status::Failed | Status::Cancelled)
        });
        let Some(position) = position else {
            return;
        };
        let mut job = inner.jobs.remove(position);
        job.status = Status::Queued;
        job.progress = 0.0;
        job.speed = None;
        job.eta = None;
        job.error = None;
        inner.jobs.push(job);
    }
    emit_all(app);
    pump(app);
}

pub fn remove(app: &AppHandle, id: u64) {
    let queue = app.state::<Queue>();
    let removed = {
        let mut inner = queue.lock();
        let position = inner
            .jobs
            .iter()
            .position(|job| job.id == id && job.status.is_finished());
        position.map(|position| inner.jobs.remove(position))
    };
    if let Some(job) = removed {
        if job.status == Status::Failed {
            let _ = fs::remove_dir_all(job.working_dir());
        }
        emit_all(app);
    }
}

pub fn pump(app: &AppHandle) {
    let limit = usize::from(app.state::<Settings>().values().concurrency);
    let queue = app.state::<Queue>();
    let mut starting = Vec::new();
    {
        let mut inner = queue.lock();
        let mut running = inner.jobs.iter().filter(|job| job.status.is_running()).count();
        for job in inner.jobs.iter_mut() {
            if running >= limit {
                break;
            }
            if job.status == Status::Queued {
                job.status = Status::Fetching;
                running += 1;
                starting.push(job.clone());
            }
        }
    }
    for job in starting {
        emit_job(app, &job);
        tauri::async_runtime::spawn(run(app.clone(), job));
    }
}

async fn run(app: AppHandle, job: Job) {
    let outcome = transfer(&app, &job).await;
    settle(&app, &job, outcome);
    pump(&app);
}

async fn transfer(app: &AppHandle, job: &Job) -> Result<u64, String> {
    let working_dir = job.working_dir();
    fs::create_dir_all(&working_dir).map_err(|error| error.to_string())?;
    let (mut child, tree) = ytdlp::spawn(app, |args| {
        ytdlp::download_args(args, job.selection, &job.url, &working_dir)
    })?;
    register_tree(app, job.id, tree);
    let stdout = child.stdout.take().ok_or("yt-dlp output is unavailable")?;
    let stderr = child.stderr.take().ok_or("yt-dlp output is unavailable")?;
    let errors = tauri::async_runtime::spawn(collect_text(stderr));
    follow_progress(app, job, stdout).await;
    let status = child.wait().await.map_err(|error| error.to_string())?;
    let errors = errors.await.map_err(|error| error.to_string())?;
    if !status.success() {
        return Err(ytdlp::last_error(&errors));
    }
    deliver(&working_dir, &job.directory).map_err(|error| error.to_string())
}

fn register_tree(app: &AppHandle, id: u64, tree: ProcessTree) {
    let queue = app.state::<Queue>();
    let mut inner = queue.lock();
    let cancelling = inner
        .jobs
        .iter()
        .any(|job| job.id == id && job.status == Status::Cancelling);
    if cancelling {
        tree.kill();
    }
    inner.trees.insert(id, tree);
}

async fn follow_progress(app: &AppHandle, job: &Job, stdout: impl AsyncRead + Unpin) {
    let mut reader = BufReader::new(stdout);
    let mut line = Vec::new();
    let mut tracker = Tracker::new(job.selection);
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        if let Some(progress) = ytdlp::parse_progress(String::from_utf8_lossy(&line).trim()) {
            apply_progress(app, job.id, &mut tracker, progress);
        }
    }
}

fn apply_progress(app: &AppHandle, id: u64, tracker: &mut Tracker, progress: ProgressLine) {
    let queue = app.state::<Queue>();
    let mut inner = queue.lock();
    let Some(job) = inner.jobs.iter_mut().find(|job| job.id == id) else {
        return;
    };
    if job.status == Status::Cancelling {
        return;
    }
    match progress {
        ProgressLine::Transfer(transfer) => {
            let (fraction, size) = tracker.apply(&transfer);
            job.status = Status::Downloading;
            job.progress = job.progress.max(fraction);
            job.speed = transfer.speed;
            job.eta = transfer.eta;
            job.size = size.or(job.size);
        }
        ProgressLine::Processing => {
            job.status = Status::Processing;
            job.speed = None;
            job.eta = None;
        }
    }
    let job = job.clone();
    drop(inner);
    emit_job(app, &job);
}

async fn collect_text(stream: impl AsyncRead + Unpin) -> String {
    let mut reader = BufReader::new(stream);
    let mut bytes = Vec::new();
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line).await {
            Ok(0) | Err(_) => return String::from_utf8_lossy(&bytes).into_owned(),
            Ok(_) => bytes.extend_from_slice(&line),
        }
    }
}

fn settle(app: &AppHandle, finished: &Job, outcome: Result<u64, String>) {
    let queue = app.state::<Queue>();
    let mut inner = queue.lock();
    inner.trees.remove(&finished.id);
    let Some(job) = inner.jobs.iter_mut().find(|job| job.id == finished.id) else {
        return;
    };
    job.speed = None;
    job.eta = None;
    if job.status == Status::Cancelled || job.status == Status::Cancelling {
        job.status = Status::Cancelled;
        job.progress = 0.0;
        job.size = None;
        let _ = fs::remove_dir_all(job.working_dir());
    } else {
        match outcome {
            Ok(size) => {
                job.status = Status::Done;
                job.progress = 1.0;
                job.size = Some(size);
            }
            Err(error) => {
                job.status = Status::Failed;
                job.error = Some(error);
            }
        }
    }
    let job = job.clone();
    drop(inner);
    emit_job(app, &job);
}

fn deliver(working_dir: &Path, destination: &Path) -> io::Result<u64> {
    let mut delivered = 0;
    let mut total_size = 0;
    for entry in fs::read_dir(working_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            total_size += entry.metadata()?.len();
            move_without_overwriting(&entry.path(), destination)?;
            delivered += 1;
        }
    }
    if delivered == 0 {
        return Err(io::Error::other("yt-dlp finished without creating a file"));
    }
    fs::remove_dir_all(working_dir)?;
    Ok(total_size)
}

fn move_without_overwriting(source: &Path, destination: &Path) -> io::Result<()> {
    let stem = source.file_stem().unwrap_or_default().to_string_lossy();
    let extension = source
        .extension()
        .map(|extension| format!(".{}", extension.to_string_lossy()))
        .unwrap_or_default();
    let mut attempt = 1;
    loop {
        let name = match attempt {
            1 => format!("{stem}{extension}"),
            _ => format!("{stem} ({attempt}){extension}"),
        };
        let target = destination.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&target) {
            Ok(_) => return fs::rename(source, target),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => attempt += 1,
            Err(error) => return Err(error),
        }
    }
}

fn emit_job(app: &AppHandle, job: &Job) {
    let _ = app.emit("job", job);
}

fn emit_all(app: &AppHandle) {
    let _ = app.emit("jobs", app.state::<Queue>().snapshot());
}
