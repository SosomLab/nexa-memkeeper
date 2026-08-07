//! settingswnd — 설정 창(FR-5, M4). `--worker settings`로 뜨는 온ديمانド 창(DR-7).
//! 그리드·팝업과 달리 **표준 Win32 컨트롤**(BUTTON/EDIT/STATIC)을 쓴다 — 입력·IME·키보드 탐색·
//! 접근성을 OS가 처리하게 두는 편이 안전하고(DR-4), 매니페스트의 공용 컨트롤 v6로 현대 테마가 적용된다.
//!
//! 저장은 [`crate::config::save`] → `data\settings.ini`. 값 검증은 mk-core가 담당(범위 클램프).

use mk_core::{tr, Settings};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetStockObject, DEFAULT_GUI_FONT, HFONT, HGDIOBJ};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    CheckDlgButton, InitCommonControlsEx, IsDlgButtonChecked, BST_CHECKED, DLG_BUTTON_CHECK_STATE,
    ICC_HOTKEY_CLASS, INITCOMMONCONTROLSEX,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRect, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetDlgItem,
    GetDlgItemInt, GetMessageW, LoadCursorW, LoadIconW, MessageBoxW, PostQuitMessage,
    RegisterClassW, SendMessageW, SetDlgItemInt, SetForegroundWindow, ShowWindow,
    SystemParametersInfoW, TranslateMessage, BS_AUTOCHECKBOX, BS_DEFPUSHBUTTON, BS_GROUPBOX,
    BS_PUSHBUTTON, CBS_DROPDOWNLIST, CW_USEDEFAULT, ES_NUMBER, HMENU, IDC_ARROW,
    MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, MSG, SPI_GETWORKAREA, SW_SHOWNORMAL,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WINDOW_EX_STYLE, WINDOW_STYLE, WM_COMMAND, WM_CREATE,
    WM_DESTROY, WM_SETFONT, WNDCLASSW, WS_BORDER, WS_CAPTION, WS_CHILD, WS_EX_CLIENTEDGE,
    WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};

const CLASS: PCWSTR = w!("NexaMemKeeperSettings");
/// **클라이언트 영역** 크기(모든 레이아웃 좌표의 기준). 창 크기는 여기에 테두리·제목 표시줄을
/// 더해 [`AdjustWindowRect`]로 역산한다 — 이 값을 그대로 창 크기로 넘기면 아래쪽이 잘린다(08-07 QA).
const W: i32 = 452;
const H: i32 = 628;
/// 하단 버튼 줄 y좌표(콘텐츠 마지막 + 여백).
const BTN_Y: i32 = 576;

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
const ID_LOWMEM: i32 = 1016;
const ID_POLL: i32 = 1020;
const ID_NOTIFY: i32 = 1021;
const ID_HIDE_PSEUDO: i32 = 1022;
const ID_REFRESH_MS: i32 = 1024;
const ID_TOPN: i32 = 1025;
const ID_HOTKEY: i32 = 1026;
const ID_LANG: i32 = 1027;
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
            tw!("Nexa MemKeeper — Settings"),
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

// 공용 컨트롤 hotkey 클래스 메시지(commctrl.h) — windows crate 미노출이라 상수로 선언.
const HKM_SETHOTKEY: u32 = 0x0400 + 1;
const HKM_GETHOTKEY: u32 = 0x0400 + 2;
const HOTKEYF_SHIFT: u32 = 0x01;
const HOTKEYF_CONTROL: u32 = 0x02;
const HOTKEYF_ALT: u32 = 0x04;

/// 전역 핫키 입력 칸(M4-2) — 사용자가 조합을 **직접 눌러** 지정한다(문자열 타이핑보다 오류가 적다).
unsafe fn hotkey_field(parent: HWND, id: i32, x: i32, y: i32, w: i32, font: HFONT) {
    if let Ok(h) = CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("msctls_hotkey32"),
        w!(""),
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | WS_BORDER.0),
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

