//! mk-app — 셸·워커 겸용 단일 bin(DR-7).
//! `--worker <작업>` 인자로 온디맨드 워커로 분기(M1), 무인자 = 상주 셸.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod shell;
#[cfg(windows)]
mod worker;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--worker") {
        #[cfg(windows)]
        {
            let code = match args.iter().position(|a| a == "--worker") {
                Some(i) if args.get(i + 1).map(String::as_str) == Some("clean") => {
                    worker::run_clean()
                }
                _ => mk_core::WORKER_ERR, // 알 수 없는 워커 작업
            };
            std::process::exit(code as i32);
        }
        #[cfg(not(windows))]
        std::process::exit(mk_core::WORKER_ERR as i32);
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
