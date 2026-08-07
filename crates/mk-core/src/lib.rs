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

/// 트레이 툴팁 텍스트(FR-1) — 사용률·사용량/전체. 마지막 정리 정보는 M1에서 추가.
pub fn tray_tooltip(s: &MemSnapshot) -> String {
    format!(
        "Nexa MemKeeper — {}% ({} / {})",
        s.used_percent(),
        fmt_gb(s.used()),
        fmt_gb(s.total)
    )
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
}
