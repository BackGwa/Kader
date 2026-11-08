mod process;
mod queue;
mod settings;
mod update;
mod ytdlp;

use std::path::PathBuf;

use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};

use queue::{Job, NewJob, Queue};
use settings::{Settings, Values};
use update::{Engine, EngineStatus};
use ytdlp::{Probe, Probes};

#[tauri::command]
async fn probe(app: AppHandle, token: u64, url: String) -> Result<Probe, String> {
    ytdlp::probe(&app, token, &url).await
}

#[tauri::command]
fn cancel_probe(probes: State<'_, Probes>, token: u64) {
    probes.cancel(token);
}

#[tauri::command]
fn enqueue(app: AppHandle, items: Vec<NewJob>) {
    queue::enqueue(&app, items);
}

#[tauri::command]
fn cancel(app: AppHandle, id: u64) {
    queue::cancel(&app, id);
}

#[tauri::command]
fn retry(app: AppHandle, id: u64) {
    queue::retry(&app, id);
}

#[tauri::command]
fn remove(app: AppHandle, id: u64) {
    queue::remove(&app, id);
}

#[tauri::command]
fn snapshot(queue: State<'_, Queue>) -> Vec<Job> {
    queue.snapshot()
}

#[tauri::command]
fn get_settings(settings: State<'_, Settings>) -> Values {
    settings.values()
}

#[tauri::command]
fn set_concurrency(app: AppHandle, concurrency: u8) -> Result<(), String> {
    app.state::<Settings>().set_concurrency(concurrency)?;
    queue::pump(&app);
    Ok(())
}

#[tauri::command]
fn set_save_dir(settings: State<'_, Settings>, save_dir: PathBuf) -> Result<(), String> {
    settings.set_save_dir(save_dir)
}

#[tauri::command]
fn engine_status(engine: State<'_, Engine>) -> EngineStatus {
    engine.status()
}

#[tauri::command]
fn confirm_quit(app: AppHandle) {
    app.state::<Queue>().shutdown();
    app.exit(0);
}

fn request_quit_confirmation(app: &AppHandle) -> bool {
    let unfinished = app.state::<Queue>().unfinished_count();
    if unfinished == 0 {
        return false;
    }
    let _ = app.emit("quit-requested", unfinished);
    true
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Queue::default())
        .manage(Probes::default())
        .manage(Engine::default())
        .setup(|app| {
            app.manage(Settings::load(app.handle())?);
            update::start(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if request_quit_confirmation(window.app_handle()) {
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            probe,
            cancel_probe,
            enqueue,
            cancel,
            retry,
            remove,
            snapshot,
            get_settings,
            set_concurrency,
            set_save_dir,
            engine_status,
            confirm_quit
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Kader")
        .run(|app, event| {
            if let RunEvent::ExitRequested { api, .. } = event {
                if request_quit_confirmation(app) {
                    api.prevent_exit();
                }
            }
        });
}
