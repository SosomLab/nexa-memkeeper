//! settings — `data\settings.ini` 스키마·파싱·직렬화(docs/09가 SSOT).
//! 파일 I/O는 mk-app이 담당하고, 여기서는 **문자열 ↔ 구조체 변환과 검증만** 한다(단위 테스트 대상).
//!
//! 규칙(docs/09 §7): ① 알 수 없는 키는 보존(왕복 시 유실 금지) ② 범위 밖 값은 경계로 클램프
//! ③ 잘못된 형식은 조용히 기본값 ④ `warn ≥ danger`처럼 모순이면 두 키 모두 기본값 복원.

use std::collections::BTreeMap;

/// 정리 영역 on/off(docs/05 §4-2 — 기본은 안전한 ①② 만).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Areas {
    pub working_sets: bool,
    pub standby_low: bool,
    pub standby_all: bool,
    pub modified: bool,
    pub system_ws: bool,
    pub combine: bool,
}

impl Default for Areas {
    fn default() -> Self {
        Self {
            working_sets: true,
            standby_low: true,
            standby_all: false,
            modified: false,
            system_ws: false,
            combine: false,
        }
    }
}

impl Areas {
    /// 하나라도 켜져 있는가(전부 끄면 정리가 무의미 — UI에서 경고용).
    pub fn any(&self) -> bool {
        self.working_sets
            || self.standby_low
            || self.standby_all
            || self.modified
            || self.system_ws
            || self.combine
    }
}

/// **시간 단위는 전부 밀리초(ms)로 통일**(사용자 확정 08-07) — 화면·파일·코드가 같은 단위를 쓰면
/// 환산 실수가 사라진다. 구 키(`poll_secs`·`cooldown_secs`·`interval_mins`)는 읽을 때 자동
/// 변환하고 저장 시 새 키로만 쓴다(docs/09 §7-4 "개명 금지, 1회 마이그레이션").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    // [general]
    pub poll_ms: u32,
    pub warn_percent: u32,
    pub danger_percent: u32,
    pub notify: bool,
    pub hotkey_clean: String,
    /// UI 언어 — `en`(**기본**) / `ko` / `auto`(OS 언어 따름). 적용은 mk-app config.rs.
    pub lang: String,
    // [auto]
    pub auto_enabled: bool,
    pub threshold_percent: u32,
    pub cooldown_ms: u32,
    /// 주기 정리 간격(ms). 0 = 끔.
    pub interval_ms: u32,
    /// **저메모리 이벤트 트리거**(docs/07 §2) — Windows가 "여유 메모리 부족"을 알릴 때 즉시 정리.
    /// 폴링(주기·임계값)의 보조다. 쿨다운은 그대로 지켜지므로 연달아 돌지 않는다.
    pub low_memory_event: bool,
    /// **전체화면 억제**(X-2) — 게임·발표 등 전체화면 중에는 자동 정리를 보류한다.
    /// 정리에는 순간 멈춤이라는 대가가 있고([21](../../docs/21-effectiveness-measurement.md)),
    /// 하필 그 순간이 프레임이 중요한 때다. 수동 정리·핫키는 그대로 동작한다.
    pub suppress_fullscreen: bool,
    // [areas]
    pub areas: Areas,
    // [ui]
    pub topn: u32,
    pub list_refresh_ms: u32,
    pub hide_pseudo: bool,
    /// 보기 모드 — `grouped`(**기본**, 작업 관리자식 분류·묶음) / `tree`(부모-자식) / `flat`(전체 나열).
    pub view_mode: String,
    /// 쓰레드 행 표시 — 기본 off(작업 관리자처럼 프로세스 계층만).
    pub show_threads: bool,
    /// 메인 창 위치·크기(M3-4). `-1` = 미지정(기본 위치에 띄운다).
    /// 화면 구성이 바뀌어 화면 밖이 되는 경우가 있어, **복원할 때 보이는 영역인지 확인**한다.
    pub win_x: i32,
    pub win_y: i32,
    pub win_w: i32,
    pub win_h: i32,
    /// 고정 열 폭(PID·Private·Working Set·Threads 순, px). 비어 있으면 기본값.
    /// 이름 열은 남는 폭을 흡수하므로 저장 대상이 아니다(M3-4).
    pub col_w: Vec<i32>,
    /// 우리가 모르는 키 원문(섹션 → "키=값" 목록) — 왕복 보존용.
    unknown: BTreeMap<String, Vec<String>>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            poll_ms: 5_000,
            warn_percent: 80,
            danger_percent: 90,
            notify: true,
            hotkey_clean: String::new(),
            lang: "en".to_string(),
            auto_enabled: true,
            threshold_percent: 90,
            cooldown_ms: 300_000,
            interval_ms: 0,
            low_memory_event: true,
            suppress_fullscreen: true,
            areas: Areas::default(),
            topn: 5,
            list_refresh_ms: 2_000,
            hide_pseudo: true,
            view_mode: "grouped".to_string(),
            show_threads: false,
            win_x: -1,
            win_y: -1,
            win_w: -1,
            win_h: -1,
            col_w: Vec::new(),
            unknown: BTreeMap::new(),
        }
    }
}

