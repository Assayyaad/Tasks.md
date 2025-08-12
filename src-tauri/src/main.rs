use tauri::{command, Manager, State, AppHandle, Emitter};
use std::fs;
use std::path::Path;
use serde_json::{Value, Map};
use std::collections::HashMap;
use uuid::Uuid;
use std::sync::Mutex;
use std::time::SystemTime;
use tauri::async_runtime;

// App state for configuration
#[derive(Default)]
pub struct AppState {
    config_dir: Mutex<String>,
    tasks_dir: Mutex<String>,
    title: Mutex<String>,
}

// File watcher state
#[derive(Default)]
pub struct WatchState {
    watching: Mutex<bool>,
}

#[command]
async fn get_tags(path: String, state: State<'_, AppState>) -> Result<Value, String> {
    let config_dir = state.config_dir.lock().unwrap().clone();
    let tags_path = format!("{}/tags.json", config_dir);

    let tags = fs::read_to_string(&tags_path)
        .map(|content| serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new())))
        .unwrap_or(Value::Object(Map::new()));

    if let Value::Object(obj) = tags {
        Ok(obj.get(&path).cloned().unwrap_or(Value::Object(Map::new())))
    } else {
        Ok(Value::Object(Map::new()))
    }
}

#[command]
async fn update_tag_background_color(path: String, colors: Value, state: State<'_, AppState>) -> Result<(), String> {
    let config_dir = state.config_dir.lock().unwrap().clone();
    let tags_path = format!("{}/tags.json", config_dir);

    let mut tags = fs::read_to_string(&tags_path)
        .map(|content| serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new())))
        .unwrap_or(Value::Object(Map::new()));

    if let Value::Object(ref mut obj) = tags {
        obj.insert(path, colors);
    }

    fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    fs::write(&tags_path, serde_json::to_string(&tags).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[command]
async fn get_title(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.title.lock().unwrap().clone())
}

#[command]
async fn get_resource(path: String, state: State<'_, AppState>) -> Result<Value, String> {
    let tasks_dir = state.tasks_dir.lock().unwrap().clone();
    let full_path = format!("{}/{}", tasks_dir, path);

    // Create directory if it doesn't exist
    if !Path::new(&full_path).exists() {
        fs::create_dir_all(&full_path).map_err(|e| e.to_string())?;
        return Ok(Value::Array(vec![]));
    }

    let entries = fs::read_dir(&full_path).map_err(|e| e.to_string())?;
    let mut lanes = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let entry_path = entry.path();

        if entry_path.is_dir() && !entry.file_name().to_string_lossy().starts_with('.') {
            let lane_name = entry.file_name().to_string_lossy().to_string();
            let lane_path = entry_path.to_string_lossy().to_string();

            let files = get_lane_files(&lane_path)?;

            lanes.push(serde_json::json!({
                "name": lane_name,
                "files": files
            }));
        }
    }

    Ok(Value::Array(lanes))
}

fn get_lane_files(lane_path: &str) -> Result<Vec<Value>, String> {
    let entries = fs::read_dir(lane_path).map_err(|e| e.to_string())?;
    let mut files = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_path = entry.path();

        if let Some(file_name) = file_path.file_name() {
            let file_name_str = file_name.to_string_lossy();

            if file_name_str.ends_with(".md") && !file_name_str.starts_with('.') {
                let content = fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
                let metadata = fs::metadata(&file_path).map_err(|e| e.to_string())?;

                let name = file_name_str.strip_suffix(".md").unwrap_or(&file_name_str);

                files.push(serde_json::json!({
                    "name": name,
                    "content": content,
                    "lastUpdated": metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                    "createdAt": metadata.created().unwrap_or(SystemTime::UNIX_EPOCH)
                }));
            }
        }
    }

    Ok(files)
}

