//! listwnd — **메인 화면**: 전체 프로세스 목록 창(FR-3, `--worker list`).
//! 상단 메뉴바(파일·메모리·설정·도움말) + 그리드(헤더 클릭 정렬·스크롤·2초 자동 갱신).
//! **워커 프로세스에서 동작**(DR-7): 창을 닫으면 프로세스가 끝나 사용 메모리를 OS가 전량 회수한다.
//! 트림·종료는 같은 워커 안에서 직접 수행(상태 변경은 워커 전용 — 15 §4).

use std::collections::HashSet;

use mk_core::{tr, ProcInfo, Row, RowKind, SortKey};
use windows::core::{w, PCWSTR, PWSTR};
use windows::Win32::Foundation::SIZE;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateSolidBrush,
    DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, GetDC, GetSysColor,
    GetTextExtentPoint32W, InvalidateRect, ReleaseDC, SelectObject, SetBkMode, SetTextColor,
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, COLOR_BTNFACE, COLOR_GRAYTEXT, COLOR_HIGHLIGHT,
    COLOR_HIGHLIGHTTEXT, COLOR_WINDOW, COLOR_WINDOWTEXT, DEFAULT_CHARSET, DEFAULT_PITCH,
    DRAW_TEXT_FORMAT, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE,
    DT_VCENTER, HBRUSH, HDC, HFONT, OUT_DEFAULT_PRECIS, PAINTSTRUCT, SRCCOPY, SYS_COLOR_INDEX,
    TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ,
};
use windows::Win32::UI::Controls::SetScrollInfo;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_F5;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CheckMenuItem, CreateMenu, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    DestroyMenu, DestroyWindow, DispatchMessageW, GetClientRect, GetCursorPos, GetDlgItem,
    GetMessageW, GetScrollInfo, GetWindowTextLengthW, GetWindowTextW, KillTimer, LoadCursorW,
    LoadIconW, MessageBoxW, MoveWindow, PostMessageW, PostQuitMessage, RegisterClassW,
    SendMessageW, SetForegroundWindow, SetMenu, SetTimer, SetWindowTextW, ShowWindow,
    TrackPopupMenu, TranslateMessage, BS_FLAT, CW_USEDEFAULT, ES_AUTOHSCROLL, HMENU, IDC_ARROW,
    IDYES, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, MB_YESNO, MF_BYCOMMAND, MF_CHECKED,
    MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MSG, SB_BOTTOM, SB_LINEDOWN,
    SB_LINEUP, SB_PAGEDOWN, SB_PAGEUP, SB_THUMBPOSITION, SB_THUMBTRACK, SB_TOP, SB_VERT,
    SCROLLINFO, SCROLLINFO_MASK, SIF_PAGE, SIF_POS, SIF_RANGE, SIF_TRACKPOS, SW_HIDE, SW_SHOW,
    SW_SHOWNORMAL, TPM_RETURNCMD, TPM_RIGHTBUTTON, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CHAR,
    WM_COMMAND, WM_CREATE, WM_DESTROY, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MOUSEWHEEL, WM_PAINT,
    WM_RBUTTONUP, WM_SETFONT, WM_SIZE, WM_TIMER, WM_VSCROLL, WNDCLASSW, WS_CHILD, WS_EX_CLIENTEDGE,
    WS_OVERLAPPEDWINDOW, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};

const CLASS: PCWSTR = w!("NexaMemKeeperList");
const TIMER_REFRESH: usize = 20;
const ROW_H: i32 = 22;
const HEAD_H: i32 = 26;
const STATUS_H: i32 = 24;
/// 하단 검색 줄 높이(사용자 요청 08-07 — 실시간 검색 입력 칸).
const SEARCH_H: i32 = 30;
/// 검색 칸 오른쪽 끝에 겹쳐 놓는 지우기(✕) 버튼 크기.
const CLEAR_W: i32 = 22;
/// 대상 작업(회수·종료) 완료 통지 — 백그라운드 스레드에서 PostMessage(wParam = 성공 여부).
const WM_ACTION_DONE: u32 = WM_APP + 1;
/// 검색 입력·지우기 컨트롤 ID.
const ID_SEARCH: usize = 300;
const ID_SEARCH_CLEAR: usize = 301;
/// EDIT 알림 코드(EN_CHANGE) — 입력이 바뀔 때마다 온다 = 실시간 검색.
const EN_CHANGE: u32 = 0x0300;
/// `EM_SETMARGINS` + `EC_RIGHTMARGIN` — 지우기 버튼 아래로 글자가 들어가지 않게 여백 확보.
const EM_SETMARGINS: u32 = 0x00D3;
/// `EM_SETCUEBANNER`(공용 컨트롤) — 빈 입력일 때 흐린 안내 문구.
const EM_SETCUEBANNER: u32 = 0x1501;
const EC_RIGHTMARGIN: u32 = 0x0002;

// 메뉴 명령 ID
const ID_REFRESH: usize = 200;
const ID_CLOSE: usize = 201;
const ID_CLEAN_ALL: usize = 210;
const ID_TRIM_SEL: usize = 211;
const ID_KILL_SEL: usize = 212;
const ID_KILL_TREE_SEL: usize = 213;
const ID_KILL_THREAD_SEL: usize = 214;
const ID_PREFS: usize = 220;
const ID_AUTORUN: usize = 221;
const ID_HIDE_PSEUDO: usize = 222;
const ID_REFRESH_1S: usize = 230;
const ID_REFRESH_2S: usize = 231;
const ID_REFRESH_5S: usize = 232;
const ID_ABOUT: usize = 240;
const ID_VIEW_FLAT: usize = 250;
const ID_VIEW_TREE: usize = 251;
const ID_EXPAND_ALL: usize = 252;
const ID_COLLAPSE_ALL: usize = 253;
const ID_CLEAR_FILTER: usize = 254;
const ID_SHOW_THREADS: usize = 255;
const ID_VIEW_GROUPED: usize = 256;
const ID_URL_HOME: usize = 241;
const ID_URL_REPO: usize = 242;
const ID_URL_APP: usize = 243;

// ---- 외부 링크(도움말 메뉴·정보 창) ----
const URL_HOME: &str = "https://sosomlab.com";
const URL_REPO: &str = "https://github.com/SosomLab/nexa-memkeeper";
/// 앱 전용 홈페이지 — **주소가 정해지면 이 한 줄만 채우면** 도움말 메뉴와 정보 창에 자동 노출된다.
/// 비어 있으면 메뉴 항목도 만들지 않는다(사용자 예고 08-07: "앱 홈페이지 신설하면 알려줄 예정").
const URL_APP: &str = "";

/// Windows 시작 시 실행 등록 위치(사용자 단위 — 관리자 권한 불필요).
const RUN_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
const RUN_VALUE: PCWSTR = w!("NexaMemKeeper");

/// 열 정의 — (제목, 정렬 기준, 오른쪽 정렬, 고정 폭). 이름 열은 남는 폭을 흡수(폭 0).
const COLS: &[(&str, SortKey, bool, i32)] = &[
    ("Process", SortKey::Name, false, 0),
    ("PID", SortKey::Pid, true, 70),
    ("Private", SortKey::PrivateWs, true, 95),
    ("Working Set", SortKey::Ws, true, 95),
    ("Threads", SortKey::Threads, true, 65),
];

