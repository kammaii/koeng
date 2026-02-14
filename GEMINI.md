# Project Rules 

1. **Language:**
   - 모든 코드의 주석(Comment)과 설명은 **한국어**로 작성한다.
   - 변수명과 함수명은 영어(snake_case for Rust, camelCase for JS)를 사용한다.

2. **Tech Stack:**
   - **Backend:** Rust, Tauri v2 (절대 Tauri v1 문법을 사용하지 않는다).
   - **Frontend:** Vanilla JS (HTML/CSS/JS). No React, No Vue.
   - **Platform:** Windows & macOS Cross-platform support.

3. **Code Style:**
   - Rust 코드는 안전성(Safety)을 최우선으로 한다. `unwrap()` 사용을 지양하고 `match`나 `if let`으로 에러를 처리한다.
   - 코드를 수정할 때는 전체 파일을 다시 쓰지 말고, 바뀐 부분만 명확하게 제시한다.

4. **Task:**
   - 우리는 현재 "OS 전역 커서 위치 추적(Global Caret Tracking)" 기능을 만들고 있다.