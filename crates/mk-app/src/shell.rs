//! shell — 상주 셸(DR-7): 트레이 아이콘(사용률 %·색 단계)·툴팁·우클릭 메뉴·5초 폴링. (M0-1)
//! 상주 최소 원칙(docs/08 §4-1): 숨김 호스트 창 + 타이머 + 트레이 외 아무것도 초기화하지 않는다.
//! 창 등록·wndproc 관용구는 nexa-dir2 crates/nexa-app/src/tip.rs 패턴 계승.

use std::cell::Cell;
use std::time::Instant;

use mk_core::{level_with, Level, MemSnapshot};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HANDLE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateSolidBrush,
    DeleteDC, DeleteObject, DrawTextW, FillRect, GetDC, PatBlt, ReleaseDC, SelectObject, SetBkMode,
    SetTextColor, BLACKNESS, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
    DEFAULT_PITCH, DT_CENTER, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, OUT_DEFAULT_PRECIS,
    TRANSPARENT,
};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{
    CreateMemoryResourceNotification, LowMemoryResourceNotification,
    QueryMemoryResourceNotification,
};
use windows::Win32::System::ProcessStatus::{
    K32EmptyWorkingSet, K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
};
use windows::Win32::System::SystemInformation::{
    GetLocalTime, GlobalMemoryStatusEx, MEMORYSTATUSEX,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
    MOD_SHIFT, MOD_WIN,
};
use windows::Win32::UI::Shell::{
    SHQueryUserNotificationState, ShellExecuteW, Shell_NotifyIconW, NIF_ICON, NIF_INFO,
    NIF_MESSAGE, NIF_TIP, NIIF_ERROR, NIIF_INFO, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon,
    DestroyMenu, DestroyWindow, DispatchMessageW, FindWindowW, GetCursorPos, GetMessageW,
    GetSystemMetrics, KillTimer, PostMessageW, PostQuitMessage, RegisterClassW,
    RegisterWindowMessageW, SetForegroundWindow, SetTimer, TrackPopupMenu, HICON, ICONINFO,
    MF_GRAYED, MF_SEPARATOR, MF_STRING, MSG, SM_CXSMICON, SM_CYSMICON, SW_SHOWNORMAL,
    TPM_BOTTOMALIGN, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WINDOW_EX_STYLE, WM_APP,
    WM_CONTEXTMENU, WM_CREATE, WM_DESTROY, WM_HOTKEY, WM_LBUTTONUP, WM_RBUTTONUP, WM_TIMER,
    WNDCLASSW, WS_OVERLAPPED,
};

const CLASS: PCWSTR = w!("NexaMemKeeperShell");
/// 트레이 콜백 메시지(창 전용 사용자 메시지).
const WM_TRAY: u32 = WM_APP + 1;
/// 워커 완료 통지(wParam = 종료 코드) — 워커 스레드에서 PostMessage(docs/08 §4-3).
const WM_WORKER_DONE: u32 = WM_APP + 2;
/// 단일 프로세스 작업(trim/kill) 완료(wParam = 성공 여부, lParam = PID).
const WM_ITEM_DONE: u32 = WM_APP + 3;
/// 설정 창이 저장을 마쳤음(M4) — 셸이 settings.ini를 다시 읽어 즉시 반영.
const WM_SETTINGS_CHANGED: u32 = WM_APP + 4;
const TRAY_ID: u32 = 1;
const TIMER_POLL: usize = 1;
/// 풍선 알림 자동 닫기 타이머(사용자 QA 08-07 — 닫기 전까지 계속 남아 있었다).
const TIMER_BALLOON: usize = 2;
/// 알림이 화면에 머무는 시간(ms). Windows 10/11은 `uTimeout`을 무시하므로 직접 지운다.
const BALLOON_MS: u32 = 6_000;
const CMD_EXIT: usize = 100;
const CMD_CLEAN: usize = 101;
const CMD_ELEVATE: usize = 102;
const CMD_LIST: usize = 103;
const CMD_SETTINGS: usize = 104;

