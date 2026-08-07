//! mk-app — 셸·워커 겸용 단일 bin(DR-7).
//! `--worker <작업>` 인자로 온디맨드 워커로 분기(M1), 무인자 = 상주 셸.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod shell;

fn main() {
    if std::env::args().any(|a| a == "--worker") {
        // M1: 정리 엔진 실행 경로
        eprintln!("worker: not implemented (M1)");
        std::process::exit(2);
    }
    #[cfg(windows)]
    if let Err(e) = shell::run() {
        eprintln!("shell error: {e}");
        std::process::exit(1);
    }
    #[cfg(not(windows))]
    {
        eprintln!("Nexa MemKeeper는 Windows 전용입니다(mk-core 테스트는 플랫폼 무관).");
        std::process::exit(1);
    }
}