struct ListState {
    /// 수집 원본(트리 구성·자손 탐색에 사용).
    procs: Vec<ProcInfo>,
    /// 화면 행(트리 평면화 결과).
    rows: Vec<Row>,
    /// 펼친 프로세스 PID — 갱신·정렬 후에도 유지된다.
    expanded: HashSet<u32>,
    /// 기본 펼침을 이미 심어 준 묶음 키 — **한 번만** 심어야 접기가 유지된다.
    seeded: HashSet<u32>,
    top: i32,
    /// 선택 행 — (pid, tid). tid≠0이면 쓰레드 행.
    sel: (u32, u32),
    sort_key: SortKey,
    sort_asc: bool,
    /// 의사 프로세스(Memory Compression 등) 숨김 — 기본 on(사용자 확정 08-07).
    hide_pseudo: bool,
    /// 보기 모드 — 0 = 묶음(작업 관리자식·기본), 1 = 트리(부모-자식), 2 = 평면.
    view: u8,
    /// 쓰레드 행 표시 — **기본 off**(사용자 확정 08-07). 작업 관리자처럼 프로세스 계층만 본다.
    show_threads: bool,
    /// 이름 검색어 — 창에 그냥 타이핑하면 채워진다(Esc = 지움). 비어 있지 않으면 항상 평면 표시.
    filter: String,
    refresh_ms: u32,
    font: HFONT,
    font_head: HFONT,
    /// 트리 디스클로저 전용 — Segoe MDL2 Assets(본문 폰트엔 PUA 글리프가 없어 tofu가 된다).
    font_glyph: HFONT,
    client: RECT,
    menu: HMENU,
}

impl ListState {
    fn visible_rows(&self) -> i32 {
        ((self.client.bottom - HEAD_H - STATUS_H - SEARCH_H) / ROW_H).max(1)
    }
    fn max_top(&self) -> i32 {
        (self.rows.len() as i32 - self.visible_rows()).max(0)
    }
    fn selected(&self) -> Option<&Row> {
        self.rows.iter().find(|r| (r.pid, r.tid) == self.sel)
    }
}

thread_local! {
    static STATE: std::cell::RefCell<Option<ListState>> = const { std::cell::RefCell::new(None) };
    /// 회수·종료 작업이 백그라운드에서 도는 중(중복 실행 방지 + 상태 줄 표시).
    static BUSY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 행에 대한 대상 작업 — 종료 대기가 길어 **UI 스레드에서 실행하면 안 된다**.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RowAction {
    Trim,
    Kill,
    KillTree,
    KillThread,
}

/// 작업을 백그라운드 스레드에서 실행하고 완료를 창에 알린다.
///
/// 왜 스레드인가 — `run_kill`은 정상 종료를 최대 3초 기다리고 실패 시 강제 종료 후 다시 2초를
/// 기다린다. 트리 종료는 프로세스 수만큼 반복되므로 UI 스레드에서 돌리면 메시지 루프가 막혀
/// 창이 "응답 없음"이 된다(08-07 사용자 QA).
fn spawn_action(hwnd: HWND, action: RowAction, target: u32) {
    if BUSY.replace(true) {
        return; // 이미 진행 중 — 중복 실행 금지
    }
    let hwnd_raw = hwnd.0 as usize;
    std::thread::spawn(move || {
        let ok = match action {
            RowAction::Trim => crate::worker::run_trim(target) == 0,
            RowAction::Kill => crate::worker::run_kill(target) == 0,
            RowAction::KillTree => crate::worker::run_kill_tree(target) == 0,
            RowAction::KillThread => crate::worker::run_kill_thread(target) == 0,
        };
        unsafe {
            let _ = PostMessageW(
                Some(HWND(hwnd_raw as *mut core::ffi::c_void)),
                WM_ACTION_DONE,
                WPARAM(usize::from(ok)),
                LPARAM(0),
            );
        }
    });
}

/// `--worker list` 진입 — 창을 닫을 때까지 메시지 루프. 종료 코드 0.
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
            // exe에 임베드한 앱 아이콘(리소스 ID 1 — build.rs) → 제목 표시줄·작업 표시줄·Alt+Tab.
            // MAKEINTRESOURCE = 포인터가 아니라 **정수 리소스 ID**를 포인터 자리에 넣는 Win32 관용구.
            hIcon: LoadIconW(Some(hinst.into()), PCWSTR(std::ptr::without_provenance(1)))
                .unwrap_or_default(),
            ..Default::default()
        };
        RegisterClassW(&wc);
        let Ok(hwnd) = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            CLASS,
            tw!("Nexa MemKeeper — Processes"),
            WS_OVERLAPPEDWINDOW | WS_VSCROLL | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            720,
            600,
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

/// 상단 메뉴바 구성(Top-down) — 파일 / 메모리 / 설정 / 도움말.
unsafe fn build_menu(hwnd: HWND) -> HMENU {
    let bar = CreateMenu().unwrap_or_default();

    let file = CreatePopupMenu().unwrap_or_default();
    let _ = AppendMenuW(file, MF_STRING, ID_REFRESH, tw!("Refresh(&R)\tF5"));
    let _ = AppendMenuW(file, MF_SEPARATOR, 0, None);
    let _ = AppendMenuW(file, MF_STRING, ID_CLOSE, tw!("Close(&X)"));
    let _ = AppendMenuW(bar, MF_POPUP, file.0 as usize, tw!("File(&F)"));

    let mem = CreatePopupMenu().unwrap_or_default();
    let _ = AppendMenuW(mem, MF_STRING, ID_CLEAN_ALL, tw!("Clean Now(&C) — All"));
    let _ = AppendMenuW(mem, MF_SEPARATOR, 0, None);
    let _ = AppendMenuW(
        mem,
        MF_STRING,
        ID_TRIM_SEL,
        tw!("Trim Selected Process(&T)"),
    );
    let _ = AppendMenuW(
        mem,
        MF_STRING,
        ID_KILL_SEL,
        tw!("End Selected Process(&K)…"),
    );
    let _ = AppendMenuW(
        mem,
        MF_STRING,
        ID_KILL_TREE_SEL,
        tw!("End Selected Process Tree(&R)…"),
    );
    let _ = AppendMenuW(
        mem,
        MF_STRING,
        ID_KILL_THREAD_SEL,
        tw!("End Selected Thread(&E)…"),
    );
    let _ = AppendMenuW(bar, MF_POPUP, mem.0 as usize, tw!("Memory(&M)"));

    let view = CreatePopupMenu().unwrap_or_default();
    let _ = AppendMenuW(view, MF_STRING, ID_VIEW_GROUPED, tw!("Grouped by app(&G)"));
    let _ = AppendMenuW(view, MF_STRING, ID_VIEW_TREE, tw!("Process Tree(&T)"));
    let _ = AppendMenuW(
        view,
        MF_STRING,
        ID_VIEW_FLAT,
        tw!("Flat List(&L) — show all"),
    );
    let _ = AppendMenuW(view, MF_SEPARATOR, 0, None);
    let _ = AppendMenuW(view, MF_STRING, ID_SHOW_THREADS, tw!("Show threads(&H)"));
    let _ = AppendMenuW(view, MF_SEPARATOR, 0, None);
    let _ = AppendMenuW(view, MF_STRING, ID_EXPAND_ALL, tw!("Expand All(&E)"));
    let _ = AppendMenuW(view, MF_STRING, ID_COLLAPSE_ALL, tw!("Collapse All(&C)"));
    let _ = AppendMenuW(view, MF_SEPARATOR, 0, None);
    let _ = AppendMenuW(
        view,
        MF_STRING,
        ID_CLEAR_FILTER,
        tw!("Clear Search(&D)\tEsc — just type to search"),
    );
    let _ = AppendMenuW(bar, MF_POPUP, view.0 as usize, tw!("View(&V)"));

    let interval = CreatePopupMenu().unwrap_or_default();
    let _ = AppendMenuW(interval, MF_STRING, ID_REFRESH_1S, tw!("1 sec"));
    let _ = AppendMenuW(interval, MF_STRING, ID_REFRESH_2S, tw!("2 sec (default)"));
    let _ = AppendMenuW(interval, MF_STRING, ID_REFRESH_5S, tw!("5 sec"));

    let cfg = CreatePopupMenu().unwrap_or_default();
    let _ = AppendMenuW(
        cfg,
        MF_STRING,
        ID_AUTORUN,
        tw!("Run at Windows startup(&A)"),
    );
    let _ = AppendMenuW(
        cfg,
        MF_POPUP,
        interval.0 as usize,
        tw!("List refresh interval(&I)"),
    );
    let _ = AppendMenuW(
        cfg,
        MF_STRING,
        ID_HIDE_PSEUDO,
        tw!("Hide pseudo processes(&H)"),
    );
    let _ = AppendMenuW(cfg, MF_SEPARATOR, 0, None);
    let _ = AppendMenuW(
        cfg,
        MF_STRING | MF_GRAYED,
        ID_PREFS,
        tw!("Preferences(&P)…"),
    );
    let _ = AppendMenuW(bar, MF_POPUP, cfg.0 as usize, tw!("Settings(&S)"));

    let help = CreatePopupMenu().unwrap_or_default();
    let _ = AppendMenuW(help, MF_STRING, ID_URL_HOME, tw!("SosomLab Website(&W)"));
    let _ = AppendMenuW(help, MF_STRING, ID_URL_REPO, tw!("GitHub Repository(&G)"));
    if !URL_APP.is_empty() {
        let _ = AppendMenuW(help, MF_STRING, ID_URL_APP, tw!("App Website(&P)"));
    }
    let _ = AppendMenuW(help, MF_SEPARATOR, 0, None);
    let _ = AppendMenuW(help, MF_STRING, ID_ABOUT, tw!("About(&A)"));
    let _ = AppendMenuW(bar, MF_POPUP, help.0 as usize, tw!("Help(&H)"));

    let _ = SetMenu(hwnd, Some(bar));
    bar
}

