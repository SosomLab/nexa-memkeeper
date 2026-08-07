//! i18n — UI 문자열 다국어 처리(사용자 확정 08-07: **기본 언어 = 영어**).
//!
//! 방식: 코드에는 **영어 원문을 그대로** 쓰고(`tr("Settings")`), 한국어는 아래 표에서 찾는다.
//! 키를 따로 두지 않는 이유 — 코드를 읽을 때 실제 문구가 보이고, 번역이 없으면 영어로 자연 폴백된다.
//!
//! 언어는 프로세스 전역 1개(`set_lang`). 셸·워커·설정 창은 각자 프로세스라 각자 설정을 읽어 지정한다.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    /// 기본값.
    En,
    Ko,
}

static CURRENT: AtomicU8 = AtomicU8::new(0); // 0 = En, 1 = Ko

pub fn set_lang(lang: Lang) {
    CURRENT.store(
        match lang {
            Lang::En => 0,
            Lang::Ko => 1,
        },
        Ordering::Relaxed,
    );
}

pub fn lang() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Lang::Ko,
        _ => Lang::En,
    }
}

/// 설정 문자열 → 언어. `auto`는 호출자가 OS 언어로 해석해 넘긴다(mk-core는 OS를 모른다).
pub fn lang_from_setting(s: &str) -> Option<Lang> {
    match s.trim().to_ascii_lowercase().as_str() {
        "en" | "english" => Some(Lang::En),
        "ko" | "korean" => Some(Lang::Ko),
        _ => None, // "auto" 포함 — 호출자가 판단
    }
}