thread_local! {
    /// 현재 설정(M4 — settings.ini). 설정 창 저장 시 WM_SETTINGS_CHANGED로 다시 읽는다.
    static SETTINGS: std::cell::RefCell<mk_core::Settings> =
        std::cell::RefCell::new(mk_core::Settings::default());
    /// 직전 표시 %(같으면 아이콘 재생성 생략 — 유휴 CPU ~0% 게이트).
    static LAST_PERCENT: Cell<u32> = const { Cell::new(u32::MAX) };
    /// 현재 트레이 아이콘(교체 시 이전 것 파괴).
    static TRAY_HICON: Cell<HICON> = const { Cell::new(HICON(std::ptr::null_mut())) };
    /// explorer 재시작 감지용 "TaskbarCreated" 등록 메시지.
    static TASKBAR_MSG: Cell<u32> = const { Cell::new(0) };
    /// 정리 워커 실행 중(이중 실행 방지 — 메뉴 비활성).
    static CLEAN_BUSY: Cell<bool> = const { Cell::new(false) };
    /// 마지막 정리(시각, 회수 MB) — 툴팁 둘째 줄.
    static LAST_CLEAN: Cell<Option<(Instant, u32)>> = const { Cell::new(None) };
    /// 전역 핫키 등록 여부(M4-2) — 재설정 전 해제해야 하므로 상태를 들고 있는다.
    static HOTKEY_ON: Cell<bool> = const { Cell::new(false) };
    /// 이번 정리를 일으킨 계기(X-3 기록용) — 수동/핫키/임계·주기/저메모리.
    static CLEAN_TRIGGER: std::cell::RefCell<&'static str> = const { std::cell::RefCell::new("manual") };
    /// 워치리스트로 트림한 시각(PID → 시각) — 같은 프로세스를 연달아 트림하지 않기 위한 쿨다운(X-1).
    static WATCH_LAST: std::cell::RefCell<std::collections::HashMap<u32, Instant>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    /// 저메모리 알림 핸들(M5-1) — 한 번 만들어 두고 폴링 때마다 상태만 물어본다.
    /// 대기 스레드를 쓰지 않는 이유: 스레드 하나가 유휴 예산(DR-8)을 갉아먹고, 이미 도는
    /// 폴링 타이머에 질의 1콜을 얹는 편이 싸다(질의는 커널 플래그 읽기 수준).
    static LOWMEM: Cell<Option<HANDLE>> = const { Cell::new(None) };
    /// 셸이 뜬 시각(X-5) — 이후 `start_delay_ms` 동안 자동 정리·워치리스트 트림을 보류한다.
    /// 타이머·스레드를 늘리지 않고 기존 폴링 틱에서 경과 시간만 비교한다(DR-8).
    static STARTED: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// 전역 핫키 ID(이 창에서 유일하면 된다).
const HOTKEY_ID: i32 = 1;

/// 상주 셸 진입 — 숨김 호스트 창 생성 후 메시지 루프(종료 메뉴 선택까지 블록).
pub fn run() -> windows::core::Result<()> {
    unsafe {
        let hinst = GetModuleHandleW(None)?;
        let wc = WNDCLASSW {
            lpszClassName: CLASS,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinst.into(),
            ..Default::default()
        };
        RegisterClassW(&wc);
        // 표시하지 않는 트레이 호스트. HWND_MESSAGE가 아닌 일반 숨김 창 —
        // TaskbarCreated 브로드캐스트(explorer 재시작) 수신을 위해 필요.
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            CLASS,
            w!("Nexa MemKeeper"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinst.into()),
            None,
        )?;
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            DispatchMessageW(&msg);
        }
        Ok(())
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            TASKBAR_MSG.set(RegisterWindowMessageW(w!("TaskbarCreated")));
            STARTED.set(Some(Instant::now()));
            apply_settings(hwnd); // settings.ini 로드 + 폴링 주기 반영
            tray_add(hwnd);
            self_trim();
            LRESULT(0)
        }
        WM_TIMER => {
            if wp.0 == TIMER_BALLOON {
                dismiss_balloon(hwnd);
            } else {
                tray_update(hwnd);
                trim_if_over_budget(); // 예산을 넘으면 스스로 반납(NFR-3·DR-8)
                                       // 시작 직후에는 자동 작업을 쉰다(X-5) — 부팅 중 다른 앱의 로딩과 겹치지 않게.
                                       // 트레이 갱신과 자기 트림은 위에서 이미 돌았고, 수동 정리·핫키는 이 경로가 아니다.
                if !startup_hold() {
                    watch_trim_if_due(); // 워치리스트 표적 트림(X-1 — 꺼져 있으면 즉시 반환)
                    auto_clean_if_due(hwnd);
                }
            }
            LRESULT(0)
        }
        WM_SETTINGS_CHANGED => {
            apply_settings(hwnd);
            LAST_PERCENT.set(u32::MAX); // 임계값이 바뀌었을 수 있으니 아이콘 강제 갱신
            tray_update(hwnd);
            LRESULT(0)
        }
        WM_TRAY => {
            let event = lp.0 as u32;
            if cfg!(debug_assertions) {
                eprintln!("shell: WM_TRAY event=0x{event:04X}");
            }
            if event == WM_RBUTTONUP || event == WM_CONTEXTMENU {
                show_menu(hwnd);
            } else if event == WM_LBUTTONUP {
                crate::popup::show(hwnd); // FR-2 Top 5 팝업
            }
            LRESULT(0)
        }
        WM_ITEM_DONE => {
            let pid = lp.0 as u32;
            let (title, body) = if wp.0 != 0 {
                ("작업 완료", format!("PID {pid} 처리 완료"))
            } else {
                (
                    "작업 실패",
                    format!("PID {pid} 처리 실패(권한·보호 대상 여부 확인)"),
                )
            };
            notify_balloon(hwnd, title, &body, wp.0 != 0);
            LRESULT(0)
        }
        WM_WORKER_DONE => {
            CLEAN_BUSY.set(false);
            let outcome = mk_core::decode_clean_result(wp.0 as u32);
            if let Some(o) = outcome {
                LAST_CLEAN.set(Some((Instant::now(), o.mb)));
                // 기록은 **표시값과 같은 숫자**로 남긴다(회수 MB + Active 감소분) — 나중에
                // "그때 실제로 얼마나 걷혔나"를 다시 따질 수 있어야 한다(X-3 · docs/21 §2-1).
                let trigger = CLEAN_TRIGGER.with(|t| *t.borrow());
                crate::config::append_history(&format!(
                    "{},{},{},{},{}",
                    now_stamp(),
                    trigger,
                    o.mb,
                    o.active_mb,
                    o.partial
                ));
            }
            let (title, body) = mk_core::clean_report(outcome);
            notify_balloon(hwnd, &title, &body, outcome.is_some());
            // 정리 직후 수치 즉시 반영
            LAST_PERCENT.set(u32::MAX);
            tray_update(hwnd);
            LRESULT(0)
        }
        WM_HOTKEY => {
            spawn_clean_as(hwnd, "hotkey"); // 전역 핫키 = '지금 정리'(M4-2)
            LRESULT(0)
        }
        WM_DESTROY => {
            let _ = KillTimer(Some(hwnd), TIMER_POLL);
            let _ = KillTimer(Some(hwnd), TIMER_BALLOON);
            if HOTKEY_ON.replace(false) {
                let _ = UnregisterHotKey(Some(hwnd), HOTKEY_ID);
            }
            let nid = base_nid(hwnd);
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            drop_icon();
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ if msg != 0 && msg == TASKBAR_MSG.get() => {
            // explorer 재시작 — 트레이 아이콘 소실 → 재등록
            LAST_PERCENT.set(u32::MAX);
            tray_add(hwnd);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

/// 현재 메모리 스냅샷 — `GlobalMemoryStatusEx`(docs/07 §1). 실패 시 total=0(표시 0%).
pub(crate) fn read_mem() -> MemSnapshot {
    let mut ms = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    if unsafe { GlobalMemoryStatusEx(&mut ms) }.is_err() {
        return MemSnapshot { total: 0, avail: 0 };
    }
    MemSnapshot {
        total: ms.ullTotalPhys,
        avail: ms.ullAvailPhys,
    }
}

unsafe fn base_nid(hwnd: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ID,
        ..Default::default()
    }
}

unsafe fn fill_nid(hwnd: HWND, snap: &MemSnapshot) -> NOTIFYICONDATAW {
    let mut nid = base_nid(hwnd);
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    let icon = make_icon(snap.used_percent());
    let old = TRAY_HICON.replace(icon);
    if !old.is_invalid() {
        let _ = DestroyIcon(old);
    }
    nid.hIcon = icon;
    let mut tip_text = mk_core::tray_tooltip(snap);
    if let Some((at, mb)) = LAST_CLEAN.get() {
        tip_text.push('\n');
        tip_text.push_str(&mk_core::fmt_last_clean(mb, at.elapsed().as_secs() / 60));
    }
    let tip: Vec<u16> = tip_text.encode_utf16().collect();
    let n = tip.len().min(nid.szTip.len() - 1);
    nid.szTip[..n].copy_from_slice(&tip[..n]);
    nid
}

/// 풍선 알림(FR-4 결과 통지). `notify = false`면 **실패만** 알린다 —
/// 성공 알림은 끌 수 있어도 실패를 조용히 삼키지는 않는다(docs/07 §5 조용한 성공 금지 원칙).
unsafe fn notify_balloon(hwnd: HWND, title: &str, body: &str, ok: bool) {
    if ok && !SETTINGS.with(|c| c.borrow().notify) {
        return;
    }
    let mut nid = base_nid(hwnd);
    nid.uFlags = NIF_INFO;
    nid.dwInfoFlags = if ok { NIIF_INFO } else { NIIF_ERROR };
    let t: Vec<u16> = title.encode_utf16().collect();
    let n = t.len().min(nid.szInfoTitle.len() - 1);
    nid.szInfoTitle[..n].copy_from_slice(&t[..n]);
    let b: Vec<u16> = body.encode_utf16().collect();
    let n = b.len().min(nid.szInfo.len() - 1);
    nid.szInfo[..n].copy_from_slice(&b[..n]);
    let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
    // 지정 시간 뒤 스스로 내린다 — Windows 10/11은 `uTimeout`을 무시해서 배너가 닫기 전까지
    // 남아 있었다(08-07 사용자 QA). 빈 정보 텍스트로 갱신하는 것이 확실한 제거 방법이다.
    SetTimer(Some(hwnd), TIMER_BALLOON, BALLOON_MS, None);
}

/// 표시 중인 풍선 알림 내리기(빈 `szInfo`로 갱신 = 배너 제거).
unsafe fn dismiss_balloon(hwnd: HWND) {
    let _ = KillTimer(Some(hwnd), TIMER_BALLOON);
    let mut nid = base_nid(hwnd);
    nid.uFlags = NIF_INFO; // szInfo·szInfoTitle 모두 빈 상태
    let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
}

/// 설정 로드 + 즉시 반영(폴링 타이머·전역 핫키 재설정). 시작 시와 설정 저장 후에 호출.
unsafe fn apply_settings(hwnd: HWND) {
    let s = crate::config::load();
    let poll_ms = s.poll_ms;
    let hotkey = mk_core::parse_hotkey(&s.hotkey_clean);
    SETTINGS.with(|c| *c.borrow_mut() = s);
    SetTimer(Some(hwnd), TIMER_POLL, poll_ms, None); // 같은 ID 재설정 = 주기 교체
    apply_hotkey(hwnd, hotkey);
}

/// 전역 핫키 재등록(M4-2). 기존 등록은 항상 먼저 해제한다 — 설정을 바꿀 때마다 쌓이면 안 된다.
/// 다른 앱이 이미 쓰는 조합이면 등록이 실패하는데, **조용히 넘기지 않고** 사용자에게 알린다.
unsafe fn apply_hotkey(hwnd: HWND, hk: Option<mk_core::Hotkey>) {
    if HOTKEY_ON.replace(false) {
        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_ID);
    }
    let Some(hk) = hk else { return };
    let mut mods = HOT_KEY_MODIFIERS(0);
    if hk.ctrl {
        mods |= MOD_CONTROL;
    }
    if hk.alt {
        mods |= MOD_ALT;
    }
    if hk.shift {
        mods |= MOD_SHIFT;
    }
    if hk.win {
        mods |= MOD_WIN;
    }
    // MOD_NOREPEAT: 누르고 있어도 1회만(정리가 연속 실행되지 않게)
    match RegisterHotKey(Some(hwnd), HOTKEY_ID, mods | MOD_NOREPEAT, hk.vk) {
        Ok(()) => {
            HOTKEY_ON.set(true);
            if cfg!(debug_assertions) {
                eprintln!("shell: 핫키 등록 성공 — {}", mk_core::hotkey_to_string(&hk));
            }
        }
        Err(_) => {
            let text = mk_core::hotkey_to_string(&hk);
            notify_balloon(
                hwnd,
                "핫키 등록 실패",
                &format!("{text} 조합은 다른 프로그램이 사용 중입니다.\n설정에서 다른 조합을 지정해 주세요."),
                false,
            );
        }
    }
}

