//! worker — 온디맨드 정리 엔진(DR-7·M1): `mk-app --worker clean`.
//! 프로세스로 격리 실행: 크래시해도 셸 생존, 종료 = 사용 메모리 OS 전량 회수(docs/08 §3).
//! 회수 영역(docs/07 §3): ① 프로세스 워킹셋 트림 + ② 우선순위 0 스탠바이 퍼지.
//! 결과 = 종료 코드(mk-core 프로토콜: 회수 MB | PARTIAL 플래그).

use windows::core::BOOL;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, LUID, WAIT_OBJECT_0, WPARAM};
use windows::Win32::System::Threading::PROCESS_ACCESS_RIGHTS;

/// SYNCHRONIZE(0x0010_0000) — 프로세스 종료 대기용. windows crate는 이 상수를
/// FILE_ACCESS_RIGHTS로만 노출해 타입이 맞지 않으므로 프로세스 권한 타입으로 직접 표기.
const SYNCHRONIZE: PROCESS_ACCESS_RIGHTS = PROCESS_ACCESS_RIGHTS(0x0010_0000);
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_DEBUG_NAME,
    SE_INCREASE_QUOTA_NAME, SE_PRIVILEGE_ENABLED, SE_PROF_SINGLE_PROCESS_NAME,
    TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
};
use windows::Win32::System::ProcessStatus::{K32EmptyWorkingSet, K32EnumProcesses};
use windows::Win32::System::SystemInformation::{
    GetSystemInfo, GlobalMemoryStatusEx, MEMORYSTATUSEX, SYSTEM_INFO,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, OpenProcess, OpenProcessToken, OpenThread,
    QueryFullProcessImageNameW, TerminateProcess, TerminateThread, WaitForSingleObject,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
    THREAD_TERMINATE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, PostMessageW, WM_CLOSE,
};

// NtSetSystemInformation은 windows crate 미노출 — ntdll raw-dylib 직접 선언(docs/07 §3).
#[link(name = "ntdll", kind = "raw-dylib")]
extern "system" {
    fn NtQuerySystemInformation(
        system_information_class: i32,
        system_information: *mut core::ffi::c_void,
        system_information_length: u32,
        return_length: *mut u32,
    ) -> i32; // NTSTATUS
    fn NtSetSystemInformation(
        system_information_class: i32,
        system_information: *mut core::ffi::c_void,
        system_information_length: u32,
    ) -> i32; // NTSTATUS
}

const SYSTEM_MEMORY_LIST_INFORMATION: i32 = 80;
// SYSTEM_MEMORY_LIST_COMMAND (docs/07 §4-2)
const MEMORY_EMPTY_WORKING_SETS: u32 = 2; // ⑤ 시스템 워킹셋
const MEMORY_FLUSH_MODIFIED_LIST: u32 = 3; // ④ 수정 페이지 플러시
const MEMORY_PURGE_STANDBY_LIST: u32 = 4; // ③ 전체 스탠바이 퍼지
const MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST: u32 = 5; // ② 우선순위 0 퍼지
/// ⑥ 페이지 결합 — `SystemCombinePhysicalMemoryInformation`(docs/07 §3). RAMMap의 "Combine memory lists"와 같은 경로.
const SYSTEM_COMBINE_PHYSICAL_MEMORY_INFORMATION: i32 = 130;

/// `MEMORY_COMBINE_INFORMATION_EX` — 커널이 결합한 **페이지 수**를 돌려준다.
/// x64에서 24바이트(8 + 8 + 4 + 정렬 4). 크기가 틀리면 `STATUS_INFO_LENGTH_MISMATCH`가 난다.
#[repr(C)]
#[derive(Default)]
struct MemoryCombineInformationEx {
    handle: usize,
    pages_combined: usize,
    flags: u32,
    _pad: u32,
}
const _: () = assert!(std::mem::size_of::<MemoryCombineInformationEx>() == 24);