/// 설정 문자열 → 핫키 칸 표시.
unsafe fn set_hotkey(hwnd: HWND, id: i32, text: &str) {
    let Some(ctl) = get_ctl(hwnd, id) else { return };
    let (vk, mods) = match mk_core::parse_hotkey(text) {
        Some(hk) => {
            let mut m = 0u32;
            if hk.shift {
                m |= HOTKEYF_SHIFT;
            }
            if hk.ctrl {
                m |= HOTKEYF_CONTROL;
            }
            if hk.alt {
                m |= HOTKEYF_ALT;
            }
            (hk.vk, m)
        }
        None => (0, 0),
    };
    SendMessageW(
        ctl,
        HKM_SETHOTKEY,
        Some(WPARAM(((vk & 0xFF) | ((mods & 0xFF) << 8)) as usize)),
        Some(LPARAM(0)),
    );
}

/// 핫키 칸 → 설정 문자열(`Ctrl+Alt+M`). 조합이 없거나 수정자가 없으면 빈 문자열.
unsafe fn get_hotkey(hwnd: HWND, id: i32) -> String {
    let Some(ctl) = get_ctl(hwnd, id) else {
        return String::new();
    };
    let r = SendMessageW(ctl, HKM_GETHOTKEY, Some(WPARAM(0)), Some(LPARAM(0))).0 as u32;
    let (vk, mods) = (r & 0xFF, (r >> 8) & 0xFF);
    if vk == 0 {
        return String::new();
    }
    let hk = mk_core::Hotkey {
        ctrl: mods & HOTKEYF_CONTROL != 0,
        alt: mods & HOTKEYF_ALT != 0,
        shift: mods & HOTKEYF_SHIFT != 0,
        win: false, // hotkey 컨트롤은 Win 키를 수정자로 주지 않는다
        vk,
    };
    mk_core::hotkey_to_string(&hk)
}

unsafe fn get_ctl(hwnd: HWND, id: i32) -> Option<HWND> {
    let h = GetDlgItem(Some(hwnd), id).ok()?;
    (!h.is_invalid()).then_some(h)
}

// 콤보 박스 메시지(commctrl/winuser) — 표준 컨트롤이라 상수만 필요하다.
const CB_ADDSTRING: u32 = 0x0143;
const CB_SETCURSEL: u32 = 0x014E;
const CB_GETCURSEL: u32 = 0x0147;
/// 콤보 항목 순서 = 설정 값(auto/en/ko).
const LANGS: [&str; 3] = ["auto", "en", "ko"];

/// 언어 선택 콤보(i18n) — 기본은 `auto`(OS 언어, 한국어가 아니면 영어).
unsafe fn lang_combo(parent: HWND, id: i32, x: i32, y: i32, w: i32, font: HFONT) {
    let Ok(h) = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("COMBOBOX"),
        w!(""),
        WINDOW_STYLE(
            WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | CBS_DROPDOWNLIST as u32 | WS_VSCROLL.0,
        ),
        x,
        y,
        w,
        160, // 목록 펼침 높이 포함
        Some(parent),
        Some(HMENU(id as isize as *mut _)),
        None,
        None,
    ) else {
        return;
    };
    SendMessageW(
        h,
        WM_SETFONT,
        Some(WPARAM(font.0 as usize)),
        Some(LPARAM(1)),
    );
    for label in ["Auto (system)", "English", "Korean"] {
        let buf: Vec<u16> = mk_core::tr(label)
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        SendMessageW(
            h,
            CB_ADDSTRING,
            Some(WPARAM(0)),
            Some(LPARAM(buf.as_ptr() as isize)),
        );
    }
}

unsafe fn set_lang_combo(hwnd: HWND, id: i32, value: &str) {
    let Some(ctl) = get_ctl(hwnd, id) else { return };
    let idx = LANGS
        .iter()
        .position(|v| v.eq_ignore_ascii_case(value.trim()))
        .unwrap_or(0);
    SendMessageW(ctl, CB_SETCURSEL, Some(WPARAM(idx)), Some(LPARAM(0)));
}

