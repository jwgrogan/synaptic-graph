#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use commands::AppState;

fn main() {
    let state = AppState::new().expect("Failed to initialize memory-graph database");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::get_all_impulses,
            commands::get_all_connections,
            commands::get_memory_stats,
            commands::search_memories,
            commands::get_impulse_detail,
            commands::get_ghost_sources,
            commands::get_ghost_nodes,
            commands::quick_save,
        ])
        .run(tauri::generate_context!())
        .expect("error while running memory-graph-ui");
}