/// `--worker clean` 진입점 — 종료 코드 반환(셸이 decode_clean_result로 해석).
/// 진단 1줄을 stderr에 남긴다(콘솔 실행 시 QA용, GUI 기동 시 보이지 않음).
pub fn run_clean() -> u32 {
    unsafe {
        // 워커는 설정을 **읽기만** 한다(DR-7). 켜진 영역만 수행하고, 권한 부족으로 건너뛴 영역이
        // 하나라도 있으면 partial로 보고한다(조용한 성공 금지 — docs/07 §5).
        let s = crate::config::load();
        enable_privileges();
        let before = avail_bytes();
        let active_before = active_bytes();

        let (mut trimmed, mut skipped) = (0, 0);
        if s.areas.working_sets {
            let r = trim_working_sets(); // ①
            trimmed = r.0;
            skipped = r.1;
        }
        let mut partial = false;
        let mut log = String::new();
        for (on, cmd, name) in [
            (
                s.areas.standby_low,
                MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST,
                "standby_low",
            ),
            (
                s.areas.standby_all,
                MEMORY_PURGE_STANDBY_LIST,
                "standby_all",
            ),
            (s.areas.modified, MEMORY_FLUSH_MODIFIED_LIST, "modified"),
            (s.areas.system_ws, MEMORY_EMPTY_WORKING_SETS, "system_ws"),
        ] {
            if !on {
                continue;
            }
            let status = memory_list_command(cmd);
            if status != 0 {
                partial = true;
            }
            log.push_str(&format!(" {name}=0x{status:08X}"));
        }
        if s.areas.combine {
            // ⑥ 페이지 결합 — 관리자 권한이 필요하고 수 초가 걸릴 수 있다(그래서 기본 off).
            // 워커에서 돌리므로 셸 UI는 멈추지 않는다(DR-7).
            let (status, pages) = combine_physical_memory();
            if status != 0 {
                partial = true;
            }
            log.push_str(&format!(
                " combine=0x{status:08X}({}MB)",
                pages * 4096 / (1024 * 1024)
            ));
        }

        let after = avail_bytes();
        let active_after = active_bytes();
        let mb = after.saturating_sub(before) / (1024 * 1024);
        // Active는 **줄어드는** 방향이 회수다(Avail과 부호가 반대). 측정 실패(0)면 병기하지 않는다.
        let active_mb = if active_before == 0 || active_after == 0 {
            0
        } else {
            active_before.saturating_sub(active_after) / (1024 * 1024)
        };
        eprintln!(
            "clean: trimmed={trimmed} skipped={skipped}{log} avail {}MB -> {}MB (+{mb}MB) active -{active_mb}MB",
            before / (1024 * 1024),
            after / (1024 * 1024)
        );
        mk_core::encode_clean_result(mb, active_mb, partial)
    }
}

/// 회수에 필요한 특권 활성화(가능한 것만 — docs/07 §4). 실패해도 계속(개별 영역에서 드러남).
unsafe fn enable_privileges() {
    let mut token = HANDLE::default();
    if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES, &mut token).is_err() {
        return;
    }
    for name in [
        SE_DEBUG_NAME,
        SE_PROF_SINGLE_PROCESS_NAME,
        SE_INCREASE_QUOTA_NAME,
    ] {
        let mut luid = LUID::default();
        if LookupPrivilegeValueW(None, name, &mut luid).is_ok() {
            let tp = TOKEN_PRIVILEGES {
                PrivilegeCount: 1,
                Privileges: [LUID_AND_ATTRIBUTES {
                    Luid: luid,
                    Attributes: SE_PRIVILEGE_ENABLED,
                }],
            };
            let _ = AdjustTokenPrivileges(token, false, Some(&tp), 0, None, None);
        }
    }
    let _ = CloseHandle(token);
}

fn avail_bytes() -> u64 {
    let mut ms = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    if unsafe { GlobalMemoryStatusEx(&mut ms) }.is_err() {
        return 0;
    }
    ms.ullAvailPhys
}

