//! popup — 트레이 좌클릭 Top 5 팝업(FR-2, M2). 셸 프로세스 안에서 뜨는 소형 커스텀 드로잉 창.
//! 열려 있는 동안 [`REFRESH_MS`] 주기로 사용량 자동 갱신(기본 2000ms — M4에서 설정 연동).
//! 트림·종료 같은 **상태 변경은 워커 프로세스**에 위임한다(DR-7·15 §4).
//! 닫히면 상태·GDI 리소스를 해제하고 자기 워킹셋을 트림한다(NFR-3).

use mk_core::{MemSnapshot, ProcInfo};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateSolidBrush,
    DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, GetSysColor, InvalidateRect,
    SelectObject, SetBkMode, SetTextColor, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
    DEFAULT_PITCH, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_VCENTER,
    HBRUSH, HFONT, OUT_DEFAULT_PRECIS, PAINTSTRUCT, SRCCOPY, TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    GetCursorPos, GetSystemMetrics, GetWindowLongPtrW, KillTimer, MessageBoxW, RegisterClassW,
    SetForegroundWindow, SetTimer, SetWindowLongPtrW, ShowWindow, SystemParametersInfoW,
    TrackPopupMenu, GWLP_USERDATA, IDYES, MB_ICONWARNING, MB_YESNO, MF_STRING, SM_CXSCREEN,
    SM_CYSCREEN, SPI_GETWORKAREA, SW_SHOW, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, WA_INACTIVE, WINDOW_EX_STYLE, WM_ACTIVATE, WM_DESTROY, WM_LBUTTONUP,
    WM_MOUSEMOVE, WM_NCDESTROY, WM_PAINT, WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};

const CLASS: PCWSTR = w!("NexaMemKeeperPopup");
static REGISTER: std::sync::Once = std::sync::Once::new();

/// 목록 자동 갱신 주기(docs/09 `list_refresh_ms` 기본값 — 사용자 확정 08-07).
pub const REFRESH_MS: u32 = 2_000;
const TIMER_REFRESH: usize = 10;

// 레이아웃(96dpi 기준 px)
const W: i32 = 380;
const PAD: i32 = 10;
const HEAD_H: i32 = 52;
const ROW_H: i32 = 26;
const ROWS: i32 = 5;
const BTN_H: i32 = 30;
const H: i32 = HEAD_H + ROW_H * ROWS + 8 + BTN_H + PAD;
const BTN_Y: i32 = HEAD_H + ROW_H * ROWS + 8;
/// 버튼 2개 — 지금 정리 · 목록 보기(설정은 트레이 우클릭 메뉴 전용, 사용자 확정 08-07).
const BTN_N: i32 = 2;
const BTN_W: i32 = (W - PAD * 2 - 6 * (BTN_N - 1)) / BTN_N;

const CMD_TRIM: usize = 1;
const CMD_KILL: usize = 2;

struct PopupState {
    /// 트레이 셸 창 — 정리 요청·결과 알림 경로.
    owner: HWND,
    snap: MemSnapshot,
    rows: Vec<ProcInfo>,
    hover_row: i32,
    hover_btn: i32,
    font: HFONT,
    font_bold: HFONT,
}

/// 팝업 표시(이미 떠 있으면 무시). 커서 근처에 작업 영역 안으로 붙여 배치한다.
pub unsafe fn show(owner: HWND) -> Option<HWND> {
    REGISTER.call_once(|| {
        let wc = WNDCLASSW {
            lpszClassName: CLASS,
            lpfnWndProc: Some(popup_proc),
            ..Default::default()
        };
        RegisterClassW(&wc);
    });

    let (snap, rows) = collect();
    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let (x, y) = place(pt);

    let created = CreateWindowExW(
        WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0),
        CLASS,
        w!("Nexa MemKeeper"),
        WS_POPUP,
        x,
        y,
        W,
        H,
        Some(owner),
        None,
        None,
        None,
    );
    let hwnd = match created {
        Ok(h) => h,
        Err(e) => {
            eprintln!("popup: CreateWindowExW 실패 — {e}");
            return None;
        }
    };

    let st = Box::new(PopupState {
        owner,
        snap,
        rows,
        hover_row: -1,
        hover_btn: -1,
        font: make_font(-13, 400),
        font_bold: make_font(-14, 600),
    });
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(st) as isize);
    SetTimer(Some(hwnd), TIMER_REFRESH, REFRESH_MS, None);
    let _ = ShowWindow(hwnd, SW_SHOW);
    // 포그라운드여야 다른 곳 클릭 시 WM_ACTIVATE(WA_INACTIVE)로 닫힌다
    let _ = SetForegroundWindow(hwnd);
    Some(hwnd)
}

