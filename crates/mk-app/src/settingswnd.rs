//! settingswnd — 설정 창(FR-5, M4). `--worker settings`로 뜨는 온ديمانド 창(DR-7).
//! 그리드·팝업과 달리 **표준 Win32 컨트롤**(BUTTON/EDIT/STATIC)을 쓴다 — 입력·IME·키보드 탐색·
//! 접근성을 OS가 처리하게 두는 편이 안전하고(DR-4), 매니페스트의 공용 컨트롤 v6로 현대 테마가 적용된다.
//!
//! 저장은 [`crate::config::save`] → `data\settings.ini`. 값 검증은 mk-core가 담당(범위 클램프).

use mk_core::Settings;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetStockObject, DEFAULT_GUI_FONT, HFONT, HGDIOBJ};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    CheckDlgButton, IsDlgButtonChecked, BST_CHECKED, DLG_BUTTON_CHECK_STATE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRect, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetDlgItemInt, GetMessageW, LoadCursorW, LoadIconW, MessageBoxW, PostQuitMessage,
    RegisterClassW, SendMessageW, SetDlgItemInt, SetForegroundWindow, ShowWindow,
    SystemParametersInfoW, TranslateMessage, BS_AUTOCHECKBOX, BS_DEFPUSHBUTTON, BS_GROUPBOX,
    BS_PUSHBUTTON, CW_USEDEFAULT, ES_NUMBER, HMENU, IDC_ARROW, MB_ICONWARNING, MB_OK, MSG,
    SPI_GETWORKAREA, SW_SHOWNORMAL, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_SETFONT, WNDCLASSW, WS_BORDER, WS_CAPTION,
    WS_CHILD, WS_EX_CLIENTEDGE, WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE,
};

const CLASS: PCWSTR = w!("NexaMemKeeperSettings");
/// **클라이언트 영역** 크기(모든 레이아웃 좌표의 기준). 창 크기는 여기에 테두리·제목 표시줄을
/// 더해 [`AdjustWindowRect`]로 역산한다 — 이 값을 그대로 창 크기로 넘기면 아래쪽이 잘린다(08-07 QA).
const W: i32 = 452;
const H: i32 = 548;
/// 하단 버튼 줄 y좌표(콘텐츠 마지막 + 여백).
const BTN_Y: i32 = 496;

// 컨트롤 ID
const ID_AUTO_ENABLED: i32 = 1001;
const ID_THRESHOLD: i32 = 1002;
const ID_COOLDOWN: i32 = 1003;
const ID_INTERVAL: i32 = 1004;
const ID_A_WS: i32 = 1010;
const ID_A_SBLOW: i32 = 1011;
const ID_A_SBALL: i32 = 1012;
const ID_A_MOD: i32 = 1013;
const ID_A_SYSWS: i32 = 1014;
const ID_A_COMBINE: i32 = 1015;
const ID_POLL: i32 = 1020;
const ID_NOTIFY: i32 = 1021;
const ID_HIDE_PSEUDO: i32 = 1022;
const ID_VIEW_FLAT: i32 = 1023;
const ID_REFRESH_MS: i32 = 1024;
const ID_TOPN: i32 = 1025;
const ID_SAVE: i32 = 1030;
const ID_CANCEL: i32 = 1031;
const ID_RESET: i32 = 1032;

/// `--worker settings` 진입 — 창을 닫을 때까지 메시지 루프. 저장 시 0, 취소도 0.
pub fn run() -> u32 {
    unsafe {
        let Ok(hinst) = GetModuleHandleW(None) else {
            return 1;
        };
        let wc = WNDCLASSW {
            lpszClassName: CLASS,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinst.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hIcon: LoadIconW(Some(hinst.into()), PCWSTR(std::ptr::without_provenance(1)))
                .unwrap_or_default(),
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(
                (windows::Win32::Graphics::Gdi::COLOR_BTNFACE.0 + 1) as isize as *mut _,
            ),
            ..Default::default()
        };
        RegisterClassW(&wc);
        // 클라이언트 W×H가 정확히 나오도록 창 크기를 역산하고, 작업 영역 가운데에 놓는다.
        let style = WINDOW_STYLE(WS_OVERLAPPED.0 | WS_CAPTION.0 | WS_SYSMENU.0 | WS_VISIBLE.0);
        let mut rc = RECT {
            left: 0,
            top: 0,
            right: W,
            bottom: H,
        };
        let _ = AdjustWindowRect(&mut rc, style, false);
        let (win_w, win_h) = (rc.right - rc.left, rc.bottom - rc.top);
        let (x, y) = center_on_work_area(win_w, win_h);
        let Ok(hwnd) = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            CLASS,
            w!("Nexa MemKeeper — 설정"),
            style,
            x,
            y,
            win_w,
            win_h,
            None,
            None,
            Some(hinst.into()),
            None,
        ) else {
            return 1;
        };
        let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
        let _ = SetForegroundWindow(hwnd);
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        0
    }
}