/// 자동 정리 판정(FR-4) — 임계값·쿨다운·주기는 mk-core가 판단한다.
/// 유휴 상태에서 워킹셋이 예산을 넘으면 반납한다(NFR-3 · DR-8).
///
/// 팝업을 띄우면 목록 버퍼·GDI·공유 DLL 페이지가 들어와 워킹셋이 1.3MB → 11MB까지 오른다.
/// 닫으면 대부분 스스로 돌아오지만(08-08 실측 2.7MB), 그러지 못한 경우를 대비한 **안전망**이다.
/// 플래그로 "팝업을 띄웠었나"를 좇는 대신 **지금 넘치는가**만 본다 — 어떤 경로로 불어났든 스스로
/// 회복하고, 조건이 아닐 때는 아무 일도 하지 않아 유휴 CPU도 그대로다.
unsafe fn trim_if_over_budget() {
    // 팝업이 떠 있는 동안은 건드리지 않는다(방금 읽어 온 페이지를 도로 뱉으면 갱신이 느려진다)
    if let Ok(h) = FindWindowW(w!("NexaMemKeeperPopup"), None) {
        if !h.is_invalid() {
            return;
        }
    }
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    // cb를 채우지 않으면 호출이 조용히 실패한다(반환값만 보고 넘기면 원인을 못 찾는다 — 08-08)
    let mut pmc = PROCESS_MEMORY_COUNTERS {
        cb: size,
        ..Default::default()
    };
    if K32GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, size).as_bool()
        && pmc.WorkingSetSize > BUDGET_RSS
    {
        self_trim();
    }
}

