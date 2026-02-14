use tauri::Manager; // 1. 이 줄이 추가됐어 (창 관리자 불러오기)

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // 2. 여기서 'main'이라는 이름의 창을 찾아서 설정을 바꿔
            let window = app.get_webview_window("main").unwrap();
            
            // 3. 마우스 이벤트를 무시하도록 설정 (클릭 투과!)
            window.set_ignore_cursor_events(true).unwrap();
            
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}