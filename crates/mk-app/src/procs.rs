//! procs — 프로세스 열거(읽기 전용). `NtQuerySystemInformation` **1콜**로 전 프로세스의
//! 이름·PID·워킹셋·프라이빗 워킹셋·쓰레드 수를 얻는다(docs/07 §2 — 프로세스별 OpenProcess 불필요).
//!
//! 셸에서 호출 가능한 이유: 부작용 없는 조회이고 팝업 즉시성(NFR-4 <100ms)에 워커 spawn을
//! 낄 수 없다([15 §4 정정](../../../docs/15-dev-methodology.md)). 대신 **버퍼 워킹은 경계 검사 필수** —
//! 매 항목에서 오프셋·구조체 크기·문자열 길이를 검증하고, 어긋나면 그 지점에서 중단한다.

use mk_core::{ProcInfo, ThreadInfo};
use windows::Win32::System::WindowsProgramming::SYSTEM_PROCESS_INFORMATION;

/// 각 프로세스 항목 뒤에 `NumberOfThreads`개가 이어지는 쓰레드 정보(x64 80바이트).
/// windows crate 미노출 — NT 문서 레이아웃대로 선언하고 크기를 컴파일 타임에 고정한다.
#[repr(C)]
struct SystemThreadInformation {
    kernel_time: i64,
    user_time: i64,
    create_time: i64,
    wait_time: u32,
    _pad: u32,
    start_address: *mut core::ffi::c_void,
    unique_process: isize,
    unique_thread: isize,
    priority: i32,
    base_priority: i32,
    context_switches: u32,
    thread_state: u32,
    wait_reason: u32,
    _pad2: u32,
}
const _: () = assert!(std::mem::size_of::<SystemThreadInformation>() == 80);
/// 프로세스 항목 헤더 크기 = 쓰레드 배열 시작 오프셋(x64 256바이트).
const _: () = assert!(std::mem::size_of::<SYSTEM_PROCESS_INFORMATION>() == 256);

#[link(name = "ntdll", kind = "raw-dylib")]
extern "system" {
    fn NtQuerySystemInformation(
        system_information_class: i32,
        system_information: *mut core::ffi::c_void,
        system_information_length: u32,
        return_length: *mut u32,
    ) -> i32; // NTSTATUS
}

/// SYSTEM_INFORMATION_CLASS::SystemProcessInformation
const SYSTEM_PROCESS_INFORMATION_CLASS: i32 = 5;
const STATUS_INFO_LENGTH_MISMATCH: i32 = 0xC0000004u32 as i32;
/// crate 정의 기준 `WorkingSetPrivateSize`(LARGE_INTEGER)는 Reserved1의 선두 8바이트.
const PRIVATE_WS_OFFSET_IN_RESERVED1: usize = 0;

/// 전 프로세스 열거. 실패 시 빈 Vec(호출자는 이전 목록 유지).
pub fn enumerate() -> Vec<ProcInfo> {
    let Some(buf) = query_buffer() else {
        return Vec::new();
    };
    parse(&buf)
}

/// 버퍼 확보 — 프로세스 수가 변해 크기가 어긋나면(STATUS_INFO_LENGTH_MISMATCH) 확장 재시도.
fn query_buffer() -> Option<Vec<u8>> {
    let mut cap = 512 * 1024; // 일반적 시스템에서 1콜로 충분한 초기값
    for _ in 0..6 {
        let mut buf = vec![0u8; cap];
        let mut need = 0u32;
        let status = unsafe {
            NtQuerySystemInformation(
                SYSTEM_PROCESS_INFORMATION_CLASS,
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                cap as u32,
                &mut need,
            )
        };
        if status == 0 {
            return Some(buf);
        }
        if status != STATUS_INFO_LENGTH_MISMATCH {
            return None;
        }
        // 재시도 사이에 프로세스가 더 생길 수 있으므로 여유를 둔다
        cap = (need as usize).max(cap * 2) + 64 * 1024;
    }
    None
}