/// 시작 후 보류 구간인가(X-5). 판정은 mk-core, 여기서는 경과 시간만 잰다.
/// 설정 창에서 값을 바꾸면 다음 틱부터 새 값으로 판정된다(시작 시각은 다시 잡지 않는다).
fn startup_hold() -> bool {
    let Some(at) = STARTED.get() else {
        return false;
    };
    let since = at.elapsed().as_millis() as u64;
    SETTINGS.with(|c| c.borrow().in_startup_hold(since))
}

/// 유휴 워킹셋 예산(DR-8) — 이 값을 넘으면 스스로 반납한다.
const BUDGET_RSS: usize = 5 * 1024 * 1024;

/// 워치리스트 표적 트림(X-1) — 지정한 프로세스가 임계를 넘으면 **그 프로세스만** 트림한다.
///
/// 전역 정리와 달리 다른 앱의 캐시를 건드리지 않아 체감 손해가 작다(docs/21의 교훈).
/// 꺼져 있으면 **열거조차 하지 않는다** — 유휴 비용 0. 켠 경우에도 열거는 시스템 콜 1회다.
/// 같은 프로세스를 연달아 트림하지 않도록 자동 정리와 **같은 쿨다운**을 프로세스별로 적용한다.
unsafe fn watch_trim_if_due() {
    let (enabled, watch, cooldown) = SETTINGS.with(|c| {
        let s = c.borrow();
        (s.watch_enabled, s.watch.clone(), s.cooldown_ms as u64)
    });
    if !enabled || watch.is_empty() {
        return;
    }
    let procs = crate::procs::enumerate_basic();
    for pid in mk_core::watch_targets(&procs, &watch) {
        let due = WATCH_LAST.with(|m| {
            m.borrow()
                .get(&pid)
                .map(|t| t.elapsed().as_millis() as u64 >= cooldown)
                .unwrap_or(true)
        });
        if !due {
            continue;
        }
        WATCH_LAST.with(|m| m.borrow_mut().insert(pid, Instant::now()));
        spawn_trim(pid);
    }
    // 종료된 프로세스의 기록은 버린다(오래 켜 둬도 표가 계속 자라지 않게)
    let alive: std::collections::HashSet<u32> = procs.iter().map(|p| p.pid).collect();
    WATCH_LAST.with(|m| m.borrow_mut().retain(|pid, _| alive.contains(pid)));
}

