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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    // [general]
    pub poll_secs: u32,
    pub warn_percent: u32,
    pub danger_percent: u32,
    pub notify: bool,
    pub hotkey_clean: String,
    // [auto]
    pub auto_enabled: bool,
    pub threshold_percent: u32,
    pub cooldown_secs: u32,
    pub interval_mins: u32,
    // [areas]
    pub areas: Areas,
    // [ui]
    pub topn: u32,
    pub list_refresh_ms: u32,
    pub hide_pseudo: bool,
    pub view_flat: bool,
    /// 우리가 모르는 키 원문(섹션 → "키=값" 목록) — 왕복 보존용.
    unknown: BTreeMap<String, Vec<String>>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            poll_secs: 5,
            warn_percent: 80,
            danger_percent: 90,
            notify: true,
            hotkey_clean: String::new(),
            auto_enabled: true,
            threshold_percent: 90,
            cooldown_secs: 300,
            interval_mins: 0,
            areas: Areas::default(),
            topn: 5,
            list_refresh_ms: 2_000,
            hide_pseudo: true,
            view_flat: true,
            unknown: BTreeMap::new(),
        }
    }
}

fn clamp(v: u32, lo: u32, hi: u32) -> u32 {
    v.clamp(lo, hi)
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
            ("general", "poll_secs") => self.poll_secs = num.unwrap_or(self.poll_secs),
            ("general", "warn_percent") => self.warn_percent = num.unwrap_or(self.warn_percent),
            ("general", "danger_percent") => {
                self.danger_percent = num.unwrap_or(self.danger_percent)
            }
            ("general", "notify") => self.notify = parse_bool(value).unwrap_or(self.notify),
            ("general", "hotkey_clean") => self.hotkey_clean = value.to_string(),
            ("auto", "enabled") => {
                self.auto_enabled = parse_bool(value).unwrap_or(self.auto_enabled)
            }
            ("auto", "threshold_percent") => {
                self.threshold_percent = num.unwrap_or(self.threshold_percent)
            }
            ("auto", "cooldown_secs") => self.cooldown_secs = num.unwrap_or(self.cooldown_secs),
            ("auto", "interval_mins") => self.interval_mins = num.unwrap_or(self.interval_mins),
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
            ("ui", "view_flat") => self.view_flat = parse_bool(value).unwrap_or(self.view_flat),
            _ => return false,
        }
        true
    }

    /// 범위 클램프 + 모순 해소(docs/09 §7-2).
    pub fn normalize(&mut self) {
        self.poll_secs = clamp(self.poll_secs, 2, 30);
        self.warn_percent = clamp(self.warn_percent, 50, 95);
        self.danger_percent = clamp(self.danger_percent, 51, 99);
        if self.warn_percent >= self.danger_percent {
            // 모순 — 어느 쪽이 맞는지 알 수 없으므로 둘 다 기본값으로 되돌린다
            let d = Settings::default();
            self.warn_percent = d.warn_percent;
            self.danger_percent = d.danger_percent;
        }
        self.threshold_percent = clamp(self.threshold_percent, 50, 95);
        self.cooldown_secs = clamp(self.cooldown_secs, 60, 1_800);
        if self.interval_mins != 0 {
            self.interval_mins = clamp(self.interval_mins, 10, 240);
        }
        self.topn = clamp(self.topn, 3, 10);
        self.list_refresh_ms = clamp(self.list_refresh_ms, 500, 10_000);
    }

    /// 설정 → ini 텍스트(알 수 없는 키는 해당 섹션 끝에 원문 그대로 되돌린다).
    pub fn to_ini(&self) -> String {
        let b = |v: bool| if v { "true" } else { "false" };
        let mut out = String::new();
        out.push_str("; Nexa MemKeeper 설정 — 스키마: docs/09-settings-schema.md\n");
        out.push_str("; 앱에서 설정 창으로 바꾸는 것을 권장합니다(범위 검증·즉시 적용).\n\n");

        out.push_str("[general]\n");
        out.push_str(&format!("poll_secs = {}\n", self.poll_secs));
        out.push_str(&format!("warn_percent = {}\n", self.warn_percent));
        out.push_str(&format!("danger_percent = {}\n", self.danger_percent));
        out.push_str(&format!("notify = {}\n", b(self.notify)));
        out.push_str(&format!("hotkey_clean = {}\n", self.hotkey_clean));
        self.push_unknown(&mut out, "general");

        out.push_str("\n[auto]\n");
        out.push_str(&format!("enabled = {}\n", b(self.auto_enabled)));
        out.push_str(&format!("threshold_percent = {}\n", self.threshold_percent));
        out.push_str(&format!("cooldown_secs = {}\n", self.cooldown_secs));
        out.push_str(&format!("interval_mins = {}\n", self.interval_mins));
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
        out.push_str(&format!("view_flat = {}\n", b(self.view_flat)));
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
        assert_eq!((s.poll_secs, s.warn_percent, s.danger_percent), (5, 80, 90));
        assert_eq!((s.threshold_percent, s.cooldown_secs), (90, 300));
        assert_eq!((s.list_refresh_ms, s.topn), (2_000, 5));
        assert!(s.auto_enabled && s.notify && s.hide_pseudo && s.view_flat);
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
poll_secs = 10
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
        assert_eq!(s.poll_secs, 10);
        assert!(!s.notify);
        assert!(!s.auto_enabled);
        assert_eq!(s.threshold_percent, 75);
        assert!(s.areas.standby_all);
        assert!(!s.view_flat);
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        let s = Settings::parse("[general]\npoll_secs = 999\n[auto]\ncooldown_secs = 1\ninterval_mins = 5\n[ui]\nlist_refresh_ms = 50\ntopn = 99\n");
        assert_eq!(s.poll_secs, 30); // 2~30
        assert_eq!(s.cooldown_secs, 60); // 60~1800
        assert_eq!(s.interval_mins, 10); // 0이거나 10~240
        assert_eq!(s.list_refresh_ms, 500);
        assert_eq!(s.topn, 10);
    }

    #[test]
    fn contradictory_thresholds_reset_both() {
        // warn ≥ danger — 어느 쪽이 맞는지 알 수 없으므로 둘 다 기본값
        let s = Settings::parse("[general]\nwarn_percent = 95\ndanger_percent = 60\n");
        assert_eq!((s.warn_percent, s.danger_percent), (80, 90));
    }

    #[test]
    fn invalid_format_falls_back_to_default() {
        let s = Settings::parse("[general]\npoll_secs = 뭐지\nnotify = maybe\n");
        assert_eq!(s.poll_secs, 5);
        assert!(s.notify);
    }

    #[test]
    fn unknown_keys_and_sections_survive_roundtrip() {
        let ini = "[general]\npoll_secs = 7\nfuture_key = 42\n\n[experimental]\nsomething = on\n";
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
            poll_secs: 12,
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
        let s = Settings::parse("; 주석\n\n# 다른 주석\n[general]\npoll_secs = 9\n");
        assert_eq!(s.poll_secs, 9);
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