/// 버퍼 워킹 — 경계 검사(15 §4): 남은 길이 < 구조체 크기이거나 오프셋이 전진하지 않으면 중단.
fn parse(buf: &[u8]) -> Vec<ProcInfo> {
    const ENTRY: usize = std::mem::size_of::<SYSTEM_PROCESS_INFORMATION>();
    let mut out = Vec::with_capacity(512);
    let mut off = 0usize;
    loop {
        if off > buf.len() || buf.len() - off < ENTRY {
            break;
        }
        // SAFETY: 위에서 off..off+ENTRY가 buf 안에 있음을 확인했고, 정렬은 커널이 8바이트로 보장.
        let p = unsafe { &*(buf.as_ptr().add(off) as *const SYSTEM_PROCESS_INFORMATION) };

        let pid = p.UniqueProcessId.0 as usize as u32;
        let private_ws = u64::from_ne_bytes(
            p.Reserved1[PRIVATE_WS_OFFSET_IN_RESERVED1..PRIVATE_WS_OFFSET_IN_RESERVED1 + 8]
                .try_into()
                .unwrap_or([0; 8]),
        );
        let name = image_name(p, pid);
        out.push(ProcInfo {
            pid,
            // Reserved2 = InheritedFromUniqueProcessId(부모 PID) — crate가 이름을 가린 필드
            ppid: p.Reserved2 as usize as u32,
            name,
            private_ws,
            ws: p.WorkingSetSize as u64,
            threads: p.NumberOfThreads,
            thread_list: parse_threads(buf, off, p.NumberOfThreads),
        });

        let next = p.NextEntryOffset as usize;
        if next == 0 {
            break; // 마지막 항목
        }
        let Some(new_off) = off.checked_add(next) else {
            break;
        };
        if new_off <= off {
            break; // 전진하지 않는 오프셋 = 손상 — 무한 루프 방지
        }
        off = new_off;
    }
    out
}

/// 프로세스 항목 뒤에 붙은 쓰레드 배열 파싱 — 버퍼 경계를 넘지 않는 개수만 취한다(15 §4).
fn parse_threads(buf: &[u8], proc_off: usize, count: u32) -> Vec<ThreadInfo> {
    const HDR: usize = std::mem::size_of::<SYSTEM_PROCESS_INFORMATION>();
    const T: usize = std::mem::size_of::<SystemThreadInformation>();
    let start = proc_off + HDR;
    if start > buf.len() {
        return Vec::new();
    }
    let fits = (buf.len() - start) / T;
    let n = (count as usize).min(fits);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        // SAFETY: start + (i+1)*T ≤ buf.len()를 위에서 보장했고 커널이 8바이트 정렬을 준다.
        let t = unsafe { &*(buf.as_ptr().add(start + i * T) as *const SystemThreadInformation) };
        out.push(ThreadInfo {
            tid: t.unique_thread as usize as u32,
            state: t.thread_state,
            context_switches: t.context_switches,
        });
    }
    out
}

/// 이미지 이름. PID 0/4는 커널이 빈 이름을 주므로 표시용 이름을 채운다.
fn image_name(p: &SYSTEM_PROCESS_INFORMATION, pid: u32) -> String {
    let len_bytes = p.ImageName.Length as usize;
    if p.ImageName.Buffer.is_null() || len_bytes == 0 || !len_bytes.is_multiple_of(2) {
        return match pid {
            0 => "System Idle Process".to_string(),
            4 => "System".to_string(),
            _ => format!("PID {pid}"),
        };
    }
    // SAFETY: 커널이 준 UNICODE_STRING — Buffer는 같은 버퍼 안을 가리키고 Length는 바이트 길이.
    let slice = unsafe { std::slice::from_raw_parts(p.ImageName.Buffer.0, len_bytes / 2) };
    String::from_utf16_lossy(slice)
}