/// 작업 영역(작업 표시줄 제외) 가운데 좌표.
unsafe fn center_on_work_area(w: i32, h: i32) -> (i32, i32) {
    let mut wa = RECT::default();
    if SystemParametersInfoW(
        SPI_GETWORKAREA,
        0,
        Some(&mut wa as *mut RECT as *mut core::ffi::c_void),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
    )
    .is_err()
    {
        return (CW_USEDEFAULT, CW_USEDEFAULT);
    }
    (
        wa.left + ((wa.right - wa.left) - w).max(0) / 2,
        wa.top + ((wa.bottom - wa.top) - h).max(0) / 2,
    )
}

unsafe fn label(parent: HWND, text: PCWSTR, x: i32, y: i32, w: i32, font: HFONT) {
    if let Ok(h) = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("STATIC"),
        text,
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0),
        x,
        y,
        w,
        18,
        Some(parent),
        None,
        None,
        None,
    ) {
        SendMessageW(
            h,
            WM_SETFONT,
            Some(WPARAM(font.0 as usize)),
            Some(LPARAM(1)),
        );
    }
}

unsafe fn group(parent: HWND, text: PCWSTR, x: i32, y: i32, w: i32, h: i32, font: HFONT) {
    if let Ok(g) = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        text,
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | BS_GROUPBOX as u32),
        x,
        y,
        w,
        h,
        Some(parent),
        None,
        None,
        None,
    ) {
        SendMessageW(
            g,
            WM_SETFONT,
            Some(WPARAM(font.0 as usize)),
            Some(LPARAM(1)),
        );
    }
}

unsafe fn checkbox(parent: HWND, text: PCWSTR, id: i32, x: i32, y: i32, w: i32, font: HFONT) {
    if let Ok(h) = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        text,
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | BS_AUTOCHECKBOX as u32),
        x,
        y,
        w,
        20,
        Some(parent),
        Some(HMENU(id as isize as *mut _)),
        None,
        None,
    ) {
        SendMessageW(
            h,
            WM_SETFONT,
            Some(WPARAM(font.0 as usize)),
            Some(LPARAM(1)),
        );
    }
}

unsafe fn number(parent: HWND, id: i32, x: i32, y: i32, w: i32, font: HFONT) {
    if let Ok(h) = CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("EDIT"),
        w!(""),
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | WS_BORDER.0 | ES_NUMBER as u32),
        x,
        y,
        w,
        22,
        Some(parent),
        Some(HMENU(id as isize as *mut _)),
        None,
        None,
    ) {
        SendMessageW(
            h,
            WM_SETFONT,
            Some(WPARAM(font.0 as usize)),
            Some(LPARAM(1)),
        );
    }
}

/// (x, y, 폭, 기본 버튼 여부) — 인자 수를 줄이기 위한 묶음.
struct BtnSpec(i32, i32, i32, bool);

unsafe fn button(parent: HWND, text: PCWSTR, id: i32, spec: BtnSpec, font: HFONT) {
    let BtnSpec(x, y, w, def) = spec;
    let style = if def { BS_DEFPUSHBUTTON } else { BS_PUSHBUTTON };
    if let Ok(h) = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        text,
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | style as u32),
        x,
        y,
        w,
        28,
        Some(parent),
        Some(HMENU(id as isize as *mut _)),
        None,
        None,
    ) {
        SendMessageW(
            h,
            WM_SETFONT,
            Some(WPARAM(font.0 as usize)),
            Some(LPARAM(1)),
        );
    }
}

unsafe fn set_check(hwnd: HWND, id: i32, on: bool) {
    let _ = CheckDlgButton(hwnd, id, DLG_BUTTON_CHECK_STATE(u32::from(on)));
}