/// 영어 원문 → 현재 언어 문자열. 번역이 없으면 원문 그대로(영어 폴백).
pub fn tr(en: &'static str) -> &'static str {
    if lang() == Lang::En {
        return en;
    }
    KO.iter()
        .find(|(k, _)| *k == en)
        .map(|(_, v)| *v)
        .unwrap_or(en)
}

/// 영어 원문 → 한국어. 새 문구를 추가할 때 **영어를 먼저 쓰고** 여기에 한 줄 더한다.
const KO: &[(&str, &str)] = &[
    // ── 공통·트레이
    ("Nexa MemKeeper", "Nexa MemKeeper"),
    ("Clean Now(&C)", "지금 정리(&C)"),
    ("Show List(&L)…", "목록 보기(&L)…"),
    ("Settings(&S)…", "설정(&S)…"),
    (
        "Restart as Administrator(&A)…",
        "관리자 권한으로 재시작(&A)…",
    ),
    ("Exit(&X)", "종료(&X)"),
    ("Memory cleanup complete", "메모리 정리 완료"),
    ("Memory cleanup failed", "메모리 정리 실패"),
    (
        "The worker exited abnormally. Please try again.",
        "워커가 비정상 종료했습니다. 다시 시도해 주세요.",
    ),
    (
        " · some areas skipped (administrator rights required)",
        " · 일부 영역 생략(관리자 권한 필요)",
    ),
    ("MB reclaimed", "MB 회수"),
    ("Task complete", "작업 완료"),
    ("Task failed", "작업 실패"),
    ("Hotkey registration failed", "핫키 등록 실패"),
    ("just now", "방금"),
    ("Last cleanup", "마지막 정리"),
    // ── 메인 화면 메뉴
    ("File(&F)", "파일(&F)"),
    ("Refresh(&R)\tF5", "새로 고침(&R)\tF5"),
    ("Close(&X)", "닫기(&X)"),
    ("Memory(&M)", "메모리(&M)"),
    ("Clean Now(&C) — All", "지금 정리(&C) — 전체"),
    ("Trim Selected Process(&T)", "선택 프로세스 메모리 회수(&T)"),
    ("End Selected Process(&K)…", "선택 프로세스 종료(&K)…"),
    (
        "End Selected Process Tree(&R)…",
        "선택 프로세스 트리 종료(&R)…",
    ),
    ("End Selected Thread(&E)…", "선택 쓰레드 종료(&E)…"),
    ("View(&V)", "보기(&V)"),
    ("Flat List(&L) — show all", "평면 목록(&L) — 전체 나열"),
    ("Grouped by app(&G)", "앱별 묶음 보기(&G)"),
    ("Process Tree(&T)", "프로세스 트리(&T)"),
    ("grouped by app", "앱별 묶음"),
    ("Apps", "앱"),
    ("Background processes", "백그라운드 프로세스"),
    ("Windows processes", "Windows 프로세스"),
    ("Show threads(&H)", "쓰레드 표시(&H)"),
    ("Expand All(&E)", "모두 펼치기(&E)"),
    ("Collapse All(&C)", "모두 접기(&C)"),
    (
        "Clear Search(&D)\tEsc — just type to search",
        "검색 지우기(&D)\tEsc — 창에 그냥 입력하면 검색",
    ),
    ("Settings(&S)", "설정(&S)"),
    (
        "Run at Windows startup(&A)",
        "Windows 시작 시 자동 실행(&A)",
    ),
    ("List refresh interval(&I)", "목록 갱신 주기(&I)"),
    ("1 sec", "1초"),
    ("2 sec (default)", "2초 (기본)"),
    ("5 sec", "5초"),
    ("Hide pseudo processes(&H)", "의사 프로세스 숨기기(&H)"),
    ("Preferences(&P)…", "환경 설정(&P)…"),
    ("Help(&H)", "도움말(&H)"),
    ("SosomLab Website(&W)", "SosomLab 홈페이지(&W)"),
    ("GitHub Repository(&G)", "GitHub 저장소(&G)"),
    ("App Website(&P)", "앱 홈페이지(&P)"),
    ("About(&A)", "정보(&A)"),
    // ── 메인 화면 본문
    (
        "Nexa MemKeeper — Processes",
        "Nexa MemKeeper — 프로세스 목록",
    ),
    ("Process", "프로세스"),
    ("PID", "PID"),
    ("Private", "프라이빗"),
    ("Working Set", "워킹셋"),
    ("Threads", "쓰레드"),
    ("Trim Memory(&T)", "메모리 회수(&T)"),
    ("End Process(&K)…", "프로세스 종료(&K)…"),
    ("End Process Tree(&R)…", "프로세스 트리 종료(&R)…"),
    ("End Thread(&E)…", "쓰레드 종료(&E)…"),
    (
        "Protected — cannot be modified",
        "보호 대상 — 조작할 수 없음",
    ),
    ("No selection", "선택 없음"),
    (
        "Select an item from the list first.",
        "목록에서 항목을 먼저 선택하세요.",
    ),
    ("Wrong target", "대상 아님"),
    ("Select a process row.", "프로세스 행을 선택하세요."),
    ("Select a thread row.", "쓰레드 행을 선택하세요."),
    ("Action failed", "처리 실패"),
    (
        "Administrator rights may be required, or the target is protected.",
        "관리자 권한이 필요하거나 보호 대상일 수 있습니다.",
    ),
    ("End Process — Confirm", "프로세스 종료 확인"),
    ("End Process Tree — Confirm", "프로세스 트리 종료 확인"),
    ("End Thread — Confirm", "쓰레드 종료 확인"),
    (
        "Unsaved work may be lost.",
        "저장하지 않은 작업이 손실될 수 있습니다.",
    ),
    (
        "Force-ending a thread can corrupt the program's state.",
        "쓰레드 강제 종료는 해당 프로그램의 상태를 깨뜨릴 수 있습니다.",
    ),
    ("Failed to open link", "링크 열기 실패"),
    ("Could not open the browser.", "브라우저를 열지 못했습니다."),
    ("Autorun", "자동 실행"),
    (
        "Could not change the setting.",
        "설정을 변경하지 못했습니다.",
    ),
    ("About Nexa MemKeeper", "Nexa MemKeeper 정보"),
    (
        "Ultra-light tray-resident memory optimizer",
        "트레이 상주 초경량 메모리 최적화 도구",
    ),
    ("Version", "버전"),
    ("Website", "홈페이지"),
    ("Repository", "저장소"),
    ("License", "라이선스"),
    (
        "Free for personal/noncommercial use; commercial use requires a separate license.",
        "개인·비상업 무료 / 상업 사용은 별도 라이선스",
    ),
    (
        "Open the website or repository from the Help menu.",
        "도움말 메뉴에서 홈페이지·저장소를 바로 열 수 있습니다.",
    ),
    // ── 상태 줄·행
    ("processes", "프로세스"),
    ("flat list", "평면 목록"),
    ("tree view", "트리 보기"),
    ("Search", "검색"),
    ("Working…", "처리 중…"),
    (
        "Type a process name or PID",
        "프로세스명이나 PID를 입력하세요",
    ),
    ("matches (Esc to clear)", "개 일치(Esc 지움)"),
    (
        "sec refresh · type=search · header=sort · right-click=trim/end",
        "초 갱신 · 입력=검색 · 헤더=정렬 · 우클릭=회수/종료",
    ),
    ("Selected", "선택"),
    ("Thread", "쓰레드"),
    ("switches", "회 전환"),
    // ── 쓰레드 상태
    ("Initialized", "초기화"),
    ("Ready", "준비"),
    ("Running", "실행 중"),
    ("Standby", "대기(Standby)"),
    ("Terminated", "종료됨"),
    ("Waiting", "대기"),
    ("Transition", "전환"),
    ("Deferred Ready", "대기 중(Deferred)"),
    // ── Top 5 팝업
    ("Memory", "메모리"),
    ("used", "사용"),
    ("Clean Now", "지금 정리"),
    ("Show List…", "목록 보기…"),
    // ── 설정 창
    ("Nexa MemKeeper — Settings", "Nexa MemKeeper — 설정"),
    ("Automatic Cleanup", "자동 정리"),
    (
        "Clean automatically when memory usage exceeds the threshold",
        "메모리 사용률이 임계값을 넘으면 자동 정리",
    ),
    ("Threshold (%, 50~95)", "임계값 (%, 50~95)"),
    (
        "Min interval (ms, 60000~1800000)",
        "최소 간격 (ms, 60000~1800000)",
    ),
    ("Periodic cleanup (ms, 0=off)", "주기 정리 (ms, 0=끔)"),
    ("Cleanup Areas", "정리 영역"),
    (
        "(1) Process working set trim (safe)",
        "① 프로세스 워킹셋 트림 (안전)",
    ),
    (
        "(2) Low-priority standby purge (safe)",
        "② 우선순위 0 스탠바이 퍼지 (안전)",
    ),
    (
        "(3) Full standby purge — file cache lost",
        "③ 전체 스탠바이 퍼지 — 파일 캐시 소실",
    ),
    (
        "(4) Flush modified pages — disk writes",
        "④ 수정 페이지 플러시 — 디스크 쓰기",
    ),
    (
        "(5) System working set — cache reload cost",
        "⑤ 시스템 워킹셋 — 캐시 재적재 비용",
    ),
    (
        "(6) Page combining — slow, admin only",
        "⑥ 페이지 결합 — 느림, 관리자 전용",
    ),
    (" · apps released ", " · 앱이 놓은 양 "),
    (
        "Not recommended — apps reclaim it within ~14s.",
        "권장하지 않음 — 앱이 14초 안에 되찾아갑니다.",
    ),
    (
        "Also clean when Windows signals low memory",
        "Windows가 메모리 부족을 알리면 함께 정리",
    ),
    (
        "(3)~(6) need admin rights; brief slowdown possible.",
        "③~⑥은 관리자 권한 필요, 일시적 체감 저하 가능.",
    ),
    ("General", "일반"),
    (
        "Poll interval (ms, 2000~30000)",
        "관찰 주기 (ms, 2000~30000)",
    ),
    ("List refresh (ms, 500~10000)", "목록 갱신 (ms, 500~10000)"),
    ("Top N (3~10)", "Top N (3~10)"),
    ("Notify on cleanup", "정리 결과 알림"),
    ("Hide pseudo processes", "의사 프로세스 숨김"),
    (
        "List defaults to flat (not tree)",
        "목록 기본 = 평면(트리 아님)",
    ),
    ("Global hotkey (Clean Now)", "전역 핫키 (지금 정리)"),
    ("Language", "언어"),
    ("Auto (system)", "자동(시스템)"),
    ("English", "English"),
    ("Korean", "한국어"),
    ("Defaults", "기본값"),
    ("Save", "저장"),
    ("Cancel", "취소"),
    ("Check settings", "설정 확인"),
    (
        "No cleanup area is selected.\nTurn on at least one for cleanup to work.",
        "정리 영역이 하나도 선택되지 않았습니다.\n최소 하나는 켜야 정리가 동작합니다.",
    ),
    ("Save failed", "저장 실패"),
    (
        "Could not save settings.\nCheck write permission.",
        "설정을 저장하지 못했습니다.\n쓰기 권한을 확인해 주세요.",
    ),
    (
        "Restart the app to apply the language change.",
        "언어 변경은 앱을 다시 시작해야 적용됩니다.",
    ),
];

/// 언어는 **프로세스 전역**(AtomicU8)이라, 언어를 바꾸는 테스트끼리 병렬로 돌면 서로를 덮어쓴다.
/// 08-07에 `korean_lookup_when_selected`가 간헐적으로 깨져 드러났다 — 언어에 의존하는 테스트는
/// 이 락을 잡고 돈다(크레이트 안에서만 쓰는 테스트 전용 장치).
#[cfg(test)]
pub(crate) static LANG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_english() {
        let _guard = super::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_lang(Lang::En);
        assert_eq!(lang(), Lang::En);
        assert_eq!(tr("Save"), "Save");
        assert_eq!(tr("Cleanup Areas"), "Cleanup Areas");
    }

    #[test]
    fn korean_lookup_when_selected() {
        let _guard = super::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_lang(Lang::Ko);
        assert_eq!(tr("Save"), "저장");
        assert_eq!(tr("Cleanup Areas"), "정리 영역");
        set_lang(Lang::En); // 다른 테스트에 영향 주지 않도록 복귀
    }

    #[test]
    fn missing_translation_falls_back_to_english() {
        let _guard = super::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_lang(Lang::Ko);
        assert_eq!(tr("Not translated yet"), "Not translated yet");
        set_lang(Lang::En);
    }

    #[test]
    fn setting_string_maps_to_lang() {
        assert_eq!(lang_from_setting("en"), Some(Lang::En));
        assert_eq!(lang_from_setting("KO"), Some(Lang::Ko));
        assert_eq!(lang_from_setting("auto"), None); // 호출자가 OS 언어로 해석
        assert_eq!(lang_from_setting("zz"), None);
    }

    #[test]
    fn table_has_no_duplicate_keys() {
        let mut seen = std::collections::HashSet::new();
        for (k, _) in KO {
            assert!(seen.insert(*k), "중복 키: {k}");
        }
    }
}
