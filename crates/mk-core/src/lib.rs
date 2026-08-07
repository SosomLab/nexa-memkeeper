//! mk-core — 관찰·정리 엔진의 순수 로직.
//! 수집(시스템 콜)은 mk-app(Win32)에서, 계산·표시·판정은 전부 여기서(단위 테스트 대상).

/// 트레이 색 단계 임계(%) — 기본값. M4에서 settings로 이동한다(docs/05 §4-3).
pub const WARN_PERCENT: u32 = 80;
pub const DANGER_PERCENT: u32 = 90;

/// 시스템 메모리 스냅샷(바이트). `GlobalMemoryStatusEx` 결과의 플랫폼 독립 표현.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemSnapshot {
    pub total: u64,
    pub avail: u64,
}

impl MemSnapshot {
    pub fn used(&self) -> u64 {
        self.total.saturating_sub(self.avail)
    }

    /// 사용률 %(0~100, 반올림). total=0(수집 실패)이면 0.
    pub fn used_percent(&self) -> u32 {
        if self.total == 0 {
            return 0;
        }
        ((self.used() as u128 * 100 + self.total as u128 / 2) / self.total as u128) as u32
    }
}

/// 트레이 아이콘 색 단계(docs/05 §4-3: 정상 / 주의 ≥80% / 위험 ≥90%).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Normal,
    Warn,
    Danger,
}

pub fn level(used_percent: u32) -> Level {
    match used_percent {
        p if p >= DANGER_PERCENT => Level::Danger,
        p if p >= WARN_PERCENT => Level::Warn,
        _ => Level::Normal,
    }
}

/// 툴팁·알림용 용량 문자열 — MB 고정, 소수 1자리(docs/05 FR-1).
pub fn fmt_mb(bytes: u64) -> String {
    format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
}