/// 워커 실행 명령 — 자기 exe + `--worker <args>`. 워커를 띄우는 곳은 전부 이 함수를 거친다.
///
/// **디버그 빌드 한정 방어(C-1)**: 디버그 exe는 콘솔 서브시스템이라(main.rs), 콘솔이 없는 부모
/// (탐색기·자동 실행)에서 워커를 띄우면 그때마다 까만 콘솔 창이 뜬다. 부모에 콘솔이 **없을 때만**
/// `CREATE_NO_WINDOW`를 준다 — 터미널에서 띄운 경우에는 주지 않아 워커의 stderr 진단
/// (docs/18 §4-1)이 그대로 보인다. 릴리스 워커는 GUI 서브시스템이라 콘솔이 생기지 않으므로
/// 이 분기를 아예 컴파일하지 않는다(릴리스 바이너리 무변경).
fn worker_cmd(args: &[&str]) -> Option<std::process::Command> {
    let exe = std::env::current_exe().ok()?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--worker").args(args);
    #[cfg(debug_assertions)]
    {
        use std::os::windows::process::CommandExt;
        use windows::Win32::System::Console::GetConsoleWindow;
        use windows::Win32::System::Threading::CREATE_NO_WINDOW;
        if unsafe { GetConsoleWindow() }.is_invalid() {
            cmd.creation_flags(CREATE_NO_WINDOW.0);
        }
    }
    Some(cmd)
}

/// 프로세스 1개 트림 — 상태 변경은 워커가 한다(DR-7). 결과는 알리지 않는다(조용한 표적 트림).
fn spawn_trim(pid: u32) {
    std::thread::spawn(move || {
        if let Some(mut cmd) = worker_cmd(&["trim", &pid.to_string()]) {
            let _ = cmd.status();
        }
    });
}

unsafe fn auto_clean_if_due(hwnd: HWND) {
    if CLEAN_BUSY.get() {
        return;
    }
    let since = LAST_CLEAN
        .get()
        .map(|(at, _)| at.elapsed().as_millis() as u64)
        .unwrap_or(u64::MAX);
    let percent = read_mem().used_percent();
    let low = low_memory_now();
    let full = fullscreen_now();
    let due =
        SETTINGS.with(|c| mk_core::should_auto_clean_full(&c.borrow(), percent, since, low, full));
    if due {
        // 같은 자동 정리라도 계기가 다르면 나중에 해석이 달라진다 — 기록에 구분해 남긴다
        let why = if low {
            "lowmem"
        } else if percent >= SETTINGS.with(|c| c.borrow().threshold_percent) {
            "threshold"
        } else {
            "interval"
        };
        spawn_clean_as(hwnd, why);
    }
}