/// **Active 바이트** = `Total − Available − Modified`(docs/21 §2-2).
///
/// 앱이 실제로 쥐고 있는 물리 메모리다. 트림은 페이지를 Active에서 내려놓지만, 더티 페이지는
/// Modified 목록으로 가서 **Available에는 아직 안 잡힌다** — 그래서 Avail 차이만 보면 실제
/// 일어난 일의 1/5도 설명하지 못한다(E-1). Modified를 못 읽으면 0을 돌려 호출부가 병기를 건너뛴다.
fn active_bytes() -> u64 {
    let mut ms = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    if unsafe { GlobalMemoryStatusEx(&mut ms) }.is_err() {
        return 0;
    }
    let Some(modified) = modified_bytes() else {
        return 0;
    };
    ms.ullTotalPhys
        .saturating_sub(ms.ullAvailPhys)
        .saturating_sub(modified)
}

/// Modified 페이지 목록의 바이트 수 — `SystemMemoryListInformation` **질의**(설정과 같은 클래스).
///
/// 구조체 앞부분은 `Zero / Free / Modified / ModifiedNoWrite / Bad` 순의 `ULONG_PTR` 배열이라
/// **세 번째 값**이 Modified다. 뒤쪽(우선순위별 배열)은 쓰지 않으므로 넉넉한 버퍼로 받고 앞만 읽는다.
fn modified_bytes() -> Option<u64> {
    const SYSTEM_MEMORY_LIST_INFORMATION: i32 = 80;
    let mut buf = [0usize; 32];
    let mut len = 0u32;
    let status = unsafe {
        NtQuerySystemInformation(
            SYSTEM_MEMORY_LIST_INFORMATION,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            std::mem::size_of_val(&buf) as u32,
            &mut len,
        )
    };
    if status != 0 {
        return None;
    }
    let mut si = SYSTEM_INFO::default();
    unsafe { GetSystemInfo(&mut si) };
    let page = si.dwPageSize.max(4096) as u64;
    Some(buf[2] as u64 * page)
}

/// 영역 ① — 전 프로세스 워킹셋 트림. 선정 파이프라인(docs/19 §3-1):
/// PID 0/4·자기 자신 스킵 → 열기 실패(PPL 등) 스킵 → 보호 목록(mk-core) 스킵 → 트림.
/// 반환: (트림 성공 수, 제외·실패 수).
unsafe fn trim_working_sets() -> (u32, u32) {
    let mut pids = vec![0u32; 4096];
    let mut cb = 0u32;
    if K32EnumProcesses(pids.as_mut_ptr(), (pids.len() * 4) as u32, &mut cb).0 == 0 {
        return (0, 0);
    }
    let count = (cb as usize) / 4;
    let me = GetCurrentProcessId();
    let (mut trimmed, mut skipped) = (0u32, 0u32);
    for &pid in &pids[..count] {
        if pid == 0 || pid == 4 || pid == me {
            skipped += 1;
            continue;
        }
        let Ok(h) = OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA,
            false,
            pid,
        ) else {
            skipped += 1; // 보호(PPL)·타 세션 등 — 접근 불가 = 대상 아님
            continue;
        };
        if !is_protected_image(h) && K32EmptyWorkingSet(h).as_bool() {
            trimmed += 1;
        } else {
            skipped += 1;
        }
        let _ = CloseHandle(h);
    }
    (trimmed, skipped)
}

/// 이미지 이름을 얻어 보호 목록 판정. 이름 조회 실패 시 보수적으로 보호 취급(true).
unsafe fn is_protected_image(h: HANDLE) -> bool {
    let mut buf = [0u16; 512];
    let mut len = buf.len() as u32;
    if QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_err()
    {
        return true;
    }
    let path = String::from_utf16_lossy(&buf[..len as usize]);
    let name = path.rsplit('\\').next().unwrap_or(&path);
    mk_core::is_protected(name)
}

/// `--worker trim <pid>` — 단일 프로세스 워킹셋 회수(FR-2). 성공 0 / 실패 1.
pub fn run_trim(pid: u32) -> u32 {
    unsafe {
        enable_privileges();
        let Ok(h) = OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA,
            false,
            pid,
        ) else {
            return 1;
        };
        let ok = !is_protected_image(h) && K32EmptyWorkingSet(h).as_bool();
        let _ = CloseHandle(h);
        u32::from(!ok)
    }
}

