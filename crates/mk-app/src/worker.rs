//! worker — 온디맨드 정리 엔진(DR-7·M1): `mk-app --worker clean`.
//! 프로세스로 격리 실행: 크래시해도 셸 생존, 종료 = 사용 메모리 OS 전량 회수(docs/08 §3).
//! 회수 영역(docs/07 §3): ① 프로세스 워킹셋 트림 + ② 우선순위 0 스탠바이 퍼지.
//! 결과 = 종료 코드(mk-core 프로토콜: 회수 MB | PARTIAL 플래그).

use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, LUID};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_DEBUG_NAME,
    SE_INCREASE_QUOTA_NAME, SE_PRIVILEGE_ENABLED, SE_PROF_SINGLE_PROCESS_NAME,
    TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
};
use windows::Win32::System::ProcessStatus::{K32EmptyWorkingSet, K32EnumProcesses};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, OpenProcess, OpenProcessToken,
    QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SET_QUOTA,
};

// NtSetSystemInformation은 windows crate 미노출 — ntdll raw-dylib 직접 선언(docs/07 §3).
#[link(name = "ntdll", kind = "raw-dylib")]
extern "system" {
    fn NtSetSystemInformation(
        system_information_class: i32,
        system_information: *mut core::ffi::c_void,
        system_information_length: u32,
    ) -> i32; // NTSTATUS
}

const SYSTEM_MEMORY_LIST_INFORMATION: i32 = 80;
/// SYSTEM_MEMORY_LIST_COMMAND::MemoryPurgeLowPriorityStandbyList
const MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST: u32 = 5;

/// `--worker clean` 진입점 — 종료 코드 반환(셸이 decode_clean_result로 해석).
/// 진단 1줄을 stderr에 남긴다(콘솔 실행 시 QA용, GUI 기동 시 보이지 않음).
pub fn run_clean() -> u32 {
    unsafe {
        enable_privileges();
        let before = avail_bytes();
        let (trimmed, skipped) = trim_working_sets();
        // 영역 ②: SeProfileSingleProcessPrivilege 필요 — 실패 = partial(조용한 성공 금지)
        let status = purge_standby_low();
        let after = avail_bytes();
        let mb = after.saturating_sub(before) / (1024 * 1024);
        eprintln!(
            "clean: trimmed={trimmed} skipped={skipped} purge_status=0x{status:08X} \
             avail {}MB -> {}MB (+{mb}MB)",
            before / (1024 * 1024),
            after / (1024 * 1024)
        );
        mk_core::encode_clean_result(mb, status != 0)
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

/// 영역 ② — 우선순위 0 스탠바이 퍼지. 반환 = NTSTATUS(0 = 성공).
unsafe fn purge_standby_low() -> i32 {
    let mut cmd: u32 = MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST;
    NtSetSystemInformation(
        SYSTEM_MEMORY_LIST_INFORMATION,
        &mut cmd as *mut u32 as *mut core::ffi::c_void,
        std::mem::size_of::<u32>() as u32,
    )
}
