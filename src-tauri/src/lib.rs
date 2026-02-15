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
            
            // ✅ [수정 1] 모든 데스크탑(Spaces)에서 보이게 설정!
            // 이 줄이 있어야 화면을 전환해도 박스가 계속 따라다님.
            window.set_visible_on_all_workspaces(true).unwrap();

            // ✅ [수정 2] 유령 모드 (클릭 투과) 및 최상단 고정 강화
            window.set_ignore_cursor_events(true).unwrap();
            window.set_always_on_top(true).unwrap();

            thread::spawn(move || {
                loop {
                    // 🚀 위치 결정 로직 실행
                    let (target_x, target_y) = get_best_position_logic();
                    
                    let handle_clone = app_handle.clone();
                    
                    app_handle.run_on_main_thread(move || {
                        let (current_lang, _) = get_mac_input_language();
                        let window = handle_clone.get_webview_window("main").unwrap();

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
                    
                    thread::sleep(Duration::from_millis(16)); // 60fps
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// =========================================================
// 🧠 위치 결정 로직 (텍스트 커서 vs 마우스)
// =========================================================
#[cfg(target_os = "macos")]
fn get_best_position_logic() -> (f64, f64) {
    let caret_opt = get_caret_position();
    let mouse_opt = get_mouse_position();

    // 1. 텍스트 커서(Caret) 우선
    if let Some(caret) = caret_opt {
        if caret.x > 1 && caret.y > 1 {
            // 마우스와의 거리 체크 (엑셀 버그 방지)
            if let Some(mouse) = mouse_opt {
                let dx = (caret.x - mouse.x).abs();
                let dy = (caret.y - mouse.y).abs();
                if (dx + dy) > 800 {
                    // 너무 멀면 마우스 위치 사용
                    return ((mouse.x as f64) + 16.0, (mouse.y as f64) + 16.0);
                }
            }
            // 텍스트 커서 위
            return ((caret.x as f64) - 35.0, (caret.y as f64) - 35.0);
        }
    }

    // 2. 마우스 커서 차선
    if let Some(mouse) = mouse_opt {
        return ((mouse.x as f64) + 16.0, (mouse.y as f64) + 16.0);
    }

    (100.0, 100.0)
}

// ... (아래 함수들은 기존과 완벽히 동일함) ...
#[cfg(target_os = "macos")]
fn get_caret_position() -> Option<CursorPosition> {
    use accessibility_sys::{ kAXBoundsForRangeParameterizedAttribute, kAXFocusedUIElementAttribute, kAXSelectedTextRangeAttribute, kAXValueTypeCGRect, AXUIElementCopyAttributeValue, AXUIElementCopyParameterizedAttributeValue, AXUIElementCreateSystemWide, AXUIElementRef, AXValueGetValue, AXValueRef };
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;
    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        let mut focused_element_ref: *const c_void = ptr::null();
        if AXUIElementCopyAttributeValue(system_wide, CFString::new(kAXFocusedUIElementAttribute).as_concrete_TypeRef(), &mut focused_element_ref) != 0 || focused_element_ref.is_null() { return None; }
        let focused_element = focused_element_ref as AXUIElementRef;
        let mut selected_range_value_ref: *const c_void = ptr::null();
        let range_result = AXUIElementCopyAttributeValue(focused_element, CFString::new(kAXSelectedTextRangeAttribute).as_concrete_TypeRef(), &mut selected_range_value_ref);
        if range_result == 0 && !selected_range_value_ref.is_null() {
            let mut bounds_value_ref: *const c_void = ptr::null();
            let bounds_result = AXUIElementCopyParameterizedAttributeValue(focused_element, CFString::new(kAXBoundsForRangeParameterizedAttribute).as_concrete_TypeRef(), selected_range_value_ref as *const c_void, &mut bounds_value_ref);
            if bounds_result == 0 && !bounds_value_ref.is_null() {
                let bounds_value = bounds_value_ref as AXValueRef;
                let mut rect: CGRect = std::mem::zeroed();
                if AXValueGetValue(bounds_value, kAXValueTypeCGRect, &mut rect as *mut _ as *mut c_void) { return Some(CursorPosition { x: rect.origin.x as i32, y: (rect.origin.y + rect.size.height) as i32 }); }
            }
        }
        None
    }
}

#[cfg(target_os = "macos")]
fn get_mouse_position() -> Option<CursorPosition> {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" { fn CGEventCreate(source: *const c_void) -> *mut c_void; fn CGEventGetLocation(event: *mut c_void) -> CGPoint; fn CFRelease(cf: *const c_void); }
    unsafe {
        let event = CGEventCreate(ptr::null());
        if event.is_null() { return None; }
        let loc = CGEventGetLocation(event);
        CFRelease(event); 
        Some(CursorPosition { x: loc.x as i32, y: loc.y as i32 })
    }
}

#[cfg(target_os = "macos")]
fn get_mac_input_language() -> (String, String) {
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;
    #[link(name = "Carbon", kind = "framework")]
    extern "C" { fn TISCopyCurrentKeyboardInputSource() -> *mut c_void; fn TISGetInputSourceProperty(source: *mut c_void, property: *const c_void) -> *const c_void; static kTISPropertyInputSourceID: *const c_void; static kTISPropertyLocalizedName: *const c_void; }
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
        if lower_id.contains("korean") || lower_id.contains("hangul") || lower_id.contains("2set") || lower_name.contains("두벌식") || lower_name.contains("korean") || lower_name.contains("한글") { return ("ko".to_string(), debug_msg); }
        ("en".to_string(), debug_msg)
    }
}

#[cfg(target_os = "windows")] fn get_caret_position() -> Option<CursorPosition> { None }
#[cfg(target_os = "windows")] fn get_mouse_position() -> Option<CursorPosition> { None }
#[cfg(target_os = "windows")] fn get_mac_input_language() -> (String, String) { ("en".to_string(), "Win".to_string()) }
#[cfg(target_os = "macos")] #[repr(C)] #[derive(Clone, Copy, Debug)] struct CGPoint { x: f64, y: f64 }
#[cfg(target_os = "macos")] #[repr(C)] #[derive(Clone, Copy, Debug)] struct CGSize { width: f64, height: f64 }
#[cfg(target_os = "macos")] #[repr(C)] #[derive(Clone, Copy, Debug)] struct CGRect { origin: CGPoint, size: CGSize }