/// 현재 스냅샷 + Top N 목록 수집(읽기 전용 — procs::enumerate 1콜).
fn collect() -> (MemSnapshot, Vec<ProcInfo>) {
    let snap = crate::shell::read_mem();
    let rows = mk_core::top_n(crate::procs::enumerate(), ROWS as usize);
    (snap, rows)
}

/// 커서 기준 배치 — 화면(작업 영역) 밖으로 나가지 않도록 클램프.
unsafe fn place(pt: POINT) -> (i32, i32) {
    let mut wa = RECT::default();
    let ok = SystemParametersInfoW(
        SPI_GETWORKAREA,
        0,
        Some(&mut wa as *mut RECT as *mut core::ffi::c_void),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
    )
    .is_ok();
    if !ok {
        wa = RECT {
            left: 0,
            top: 0,
            right: GetSystemMetrics(SM_CXSCREEN),
            bottom: GetSystemMetrics(SM_CYSCREEN),
        };
    }
    let x = (pt.x - W / 2).clamp(wa.left, (wa.right - W).max(wa.left));
    // 트레이는 보통 하단 — 커서 위쪽에 띄우되 위로 넘치면 아래로
    let y = if pt.y - H - 8 >= wa.top {
        pt.y - H - 8
    } else {
        (pt.y + 8).min(wa.bottom - H)
    };
    (x, y)
}

unsafe fn make_font(height: i32, weight: i32) -> HFONT {
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
        w!("Segoe UI"),
    )
}