fn clamp(v: u32, lo: u32, hi: u32) -> u32 {
    v.clamp(lo, hi)
}

fn parse_i32(s: &str) -> Option<i32> {
    s.trim().parse::<i32>().ok()
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

impl Settings {
    /// ini 텍스트 → 설정. 어떤 입력이든 실패하지 않는다(잘못된 값은 기본값·클램프).
    pub fn parse(text: &str) -> Self {
        let mut s = Settings::default();
        let mut section = String::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = name.trim().to_ascii_lowercase();
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let (key, value) = (key.trim().to_ascii_lowercase(), value.trim());
            if !s.apply(&section, &key, value) {
                // 모르는 키 — 원문 그대로 보존(다른 버전이 쓴 설정을 지우지 않는다)
                s.unknown
                    .entry(section.clone())
                    .or_default()
                    .push(line.to_string());
            }
        }
        s.normalize();
        s
    }

    /// 알려진 키면 적용하고 true. 형식이 틀리면 기본값 유지(적용된 것으로 간주 — 보존 대상 아님).
    fn apply(&mut self, section: &str, key: &str, value: &str) -> bool {
        let num = value.parse::<u32>().ok();
        match (section, key) {
            ("general", "poll_ms") => self.poll_ms = num.unwrap_or(self.poll_ms),
            // 구 키(초·분) — 1회 마이그레이션: 읽어서 ms로 환산하고, 저장 시엔 새 키로만 쓴다
            ("general", "poll_secs") => {
                self.poll_ms = num.map(|v| v.saturating_mul(1_000)).unwrap_or(self.poll_ms)
            }
            ("auto", "cooldown_secs") => {
                self.cooldown_ms = num
                    .map(|v| v.saturating_mul(1_000))
                    .unwrap_or(self.cooldown_ms)
            }
            ("auto", "interval_mins") => {
                self.interval_ms = num
                    .map(|v| v.saturating_mul(60_000))
                    .unwrap_or(self.interval_ms)
            }
            ("general", "warn_percent") => self.warn_percent = num.unwrap_or(self.warn_percent),
            ("general", "danger_percent") => {
                self.danger_percent = num.unwrap_or(self.danger_percent)
            }
            ("general", "notify") => self.notify = parse_bool(value).unwrap_or(self.notify),
            ("general", "hotkey_clean") => self.hotkey_clean = value.to_string(),
            ("general", "lang") => self.lang = value.to_string(),
            ("auto", "enabled") => {
                self.auto_enabled = parse_bool(value).unwrap_or(self.auto_enabled)
            }
            ("auto", "threshold_percent") => {
                self.threshold_percent = num.unwrap_or(self.threshold_percent)
            }
            ("auto", "cooldown_ms") => self.cooldown_ms = num.unwrap_or(self.cooldown_ms),
            ("auto", "interval_ms") => self.interval_ms = num.unwrap_or(self.interval_ms),
            ("auto", "suppress_fullscreen") => {
                self.suppress_fullscreen = parse_bool(value).unwrap_or(self.suppress_fullscreen)
            }
            ("auto", "low_memory_event") => {
                self.low_memory_event = parse_bool(value).unwrap_or(self.low_memory_event)
            }
            ("areas", "working_sets") => {
                self.areas.working_sets = parse_bool(value).unwrap_or(self.areas.working_sets)
            }
            ("areas", "standby_low") => {
                self.areas.standby_low = parse_bool(value).unwrap_or(self.areas.standby_low)
            }
            ("areas", "standby_all") => {
                self.areas.standby_all = parse_bool(value).unwrap_or(self.areas.standby_all)
            }
            ("areas", "modified") => {
                self.areas.modified = parse_bool(value).unwrap_or(self.areas.modified)
            }
            ("areas", "system_ws") => {
                self.areas.system_ws = parse_bool(value).unwrap_or(self.areas.system_ws)
            }
            ("areas", "combine") => {
                self.areas.combine = parse_bool(value).unwrap_or(self.areas.combine)
            }
            ("ui", "topn") => self.topn = num.unwrap_or(self.topn),
            ("ui", "list_refresh_ms") => self.list_refresh_ms = num.unwrap_or(self.list_refresh_ms),
            ("ui", "hide_pseudo") => {
                self.hide_pseudo = parse_bool(value).unwrap_or(self.hide_pseudo)
            }
            ("ui", "view_mode") => self.view_mode = value.to_string(),
            // 구 키 — 1회 마이그레이션(true였으면 평면, false였으면 트리)
            ("ui", "view_flat") => {
                self.view_mode = match parse_bool(value) {
                    Some(true) => "flat".to_string(),
                    Some(false) => "tree".to_string(),
                    None => self.view_mode.clone(),
                }
            }
            ("ui", "win_x") => self.win_x = parse_i32(value).unwrap_or(self.win_x),
            ("ui", "win_y") => self.win_y = parse_i32(value).unwrap_or(self.win_y),
            ("ui", "win_w") => self.win_w = parse_i32(value).unwrap_or(self.win_w),
            ("ui", "win_h") => self.win_h = parse_i32(value).unwrap_or(self.win_h),
            ("ui", "col_w") => {
                let cols: Vec<i32> = value
                    .split(',')
                    .filter_map(|v| parse_i32(v.trim()))
                    .collect();
                self.col_w = cols;
            }
            ("ui", "show_threads") => {
                self.show_threads = parse_bool(value).unwrap_or(self.show_threads)
            }
            _ => return false,
        }
        true
    }

    /// 범위 클램프 + 모순 해소(docs/09 §7-2).
    pub fn normalize(&mut self) {
        self.poll_ms = clamp(self.poll_ms, 2_000, 30_000);
        self.warn_percent = clamp(self.warn_percent, 50, 95);
        self.danger_percent = clamp(self.danger_percent, 51, 99);
        if self.warn_percent >= self.danger_percent {
            // 모순 — 어느 쪽이 맞는지 알 수 없으므로 둘 다 기본값으로 되돌린다
            let d = Settings::default();
            self.warn_percent = d.warn_percent;
            self.danger_percent = d.danger_percent;
        }
        self.threshold_percent = clamp(self.threshold_percent, 50, 95);
        self.cooldown_ms = clamp(self.cooldown_ms, 60_000, 1_800_000);
        if self.interval_ms != 0 {
            self.interval_ms = clamp(self.interval_ms, 600_000, 14_400_000);
        }
        self.topn = clamp(self.topn, 3, 10);
        self.list_refresh_ms = clamp(self.list_refresh_ms, 500, 10_000);
        // 창 크기가 0에 가깝게 저장되면 복원했을 때 쓸 수 없는 창이 된다 — 최소치를 보장한다.
        if self.win_w > 0 || self.win_h > 0 {
            self.win_w = self.win_w.max(420);
            self.win_h = self.win_h.max(300);
        }
        // 열 폭은 **개수가 맞을 때만** 인정한다(열이 늘거나 준 버전의 파일을 그대로 믿지 않는다).
        if self.col_w.len() != 4 {
            self.col_w.clear();
        } else {
            for w in &mut self.col_w {
                *w = (*w).clamp(40, 400);
            }
        }
    }

    /// 설정 → ini 텍스트(알 수 없는 키는 해당 섹션 끝에 원문 그대로 되돌린다).
    pub fn to_ini(&self) -> String {
        let b = |v: bool| if v { "true" } else { "false" };
        let mut out = String::new();
        out.push_str("; Nexa MemKeeper 설정 — 스키마: docs/09-settings-schema.md\n");
        out.push_str("; 앱에서 설정 창으로 바꾸는 것을 권장합니다(범위 검증·즉시 적용).\n\n");

        out.push_str("; 시간 단위는 모두 밀리초(ms)입니다.\n");
        out.push_str("[general]\n");
        out.push_str(&format!("poll_ms = {}\n", self.poll_ms));
        out.push_str(&format!("warn_percent = {}\n", self.warn_percent));
        out.push_str(&format!("danger_percent = {}\n", self.danger_percent));
        out.push_str(&format!("notify = {}\n", b(self.notify)));
        out.push_str(&format!("hotkey_clean = {}\n", self.hotkey_clean));
        self.push_unknown(&mut out, "general");

        out.push_str("\n[auto]\n");
        out.push_str(&format!("enabled = {}\n", b(self.auto_enabled)));
        out.push_str(&format!("threshold_percent = {}\n", self.threshold_percent));
        out.push_str(&format!("cooldown_ms = {}\n", self.cooldown_ms));
        out.push_str(&format!("interval_ms = {}\n", self.interval_ms));
        out.push_str(&format!(
            "low_memory_event = {}\n",
            b(self.low_memory_event)
        ));
        out.push_str(&format!(
            "suppress_fullscreen = {}\n",
            b(self.suppress_fullscreen)
        ));
        self.push_unknown(&mut out, "auto");

        out.push_str("\n[areas]\n");
        out.push_str(&format!("working_sets = {}\n", b(self.areas.working_sets)));
        out.push_str(&format!("standby_low = {}\n", b(self.areas.standby_low)));
        out.push_str(&format!("standby_all = {}\n", b(self.areas.standby_all)));
        out.push_str(&format!("modified = {}\n", b(self.areas.modified)));
        out.push_str(&format!("system_ws = {}\n", b(self.areas.system_ws)));
        out.push_str(&format!("combine = {}\n", b(self.areas.combine)));
        self.push_unknown(&mut out, "areas");

        out.push_str("\n[ui]\n");
        out.push_str(&format!("topn = {}\n", self.topn));
        out.push_str(&format!("list_refresh_ms = {}\n", self.list_refresh_ms));
        out.push_str(&format!("hide_pseudo = {}\n", b(self.hide_pseudo)));
        out.push_str(&format!("view_mode = {}\n", self.view_mode));
        out.push_str(&format!("show_threads = {}\n", b(self.show_threads)));
        // 창 위치·열 폭은 사람이 손으로 고칠 값이 아니라 창이 기억해 두는 값이다(M3-4).
        // 미지정(-1)이면 쓰지 않는다 — 파일이 기본값으로 지저분해지지 않게.
        if self.win_w > 0 && self.win_h > 0 {
            out.push_str(&format!("win_x = {}\n", self.win_x));
            out.push_str(&format!("win_y = {}\n", self.win_y));
            out.push_str(&format!("win_w = {}\n", self.win_w));
            out.push_str(&format!("win_h = {}\n", self.win_h));
        }
        if !self.col_w.is_empty() {
            let cols: Vec<String> = self.col_w.iter().map(|v| v.to_string()).collect();
            out.push_str(&format!("col_w = {}\n", cols.join(",")));
        }
        self.push_unknown(&mut out, "ui");

        // 우리가 모르는 섹션 전체도 보존
        for (sec, lines) in &self.unknown {
            if matches!(sec.as_str(), "general" | "auto" | "areas" | "ui") {
                continue;
            }
            out.push_str(&format!("\n[{sec}]\n"));
            for l in lines {
                out.push_str(l);
                out.push('\n');
            }
        }
        out
    }

    fn push_unknown(&self, out: &mut String, section: &str) {
        if let Some(lines) = self.unknown.get(section) {
            for l in lines {
                out.push_str(l);
                out.push('\n');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_documented_schema() {
        let s = Settings::default();
        assert_eq!(
            (s.poll_ms, s.warn_percent, s.danger_percent),
            (5_000, 80, 90)
        );
        assert_eq!((s.threshold_percent, s.cooldown_ms), (90, 300_000));
        assert_eq!((s.list_refresh_ms, s.topn), (2_000, 5));
        assert!(s.auto_enabled && s.notify && s.hide_pseudo);
        // 기본 보기 = 프로세스 트리, 쓰레드는 숨김(사용자 확정 08-07)
        assert_eq!(
            s.view_mode, "grouped",
            "기본은 작업 관리자식 묶음 보기여야 한다"
        );
        assert!(!s.show_threads, "쓰레드는 기본 숨김이어야 한다");
        // 안전 우선 — ①② 만 기본 on
        assert_eq!((s.areas.working_sets, s.areas.standby_low), (true, true));
        assert!(
            !s.areas.standby_all && !s.areas.modified && !s.areas.system_ws && !s.areas.combine
        );
    }

    #[test]
    fn parse_reads_all_sections() {
        let ini = "\
[general]
poll_ms = 10000
notify = off
[auto]
enabled = 0
threshold_percent = 75
[areas]
standby_all = yes
[ui]
view_flat = false
";
        let s = Settings::parse(ini);
        assert_eq!(s.poll_ms, 10_000);
        assert!(!s.notify);
        assert!(!s.auto_enabled);
        assert_eq!(s.threshold_percent, 75);
        assert!(s.areas.standby_all);
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        let s = Settings::parse("[general]\npoll_ms = 999999\n[auto]\ncooldown_ms = 1\ninterval_ms = 5000\n[ui]\nlist_refresh_ms = 50\ntopn = 99\n");
        assert_eq!(s.poll_ms, 30_000); // 2000~30000
        assert_eq!(s.cooldown_ms, 60_000); // 60000~1800000
        assert_eq!(s.interval_ms, 600_000); // 0이거나 600000~14400000
        assert_eq!(s.list_refresh_ms, 500);
        assert_eq!(s.topn, 10);
    }

    #[test]
    fn legacy_time_keys_migrate_to_ms() {
        // 구 버전 파일(초·분) — 값이 그대로 살아나야 하고, 저장 시엔 ms 키로만 남는다
        let s = Settings::parse(
            "[general]\npoll_secs = 10\n[auto]\ncooldown_secs = 600\ninterval_mins = 30\n",
        );
        assert_eq!(s.poll_ms, 10_000);
        assert_eq!(s.cooldown_ms, 600_000);
        assert_eq!(s.interval_ms, 1_800_000);
        let out = s.to_ini();
        assert!(out.contains("poll_ms = 10000"));
        assert!(out.contains("cooldown_ms = 600000"));
        assert!(out.contains("interval_ms = 1800000"));
        // 구 키는 남기지 않는다(1회 마이그레이션 — 보존 대상 아님)
        assert!(!out.contains("poll_secs"), "구 키가 남았다:\n{out}");
        assert!(!out.contains("cooldown_secs"));
        assert!(!out.contains("interval_mins"));
    }

    #[test]
    fn contradictory_thresholds_reset_both() {
        // warn ≥ danger — 어느 쪽이 맞는지 알 수 없으므로 둘 다 기본값
        let s = Settings::parse("[general]\nwarn_percent = 95\ndanger_percent = 60\n");
        assert_eq!((s.warn_percent, s.danger_percent), (80, 90));
    }

    #[test]
    fn invalid_format_falls_back_to_default() {
        let s = Settings::parse("[general]\npoll_ms = 뭐지\nnotify = maybe\n");
        assert_eq!(s.poll_ms, 5_000);
        assert!(s.notify);
    }

    #[test]
    fn unknown_keys_and_sections_survive_roundtrip() {
        let ini = "[general]\npoll_ms = 7000\nfuture_key = 42\n\n[experimental]\nsomething = on\n";
        let s = Settings::parse(ini);
        let out = s.to_ini();
        assert!(
            out.contains("future_key = 42"),
            "모르는 키가 사라졌다:\n{out}"
        );
        assert!(
            out.contains("[experimental]"),
            "모르는 섹션이 사라졌다:\n{out}"
        );
        assert!(out.contains("something = on"));
        // 다시 읽어도 동일해야 한다
        assert_eq!(Settings::parse(&out), s);
    }

    #[test]
    fn roundtrip_is_stable() {
        let s = Settings {
            poll_ms: 12_000,
            hotkey_clean: "Ctrl+Alt+M".into(),
            areas: Areas {
                standby_all: true,
                ..Areas::default()
            },
            ..Settings::default()
        };
        let back = Settings::parse(&s.to_ini());
        assert_eq!(back, s);
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let s = Settings::parse("; 주석\n\n# 다른 주석\n[general]\npoll_ms = 9000\n");
        assert_eq!(s.poll_ms, 9_000);
    }

    #[test]
    fn areas_any_detects_all_off() {
        let mut a = Areas::default();
        assert!(a.any());
        a = Areas {
            working_sets: false,
            standby_low: false,
            ..a
        };
        assert!(!a.any());
    }
}
