//! mk-app — 셸·워커 겸용 단일 bin(DR-7).
//! `--worker <작업>` 인자로 온디맨드 워커로 분기, 무인자 = 상주 셸(M0에서 트레이 구현).

fn main() {
    // M0 전 골격: 파이프라인 검증용 최소 동작(트레이는 M0-1).
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--worker") {
        // M1: 정리 엔진 실행 경로
        eprintln!("worker: not implemented (M1)");
        std::process::exit(2);
    }
    let sample = mk_core::MemSnapshot {
        total: 8 << 30,
        avail: 4 << 30,
    };
    println!(
        "Nexa MemKeeper {} — skeleton (M0 대기): sample {}% used, {}",
        env!("CARGO_PKG_VERSION"),
        sample.used_percent(),
        mk_core::fmt_mb(sample.used()),
    );
}