unsafe fn get_lang_combo(hwnd: HWND, id: i32) -> String {
    let Some(ctl) = get_ctl(hwnd, id) else {
        return "auto".into();
    };
    let idx = SendMessageW(ctl, CB_GETCURSEL, Some(WPARAM(0)), Some(LPARAM(0))).0;
    LANGS
        .get(idx.max(0) as usize)
        .unwrap_or(&"auto")
        .to_string()
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
            // hotkey 컨트롤 클래스 등록(공용 컨트롤) — 없으면 칸이 생성되지 않는다
            let icc = INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                dwICC: ICC_HOTKEY_CLASS,
            };
            let _ = InitCommonControlsEx(&icc);
            // 설정을 먼저 읽어 UI 언어를 확정한 뒤 컨트롤을 만든다(문구가 섞이지 않게)
            let cfg = crate::config::load();
            let font = HFONT(GetStockObject(DEFAULT_GUI_FONT).0);
            build_ui(hwnd, font);
            load_into_ui(hwnd, &cfg);
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
    group(hwnd, tw!("Automatic Cleanup"), x, 10, gw, 154, font);
    checkbox(
        hwnd,
        tw!("Clean automatically when memory usage exceeds the threshold"),
        ID_AUTO_ENABLED,
        x + 14,
        32,
        gw - 30,
        font,
    );
    label(hwnd, tw!("Threshold (%, 50~95)"), x + 14, 60, LBL_W, font);
    number(hwnd, ID_THRESHOLD, x + NUM_X, 57, NUM_W, font);
    label(
        hwnd,
        tw!("Min interval (ms, 60000~1800000)"),
        x + 14,
        86,
        LBL_W,
        font,
    );
    number(hwnd, ID_COOLDOWN, x + NUM_X, 83, NUM_W, font);
    label(
        hwnd,
        tw!("Periodic cleanup (ms, 0=off)"),
        x + 14,
        112,
        LBL_W,
        font,
    );
    number(hwnd, ID_INTERVAL, x + NUM_X, 109, NUM_W, font);
    // 저메모리 이벤트(M5-1) — 임계값에 못 미쳐도 OS가 부족을 알리면 즉시 정리(쿨다운은 지킨다)
    checkbox(
        hwnd,
        tw!("Also clean when Windows signals low memory"),
        ID_LOWMEM,
        x + 14,
        134,
        gw - 30,
        font,
    );

    // ── 정리 영역
    group(hwnd, tw!("Cleanup Areas"), x, 174, gw, 178, font);
    checkbox(
        hwnd,
        tw!("(1) Process working set trim (safe)"),
        ID_A_WS,
        x + 14,
        196,
        gw - 30,
        font,
    );
    checkbox(
        hwnd,
        tw!("(2) Low-priority standby purge (safe)"),
        ID_A_SBLOW,
        x + 14,
        218,
        gw - 30,
        font,
    );
    checkbox(
        hwnd,
        tw!("(3) Full standby purge — file cache lost"),
        ID_A_SBALL,
        x + 14,
        240,
        gw - 30,
        font,
    );
    checkbox(
        hwnd,
        tw!("(4) Flush modified pages — disk writes"),
        ID_A_MOD,
        x + 14,
        262,
        gw - 30,
        font,
    );
    checkbox(
        hwnd,
        tw!("(5) System working set — cache reload cost"),
        ID_A_SYSWS,
        x + 14,
        284,
        gw - 30,
        font,
    );
    checkbox(
        hwnd,
        tw!("(6) Page combining — slow, admin only"),
        ID_A_COMBINE,
        x + 14,
        306,
        gw - 30,
        font,
    );
    label(
        hwnd,
        tw!("(3)~(6) need administrator rights and may cause a brief slowdown."),
        x + 14,
        328,
        gw - 30,
        font,
    );

    // ── 일반
    group(hwnd, tw!("General"), x, 362, gw, 202, font);
    label(
        hwnd,
        tw!("Poll interval (ms, 2000~30000)"),
        x + 14,
        386,
        LBL_W,
        font,
    );
    number(hwnd, ID_POLL, x + NUM_X, 383, NUM_W, font);
    label(
        hwnd,
        tw!("List refresh (ms, 500~10000)"),
        x + 14,
        412,
        LBL_W,
        font,
    );
    number(hwnd, ID_REFRESH_MS, x + NUM_X, 409, NUM_W, font);
    label(hwnd, w!("Top N (3~10)"), x + 14, 438, LBL_W, font);
    number(hwnd, ID_TOPN, x + NUM_X, 435, NUM_W, font);
    checkbox(
        hwnd,
        tw!("Notify on cleanup"),
        ID_NOTIFY,
        x + 14,
        462,
        190,
        font,
    );
    checkbox(
        hwnd,
        tw!("Hide pseudo processes"),
        ID_HIDE_PSEUDO,
        x + 210,
        462,
        200,
        font,
    );
    // 전역 핫키(M4-2) — 공용 컨트롤의 hotkey 클래스가 조합 입력·표시를 대신 처리한다.
    label(
        hwnd,
        tw!("Global hotkey (Clean Now)"),
        x + 14,
        510,
        LBL_W,
        font,
    );
    hotkey_field(hwnd, ID_HOTKEY, x + NUM_X, 507, 140, font);
    // 언어(i18n) — 기본 auto(OS 언어). 바꾸면 재시작 안내를 띄운다.
    label(hwnd, tw!("Language"), x + 14, 536, LBL_W, font);
    lang_combo(hwnd, ID_LANG, x + NUM_X, 533, 140, font);

    // ── 버튼
    button(
        hwnd,
        tw!("Defaults"),
        ID_RESET,
        BtnSpec(x, BTN_Y, 92, false),
        font,
    );
    button(
        hwnd,
        tw!("Save"),
        ID_SAVE,
        BtnSpec(W - 202, BTN_Y, 92, true),
        font,
    );
    button(
        hwnd,
        tw!("Cancel"),
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
    set_check(hwnd, ID_LOWMEM, s.low_memory_event);
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
    set_hotkey(hwnd, ID_HOTKEY, &s.hotkey_clean);
    set_lang_combo(hwnd, ID_LANG, &s.lang);
}