unsafe fn get_check(hwnd: HWND, id: i32) -> bool {
    IsDlgButtonChecked(hwnd, id) == BST_CHECKED.0
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let font = HFONT(GetStockObject(DEFAULT_GUI_FONT).0);
            build_ui(hwnd, font);
            load_into_ui(hwnd, &crate::config::load());
            LRESULT(0)
        }
        WM_COMMAND => {
            match (wp.0 & 0xFFFF) as i32 {
                ID_SAVE => {
                    if save_from_ui(hwnd) {
                        let _ = DestroyWindow(hwnd);
                    }
                }
                ID_CANCEL => {
                    let _ = DestroyWindow(hwnd);
                }
                ID_RESET => load_into_ui(hwnd, &Settings::default()),
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe fn build_ui(hwnd: HWND, font: HFONT) {
    // x = 좌측 여백, gw = 그룹 폭(좌우 여백 동일하게 — 08-07 QA에서 우측이 좁아 보였다)
    let (x, gw) = (14, W - 28);
    // 라벨/입력 칸 규격 — 모든 그룹이 같은 열에 정렬된다. 라벨은 "최소 간격 (ms, 60000~1800000)"
    // 같은 가장 긴 문구가 **한 줄에 들어가는 폭**이어야 한다(08-07 QA: 2줄로 접혀 보였다).
    const LBL_W: i32 = 218;
    const NUM_X: i32 = 232;
    const NUM_W: i32 = 96;
    // ── 자동 정리
    group(hwnd, w!("자동 정리"), x, 10, gw, 128, font);
    checkbox(
        hwnd,
        w!("메모리 사용률이 임계값을 넘으면 자동 정리"),
        ID_AUTO_ENABLED,
        x + 14,
        32,
        gw - 30,
        font,
    );
    label(hwnd, w!("임계값 (%, 50~95)"), x + 14, 60, LBL_W, font);
    number(hwnd, ID_THRESHOLD, x + NUM_X, 57, NUM_W, font);
    label(
        hwnd,
        w!("최소 간격 (ms, 60000~1800000)"),
        x + 14,
        86,
        LBL_W,
        font,
    );
    number(hwnd, ID_COOLDOWN, x + NUM_X, 83, NUM_W, font);
    label(hwnd, w!("주기 정리 (ms, 0=끔)"), x + 14, 112, LBL_W, font);
    number(hwnd, ID_INTERVAL, x + NUM_X, 109, NUM_W, font);

    // ── 정리 영역
    group(hwnd, w!("정리 영역"), x, 148, gw, 178, font);
    checkbox(
        hwnd,
        w!("① 프로세스 워킹셋 트림 (안전)"),
        ID_A_WS,
        x + 14,
        170,
        gw - 30,
        font,
    );
    checkbox(
        hwnd,
        w!("② 우선순위 0 스탠바이 퍼지 (안전)"),
        ID_A_SBLOW,
        x + 14,
        192,
        gw - 30,
        font,
    );
    checkbox(
        hwnd,
        w!("③ 전체 스탠바이 퍼지 — 파일 캐시 소실"),
        ID_A_SBALL,
        x + 14,
        214,
        gw - 30,
        font,
    );
    checkbox(
        hwnd,
        w!("④ 수정 페이지 플러시 — 디스크 쓰기"),
        ID_A_MOD,
        x + 14,
        236,
        gw - 30,
        font,
    );
    checkbox(
        hwnd,
        w!("⑤ 시스템 워킹셋 — 캐시 재적재 비용"),
        ID_A_SYSWS,
        x + 14,
        258,
        gw - 30,
        font,
    );
    checkbox(
        hwnd,
        w!("⑥ 페이지 결합 — 느림 (M5 예정)"),
        ID_A_COMBINE,
        x + 14,
        280,
        gw - 30,
        font,
    );
    label(
        hwnd,
        w!("③~⑥은 관리자 권한이 필요하고 일시적 체감 저하가 있을 수 있습니다."),
        x + 14,
        302,
        gw - 30,
        font,
    );

    // ── 일반
    group(hwnd, w!("일반"), x, 336, gw, 150, font);
    label(
        hwnd,
        w!("관찰 주기 (ms, 2000~30000)"),
        x + 14,
        360,
        LBL_W,
        font,
    );
    number(hwnd, ID_POLL, x + NUM_X, 357, NUM_W, font);
    label(
        hwnd,
        w!("목록 갱신 (ms, 500~10000)"),
        x + 14,
        386,
        LBL_W,
        font,
    );
    number(hwnd, ID_REFRESH_MS, x + NUM_X, 383, NUM_W, font);
    label(hwnd, w!("Top N (3~10)"), x + 14, 412, LBL_W, font);
    number(hwnd, ID_TOPN, x + NUM_X, 409, NUM_W, font);
    checkbox(
        hwnd,
        w!("정리 결과 알림"),
        ID_NOTIFY,
        x + 14,
        436,
        190,
        font,
    );
    checkbox(
        hwnd,
        w!("의사 프로세스 숨김"),
        ID_HIDE_PSEUDO,
        x + 210,
        436,
        200,
        font,
    );
    checkbox(
        hwnd,
        w!("목록 기본 = 평면(트리 아님)"),
        ID_VIEW_FLAT,
        x + 14,
        458,
        300,
        font,
    );

    // ── 버튼
    button(
        hwnd,
        w!("기본값"),
        ID_RESET,
        BtnSpec(x, BTN_Y, 92, false),
        font,
    );
    button(
        hwnd,
        w!("저장"),
        ID_SAVE,
        BtnSpec(W - 202, BTN_Y, 92, true),
        font,
    );
    button(
        hwnd,
        w!("취소"),
        ID_CANCEL,
        BtnSpec(W - 106, BTN_Y, 92, false),
        font,
    );
}

unsafe fn load_into_ui(hwnd: HWND, s: &Settings) {
    set_check(hwnd, ID_AUTO_ENABLED, s.auto_enabled);
    let _ = SetDlgItemInt(hwnd, ID_THRESHOLD, s.threshold_percent, false);
    let _ = SetDlgItemInt(hwnd, ID_COOLDOWN, s.cooldown_ms, false);
    let _ = SetDlgItemInt(hwnd, ID_INTERVAL, s.interval_ms, false);
    set_check(hwnd, ID_A_WS, s.areas.working_sets);
    set_check(hwnd, ID_A_SBLOW, s.areas.standby_low);
    set_check(hwnd, ID_A_SBALL, s.areas.standby_all);
    set_check(hwnd, ID_A_MOD, s.areas.modified);
    set_check(hwnd, ID_A_SYSWS, s.areas.system_ws);
    set_check(hwnd, ID_A_COMBINE, s.areas.combine);
    let _ = SetDlgItemInt(hwnd, ID_POLL, s.poll_ms, false);
    let _ = SetDlgItemInt(hwnd, ID_REFRESH_MS, s.list_refresh_ms, false);
    let _ = SetDlgItemInt(hwnd, ID_TOPN, s.topn, false);
    set_check(hwnd, ID_NOTIFY, s.notify);
    set_check(hwnd, ID_HIDE_PSEUDO, s.hide_pseudo);
    set_check(hwnd, ID_VIEW_FLAT, s.view_flat);
}

/// UI → 설정 저장. 값 검증(클램프·모순 해소)은 mk-core `normalize`가 한다.
unsafe fn save_from_ui(hwnd: HWND) -> bool {
    // 모르는 키를 보존하려면 기존 파일을 읽어 그 위에 덮어써야 한다(docs/09 §7-1)
    let mut s = crate::config::load();
    s.auto_enabled = get_check(hwnd, ID_AUTO_ENABLED);
    s.threshold_percent = GetDlgItemInt(hwnd, ID_THRESHOLD, None, false);
    s.cooldown_ms = GetDlgItemInt(hwnd, ID_COOLDOWN, None, false);
    s.interval_ms = GetDlgItemInt(hwnd, ID_INTERVAL, None, false);
    s.areas.working_sets = get_check(hwnd, ID_A_WS);
    s.areas.standby_low = get_check(hwnd, ID_A_SBLOW);
    s.areas.standby_all = get_check(hwnd, ID_A_SBALL);
    s.areas.modified = get_check(hwnd, ID_A_MOD);
    s.areas.system_ws = get_check(hwnd, ID_A_SYSWS);
    s.areas.combine = get_check(hwnd, ID_A_COMBINE);
    s.poll_ms = GetDlgItemInt(hwnd, ID_POLL, None, false);
    s.list_refresh_ms = GetDlgItemInt(hwnd, ID_REFRESH_MS, None, false);
    s.topn = GetDlgItemInt(hwnd, ID_TOPN, None, false);
    s.notify = get_check(hwnd, ID_NOTIFY);
    s.hide_pseudo = get_check(hwnd, ID_HIDE_PSEUDO);
    s.view_flat = get_check(hwnd, ID_VIEW_FLAT);
    s.normalize();

    if !s.areas.any() {
        let _ = MessageBoxW(
            Some(hwnd),
            w!("정리 영역이 하나도 선택되지 않았습니다.\n최소 하나는 켜야 정리가 동작합니다."),
            w!("설정 확인"),
            MB_OK | MB_ICONWARNING,
        );
        return false;
    }
    if !crate::config::save(&s) {
        let _ = MessageBoxW(
            Some(hwnd),
            w!("설정을 저장하지 못했습니다.\n쓰기 권한을 확인해 주세요."),
            w!("저장 실패"),
            MB_OK | MB_ICONWARNING,
        );
        return false;
    }
    // 상주 셸에 즉시 반영 요청(설정 창은 워커 프로세스이므로 창 메시지로 알린다)
    crate::shell::notify_settings_changed();
    true
}

/// 미사용 경고 방지(레이아웃 상수 참조용).
const _: RECT = RECT {
    left: 0,
    top: 0,
    right: W,
    bottom: H,
};

/// GDI 핸들 변환 도우미(가독성).
#[allow(dead_code)]
fn as_obj(f: HFONT) -> HGDIOBJ {
    f.into()
}