unsafe extern "system" fn popup_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let st = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PopupState;
    match msg {
        WM_PAINT => {
            if let Some(st) = st.as_mut() {
                paint(hwnd, st);
            }
            LRESULT(0)
        }
        WM_TIMER => {
            if let Some(st) = st.as_mut() {
                let (snap, rows) = collect();
                st.snap = snap;
                st.rows = rows;
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if let Some(st) = st.as_mut() {
                let (x, y) = (lo_i16(lp), hi_i16(lp));
                let (r, b) = (hit_row(y), hit_btn(x, y));
                if r != st.hover_row || b != st.hover_btn {
                    st.hover_row = r;
                    st.hover_btn = b;
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            if let Some(st) = st.as_mut() {
                let owner = st.owner;
                match hit_btn(lo_i16(lp), hi_i16(lp)) {
                    0 => {
                        let _ = DestroyWindow(hwnd);
                        crate::shell::request_clean(owner);
                    }
                    1 => {
                        let _ = DestroyWindow(hwnd);
                        crate::shell::open_list();
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            if let Some(st) = st.as_mut() {
                let row = hit_row(hi_i16(lp));
                if row >= 0 {
                    if let Some(p) = st.rows.get(row as usize) {
                        item_menu(hwnd, st.owner, p.pid, &p.name);
                    }
                }
            }
            LRESULT(0)
        }
        WM_ACTIVATE => {
            let state = (wp.0 & 0xFFFF) as u32;
            if cfg!(debug_assertions) {
                eprintln!("popup: WM_ACTIVATE state={state}");
            }
            if state == WA_INACTIVE {
                let _ = DestroyWindow(hwnd); // 포커스 잃으면 닫힘(트레이 팝업 관례)
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let _ = KillTimer(Some(hwnd), TIMER_REFRESH);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            if !st.is_null() {
                let st = Box::from_raw(st);
                let _ = DeleteObject(st.font.into());
                let _ = DeleteObject(st.font_bold.into());
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            crate::shell::self_trim(); // 기능 종료 = 점유 반납(NFR-3)
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn lo_i16(lp: LPARAM) -> i32 {
    (lp.0 & 0xFFFF) as i16 as i32
}
fn hi_i16(lp: LPARAM) -> i32 {
    ((lp.0 >> 16) & 0xFFFF) as i16 as i32
}

/// y좌표 → 행 인덱스(없으면 -1).
fn hit_row(y: i32) -> i32 {
    if !(HEAD_H..HEAD_H + ROW_H * ROWS).contains(&y) {
        return -1;
    }
    (y - HEAD_H) / ROW_H
}

/// (x,y) → 버튼 인덱스(없으면 -1).
fn hit_btn(x: i32, y: i32) -> i32 {
    if !(BTN_Y..BTN_Y + BTN_H).contains(&y) {
        return -1;
    }
    for i in 0..BTN_N {
        let bx = PAD + i * (BTN_W + 6);
        if x >= bx && x < bx + BTN_W {
            return i;
        }
    }
    -1
}

unsafe fn sys(idx: windows::Win32::Graphics::Gdi::SYS_COLOR_INDEX) -> COLORREF {
    COLORREF(GetSysColor(idx))
}

unsafe fn fill(dc: windows::Win32::Graphics::Gdi::HDC, rc: &RECT, color: COLORREF) {
    let br: HBRUSH = CreateSolidBrush(color);
    FillRect(dc, rc, br);
    let _ = DeleteObject(br.into());
}

unsafe fn text(
    dc: windows::Win32::Graphics::Gdi::HDC,
    s: &str,
    rc: RECT,
    flags: windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT,
    color: COLORREF,
) {
    let mut buf: Vec<u16> = s.encode_utf16().collect();
    if buf.is_empty() {
        return;
    }
    let mut rc = rc;
    SetTextColor(dc, color);
    DrawTextW(dc, &mut buf, &mut rc, flags | DT_SINGLELINE | DT_NOPREFIX);
}

/// 더블 버퍼 페인트 — 2초 갱신 시 깜빡임 방지.
unsafe fn paint(hwnd: HWND, st: &PopupState) {
    use windows::Win32::Graphics::Gdi::{
        COLOR_BTNFACE, COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_WINDOW,
        COLOR_WINDOWTEXT,
    };

    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    let mem = CreateCompatibleDC(Some(hdc));
    let bmp = CreateCompatibleBitmap(hdc, W, H);
    let old_bmp = SelectObject(mem, bmp.into());
    SetBkMode(mem, TRANSPARENT);

    let bg = sys(COLOR_WINDOW);
    let fg = sys(COLOR_WINDOWTEXT);
    let gray = sys(COLOR_GRAYTEXT);
    let face = sys(COLOR_BTNFACE);
    let hi = sys(COLOR_HIGHLIGHT);
    let hi_text = sys(COLOR_HIGHLIGHTTEXT);
    let full = RECT {
        left: 0,
        top: 0,
        right: W,
        bottom: H,
    };
    fill(mem, &full, bg);
    // 테두리
    fill(
        mem,
        &RECT {
            left: 0,
            top: 0,
            right: W,
            bottom: 1,
        },
        gray,
    );
    fill(
        mem,
        &RECT {
            left: 0,
            top: H - 1,
            right: W,
            bottom: H,
        },
        gray,
    );
    fill(
        mem,
        &RECT {
            left: 0,
            top: 0,
            right: 1,
            bottom: H,
        },
        gray,
    );
    fill(
        mem,
        &RECT {
            left: W - 1,
            top: 0,
            right: W,
            bottom: H,
        },
        gray,
    );

    // 헤더: 요약 + 사용률 바
    let old_font = SelectObject(mem, st.font_bold.into());
    let pct = st.snap.used_percent();
    text(
        mem,
        &format!(
            "메모리 {pct}% 사용 · {} / {}",
            mk_core::fmt_size(st.snap.used()),
            mk_core::fmt_size(st.snap.total)
        ),
        RECT {
            left: PAD,
            top: 8,
            right: W - PAD,
            bottom: 30,
        },
        DT_LEFT | DT_VCENTER,
        fg,
    );
    let bar = RECT {
        left: PAD,
        top: 34,
        right: W - PAD,
        bottom: 42,
    };
    fill(mem, &bar, face);
    let filled = PAD + ((W - PAD * 2) as i64 * pct as i64 / 100) as i32;
    fill(
        mem,
        &RECT {
            left: PAD,
            top: 34,
            right: filled,
            bottom: 42,
        },
        crate::shell::level_color(pct),
    );

    // 행: 이름 + 프라이빗 워킹셋
    SelectObject(mem, st.font.into());
    for i in 0..ROWS {
        let top = HEAD_H + i * ROW_H;
        let rc = RECT {
            left: 1,
            top,
            right: W - 1,
            bottom: top + ROW_H,
        };
        let hovered = st.hover_row == i;
        if hovered {
            fill(mem, &rc, hi);
        }
        let (tc, sc) = if hovered {
            (hi_text, hi_text)
        } else {
            (fg, gray)
        };
        match st.rows.get(i as usize) {
            Some(p) => {
                text(
                    mem,
                    &format!("{}  ({})", p.name, p.pid),
                    RECT {
                        left: PAD,
                        top,
                        right: W - 110,
                        bottom: top + ROW_H,
                    },
                    DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
                    tc,
                );
                text(
                    mem,
                    &mk_core::fmt_size(p.private_ws),
                    RECT {
                        left: W - 110,
                        top,
                        right: W - PAD,
                        bottom: top + ROW_H,
                    },
                    DT_RIGHT | DT_VCENTER,
                    sc,
                );
            }
            None => text(
                mem,
                "—",
                RECT {
                    left: PAD,
                    top,
                    right: W - PAD,
                    bottom: top + ROW_H,
                },
                DT_LEFT | DT_VCENTER,
                gray,
            ),
        }
    }

    // 버튼 2개 — 설정은 트레이 우클릭 메뉴 전용(사용자 확정 08-07)
    let labels = ["지금 정리", "목록 보기…"];
    for (i, label) in labels.iter().enumerate() {
        let i = i as i32;
        let bx = PAD + i * (BTN_W + 6);
        let rc = RECT {
            left: bx,
            top: BTN_Y,
            right: bx + BTN_W,
            bottom: BTN_Y + BTN_H,
        };
        fill(mem, &rc, if st.hover_btn == i { hi } else { face });
        let color = if st.hover_btn == i { hi_text } else { fg };
        text(
            mem,
            label,
            rc,
            windows::Win32::Graphics::Gdi::DT_CENTER | DT_VCENTER,
            color,
        );
    }

    SelectObject(mem, old_font);
    let _ = BitBlt(hdc, 0, 0, W, H, Some(mem), 0, 0, SRCCOPY);
    SelectObject(mem, old_bmp);
    let _ = DeleteObject(bmp.into());
    let _ = DeleteDC(mem);
    let _ = EndPaint(hwnd, &ps);
}

/// 항목 우클릭 메뉴 — 트림/종료(FR-2). 실행은 워커 프로세스에 위임.
unsafe fn item_menu(hwnd: HWND, owner: HWND, pid: u32, name: &str) {
    if mk_core::is_protected(name) {
        return; // 보호 프로세스 — 메뉴 자체를 열지 않는다(15 §4)
    }
    let Ok(menu) = CreatePopupMenu() else { return };
    let _ = AppendMenuW(menu, MF_STRING, CMD_TRIM, w!("이 프로세스 메모리 회수(&T)"));
    let _ = AppendMenuW(menu, MF_STRING, CMD_KILL, w!("프로세스 종료(&K)…"));
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
    match cmd.0 as usize {
        CMD_TRIM => crate::shell::request_worker(owner, "trim", pid),
        CMD_KILL => {
            let msg: Vec<u16> = format!(
                "{name} (PID {pid})을(를) 종료할까요?\n저장하지 않은 작업이 손실될 수 있습니다."
            )
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
            let r = MessageBoxW(
                Some(hwnd),
                PCWSTR(msg.as_ptr()),
                w!("프로세스 종료 확인"),
                MB_YESNO | MB_ICONWARNING,
            );
            if r == IDYES {
                crate::shell::request_worker(owner, "kill", pid);
            }
        }
        _ => {}
    }
}