/// 메뉴 체크 상태 동기화(자동 실행·의사 프로세스 숨김·보기 모드·갱신 주기).
/// 메뉴 체크 동기화.
///
/// ⚠️ **STATE를 다시 빌리지 않는다** — 호출부 대부분이 `with_state(...)` 안이라, 여기서
/// `with_state_ret`을 부르면 `RefCell` 이중 대여로 패닉하고 릴리스는 `panic=abort`라
/// 창이 그대로 사라진다(08-07 사용자 QA: 의사 프로세스 숨김 해제 시 UI 종료).
/// 필요한 값은 전부 인자로 받는다.
unsafe fn sync_menu_checks(
    menu: HMENU,
    refresh_ms: u32,
    hide_pseudo: bool,
    view: u8,
    show_threads: bool,
) {
    CheckMenuItem(
        menu,
        ID_SHOW_THREADS as u32,
        MF_BYCOMMAND.0
            | if show_threads {
                MF_CHECKED.0
            } else {
                MF_UNCHECKED.0
            },
    );
    for (id, mode) in [(ID_VIEW_GROUPED, 0u8), (ID_VIEW_TREE, 1), (ID_VIEW_FLAT, 2)] {
        CheckMenuItem(
            menu,
            id as u32,
            MF_BYCOMMAND.0
                | if view == mode {
                    MF_CHECKED.0
                } else {
                    MF_UNCHECKED.0
                },
        );
    }
    let on = autorun_enabled();
    CheckMenuItem(
        menu,
        ID_AUTORUN as u32,
        MF_BYCOMMAND.0 | if on { MF_CHECKED.0 } else { MF_UNCHECKED.0 },
    );
    CheckMenuItem(
        menu,
        ID_HIDE_PSEUDO as u32,
        MF_BYCOMMAND.0
            | if hide_pseudo {
                MF_CHECKED.0
            } else {
                MF_UNCHECKED.0
            },
    );
    for (id, ms) in [
        (ID_REFRESH_1S, 1_000u32),
        (ID_REFRESH_2S, 2_000),
        (ID_REFRESH_5S, 5_000),
    ] {
        CheckMenuItem(
            menu,
            id as u32,
            MF_BYCOMMAND.0
                | if refresh_ms == ms {
                    MF_CHECKED.0
                } else {
                    MF_UNCHECKED.0
                },
        );
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let mut client = RECT::default();
            let _ = GetClientRect(hwnd, &mut client);
            // 설정 로드가 UI 언어도 정한다 — **메뉴를 만들기 전에** 불러야 메뉴 문구가 같은 언어가 된다
            // (08-07 QA: 메뉴만 영어, 본문은 한국어로 섞여 나왔다)
            let cfg = crate::config::load();
            let menu = build_menu(hwnd);
            STATE.with(|s| {
                *s.borrow_mut() = Some(ListState {
                    procs: Vec::new(),
                    rows: Vec::new(),
                    expanded: HashSet::new(),
                    seeded: HashSet::new(),
                    top: 0,
                    sel: (u32::MAX, u32::MAX), // 분류 행(pid 0)과 겹치지 않는 "선택 없음" 값
                    sort_key: SortKey::PrivateWs,
                    sort_asc: false, // 점유 큰 순 = 기본
                    hide_pseudo: cfg.hide_pseudo,
                    view: match cfg.view_mode.as_str() {
                        "tree" => 1,
                        "flat" => 2,
                        _ => 0,
                    },
                    show_threads: cfg.show_threads,
                    filter: String::new(),
                    refresh_ms: cfg.list_refresh_ms,
                    font: make_font(-12, 400),
                    font_head: make_font(-12, 600),
                    font_glyph: make_font_face(GLYPH_SIZE, 400, w!("Segoe MDL2 Assets")),
                    client,
                    menu,
                })
            });
            reload();
            with_state(|st| {
                if st.view == 1 {
                    expand_all_parents(st);
                    rebuild(st);
                }
            });
            sync_menu_checks(
                menu,
                cfg.list_refresh_ms,
                cfg.hide_pseudo,
                match cfg.view_mode.as_str() {
                    "tree" => 1,
                    "flat" => 2,
                    _ => 0,
                },
                cfg.show_threads,
            );
            create_search(hwnd, make_font(-12, 400));
            layout_search(hwnd);
            SetTimer(Some(hwnd), TIMER_REFRESH, cfg.list_refresh_ms, None);
            sync_scrollbar(hwnd);
            LRESULT(0)
        }
        WM_SIZE => {
            with_state(|st| {
                let _ = GetClientRect(hwnd, &mut st.client);
                st.top = st.top.min(st.max_top());
            });
            layout_search(hwnd);
            sync_scrollbar(hwnd);
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        WM_TIMER => {
            reload();
            sync_scrollbar(hwnd);
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        // 창에 그냥 타이핑 = 이름 검색(별도 입력 상자 없이 — 커스텀 드로잉 창의 관용구)
        // 목록에 포커스가 있을 때 타이핑하면 **검색 칸으로 넘긴다**(입력 상태를 한 곳에만 둔다)
        WM_CHAR => {
            let ch = char::from_u32(wp.0 as u32).unwrap_or('\0');
            if ch == '\u{1B}' {
                clear_search(hwnd); // Esc = 비우기
            } else if let Some(e) = get_ctl(hwnd, ID_SEARCH) {
                let _ = SetFocus(Some(e));
                SendMessageW(e, WM_CHAR, Some(wp), Some(lp));
            }
            LRESULT(0)
        }
        WM_KEYDOWN if wp.0 as u32 == VK_F5.0 as u32 => {
            on_command(hwnd, ID_REFRESH);
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = wp.0 & 0xFFFF;
            let code = ((wp.0 >> 16) & 0xFFFF) as u32;
            if id == ID_SEARCH && code == EN_CHANGE {
                on_search_changed(hwnd); // 실시간 검색
            } else if id == ID_SEARCH_CLEAR {
                clear_search(hwnd);
            } else {
                on_command(hwnd, id);
            }
            LRESULT(0)
        }
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_VSCROLL => {
            let code = (wp.0 & 0xFFFF) as i32;
            with_state(|st| {
                let page = st.visible_rows();
                let new_top = match code {
                    c if c == SB_LINEUP.0 => st.top - 1,
                    c if c == SB_LINEDOWN.0 => st.top + 1,
                    c if c == SB_PAGEUP.0 => st.top - page,
                    c if c == SB_PAGEDOWN.0 => st.top + page,
                    c if c == SB_TOP.0 => 0,
                    c if c == SB_BOTTOM.0 => st.max_top(),
                    c if c == SB_THUMBPOSITION.0 || c == SB_THUMBTRACK.0 => track_pos(hwnd),
                    _ => st.top,
                };
                st.top = new_top.clamp(0, st.max_top());
            });
            sync_scrollbar(hwnd);
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wp.0 >> 16) & 0xFFFF) as i16 as i32;
            with_state(|st| st.top = (st.top - delta / 120 * 3).clamp(0, st.max_top()));
            sync_scrollbar(hwnd);
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let (x, y) = (lo_i16(lp), hi_i16(lp));
            if y < HEAD_H {
                // 헤더 클릭 = 정렬(같은 열 재클릭 시 방향 반전)
                let width = with_state_ret(|st| st.client.right).unwrap_or(0);
                let hit_key = col_at(hwnd, width, x);
                with_state(|st| {
                    if let Some(key) = hit_key {
                        if st.sort_key == key {
                            st.sort_asc = !st.sort_asc;
                        } else {
                            st.sort_key = key;
                            // 숫자 열은 큰 값부터, 이름은 사전순이 자연스럽다
                            st.sort_asc = matches!(key, SortKey::Name);
                        }
                        rebuild(st);
                    }
                });
                sync_scrollbar(hwnd);
            } else {
                let toggled = with_state_ret(|st| {
                    let Some(r) = row_at(st, y).cloned() else {
                        st.sel = (u32::MAX, u32::MAX);
                        return false;
                    };
                    // 분류 머리글(앱/백그라운드/Windows)은 조작 대상이 아니라 선택하지 않는다
                    if r.kind != RowKind::Category {
                        st.sel = (r.pid, r.tid);
                    }
                    // 좌측 확장 버튼 영역 클릭 = 펼침/접힘
                    if r.expandable && x >= expander_x(r.depth) && x < expander_x(r.depth) + EXP_W {
                        if !st.expanded.insert(r.pid) {
                            st.expanded.remove(&r.pid);
                        }
                        rebuild(st);
                        return true;
                    }
                    false
                })
                .unwrap_or(false);
                if toggled {
                    sync_scrollbar(hwnd);
                }
            }
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            let y = hi_i16(lp);
            let target = with_state_ret(|st| {
                let hit = row_at(st, y).cloned();
                if let Some(r) = &hit {
                    st.sel = (r.pid, r.tid);
                }
                hit
            })
            .flatten();
            let _ = InvalidateRect(Some(hwnd), None, false);
            if let Some(r) = target {
                item_menu(hwnd, &r);
            }
            LRESULT(0)
        }
        WM_ACTION_DONE => {
            BUSY.set(false);
            act(hwnd, wp.0 != 0); // 실패면 안내 + 목록 갱신
            LRESULT(0)
        }
        WM_DESTROY => {
            let _ = KillTimer(Some(hwnd), TIMER_REFRESH);
            STATE.with(|s| {
                if let Some(st) = s.borrow_mut().take() {
                    let _ = DeleteObject(st.font.into());
                    let _ = DeleteObject(st.font_head.into());
                    let _ = DeleteObject(st.font_glyph.into());
                    let _ = DestroyMenu(st.menu);
                }
            });
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe fn on_command(hwnd: HWND, id: usize) {
    match id {
        ID_REFRESH => {
            reload();
            sync_scrollbar(hwnd);
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        ID_CLOSE => {
            let _ = DestroyWindow(hwnd);
        }
        ID_CLEAN_ALL => {
            let r = crate::worker::run_clean();
            let outcome = mk_core::decode_clean_result(r);
            let (title, body) = mk_core::clean_report(outcome);
            info_box(hwnd, &title, &body);
            reload();
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        ID_TRIM_SEL | ID_KILL_SEL | ID_KILL_TREE_SEL | ID_KILL_THREAD_SEL => {
            let sel = with_state_ret(|st| st.selected().cloned()).flatten();
            let Some(row) = sel else {
                info_box(
                    hwnd,
                    tr("No selection"),
                    tr("Select an item from the list first."),
                );
                return;
            };
            run_row_action(hwnd, &row, id);
        }
        ID_AUTORUN => {
            let now = autorun_enabled();
            let ok = set_autorun(!now);
            if !ok {
                info_box(hwnd, tr("Autorun"), tr("Could not change the setting."));
            }
            with_state(|st| {
                sync_menu_checks(
                    st.menu,
                    st.refresh_ms,
                    st.hide_pseudo,
                    st.view,
                    st.show_threads,
                )
            });
        }
        ID_VIEW_GROUPED | ID_VIEW_TREE | ID_VIEW_FLAT => {
            with_state(|st| {
                st.view = match id {
                    ID_VIEW_GROUPED => 0,
                    ID_VIEW_TREE => 1,
                    _ => 2,
                };
                if st.view == 1 && st.expanded.is_empty() {
                    expand_all_parents(st); // 트리로 바꾸면 기본은 펼친 상태
                }
                rebuild(st);
                sync_menu_checks(
                    st.menu,
                    st.refresh_ms,
                    st.hide_pseudo,
                    st.view,
                    st.show_threads,
                );
            });
            sync_scrollbar(hwnd);
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        ID_EXPAND_ALL | ID_COLLAPSE_ALL => {
            with_state(|st| {
                st.expanded.clear();
                if id == ID_EXPAND_ALL {
                    // 트리 보기: 자식 있는 프로세스만 펼친다(전부 펼치면 쓰레드 수천 줄이 된다)
                    let has_child: HashSet<u32> = st.procs.iter().map(|p| p.ppid).collect();
                    for p in &st.procs {
                        if has_child.contains(&p.pid) {
                            st.expanded.insert(p.pid);
                        }
                    }
                }
                st.top = 0;
                rebuild(st);
            });
            sync_scrollbar(hwnd);
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        ID_SHOW_THREADS => {
            with_state(|st| {
                st.show_threads = !st.show_threads;
                sync_menu_checks(
                    st.menu,
                    st.refresh_ms,
                    st.hide_pseudo,
                    st.view,
                    st.show_threads,
                );
                rebuild(st);
            });
            sync_scrollbar(hwnd);
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        ID_CLEAR_FILTER => set_filter(hwnd, None),
        ID_HIDE_PSEUDO => {
            with_state(|st| {
                st.hide_pseudo = !st.hide_pseudo;
                sync_menu_checks(
                    st.menu,
                    st.refresh_ms,
                    st.hide_pseudo,
                    st.view,
                    st.show_threads,
                );
            });
            reload();
            sync_scrollbar(hwnd);
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        ID_REFRESH_1S | ID_REFRESH_2S | ID_REFRESH_5S => {
            let ms = match id {
                ID_REFRESH_1S => 1_000,
                ID_REFRESH_5S => 5_000,
                _ => 2_000,
            };
            with_state(|st| {
                st.refresh_ms = ms;
                sync_menu_checks(st.menu, ms, st.hide_pseudo, st.view, st.show_threads);
            });
            SetTimer(Some(hwnd), TIMER_REFRESH, ms, None);
        }
        ID_URL_HOME => open_url(hwnd, URL_HOME),
        ID_URL_REPO => open_url(hwnd, URL_REPO),
        ID_URL_APP => open_url(hwnd, URL_APP),
        ID_ABOUT => {
            let mut body = format!(
                "{}\n{} {}\n\n{}: {URL_HOME}\n{}: {URL_REPO}\n",
                tr("Ultra-light tray-resident memory optimizer"),
                tr("Version"),
                env!("CARGO_PKG_VERSION"),
                tr("Website"),
                tr("Repository")
            );
            if !URL_APP.is_empty() {
                body.push_str(&format!("{}: {URL_APP}\n", tr("App Website(&P)")));
            }
            body.push_str(&format!(
                "\nSosomLab · Sangyong Bae\n{}: PolyForm Noncommercial 1.0.0\n{}\n\n{}",
                tr("License"),
                tr("Free for personal/noncommercial use; commercial use requires a separate license."),
                tr("Open the website or repository from the Help menu.")
            ));
            info_box(hwnd, tr("About Nexa MemKeeper"), &body);
        }
        _ => {}
    }
}

/// 하단 검색 줄 컨트롤 생성(사용자 요청 08-07).
///
/// 표준 `EDIT`를 쓰는 이유 — 한글 입력(IME)·캐럿·선택·클립보드를 OS가 처리한다. 커스텀 드로잉으로
/// 흉내 내면 IME 조합 중 글자가 깨진다. 지우기 버튼은 **에디트 위에 겹쳐** 놓고(나중에 만든 자식이
/// 위에 그려진다) 입력이 있을 때만 보인다.
unsafe fn create_search(hwnd: HWND, font: HFONT) {
    let edit = CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("EDIT"),
        w!(""),
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | ES_AUTOHSCROLL as u32),
        0,
        0,
        10,
        10,
        Some(hwnd),
        Some(HMENU(ID_SEARCH as isize as *mut _)),
        None,
        None,
    );
    if let Ok(e) = edit {
        SendMessageW(
            e,
            WM_SETFONT,
            Some(WPARAM(font.0 as usize)),
            Some(LPARAM(1)),
        );
        // 오른쪽 여백 = 지우기 버튼 폭(글자가 버튼 밑으로 숨지 않게)
        SendMessageW(
            e,
            EM_SETMARGINS,
            Some(WPARAM(EC_RIGHTMARGIN as usize)),
            Some(LPARAM(((CLEAR_W + 2) as isize) << 16)),
        );
        // 안내 문구(placeholder) — 입력이 비었을 때만 흐리게 보인다. 언어 설정을 따른다.
        let cue: Vec<u16> = mk_core::tr("Type a process name or PID")
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        SendMessageW(
            e,
            EM_SETCUEBANNER,
            Some(WPARAM(0)),
            Some(LPARAM(cue.as_ptr() as isize)),
        );
    }
    if let Ok(b) = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        w!("✕"),
        WINDOW_STYLE(WS_CHILD.0 | BS_FLAT as u32), // 처음엔 숨김(입력이 있을 때만 표시)
        0,
        0,
        10,
        10,
        Some(hwnd),
        Some(HMENU(ID_SEARCH_CLEAR as isize as *mut _)),
        None,
        None,
    ) {
        SendMessageW(
            b,
            WM_SETFONT,
            Some(WPARAM(font.0 as usize)),
            Some(LPARAM(1)),
        );
    }
}

/// 창 크기에 맞춰 검색 줄 배치 — 버튼은 에디트 오른쪽 안쪽에 겹친다.
unsafe fn layout_search(hwnd: HWND) {
    let (w, h) = with_state_ret(|st| (st.client.right, st.client.bottom)).unwrap_or((0, 0));
    let y = h - SEARCH_H + 3;
    if let Some(e) = get_ctl(hwnd, ID_SEARCH) {
        let _ = MoveWindow(e, 8, y, (w - 16).max(60), SEARCH_H - 8, true);
    }
    if let Some(b) = get_ctl(hwnd, ID_SEARCH_CLEAR) {
        let _ = MoveWindow(b, w - 8 - CLEAR_W - 2, y + 2, CLEAR_W, SEARCH_H - 12, true);
    }
}

unsafe fn get_ctl(hwnd: HWND, id: usize) -> Option<HWND> {
    let h = GetDlgItem(Some(hwnd), id as i32).ok()?;
    (!h.is_invalid()).then_some(h)
}

/// 검색 칸의 현재 문자열.
unsafe fn search_text(hwnd: HWND) -> String {
    let Some(e) = get_ctl(hwnd, ID_SEARCH) else {
        return String::new();
    };
    let len = GetWindowTextLengthW(e);
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize + 1];
    let n = GetWindowTextW(e, &mut buf);
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

/// 검색 칸 내용 → 필터 반영(실시간). 지우기 버튼은 입력이 있을 때만 보인다.
unsafe fn on_search_changed(hwnd: HWND) {
    let text = search_text(hwnd);
    if let Some(b) = get_ctl(hwnd, ID_SEARCH_CLEAR) {
        let _ = ShowWindow(b, if text.is_empty() { SW_HIDE } else { SW_SHOW });
    }
    with_state(|st| {
        st.filter = text;
        st.top = 0;
    });
    reload();
    sync_scrollbar(hwnd);
    let _ = InvalidateRect(Some(hwnd), None, false);
}

/// 검색 칸 비우기(지우기 버튼·Esc) — 포커스는 다시 검색 칸으로.
unsafe fn clear_search(hwnd: HWND) {
    if let Some(e) = get_ctl(hwnd, ID_SEARCH) {
        let _ = SetWindowTextW(e, w!(""));
        let _ = SetFocus(Some(e));
    }
    on_search_changed(hwnd);
}

/// 검색어 갱신 — `None` = 전체 지움, `Some(None)` = 한 글자 지움, `Some(Some(c))` = 한 글자 추가.
unsafe fn set_filter(hwnd: HWND, edit: Option<Option<char>>) {
    with_state(|st| {
        match edit {
            None => st.filter.clear(),
            Some(None) => {
                st.filter.pop();
            }
            Some(Some(c)) => st.filter.push(c),
        }
        st.top = 0;
    });
    reload();
    sync_scrollbar(hwnd);
    let _ = InvalidateRect(Some(hwnd), None, false);
}

unsafe fn act(hwnd: HWND, ok: bool) {
    if !ok {
        info_box(
            hwnd,
            tr("Action failed"),
            tr("Administrator rights may be required, or the target is protected."),
        );
    }
    reload();
    let _ = InvalidateRect(Some(hwnd), None, false);
}

/// 기본 브라우저로 URL 열기(도움말 메뉴). 실패해도 조용히 넘기지 않고 알린다.
unsafe fn open_url(hwnd: HWND, url: &str) {
    if url.is_empty() {
        return;
    }
    let wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
    let r = ShellExecuteW(
        Some(hwnd),
        w!("open"),
        PCWSTR(wide.as_ptr()),
        None,
        None,
        SW_SHOWNORMAL,
    );
    if r.0 as isize <= 32 {
        info_box(
            hwnd,
            tr("Failed to open link"),
            &format!("{}\n{url}", tr("Could not open the browser.")),
        );
    }
}

unsafe fn info_box(hwnd: HWND, title: &str, body: &str) {
    let t: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let b: Vec<u16> = body.encode_utf16().chain(std::iter::once(0)).collect();
    MessageBoxW(
        Some(hwnd),
        PCWSTR(b.as_ptr()),
        PCWSTR(t.as_ptr()),
        MB_OK | MB_ICONINFORMATION,
    );
}

unsafe fn confirm(hwnd: HWND, title: &str, body: &str) -> bool {
    let t: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let b: Vec<u16> = body.encode_utf16().chain(std::iter::once(0)).collect();
    MessageBoxW(
        Some(hwnd),
        PCWSTR(b.as_ptr()),
        PCWSTR(t.as_ptr()),
        MB_YESNO | MB_ICONWARNING,
    ) == IDYES
}

fn with_state(f: impl FnOnce(&mut ListState)) {
    STATE.with(|s| {
        if let Some(st) = s.borrow_mut().as_mut() {
            f(st)
        }
    });
}

fn with_state_ret<T>(f: impl FnOnce(&mut ListState) -> T) -> Option<T> {
    STATE.with(|s| s.borrow_mut().as_mut().map(f))
}

/// 자식이 있는 프로세스를 모두 펼친다 — 트리 기본 보기에서 자식이 접혀 안 보이는 일을 막는다
/// (08-07: 접힌 부모 밑에 있어 프로그램을 못 찾는 사례가 있었다. 쓰레드는 옵션이므로 영향 없음).
fn expand_all_parents(st: &mut ListState) {
    let has_child: HashSet<u32> = st.procs.iter().map(|p| p.ppid).collect();
    st.expanded.clear();
    for p in &st.procs {
        if has_child.contains(&p.pid) {
            st.expanded.insert(p.pid);
        }
    }
}

/// 목록 재수집 + 재구성(정렬·펼침·검색·스크롤·선택 유지).
fn reload() {
    let all = crate::procs::enumerate();
    with_state(|st| {
        let visible = mk_core::filter_pseudo(all, st.hide_pseudo);
        // 트리 보기에서는 **일치 항목 + 조상**을 남겨 계층이 보이게 한다(사용자 확정 08-07).
        // 다른 보기에서는 일치 항목만 평면으로.
        st.procs = match st.view {
            1 => mk_core::filter_tree(visible, &st.filter), // 트리: 일치 + 조상
            0 => mk_core::filter_grouped(visible, &st.filter), // 묶음: 창 제목까지, 구조 유지
            _ => mk_core::filter_procs(visible, &st.filter),
        };
        rebuild(st);
    });
}

/// 수집본은 그대로 두고 화면 행만 다시 만든다(정렬·펼침·보기 모드 변경 시).
/// 검색 중에는 결과가 부모 아래 숨지 않도록 **항상 평면**으로 그린다.
fn rebuild(st: &mut ListState) {
    // 검색 중 트리 보기는 계층을 유지한 채(조상 포함) 모두 펼쳐 보여준다.
    if st.view == 1 && !st.filter.is_empty() {
        expand_all_parents(st);
    }
    // 자식이 하나뿐인 앱 묶음은 **처음 볼 때만** 펼쳐 둔다. 매번 펼치면 사용자가 접어도
    // 2초 뒤 갱신에서 다시 열려 축소 버튼이 먹지 않는다(08-07 QA).
    if st.view == 0 {
        for gid in mk_core::auto_expand_groups(&st.procs) {
            if st.seeded.insert(gid) {
                st.expanded.insert(gid);
            }
        }
    }
    st.rows = if st.view == 0 {
        // 검색 중에는 접어 둔 묶음이라도 펼쳐 그린다 — 일치한 자식이 접힘에 가려지면 안 된다.
        // 접힘 상태(`expanded`)는 건드리지 않으므로 검색을 지우면 원래대로 돌아온다.
        let all: HashSet<u32>;
        let open = if st.filter.is_empty() {
            &st.expanded
        } else {
            all = st.procs.iter().map(|p| p.pid).collect();
            &all
        };
        mk_core::build_grouped_rows(&st.procs, st.sort_key, st.sort_asc, open, st.show_threads)
    } else if st.view == 1 {
        mk_core::build_rows(
            &st.procs,
            st.sort_key,
            st.sort_asc,
            &st.expanded,
            st.show_threads,
        )
    } else {
        mk_core::build_rows_flat(
            &st.procs,
            st.sort_key,
            st.sort_asc,
            &st.expanded,
            st.show_threads,
        )
    };
    st.top = st.top.min(st.max_top());
}

fn lo_i16(lp: LPARAM) -> i32 {
    (lp.0 & 0xFFFF) as i16 as i32
}
fn hi_i16(lp: LPARAM) -> i32 {
    ((lp.0 >> 16) & 0xFFFF) as i16 as i32
}

fn row_at(st: &ListState, y: i32) -> Option<&Row> {
    if y < HEAD_H {
        return None;
    }
    let idx = st.top + (y - HEAD_H) / ROW_H;
    if idx - st.top >= st.visible_rows() {
        return None;
    }
    st.rows.get(idx as usize)
}

/// 들여쓰기 폭·확장 버튼 크기(트리 표현).
const INDENT: i32 = 16;
const EXP_W: i32 = 14;

// 트리 디스클로저 셰브론 — `nexa-dir2` 규약 계승(crates/nexa-app/src/ctl/ordertree.rs:848,
// prefs.rs:1557 — MDL2 ChevronRight/ChevronDown을 9 DIP 소형으로).
const GLYPH_COLLAPSED: &str = "\u{E76C}"; // ChevronRight
const GLYPH_EXPANDED: &str = "\u{E70D}"; // ChevronDown
/// 원본 디스클로저 FontSize 9 규약(dw.rs:167).
const GLYPH_SIZE: i32 = -9;

/// 깊이별 확장 버튼 x 시작 좌표.
fn expander_x(depth: i32) -> i32 {
    8 + depth * INDENT
}

/// x좌표 → 헤더 열의 정렬 기준.
unsafe fn col_at(hwnd: HWND, width: i32, x: i32) -> Option<SortKey> {
    col_bounds_for(hwnd, width)
        .iter()
        .enumerate()
        .find(|(_, (x0, x1))| x >= *x0 && x < *x1)
        .map(|(i, _)| COLS[i].1)
}

/// 정렬 화살표(▲▼)가 차지하는 폭 — 제목과 겹치지 않게 항상 따로 확보한다.
const ARROW_W: i32 = 16;
/// 셀 좌우 안쪽 여백.
const CELL_PAD: i32 = 8;

/// 헤더 글자 폭 실측 — 고정 열이 제목보다 좁아 잘리는 것을 막는다(08-07 QA: 헤더 글자 깨짐).
/// DC가 없을 때(히트 테스트 등)는 `None`을 넘기면 선언된 기본 폭을 쓴다.
unsafe fn measure(dc: Option<(HDC, HFONT)>, s: &str) -> i32 {
    let Some((dc, font)) = dc else {
        return 0;
    };
    let buf: Vec<u16> = s.encode_utf16().collect();
    let old = SelectObject(dc, font.into());
    let mut sz = SIZE::default();
    let ok = GetTextExtentPoint32W(dc, &buf, &mut sz).as_bool();
    SelectObject(dc, old);
    if ok {
        sz.cx
    } else {
        0
    }
}

/// 열 x 경계. 규칙(08-07 QA — 헤더 글자 깨짐·마지막 열 잘림):
/// ① 고정 열 최소 폭 = 제목 실측 폭 + 여백(+ **정렬 중인 열만** 화살표 자리)
/// ② 이름 열이 남는 폭을 흡수하되 최소 90px
/// ③ 그래도 창을 넘치면 고정 열을 최소 폭까지 **비례 축소**해 마지막 열이 잘리지 않게 한다
unsafe fn col_bounds_dc(dc: Option<(HDC, HFONT)>, width: i32, skey: SortKey) -> Vec<(i32, i32)> {
    let mins: Vec<i32> = COLS
        .iter()
        .map(|c| {
            let arrow = if c.1 == skey { ARROW_W } else { 0 };
            measure(dc, mk_core::tr(c.0)) + arrow + CELL_PAD * 2
        })
        .collect();
    let mut widths: Vec<i32> = COLS
        .iter()
        .enumerate()
        .map(|(i, c)| if i == 0 { 0 } else { c.3.max(mins[i]) })
        .collect();

    let avail = (width - CELL_PAD * 2).max(120);
    let fixed: i32 = widths.iter().skip(1).sum();
    widths[0] = (avail - fixed).max(90);

    // 총 폭이 넘치면 고정 열부터 최소 폭까지 줄인다(이름 열은 이미 최소)
    let mut over: i32 = widths.iter().sum::<i32>() - avail;
    if over > 0 {
        for i in (1..widths.len()).rev() {
            let slack = widths[i] - mins[i];
            if slack <= 0 {
                continue;
            }
            let cut = slack.min(over);
            widths[i] -= cut;
            over -= cut;
            if over <= 0 {
                break;
            }
        }
    }

    let mut out = Vec::with_capacity(COLS.len());
    let mut x = CELL_PAD;
    for w in widths {
        out.push((x, x + w));
        x += w;
    }
    out
}

/// 히트 테스트용 — 창 DC로 같은 폭을 재현한다(그리기와 어긋나면 클릭 위치가 밀린다).
unsafe fn col_bounds_for(hwnd: HWND, width: i32) -> Vec<(i32, i32)> {
    let (font, skey) = with_state_ret(|st| (st.font_head, st.sort_key))
        .unwrap_or((HFONT::default(), SortKey::PrivateWs));
    let dc = GetDC(Some(hwnd));
    let bounds = col_bounds_dc(Some((dc, font)), width, skey);
    ReleaseDC(Some(hwnd), dc);
    bounds
}

unsafe fn track_pos(hwnd: HWND) -> i32 {
    let mut si = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_TRACKPOS,
        ..Default::default()
    };
    let _ = GetScrollInfo(hwnd, SB_VERT, &mut si);
    si.nTrackPos
}

unsafe fn sync_scrollbar(hwnd: HWND) {
    let (len, page, pos) =
        with_state_ret(|st| (st.rows.len() as i32, st.visible_rows(), st.top)).unwrap_or((0, 1, 0));
    let si = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SCROLLINFO_MASK(SIF_RANGE.0 | SIF_POS.0 | SIF_PAGE.0),
        nMin: 0,
        nMax: (len - 1).max(0),
        nPage: page as u32,
        nPos: pos,
        nTrackPos: 0,
    };
    SetScrollInfo(hwnd, SB_VERT, &si, true);
}

unsafe fn make_font(height: i32, weight: i32) -> HFONT {
    make_font_face(height, weight, w!("Segoe UI"))
}

unsafe fn make_font_face(height: i32, weight: i32, face: PCWSTR) -> HFONT {
    CreateFontW(
        height,
        0,
        0,
        0,
        weight,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        DEFAULT_PITCH.0 as u32,
        face,
    )
}

unsafe fn sys(idx: SYS_COLOR_INDEX) -> COLORREF {
    COLORREF(GetSysColor(idx))
}

unsafe fn fill(dc: HDC, rc: &RECT, color: COLORREF) {
    let br: HBRUSH = CreateSolidBrush(color);
    FillRect(dc, rc, br);
    let _ = DeleteObject(br.into());
}

unsafe fn text(dc: HDC, s: &str, rc: RECT, flags: DRAW_TEXT_FORMAT, color: COLORREF) {
    let mut buf: Vec<u16> = s.encode_utf16().collect();
    if buf.is_empty() {
        return;
    }
    let mut rc = rc;
    SetTextColor(dc, color);
    DrawTextW(dc, &mut buf, &mut rc, flags | DT_SINGLELINE | DT_NOPREFIX);
}

/// 색 혼합 — 그리드 선·교대 행 배경용(시스템 색에서 파생해 테마와 어울리게).
fn blend(a: COLORREF, b: COLORREF, t: u32) -> COLORREF {
    let mix = |sa: u32, sb: u32| ((sa * (100 - t) + sb * t) / 100) & 0xFF;
    let (ar, ag, ab) = (a.0 & 0xFF, (a.0 >> 8) & 0xFF, (a.0 >> 16) & 0xFF);
    let (br, bg, bb) = (b.0 & 0xFF, (b.0 >> 8) & 0xFF, (b.0 >> 16) & 0xFF);
    COLORREF(mix(ar, br) | (mix(ag, bg) << 8) | (mix(ab, bb) << 16))
}

unsafe fn paint(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    let (w, h) = with_state_ret(|st| (st.client.right, st.client.bottom)).unwrap_or((0, 0));
    if w <= 0 || h <= 0 {
        let _ = EndPaint(hwnd, &ps);
        return;
    }
    let mem = CreateCompatibleDC(Some(hdc));
    let bmp = CreateCompatibleBitmap(hdc, w, h);
    let old_bmp = SelectObject(mem, bmp.into());
    SetBkMode(mem, TRANSPARENT);

    let bg = sys(COLOR_WINDOW);
    let fg = sys(COLOR_WINDOWTEXT);
    let gray = sys(COLOR_GRAYTEXT);
    let face = sys(COLOR_BTNFACE);
    let hi = sys(COLOR_HIGHLIGHT);
    let hi_text = sys(COLOR_HIGHLIGHTTEXT);
    let grid = blend(bg, gray, 25); // 그리드 선
    let alt = blend(bg, face, 45); // 교대 행 배경
    fill(
        mem,
        &RECT {
            left: 0,
            top: 0,
            right: w,
            bottom: h,
        },
        bg,
    );
    let head_font = with_state_ret(|st| st.font_head).unwrap();
    let (skey, sasc) =
        with_state_ret(|st| (st.sort_key, st.sort_asc)).unwrap_or((SortKey::PrivateWs, false));
    let cols = col_bounds_dc(Some((mem, head_font)), w, skey);

    // ── 헤더(정렬 표시 포함)
    fill(
        mem,
        &RECT {
            left: 0,
            top: 0,
            right: w,
            bottom: HEAD_H,
        },
        face,
    );
    let old_font = SelectObject(mem, head_font.into());
    for (i, (title, key, right, _)) in COLS.iter().enumerate() {
        let (x0, x1) = cols[i];
        // 화살표는 셀 오른쪽 끝에 **따로** 그린다 — 제목과 한 문자열로 합치면 좁은 열에서
        // 서로 밀려 잘린다(08-07 QA: 헤더 글자 깨짐).
        if *key == skey {
            text(
                mem,
                if sasc { "▲" } else { "▼" },
                RECT {
                    left: x1 - ARROW_W,
                    top: 0,
                    right: x1,
                    bottom: HEAD_H,
                },
                DT_CENTER | DT_VCENTER,
                fg,
            );
        }
        // 제목은 화살표 자리를 뺀 영역에. 그래도 넘치면 말줄임(옆 열을 침범하지 않는다).
        text(
            mem,
            mk_core::tr(title),
            RECT {
                left: x0,
                top: 0,
                right: x1 - ARROW_W,
                bottom: HEAD_H,
            },
            (if *right { DT_RIGHT } else { DT_LEFT }) | DT_VCENTER | DT_END_ELLIPSIS,
            fg,
        );
        if i > 0 {
            fill(
                mem,
                &RECT {
                    left: x0 - 4,
                    top: 4,
                    right: x0 - 3,
                    bottom: HEAD_H - 4,
                },
                grid,
            );
        }
    }
    fill(
        mem,
        &RECT {
            left: 0,
            top: HEAD_H - 1,
            right: w,
            bottom: HEAD_H,
        },
        grid,
    );

    // ── 행
    SelectObject(mem, with_state_ret(|st| st.font).unwrap().into());
    with_state(|st| {
        let vis = st.visible_rows();
        for i in 0..vis {
            let Some(p) = st.rows.get((st.top + i) as usize) else {
                break;
            };
            let top = HEAD_H + i * ROW_H;
            let rc = RECT {
                left: 0,
                top,
                right: w,
                bottom: top + ROW_H,
            };
            let is_category = p.kind == RowKind::Category;
            let selected = !is_category && st.sel == (p.pid, p.tid);
            if selected {
                fill(mem, &rc, hi);
            } else if is_category {
                fill(mem, &rc, face); // 구획 머리글 — 선택색이 아니라 헤더색
            } else if (st.top + i) % 2 == 1 {
                fill(mem, &rc, alt);
            }
            let is_thread = p.kind == RowKind::Thread;
            let protected = !is_thread && mk_core::is_protected(&p.label);
            let (name_c, val_c) = if selected {
                (hi_text, hi_text)
            } else if protected || is_thread {
                (gray, gray) // 보호 대상·쓰레드 = 보조 정보
            } else {
                (fg, gray)
            };

            // 트리 들여쓰기 + 확장 버튼([+]/[-])
            let ex = expander_x(p.depth);
            if p.expandable {
                // 셰브론은 MDL2 전용 폰트로 — 본문 폰트로 그리면 tofu(nexa-dir2 dw.rs:811 교훈)
                SelectObject(mem, st.font_glyph.into());
                text(
                    mem,
                    if p.expanded {
                        GLYPH_EXPANDED
                    } else {
                        GLYPH_COLLAPSED
                    },
                    RECT {
                        left: ex,
                        top,
                        right: ex + EXP_W,
                        bottom: top + ROW_H,
                    },
                    DT_LEFT | DT_VCENTER,
                    if selected { hi_text } else { fg },
                );
                SelectObject(mem, st.font.into()); // 본문 폰트 복귀
            }
            let name_x = ex + EXP_W + 2;
            let cells = [
                if protected {
                    format!("{} 🔒", p.label)
                } else {
                    p.label.clone()
                },
                if is_thread || matches!(p.kind, RowKind::Category | RowKind::Group) {
                    String::new() // 분류·묶음 행에는 PID가 없다(작업 관리자와 동일)
                } else {
                    p.pid.to_string()
                },
                if is_thread || p.kind == RowKind::Window {
                    String::new() // 창 행은 프로세스의 일부 — 메모리를 중복 표기하지 않는다
                } else {
                    mk_core::fmt_size(p.private_ws)
                },
                if is_thread || p.kind == RowKind::Window {
                    String::new()
                } else {
                    mk_core::fmt_size(p.ws)
                },
                if is_thread {
                    format!("{} {}", p.threads, tr("switches"))
                } else if matches!(p.kind, RowKind::Category | RowKind::Window) {
                    String::new()
                } else {
                    p.threads.to_string()
                },
            ];
            for (ci, s) in cells.iter().enumerate() {
                let (x0, x1) = cols[ci];
                let left = if ci == 0 { name_x } else { x0 };
                text(
                    mem,
                    s,
                    RECT {
                        left,
                        top,
                        right: x1,
                        bottom: top + ROW_H,
                    },
                    (if COLS[ci].2 { DT_RIGHT } else { DT_LEFT }) | DT_VCENTER | DT_END_ELLIPSIS,
                    if ci == 0 { name_c } else { val_c },
                );
            }
            fill(
                mem,
                &RECT {
                    left: 0,
                    top: top + ROW_H - 1,
                    right: w,
                    bottom: top + ROW_H,
                },
                grid,
            );
        }

        // ── 상태 줄
        let sy = h - STATUS_H - SEARCH_H;
        // 상태 줄 + 검색 줄 배경(검색 칸·버튼은 자식 컨트롤이 그 위에 그린다)
        fill(
            mem,
            &RECT {
                left: 0,
                top: sy,
                right: w,
                bottom: h,
            },
            face,
        );
        let sel_txt = st
            .selected()
            .filter(|r| r.kind != RowKind::Category)
            .map(|r| {
                if r.kind == RowKind::Thread {
                    format!(
                        " · {}: {} {} (PID {})",
                        tr("Selected"),
                        tr("Thread"),
                        r.tid,
                        r.pid
                    )
                } else {
                    format!(" · {}: {} ({})", tr("Selected"), r.label, r.pid)
                }
            })
            .unwrap_or_default();
        let mode = if BUSY.get() {
            tr("Working…").to_string()
        } else if !st.filter.is_empty() {
            format!(
                "🔍 {} \"{}\" — {} {}",
                tr("Search"),
                st.filter,
                st.procs.len(),
                tr("matches (Esc to clear)")
            )
        } else {
            let mode_name = match st.view {
                1 => tr("tree view"),
                2 => tr("flat list"),
                _ => tr("grouped by app"),
            };
            format!("{} {} · {}", st.procs.len(), tr("processes"), mode_name)
        };
        text(
            mem,
            &format!(
                "{mode} · {} {}{sel_txt}",
                st.refresh_ms / 1000,
                tr("sec refresh · type=search · header=sort · right-click=trim/end")
            ),
            RECT {
                left: 8,
                top: sy,
                right: w - 8,
                bottom: sy + STATUS_H,
            },
            DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
            fg,
        );
    });

    SelectObject(mem, old_font);
    let _ = BitBlt(hdc, 0, 0, w, h, Some(mem), 0, 0, SRCCOPY);
    SelectObject(mem, old_bmp);
    let _ = DeleteObject(bmp.into());
    let _ = DeleteDC(mem);
    let _ = EndPaint(hwnd, &ps);
}

/// 행 우클릭 메뉴 — 쓰레드 행이면 쓰레드 종료, 프로세스 행이면 회수/종료/트리 종료(FR-3).
unsafe fn item_menu(hwnd: HWND, row: &Row) {
    let Ok(menu) = CreatePopupMenu() else { return };
    if row.kind == RowKind::Thread {
        let _ = AppendMenuW(menu, MF_STRING, ID_KILL_THREAD_SEL, tw!("End Thread(&E)…"));
    } else {
        let protected = mk_core::is_protected(&row.label);
        let flags = if protected {
            MF_STRING | MF_GRAYED
        } else {
            MF_STRING
        };
        let _ = AppendMenuW(menu, flags, ID_TRIM_SEL, tw!("Trim Memory(&T)"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(menu, flags, ID_KILL_SEL, tw!("End Process(&K)…"));
        let _ = AppendMenuW(menu, flags, ID_KILL_TREE_SEL, tw!("End Process Tree(&R)…"));
        if protected {
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
            let _ = AppendMenuW(
                menu,
                MF_STRING | MF_GRAYED,
                0,
                tw!("Protected — cannot be modified"),
            );
        }
    }
    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let _ = SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenu(
        menu,
        TPM_RIGHTBUTTON | TPM_RETURNCMD,
        pt.x,
        pt.y,
        None,
        hwnd,
        None,
    );
    let _ = DestroyMenu(menu);
    if cmd.0 != 0 {
        run_row_action(hwnd, row, cmd.0 as usize);
    }
}

/// 선택 행에 대한 실제 동작 — 메뉴바·컨텍스트 메뉴 공용 경로.
unsafe fn run_row_action(hwnd: HWND, row: &Row, id: usize) {
    match id {
        ID_TRIM_SEL if row.kind == RowKind::Process => spawn_action(hwnd, RowAction::Trim, row.pid),
        ID_KILL_SEL if row.kind == RowKind::Process => {
            if confirm(
                hwnd,
                tr("End Process — Confirm"),
                &format!(
                    "{} (PID {})?\n{}",
                    row.label,
                    row.pid,
                    tr("Unsaved work may be lost.")
                ),
            ) {
                spawn_action(hwnd, RowAction::Kill, row.pid);
            }
        }
        ID_KILL_TREE_SEL if row.kind == RowKind::Process => {
            let n =
                with_state_ret(|st| mk_core::descendants(&st.procs, row.pid).len()).unwrap_or(0);
            if confirm(
                hwnd,
                tr("End Process Tree — Confirm"),
                &format!(
                    "{} (PID {}) + {} child process(es)?\n{}",
                    row.label,
                    row.pid,
                    n,
                    tr("Unsaved work may be lost.")
                ),
            ) {
                spawn_action(hwnd, RowAction::KillTree, row.pid);
            }
        }
        ID_KILL_THREAD_SEL if row.kind == RowKind::Thread => {
            if confirm(
                hwnd,
                tr("End Thread — Confirm"),
                &format!(
                    "{} {} (PID {})?\n⚠️ {}",
                    tr("Thread"),
                    row.tid,
                    row.pid,
                    tr("Force-ending a thread can corrupt the program's state.")
                ),
            ) {
                spawn_action(hwnd, RowAction::KillThread, row.tid);
            }
        }
        ID_TRIM_SEL | ID_KILL_SEL | ID_KILL_TREE_SEL => {
            info_box(hwnd, tr("Wrong target"), tr("Select a process row."))
        }
        ID_KILL_THREAD_SEL => info_box(hwnd, tr("Wrong target"), tr("Select a thread row.")),
        _ => {}
    }
}

// ---- 자동 실행(HKCU Run) — 사용자가 메뉴로 켤 때만 레지스트리를 건드린다 ----

/// 현재 자동 실행 등록 여부.
pub fn autorun_enabled() -> bool {
    unsafe {
        let mut key = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, Some(0), KEY_READ, &mut key).is_err() {
            return false;
        }
        let mut size = 0u32;
        let found = RegQueryValueExW(key, RUN_VALUE, None, None, None, Some(&mut size)).is_ok();
        let _ = RegCloseKey(key);
        found
    }
}

/// 자동 실행 등록/해제. 실패 시 false.
pub fn set_autorun(enable: bool) -> bool {
    unsafe {
        let mut key = HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            Some(0),
            KEY_READ | KEY_WRITE,
            &mut key,
        )
        .is_err()
        {
            return false;
        }
        let ok = if enable {
            let Ok(exe) = std::env::current_exe() else {
                let _ = RegCloseKey(key);
                return false;
            };
            // 경로에 공백이 있어도 안전하도록 따옴표로 감싼다
            let quoted: Vec<u16> = format!("\"{}\"", exe.display())
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let bytes = std::slice::from_raw_parts(quoted.as_ptr() as *const u8, quoted.len() * 2);
            RegSetValueExW(key, RUN_VALUE, Some(0), REG_SZ, Some(bytes)).is_ok()
        } else {
            RegDeleteValueW(key, RUN_VALUE).is_ok()
        };
        let _ = RegCloseKey(key);
        ok
    }
}

/// 자동 실행 값에 남는 PWSTR 경고 방지용(참조 유지 목적 — 사용하지 않음).
const _: Option<PWSTR> = None;
