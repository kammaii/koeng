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
            
            // ✅ [수정 1] 유령 모드 다시 켜기! (클릭 투과)
            // 이제 마우스가 이 박스를 통과해서 뒤에 있는 버튼을 누를 수 있어.
            window.set_ignore_cursor_events(true).unwrap();

            thread::spawn(move || {
                loop {
                    let caret_pos_opt = get_caret_position();
                    let handle_clone = app_handle.clone();
                    
                    app_handle.run_on_main_thread(move || {
                        let (current_lang, _) = get_mac_input_language();
                        let window = handle_clone.get_webview_window("main").unwrap();

                        let (target_x, target_y) = match caret_pos_opt {
                            Some(pos) => {
                                // ✅ [수정 3] 박스가 작아졌으니 오프셋 조정
                                // 커서보다 살짝 왼쪽 위로 (박스 크기 24px 고려)
                                ((pos.x as f64) - 28.0, (pos.y as f64) - 30.0)
                            }
                            None => {
                                if let Ok(Some(monitor)) = window.current_monitor() {
                                    let size = monitor.size();
                                    let scale = monitor.scale_factor();
                                    let logical_width = size.width as f64 / scale;
                                    let logical_height = size.height as f64 / scale;

                                    // ✅ [수정 2] 독(Dock) 회피를 위해 높이를 많이 띄움
                                    // 바닥에서 120px 위, 오른쪽에서 50px 안쪽
                                    (logical_width - 50.0, logical_height - 120.0)
                                } else {
                                    (100.0, 100.0)
                                }
                            }
                        };

                        let _ = window.set_position(Position::Logical(LogicalPosition {
                            x: target_x,
                            y: target_y,
                        }));

                        let _ = handle_clone.emit("update-status", CursorPayload {
                            x: target_x,
                            y: target_y,
                            lang: current_lang,
                        });
                    });
                    thread::sleep(Duration::from_millis(50)); // 반응속도 빠르게 (0.05초)
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ... (아래 get_mac_input_language, get_caret_position 등 함수들은 기존과 완벽히 동일하므로 그대로 둬!) ...
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
        if AXUIElementCopyAttributeValue(focused_element, CFString::new(kAXSelectedTextRangeAttribute).as_concrete_TypeRef(), &mut selected_range_value_ref) != 0 || selected_range_value_ref.is_null() { return None; }
        let mut bounds_value_ref: *const c_void = ptr::null();
        if AXUIElementCopyParameterizedAttributeValue(focused_element, CFString::new(kAXBoundsForRangeParameterizedAttribute).as_concrete_TypeRef(), selected_range_value_ref as *const c_void, &mut bounds_value_ref) == 0 && !bounds_value_ref.is_null() {
            let bounds_value = bounds_value_ref as AXValueRef;
            let mut rect: CGRect = std::mem::zeroed();
            if AXValueGetValue(bounds_value, kAXValueTypeCGRect, &mut rect as *mut _ as *mut c_void) { return Some(CursorPosition { x: rect.origin.x as i32, y: (rect.origin.y + rect.size.height) as i32 }); }
        }
        None
    }
}
#[cfg(target_os = "windows")] fn get_caret_position() -> Option<CursorPosition> { None }
#[cfg(target_os = "windows")] fn get_mac_input_language() -> (String, String) { ("en".to_string(), "Win".to_string()) }
#[cfg(target_os = "macos")] #[repr(C)] #[derive(Clone, Copy, Debug)] struct CGPoint { x: f64, y: f64 }
#[cfg(target_os = "macos")] #[repr(C)] #[derive(Clone, Copy, Debug)] struct CGSize { width: f64, height: f64 }
#[cfg(target_os = "macos")] #[repr(C)] #[derive(Clone, Copy, Debug)] struct CGRect { origin: CGPoint, size: CGSize }