/// 지금 **전체화면·발표 모드**인가(X-2) — 게임·발표 중 자동 정리를 보류하는 판단 재료.
///
/// `SHQueryUserNotificationState`는 Windows가 알림을 띄워도 되는지 판단할 때 쓰는 값이라
/// "지금 방해하면 안 되는 상태"를 그대로 알려 준다. 알림을 미룰 상황이면 정리도 미루는 게 맞다.
/// 조회에 실패하면 **false**(정리를 막지 않는다 — 기능이 조용히 멈추는 쪽보다 낫다).
unsafe fn fullscreen_now() -> bool {
    match SHQueryUserNotificationState() {
        Ok(state) => {
            state == QUNS_BUSY
                || state == QUNS_RUNNING_D3D_FULL_SCREEN
                || state == QUNS_PRESENTATION_MODE
        }
        Err(_) => false,
    }
}

/// Windows가 "여유 메모리 부족"을 알리고 있는가(M5-1 — docs/07 §2).
///
/// `CreateMemoryResourceNotification`은 **핸들 1개**를 만들고, 질의는 커널이 들고 있는 플래그를
/// 읽어 오는 수준이라 폴링에 얹어도 부담이 없다. 만들지 못하면(정책·오래된 OS) 조용히 false —
/// 임계값·주기 트리거는 그대로 동작한다.
unsafe fn low_memory_now() -> bool {
    let h = match LOWMEM.get() {
        Some(h) => h,
        None => {
            let Ok(h) = CreateMemoryResourceNotification(LowMemoryResourceNotification) else {
                return false;
            };
            LOWMEM.set(Some(h));
            h
        }
    };
    let mut low = windows::core::BOOL(0);
    QueryMemoryResourceNotification(h, &mut low).is_ok() && low.as_bool()
}

/// 설정 창(워커 프로세스)이 저장 후 호출 — 상주 셸을 찾아 재적용을 알린다.
pub(crate) fn notify_settings_changed() {
    unsafe {
        if let Ok(h) = FindWindowW(CLASS, None) {
            if !h.is_invalid() {
                let _ = PostMessageW(Some(h), WM_SETTINGS_CHANGED, WPARAM(0), LPARAM(0));
            }
        }
    }
}

/// 설정 창 열기 — 이미 떠 있으면 앞으로 가져온다(워커 프로세스, DR-7).
pub(crate) fn open_settings() {
    unsafe {
        if let Ok(h) = FindWindowW(w!("NexaMemKeeperSettings"), None) {
            if !h.is_invalid() {
                let _ = SetForegroundWindow(h);
                return;
            }
        }
    }
    if let Some(mut cmd) = worker_cmd(&["settings"]) {
        let _ = cmd.spawn();
    }
}

/// 팝업 등 외부 모듈에서 정리 요청(팝업 [지금 정리] 버튼).
pub(crate) fn request_clean(hwnd: HWND) {
    spawn_clean(hwnd);
}

/// 전체 목록 창 열기 — 워커 프로세스(DR-7). 이미 떠 있으면 그 창을 앞으로 가져온다.
pub(crate) fn open_list() {
    unsafe {
        let existing = FindWindowW(w!("NexaMemKeeperList"), None);
        if let Ok(h) = existing {
            if !h.is_invalid() {
                let _ = SetForegroundWindow(h);
                return;
            }
        }
    }
    if let Some(mut cmd) = worker_cmd(&["list"]) {
        // 창이 살아 있는 동안 워커가 상주하므로 대기하지 않는다(spawn 후 분리).
        let _ = cmd.spawn();
    }
}

/// 단일 프로세스 대상 워커 요청(`trim`/`kill`) — 완료 시 셸이 결과를 풍선으로 알린다.
pub(crate) fn request_worker(hwnd: HWND, action: &'static str, pid: u32) {
    let hwnd_raw = hwnd.0 as usize;
    std::thread::spawn(move || {
        let ok = worker_cmd(&[action, &pid.to_string()])
            .and_then(|mut cmd| cmd.status().ok())
            .map(|s| s.success())
            .unwrap_or(false);
        unsafe {
            let _ = PostMessageW(
                Some(HWND(hwnd_raw as *mut core::ffi::c_void)),
                WM_ITEM_DONE,
                WPARAM(usize::from(ok)),
                LPARAM(pid as isize),
            );
        }
    });
}