#[command]
async fn create_resource(path: String, is_file: Option<bool>, content: Option<String>, state: State<'_, AppState>) -> Result<(), String> {
    let tasks_dir = state.tasks_dir.lock().unwrap().clone();
    let full_path = format!("{}/{}", tasks_dir, path);

    if is_file.unwrap_or(false) {
        if let Some(parent) = Path::new(&full_path).parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&full_path, content.unwrap_or_default()).map_err(|e| e.to_string())?;
    } else {
        fs::create_dir_all(&full_path).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[command]
async fn update_resource(path: String, new_path: Option<String>, content: Option<String>, state: State<'_, AppState>) -> Result<(), String> {
    let tasks_dir = state.tasks_dir.lock().unwrap().clone();
    let old_full_path = format!("{}/{}", tasks_dir, path);

    let new_path_clean = new_path.unwrap_or(path.clone())
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) { ' ' } else { c })
        .collect::<String>();

    let new_full_path = format!("{}/{}", tasks_dir, new_path_clean);

    if old_full_path != new_full_path {
        if let Some(parent) = Path::new(&new_full_path).parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::rename(&old_full_path, &new_full_path).map_err(|e| e.to_string())?;
    }

    if let Some(new_content) = content {
        let metadata = fs::metadata(&new_full_path).map_err(|e| e.to_string())?;
        if metadata.is_file() {
            fs::write(&new_full_path, new_content).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[command]
async fn delete_resource(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let tasks_dir = state.tasks_dir.lock().unwrap().clone();
    let full_path = format!("{}/{}", tasks_dir, path);

    if Path::new(&full_path).is_dir() {
        fs::remove_dir_all(&full_path).map_err(|e| e.to_string())?;
    } else {
        fs::remove_file(&full_path).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[command]
async fn upload_image(file_data: Vec<u8>, filename: String, state: State<'_, AppState>) -> Result<String, String> {
    let config_dir = state.config_dir.lock().unwrap().clone();
    let images_dir = format!("{}/images", config_dir);

    fs::create_dir_all(&images_dir).map_err(|e| e.to_string())?;

    let extension = filename.split('.').last().unwrap_or("png");
    let image_name = format!("{}.{}", Uuid::new_v4(), extension);
    let image_path = format!("{}/{}", images_dir, image_name);

    fs::write(&image_path, file_data).map_err(|e| e.to_string())?;

    Ok(image_name)
}

#[command]
async fn update_sort(path: String, sort_data: Value, state: State<'_, AppState>) -> Result<(), String> {
    let config_dir = state.config_dir.lock().unwrap().clone();
    let sort_path = format!("{}/sort.json", config_dir);

    let mut current_sort = fs::read_to_string(&sort_path)
        .map(|content| serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new())))
        .unwrap_or(Value::Object(Map::new()));

    if let Value::Object(ref mut obj) = current_sort {
        obj.insert(path, sort_data);
    }

    fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    fs::write(&sort_path, serde_json::to_string(&current_sort).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[command]
async fn get_sort(path: String, state: State<'_, AppState>) -> Result<Value, String> {
    let config_dir = state.config_dir.lock().unwrap().clone();
    let sort_path = format!("{}/sort.json", config_dir);

    let sort = fs::read_to_string(&sort_path)
        .map(|content| serde_json::from_str::<Value>(&content).unwrap_or(Value::Object(Map::new())))
        .unwrap_or(Value::Object(Map::new()));

    if let Value::Object(obj) = sort {
        Ok(obj.get(&path).cloned().unwrap_or(Value::Object(Map::new())))
    } else {
        Ok(Value::Object(Map::new()))
    }
}

#[command]
async fn get_image(filename: String, state: State<'_, AppState>) -> Result<Vec<u8>, String> {
    let config_dir = state.config_dir.lock().unwrap().clone();
    let image_path = format!("{}/images/{}", config_dir, filename);

    fs::read(&image_path).map_err(|e| e.to_string())
}

#[command]
async fn start_file_watcher(app_handle: AppHandle, state: State<'_, AppState>, watch_state: State<'_, WatchState>) -> Result<(), String> {
    let tasks_dir = state.tasks_dir.lock().unwrap().clone();
    *watch_state.watching.lock().unwrap() = true;

    // Simple file watcher using polling (you might want to use notify crate for better performance)
    tauri::async_runtime::spawn(async move {
        let mut last_modified = std::collections::HashMap::new();

        loop {
            if let Ok(entries) = fs::read_dir(&tasks_dir) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            let path = entry.path();
                            let path_str = path.to_string_lossy().to_string();

                            if let Some(&last_mod) = last_modified.get(&path_str) {
                                if modified != last_mod {
                                    let _ = app_handle.emit("files-changed", ());
                                }
                            }

                            last_modified.insert(path_str, modified);
                        }
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            config_dir: Mutex::new(std::env::var("CONFIG_DIR").unwrap_or_else(|_| "config".to_string())),
            tasks_dir: Mutex::new(std::env::var("TASKS_DIR").unwrap_or_else(|_| "tasks".to_string())),
            title: Mutex::new(std::env::var("TITLE").unwrap_or_default()),
        })
        .manage(WatchState::default())
        .invoke_handler(tauri::generate_handler![
            get_tags,
            update_tag_background_color,
            get_title,
            get_resource,
            create_resource,
            update_resource,
            delete_resource,
            upload_image,
            update_sort,
            get_sort,
            get_image,
            start_file_watcher
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}