/// `--worker kill <pid>` — 2단계 종료(FR-3): WM_CLOSE 요청 → 타임아웃 후 TerminateProcess.
pub fn run_kill(pid: u32) -> u32 {
    unsafe {
        enable_privileges();
        let Ok(h) = OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | SYNCHRONIZE,
            false,
            pid,
        ) else {
            return 1;
        };
        if is_protected_image(h) {
            let _ = CloseHandle(h); // 보호 대상 — 진입 차단(15 §4)
            return 1;
        }
        post_close_to_windows(pid);
        // 정상 종료 대기(기본 3초) — 저장 기회를 준 뒤에만 강제 종료
        let waited = WaitForSingleObject(h, GRACEFUL_WAIT_MS);
        let ok = if waited == WAIT_OBJECT_0 {
            true
        } else {
            TerminateProcess(h, 1).is_ok() && WaitForSingleObject(h, 2_000) == WAIT_OBJECT_0
        };
        let _ = CloseHandle(h);
        u32::from(!ok)
    }
}

/// `--worker killtree <pid>` — 프로세스 트리 종료(FR-3). 자식 → 부모 순으로 종료해
/// 부모가 죽는 사이 자식이 다시 뜨는 것을 막는다. 하나라도 실패하면 1.
pub fn run_kill_tree(pid: u32) -> u32 {
    let procs = crate::procs::enumerate();
    let mut targets = mk_core::descendants(&procs, pid);
    // 깊은 자손이 뒤에 쌓이므로 역순 = 잎부터
    targets.reverse();
    targets.push(pid);
    let mut failed = 0;
    for t in targets {
        if run_kill(t) != 0 {
            failed += 1;
        }
    }
    u32::from(failed > 0)
}

/// `--worker killthread <tid>` — 단일 쓰레드 종료(FR-3). 성공 0 / 실패 1.
/// ⚠️ 프로세스 상태를 깨뜨릴 수 있는 조작이라 UI에서 확인을 받은 뒤에만 호출한다.
pub fn run_kill_thread(tid: u32) -> u32 {
    unsafe {
        enable_privileges();
        let Ok(h) = OpenThread(THREAD_TERMINATE, false, tid) else {
            return 1;
        };
        let ok = TerminateThread(h, 1).is_ok();
        let _ = CloseHandle(h);
        u32::from(!ok)
    }
}

const GRACEFUL_WAIT_MS: u32 = 3_000;

/// 대상 PID의 최상위 창에 WM_CLOSE 게시(정상 종료 요청).
unsafe fn post_close_to_windows(pid: u32) {
    let _ = EnumWindows(Some(close_proc), LPARAM(pid as isize));
}

unsafe extern "system" fn close_proc(hwnd: HWND, lp: LPARAM) -> BOOL {
    let mut owner = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut owner));
    if owner == lp.0 as u32 {
        let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
    }
    true.into()
}

/// 영역 ⑥ — 내용이 같은 물리 페이지를 하나로 합친다. 반환 = (NTSTATUS, 합쳐진 페이지 수).
///
/// 일반 권한으로는 `STATUS_PRIVILEGE_NOT_HELD(0xC0000061)`로 실패한다(승격 필요 — docs/07 §4).
/// 효과는 완만하고 소요가 길어 **기본 off**이며, 켜져 있어도 실패하면 partial로 보고한다(조용한 성공 금지).
unsafe fn combine_physical_memory() -> (i32, usize) {
    let mut info = MemoryCombineInformationEx::default();
    let status = NtSetSystemInformation(
        SYSTEM_COMBINE_PHYSICAL_MEMORY_INFORMATION,
        &mut info as *mut _ as *mut core::ffi::c_void,
        std::mem::size_of::<MemoryCombineInformationEx>() as u32,
    );
    (status, if status == 0 { info.pages_combined } else { 0 })
}

/// 영역 ②③④⑤ 공통 — `SystemMemoryListInformation` 명령 1개 실행. 반환 = NTSTATUS(0 = 성공).
unsafe fn memory_list_command(command: u32) -> i32 {
    let mut cmd: u32 = command;
    NtSetSystemInformation(
        SYSTEM_MEMORY_LIST_INFORMATION,
        &mut cmd as *mut u32 as *mut core::ffi::c_void,
        std::mem::size_of::<u32>() as u32,
    )
}
