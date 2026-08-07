//! shell — 상주 셸(DR-7): 트레이 아이콘(사용률 %·색 단계)·툴팁·우클릭 메뉴·5초 폴링. (M0-1)
//! 상주 최소 원칙(docs/08 §4-1): 숨김 호스트 창 + 타이머 + 트레이 외 아무것도 초기화하지 않는다.
//! 창 등록·wndproc 관용구는 nexa-dir2 crates/nexa-app/src/tip.rs 패턴 계승.

use std::cell::Cell;

use mk_core::{level, Level, MemSnapshot};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateSolidBrush,
    DeleteDC, DeleteObject, DrawTextW, FillRect, GetDC, PatBlt, ReleaseDC, SelectObject, SetBkMode,
    SetTextColor, BLACKNESS, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
    DEFAULT_PITCH, DT_CENTER, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, OUT_DEFAULT_PRECIS,
    TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::ProcessStatus::K32EmptyWorkingSet;
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon,
    DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW, GetSystemMetrics,
    KillTimer, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow,
    SetTimer, TrackPopupMenu, HICON, ICONINFO, MF_GRAYED, MF_SEPARATOR, MF_STRING, MSG,
    SM_CXSMICON, SM_CYSMICON, TPM_BOTTOMALIGN, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    WINDOW_EX_STYLE, WM_APP, WM_CONTEXTMENU, WM_CREATE, WM_DESTROY, WM_RBUTTONUP, WM_TIMER,
    WNDCLASSW, WS_OVERLAPPED,
};

const CLASS: PCWSTR = w!("NexaMemKeeperShell");
/// 트레이 콜백 메시지(창 전용 사용자 메시지).
const WM_TRAY: u32 = WM_APP + 1;
const TRAY_ID: u32 = 1;
const TIMER_POLL: usize = 1;
/// 관찰 폴링 주기(docs/05 §4-3 기본값 5초 — M4에서 설정으로 이동).
const POLL_MS: u32 = 5_000;
const CMD_EXIT: usize = 100;

thread_local! {
    /// 직전 표시 %(같으면 아이콘 재생성 생략 — 유휴 CPU ~0% 게이트).
    static LAST_PERCENT: Cell<u32> = const { Cell::new(u32::MAX) };
    /// 현재 트레이 아이콘(교체 시 이전 것 파괴).
    static TRAY_HICON: Cell<HICON> = const { Cell::new(HICON(std::ptr::null_mut())) };
    /// explorer 재시작 감지용 "TaskbarCreated" 등록 메시지.
    static TASKBAR_MSG: Cell<u32> = const { Cell::new(0) };
}

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
            SetTimer(Some(hwnd), TIMER_POLL, POLL_MS, None);
            tray_add(hwnd);
            self_trim();
            LRESULT(0)
        }
        WM_TIMER => {
            tray_update(hwnd);
            LRESULT(0)
        }
        WM_TRAY => {
            let event = lp.0 as u32;
            if event == WM_RBUTTONUP || event == WM_CONTEXTMENU {
                show_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let _ = KillTimer(Some(hwnd), TIMER_POLL);
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
fn read_mem() -> MemSnapshot {
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
    let tip: Vec<u16> = mk_core::tray_tooltip(snap).encode_utf16().collect();
    let n = tip.len().min(nid.szTip.len() - 1);
    nid.szTip[..n].copy_from_slice(&tip[..n]);
    nid
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
unsafe fn self_trim() {
    let _ = K32EmptyWorkingSet(GetCurrentProcess());
}

unsafe fn drop_icon() {
    let old = TRAY_HICON.replace(HICON(std::ptr::null_mut()));
    if !old.is_invalid() {
        let _ = DestroyIcon(old);
    }
}

/// 색 단계(docs/05 §4-3): 정상 녹색 / 주의(≥80%) 주황 / 위험(≥90%) 적색. COLORREF=0x00BBGGRR.
fn level_color(percent: u32) -> COLORREF {
    match level(percent) {
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

/// 트레이 우클릭 메뉴 — M0 범위: 종료만(정리·목록·설정은 M1~M4에서 추가).
unsafe fn show_menu(hwnd: HWND) {
    let Ok(menu) = CreatePopupMenu() else { return };
    let _ = AppendMenuW(menu, MF_GRAYED, 0, w!("Nexa MemKeeper 0.0.1"));
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
    let _ = AppendMenuW(menu, MF_STRING, CMD_EXIT, w!("종료(&X)"));
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
    if cmd.0 as usize == CMD_EXIT {
        let _ = DestroyWindow(hwnd);
    }
}