/// UI → 설정 저장. 값 검증(클램프·모순 해소)은 mk-core `normalize`가 한다.
unsafe fn save_from_ui(hwnd: HWND) -> bool {
    // 모르는 키를 보존하려면 기존 파일을 읽어 그 위에 덮어써야 한다(docs/09 §7-1)
    let mut s = crate::config::load();
    s.auto_enabled = get_check(hwnd, ID_AUTO_ENABLED);
    s.threshold_percent = GetDlgItemInt(hwnd, ID_THRESHOLD, None, false);
    s.cooldown_ms = GetDlgItemInt(hwnd, ID_COOLDOWN, None, false);
    s.interval_ms = GetDlgItemInt(hwnd, ID_INTERVAL, None, false);
    s.low_memory_event = get_check(hwnd, ID_LOWMEM);
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
    s.hotkey_clean = get_hotkey(hwnd, ID_HOTKEY);
    let prev_lang = s.lang.clone();
    s.lang = get_lang_combo(hwnd, ID_LANG);
    let lang_changed = prev_lang != s.lang;
    s.normalize();

    if !s.areas.any() {
        let _ = MessageBoxW(
            Some(hwnd),
            tw!("No cleanup area is selected.\nTurn on at least one for cleanup to work."),
            tw!("Check settings"),
            MB_OK | MB_ICONWARNING,
        );
        return false;
    }
    if !crate::config::save(&s) {
        let _ = MessageBoxW(
            Some(hwnd),
            tw!("Could not save settings.\nCheck write permission."),
            tw!("Save failed"),
            MB_OK | MB_ICONWARNING,
        );
        return false;
    }
    // 상주 셸에 즉시 반영 요청(설정 창은 워커 프로세스이므로 창 메시지로 알린다)
    crate::shell::notify_settings_changed();
    if lang_changed {
        // 이미 만들어진 컨트롤·메뉴 문구는 다시 만들지 않는다 — 재시작이 가장 확실하다
        let title: Vec<u16> = tr("Language")
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let body: Vec<u16> = tr("Restart the app to apply the language change.")
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        MessageBoxW(
            Some(hwnd),
            PCWSTR(body.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONINFORMATION,
        );
    }
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