/// 정리 워커 기동(DR-7) — 별도 스레드에서 프로세스 실행·대기 후 결과를 PostMessage.
/// 워커 크래시(code=None)도 WORKER_ERR로 통지 — 셸은 어떤 실패에도 죽지 않는다(docs/08 §4-4).
fn spawn_clean(hwnd: HWND) {
    spawn_clean_as(hwnd, "manual")
}

/// 계기를 남기며 정리 실행(X-3) — 기록에 "왜 돌았는지"가 없으면 나중에 해석할 수 없다.
fn spawn_clean_as(hwnd: HWND, trigger: &'static str) {
    if CLEAN_BUSY.get() {
        return;
    }
    CLEAN_TRIGGER.with(|t| *t.borrow_mut() = trigger);
    CLEAN_BUSY.set(true);
    let hwnd_raw = hwnd.0 as usize;
    std::thread::spawn(move || {
        let code = worker_cmd(&["clean"])
            .and_then(|mut cmd| cmd.status().ok())
            .and_then(|s| s.code())
            .map(|c| c as u32)
            .unwrap_or(mk_core::WORKER_ERR);
        unsafe {
            let _ = PostMessageW(
                Some(HWND(hwnd_raw as *mut core::ffi::c_void)),
                WM_WORKER_DONE,
                WPARAM(code as usize),
                LPARAM(0),
            );
        }
    });
}

/// 기록용 현지 시각 문자열 `YYYY-MM-DD HH:MM:SS` — 날짜 crate 없이 Win32로 얻는다(DR-5).
fn now_stamp() -> String {
    let t = unsafe { GetLocalTime() };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond
    )
}

/// 현재 프로세스 승격 여부(관리자) — 메뉴 표시 제어.
unsafe fn is_elevated() -> bool {
    let mut token = windows::Win32::Foundation::HANDLE::default();
    if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
        return false;
    }
    let mut elev = TOKEN_ELEVATION::default();
    let mut len = 0u32;
    let ok = GetTokenInformation(
        token,
        TokenElevation,
        Some(&mut elev as *mut _ as *mut core::ffi::c_void),
        std::mem::size_of::<TOKEN_ELEVATION>() as u32,
        &mut len,
    )
    .is_ok();
    let _ = windows::Win32::Foundation::CloseHandle(token);
    ok && elev.TokenIsElevated != 0
}

/// 관리자 권한으로 재시작(FR-6) — 성공 시 현재 인스턴스 종료.
unsafe fn restart_elevated(hwnd: HWND) {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let mut wide: Vec<u16> = exe.to_string_lossy().encode_utf16().collect();
    wide.push(0);
    let inst = ShellExecuteW(
        None,
        w!("runas"),
        PCWSTR(wide.as_ptr()),
        None,
        None,
        SW_SHOWNORMAL,
    );
    if inst.0 as isize > 32 {
        let _ = DestroyWindow(hwnd); // UAC 승인됨 — 새 인스턴스에 양보
    }
}

unsafe fn tray_add(hwnd: HWND) {
    let snap = read_mem();
    LAST_PERCENT.set(snap.used_percent());
    let nid = fill_nid(hwnd, &snap);
    let _ = Shell_NotifyIconW(NIM_ADD, &nid);
}

unsafe fn tray_update(hwnd: HWND) {
    let snap = read_mem();
    if snap.used_percent() == LAST_PERCENT.get() {
        return; // 변화 없음 — 아이콘 재생성·셸 호출 생략
    }
    LAST_PERCENT.set(snap.used_percent());
    let nid = fill_nid(hwnd, &snap);
    let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
}

/// 자기 워킹셋 트림(docs/08 §4-2) — 초기화가 끝난 시점에 1회. 시작 과정에서 적재된
/// 공유 DLL·초기화 페이지를 스탠바이로 반환해 상주 점유를 실사용분만 남긴다(DR-8 게이트).
pub(crate) unsafe fn self_trim() {
    let _ = K32EmptyWorkingSet(GetCurrentProcess());
}

unsafe fn drop_icon() {
    let old = TRAY_HICON.replace(HICON(std::ptr::null_mut()));
    if !old.is_invalid() {
        let _ = DestroyIcon(old);
    }
}

/// 색 단계(docs/05 §4-3): 정상 녹색 / 주의(≥80%) 주황 / 위험(≥90%) 적색. COLORREF=0x00BBGGRR.
pub(crate) fn level_color(percent: u32) -> COLORREF {
    // 트레이 색 단계는 설정된 임계값을 따른다(M4). 팝업도 같은 색을 쓴다.
    let (warn, danger) = SETTINGS.with(|c| {
        let s = c.borrow();
        (s.warn_percent, s.danger_percent)
    });
    match level_with(percent, warn, danger) {
        Level::Normal => COLORREF(0x00327D2E),
        Level::Warn => COLORREF(0x00007CF5),
        Level::Danger => COLORREF(0x002828C6),
    }
}

