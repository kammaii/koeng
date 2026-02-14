use std::ffi::c_void;
use std::ptr;
use std::thread;
use std::time::Duration;
use tauri::{LogicalPosition, Manager, Position}; 

#[derive(Clone, Copy, Debug)]
struct CursorPosition {
    x: i32,
    y: i32,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            let window_clone = window.clone();

            window.set_ignore_cursor_events(true).unwrap();

            thread::spawn(move || {
                println!("🚀 [Rust] 최종 위치 추적 모드 시작!");
                loop {
                    // 1. 커서 위치 확인
                    let caret_pos = get_caret_position();
                    
                    match caret_pos {
                        // ✅ Case 1: 커서 찾음 (메모장 등) -> 커서 따라가기
                        Some(pos) => {
                            let new_pos = Position::Logical(LogicalPosition {
                                x: (pos.x as f64) - 105.0, 
                                y: (pos.y as f64) - 50.0,  
                            });
                            let _ = window_clone.set_position(new_pos);
                        }

                        // ⚠️ Case 2: 커서 못 찾음 (크롬 등) -> 우측 하단 고정석
                        None => {
                            // 모니터 정보를 가져와서 화면 크기 계산
                            if let Ok(Some(monitor)) = window_clone.current_monitor() {
                                let screen_size = monitor.size(); // 픽셀 단위 크기 (예: 3000x2000)
                                let scale = monitor.scale_factor(); // 배율 (예: 2.0)
                                
                                // 픽셀을 포인트(Logical)로 변환
                                let logical_width = screen_size.width as f64 / scale;
                                let logical_height = screen_size.height as f64 / scale;

                                // 우측 하단 좌표 계산 (여백: 오른쪽 150, 아래 100)
                                let target_x = logical_width - 150.0;
                                let target_y = logical_height - 100.0;

                                let safe_pos = Position::Logical(LogicalPosition {
                                    x: target_x,
                                    y: target_y,
                                });
                                let _ = window_clone.set_position(safe_pos);
                            }
                        }
                    }
                    thread::sleep(Duration::from_millis(20));
                }
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ... (아래 macOS/Windows get_caret_position 코드는 기존과 동일하므로 유지!) ...
// =========================================================
// 🍎 macOS: 접근성 API(AX)
// =========================================================
#[cfg(target_os = "macos")]
fn get_caret_position() -> Option<CursorPosition> {
    use accessibility_sys::{
        kAXBoundsForRangeParameterizedAttribute, kAXFocusedUIElementAttribute,
        kAXSelectedTextRangeAttribute, kAXValueTypeCGRect, AXUIElementCopyAttributeValue,
        AXUIElementCopyParameterizedAttributeValue, AXUIElementCreateSystemWide, AXUIElementRef,
        AXValueGetValue, AXValueRef,
    };
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;

    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        let mut focused_element_ref: *const c_void = ptr::null();
        let result = AXUIElementCopyAttributeValue(system_wide, CFString::new(kAXFocusedUIElementAttribute).as_concrete_TypeRef(), &mut focused_element_ref);
        if result != 0 || focused_element_ref.is_null() { return None; }
        let focused_element = focused_element_ref as AXUIElementRef;

        let mut selected_range_value_ref: *const c_void = ptr::null();
        let range_result = AXUIElementCopyAttributeValue(focused_element, CFString::new(kAXSelectedTextRangeAttribute).as_concrete_TypeRef(), &mut selected_range_value_ref);
        if range_result != 0 || selected_range_value_ref.is_null() { return None; }
        let selected_range_value = selected_range_value_ref as AXValueRef;

        let mut bounds_value_ref: *const c_void = ptr::null();
        let bounds_result = AXUIElementCopyParameterizedAttributeValue(focused_element, CFString::new(kAXBoundsForRangeParameterizedAttribute).as_concrete_TypeRef(), selected_range_value as *const c_void, &mut bounds_value_ref);

        if bounds_result == 0 && !bounds_value_ref.is_null() {
            let bounds_value = bounds_value_ref as AXValueRef;
            let mut rect: CGRect = std::mem::zeroed();
            if AXValueGetValue(bounds_value, kAXValueTypeCGRect, &mut rect as *mut _ as *mut c_void) {
                return Some(CursorPosition { x: rect.origin.x as i32, y: (rect.origin.y + rect.size.height) as i32 });
            }
        }
        None
    }
}
// ... (Windows 및 구조체 정의 부분은 그대로 두세요) ...
#[cfg(target_os = "windows")]
fn get_caret_position() -> Option<CursorPosition> { None }
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CGPoint { x: f64, y: f64 }
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CGSize { width: f64, height: f64 }
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CGRect { origin: CGPoint, size: CGSize }