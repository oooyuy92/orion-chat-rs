pub mod db;
pub mod error;
pub mod models;
pub mod providers;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