/// 사용률 숫자를 그린 소형 트레이 아이콘 생성(호출자가 DestroyIcon 책임).
unsafe fn make_icon(percent: u32) -> HICON {
    let cx = GetSystemMetrics(SM_CXSMICON);
    let cy = GetSystemMetrics(SM_CYSMICON);
    let screen = GetDC(None);
    let dc = CreateCompatibleDC(Some(screen));
    let bmp = CreateCompatibleBitmap(screen, cx, cy);
    let old_bmp = SelectObject(dc, bmp.into());

    let brush = CreateSolidBrush(level_color(percent));
    let rc = RECT {
        left: 0,
        top: 0,
        right: cx,
        bottom: cy,
    };
    FillRect(dc, &rc, brush);
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, COLORREF(0x00FFFFFF));
    // 3자리(100)는 축소 폰트로 수용
    let height = if percent >= 100 {
        -(cy * 5 / 8)
    } else {
        -(cy * 3 / 4)
    };
    let font = CreateFontW(
        height,
        0,
        0,
        0,
        600,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        CLEARTYPE_QUALITY,
        DEFAULT_PITCH.0 as u32,
        w!("Segoe UI"),
    );
    let old_font = SelectObject(dc, font.into());
    let mut text: Vec<u16> = percent.to_string().encode_utf16().collect();
    let mut trc = rc;
    DrawTextW(
        dc,
        &mut text,
        &mut trc,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    SelectObject(dc, old_font);
    SelectObject(dc, old_bmp);

    // 마스크 = 전부 0(불투명 사각) — 모노크롬 비트맵을 BLACKNESS로 초기화
    let mask = CreateBitmap(cx, cy, 1, 1, None);
    let mdc = CreateCompatibleDC(Some(screen));
    let old_mask = SelectObject(mdc, mask.into());
    let _ = PatBlt(mdc, 0, 0, cx, cy, BLACKNESS);
    SelectObject(mdc, old_mask);
    let _ = DeleteDC(mdc);

    let info = ICONINFO {
        fIcon: true.into(),
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: mask,
        hbmColor: bmp,
    };
    let icon = CreateIconIndirect(&info).unwrap_or_default();

    let _ = DeleteObject(font.into());
    let _ = DeleteObject(brush.into());
    let _ = DeleteObject(bmp.into());
    let _ = DeleteObject(mask.into());
    let _ = DeleteDC(dc);
    ReleaseDC(None, screen);
    icon
}

/// 트레이 우클릭 메뉴 — M1: 지금 정리·승격 재시작·종료(목록·설정은 M2~M4에서 추가).
unsafe fn show_menu(hwnd: HWND) {
    let Ok(menu) = CreatePopupMenu() else { return };
    // 머리줄 = 제품명 + 버전 — 버전 리소스와 같은 출처(build.rs `PRODUCT` + Cargo 버전).
    // 하드코딩하면 릴리스마다 어긋난다(0.7.1까지 "0.0.1" 고정이던 버그).
    let header: Vec<u16> = concat!(env!("MK_PRODUCT_NAME"), " ", env!("CARGO_PKG_VERSION"))
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let _ = AppendMenuW(menu, MF_GRAYED, 0, PCWSTR(header.as_ptr()));
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
    let clean_flags = if CLEAN_BUSY.get() {
        MF_STRING | MF_GRAYED
    } else {
        MF_STRING
    };
    let _ = AppendMenuW(menu, clean_flags, CMD_CLEAN, tw!("Clean Now(&C)"));
    let _ = AppendMenuW(menu, MF_STRING, CMD_LIST, tw!("Show List(&L)…"));
    let _ = AppendMenuW(menu, MF_STRING, CMD_SETTINGS, tw!("Settings(&S)…"));
    if !is_elevated() {
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_ELEVATE,
            tw!("Restart as Administrator(&A)…"),
        );
    }
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
    let _ = AppendMenuW(menu, MF_STRING, CMD_EXIT, tw!("Exit(&X)"));
    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    // TrackPopupMenu 규칙: 포그라운드 창이어야 메뉴 밖 클릭 시 닫힌다
    let _ = SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenu(
        menu,
        TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_RETURNCMD | TPM_NONOTIFY,
        pt.x,
        pt.y,
        None,
        hwnd,
        None,
    );
    let _ = DestroyMenu(menu);
    match cmd.0 as usize {
        CMD_EXIT => {
            let _ = DestroyWindow(hwnd);
        }
        CMD_CLEAN => spawn_clean(hwnd),
        CMD_LIST => open_list(),
        CMD_SETTINGS => open_settings(),
        CMD_ELEVATE => restart_elevated(hwnd),
        _ => {}
    }
}