/// 총량급 용량 문자열 — GB 고정, 소수 1자리(트레이 툴팁용).
pub fn fmt_gb(bytes: u64) -> String {
    format!("{:.1}GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

/// 트레이 툴팁 텍스트(FR-1) — 사용률·사용량/전체.
pub fn tray_tooltip(s: &MemSnapshot) -> String {
    format!(
        "Nexa MemKeeper — {}% ({} / {})",
        s.used_percent(),
        fmt_gb(s.used()),
        fmt_gb(s.total)
    )
}

/// 툴팁 둘째 줄(FR-1) — 마지막 정리 회수량·경과.
pub fn fmt_last_clean(mb: u32, elapsed_mins: u64) -> String {
    let ago = match elapsed_mins {
        0 => "방금".to_string(),
        m if m < 60 => format!("{m}분 전"),
        m => format!("{}시간 전", m / 60),
    };
    format!("마지막 정리 {mb}MB · {ago}")
}

// ---- 보호 프로세스 (docs/05 §4-4 — 종료·트림 하드 제외, 해제 불가) ----

/// 이미지 이름 기준 고정 보호 목록(소문자). PID 0/4·자기 자신·PPL은 호출측에서 추가 차단.
const PROTECTED: &[&str] = &[
    "csrss.exe",
    "wininit.exe",
    "winlogon.exe",
    "services.exe",
    "lsass.exe",
    "smss.exe",
    "dwm.exe",
    "fontdrvhost.exe",
    "memcompression",
    "registry",
    "secure system",
];

/// 보호 프로세스 여부(대소문자 무시). 트림·종료 경로는 이 판정을 반드시 통과해야 한다(15 §4).
pub fn is_protected(image_name: &str) -> bool {
    let lower = image_name.to_ascii_lowercase();
    PROTECTED.contains(&lower.as_str())
}

// ---- 워커 결과 프로토콜 (DR-7 — 결과 = 종료 코드 1개) ----

/// 종료 코드 비트 31: 일부 영역이 권한 부족으로 생략됨(관리자 필요).
pub const WORKER_FLAG_PARTIAL: u32 = 0x8000_0000;
/// 워커 내부 오류(예약값 — 회수량으로 해석 금지).
pub const WORKER_ERR: u32 = 0xFFFF_FFFF;
const WORKER_MB_MAX: u32 = 0x7FFF_FFFE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleanOutcome {
    /// 회수량(MB, Avail 전후 차 — docs/19 §4-3).
    pub mb: u32,
    /// true = 영역 일부 생략(승격 필요).
    pub partial: bool,
}

pub fn encode_clean_result(mb: u64, partial: bool) -> u32 {
    let mb = (mb.min(WORKER_MB_MAX as u64)) as u32;
    if partial {
        mb | WORKER_FLAG_PARTIAL
    } else {
        mb
    }
}

/// 워커 종료 코드 해석 — `None` = 오류(크래시·예약값).
pub fn decode_clean_result(code: u32) -> Option<CleanOutcome> {
    if code == WORKER_ERR {
        return None;
    }
    Some(CleanOutcome {
        mb: code & !WORKER_FLAG_PARTIAL,
        partial: code & WORKER_FLAG_PARTIAL != 0,
    })
}

/// 정리 결과 알림 (제목, 본문) — 부풀리기 금지 규칙(docs/19 §4-3): 생략 영역 명시.
pub fn clean_report(outcome: Option<CleanOutcome>) -> (String, String) {
    match outcome {
        Some(o) => {
            let mut body = format!("{}MB 회수", o.mb);
            if o.partial {
                body.push_str(" · 일부 영역 생략(관리자 권한 필요)");
            }
            ("메모리 정리 완료".to_string(), body)
        }
        None => (
            "메모리 정리 실패".to_string(),
            "워커가 비정상 종료했습니다. 다시 시도해 주세요.".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn used_percent_rounds_half_up() {
        // 8GB 중 4GB 사용 = 50%
        let s = MemSnapshot {
            total: 8 << 30,
            avail: 4 << 30,
        };
        assert_eq!(s.used_percent(), 50);
        // 반올림 경계: 1000 중 995 사용 = 99.5% → 100
        let s = MemSnapshot {
            total: 1000,
            avail: 5,
        };
        assert_eq!(s.used_percent(), 100);
        // 1000 중 994 사용 = 99.4% → 99
        let s = MemSnapshot {
            total: 1000,
            avail: 6,
        };
        assert_eq!(s.used_percent(), 99);
    }

    #[test]
    fn used_percent_zero_total_is_zero() {
        let s = MemSnapshot { total: 0, avail: 0 };
        assert_eq!(s.used_percent(), 0);
    }

    #[test]
    fn used_saturates_when_avail_exceeds_total() {
        let s = MemSnapshot {
            total: 100,
            avail: 200,
        };
        assert_eq!(s.used(), 0);
    }

    #[test]
    fn level_boundaries() {
        assert_eq!(level(0), Level::Normal);
        assert_eq!(level(79), Level::Normal);
        assert_eq!(level(80), Level::Warn);
        assert_eq!(level(89), Level::Warn);
        assert_eq!(level(90), Level::Danger);
        assert_eq!(level(100), Level::Danger);
    }

    #[test]
    fn fmt_mb_one_decimal() {
        assert_eq!(fmt_mb(0), "0.0MB");
        assert_eq!(fmt_mb(1024 * 1024), "1.0MB");
        assert_eq!(fmt_mb(1536 * 1024), "1.5MB");
        assert_eq!(fmt_mb(16 * 1024 * 1024 * 1024), "16384.0MB");
    }

    #[test]
    fn fmt_gb_one_decimal() {
        assert_eq!(fmt_gb(16 << 30), "16.0GB");
        assert_eq!(fmt_gb((10 << 30) + (1 << 29)), "10.5GB");
    }

    #[test]
    fn tray_tooltip_shape() {
        let s = MemSnapshot {
            total: 16 << 30,
            avail: 8 << 30,
        };
        assert_eq!(tray_tooltip(&s), "Nexa MemKeeper — 50% (8.0GB / 16.0GB)");
    }

    #[test]
    fn fmt_last_clean_ago() {
        assert_eq!(fmt_last_clean(12, 0), "마지막 정리 12MB · 방금");
        assert_eq!(fmt_last_clean(340, 3), "마지막 정리 340MB · 3분 전");
        assert_eq!(fmt_last_clean(5, 120), "마지막 정리 5MB · 2시간 전");
    }

    #[test]
    fn protected_matches_case_insensitive() {
        assert!(is_protected("LSASS.exe"));
        assert!(is_protected("dwm.exe"));
        assert!(is_protected("MemCompression"));
        assert!(!is_protected("notepad.exe"));
        assert!(!is_protected("chrome.exe"));
    }

    #[test]
    fn worker_result_roundtrip() {
        let c = decode_clean_result(encode_clean_result(340, false)).unwrap();
        assert_eq!((c.mb, c.partial), (340, false));
        let c = decode_clean_result(encode_clean_result(0, true)).unwrap();
        assert_eq!((c.mb, c.partial), (0, true));
        // 오류 예약값
        assert_eq!(decode_clean_result(WORKER_ERR), None);
        // 클램프: 예약값과 충돌하지 않음
        let c = decode_clean_result(encode_clean_result(u64::MAX, true)).unwrap();
        assert_eq!((c.mb, c.partial), (0x7FFF_FFFE, true));
    }

    #[test]
    fn clean_report_texts() {
        let (t, b) = clean_report(Some(CleanOutcome {
            mb: 340,
            partial: false,
        }));
        assert_eq!(t, "메모리 정리 완료");
        assert_eq!(b, "340MB 회수");
        let (_, b) = clean_report(Some(CleanOutcome {
            mb: 12,
            partial: true,
        }));
        assert!(b.contains("관리자 권한 필요"));
        let (t, _) = clean_report(None);
        assert_eq!(t, "메모리 정리 실패");
    }
}
