//! mk-app — 셸·워커 겸용 단일 bin(DR-7).
//! `--worker <작업>` 인자로 온디맨드 워커로 분기(M1), 무인자 = 상주 셸.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod config;
#[cfg(windows)]
mod listwnd;
#[cfg(windows)]
mod popup;
#[cfg(windows)]
mod procs;
#[cfg(windows)]
mod settingswnd;
#[cfg(windows)]
mod shell;
#[cfg(windows)]
mod worker;

/// `--worker bench` — 열거+Top5 선정 소요를 5회 측정해 stderr에 출력(NFR-4 검증용).
#[cfg(windows)]
fn bench_enum() -> u32 {
    for i in 1..=5 {
        let t = std::time::Instant::now();
        let all = procs::enumerate();
        let n = all.len();
        let top = mk_core::top_n(all, 5);
        eprintln!(
            "bench#{i}: {n} procs, enumerate+top5 {:?}, top1={} {}",
            t.elapsed(),
            top.first().map(|p| p.name.as_str()).unwrap_or("-"),
            top.first()
                .map(|p| mk_core::fmt_size(p.private_ws))
                .unwrap_or_default()
        );
    }
    0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--worker") {
        #[cfg(windows)]
        {
            let i = args.iter().position(|a| a == "--worker").unwrap_or(0);
            let action = args.get(i + 1).map(String::as_str).unwrap_or("");
            let pid = args.get(i + 2).and_then(|s| s.parse::<u32>().ok());
            let code = match (action, pid) {
                ("clean", _) => worker::run_clean(),
                ("list", _) => listwnd::run(),
                ("settings", _) => settingswnd::run(),
                ("trim", Some(pid)) => worker::run_trim(pid),
                ("kill", Some(pid)) => worker::run_kill(pid),
                ("killtree", Some(pid)) => worker::run_kill_tree(pid),
                ("killthread", Some(tid)) => worker::run_kill_thread(tid),
                // QA용 열거 벤치(NFR-4 게이트 측정 — docs/18 §4)
                ("bench", _) => bench_enum(),
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
