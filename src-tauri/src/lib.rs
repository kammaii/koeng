use std::ffi::c_void;
use std::ptr;
use std::thread;
use std::time::Duration;
use tauri::{LogicalPosition, Manager, Position, Emitter, AppHandle}; 

#[derive(Clone, serde::Serialize, Debug)]
struct CursorPayload {
    x: f64,
    y: f64,
    lang: String, 
}

#[derive(Clone, Copy, Debug)]
struct CursorPosition {
    x: i32,
    y: i32,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone(); 
            let window = app.get_webview_window("main").unwrap();
            
            // 유령 모드 (클릭 투과)
            window.set_ignore_cursor_events(true).unwrap();

            thread::spawn(move || {
                loop {
                    // 🚀 오직 마우스 위치만 추적한다!
                    let mouse_pos = get_mouse_position();
                    let handle_clone = app_handle.clone();
                    
                    app_handle.run_on_main_thread(move || {
                        let (current_lang, _) = get_mac_input_language();
                        let window = handle_clone.get_webview_window("main").unwrap();

                        // 기본값 (마우스를 못 찾을 경우)
                        let mut target_x = 100.0;
                        let mut target_y = 100.0;

                        if let Some(pos) = mouse_pos {
                            // 🐭 마우스 커서 오른쪽 아래에 배치 (가리지 않게 +16)
                            target_x = (pos.x as f64) + 16.0;
                            target_y = (pos.y as f64) + 16.0;
                        }

                        // 윈도우 이동
                        let _ = window.set_position(Position::Logical(LogicalPosition {
                            x: target_x,
                            y: target_y,
                        }));

                        // 상태 업데이트
                        let _ = handle_clone.emit("update-status", CursorPayload {
                            x: target_x,
                            y: target_y,
                            lang: current_lang,
                        });
                    });
                    
                    // 반응 속도: 60fps 수준 (약 16ms)
                    thread::sleep(Duration::from_millis(16)); 
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// =========================================================
// 🐭 macOS: 마우스 커서 위치 추적 (CoreGraphics)
// =========================================================
#[cfg(target_os = "macos")]
fn get_mouse_position() -> Option<CursorPosition> {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventCreate(source: *const c_void) -> *mut c_void;
        fn CGEventGetLocation(event: *mut c_void) -> CGPoint;
        fn CFRelease(cf: *const c_void); 
    }

    unsafe {
        let event = CGEventCreate(ptr::null());
        if event.is_null() { return None; }

        let loc = CGEventGetLocation(event);
        CFRelease(event); // 메모리 해제 필수

        Some(CursorPosition {
            x: loc.x as i32,
            y: loc.y as i32,
        })
    }
}

// =========================================================
// 🍎 macOS: 언어 감지
// =========================================================
#[cfg(target_os = "macos")]
fn get_mac_input_language() -> (String, String) {
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;
    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        fn TISCopyCurrentKeyboardInputSource() -> *mut c_void;
        fn TISGetInputSourceProperty(source: *mut c_void, property: *const c_void) -> *const c_void;
        static kTISPropertyInputSourceID: *const c_void;
        static kTISPropertyLocalizedName: *const c_void;
    }
    unsafe {
        let source = TISCopyCurrentKeyboardInputSource();
        if source.is_null() { return ("en".to_string(), "NULL".to_string()); }
        let id_ptr = TISGetInputSourceProperty(source, kTISPropertyInputSourceID);
        let id_str = if !id_ptr.is_null() { CFString::wrap_under_get_rule(id_ptr as *const _).to_string() } else { "None".to_string() };
        let name_ptr = TISGetInputSourceProperty(source, kTISPropertyLocalizedName);
        let name_str = if !name_ptr.is_null() { CFString::wrap_under_get_rule(name_ptr as *const _).to_string() } else { "None".to_string() };
        let debug_msg = format!("ID=[{}] Name=[{}]", id_str, name_str);
        let lower_id = id_str.to_lowercase();
        let lower_name = name_str.to_lowercase();
        if lower_id.contains("korean") || lower_id.contains("hangul") || lower_id.contains("2set") || lower_name.contains("두벌식") || lower_name.contains("korean") || lower_name.contains("한글") {
            return ("ko".to_string(), debug_msg);
        }
        ("en".to_string(), debug_msg)
    }
}

// 🪟 윈도우 지원을 위한 뼈대 (나중에 채워넣으면 됨)
#[cfg(target_os = "windows")] 
fn get_mouse_position() -> Option<CursorPosition> { 
    // TODO: Win32 API (GetCursorPos) 사용
    None 
}
#[cfg(target_os = "windows")] 
fn get_mac_input_language() -> (String, String) { 
    // TODO: Win32 API (ImmGetConversionStatus) 사용
    ("en".to_string(), "Win".to_string()) 
}

#[cfg(target_os = "macos")] #[repr(C)] #[derive(Clone, Copy, Debug)] struct CGPoint { x: f64, y: f64 }