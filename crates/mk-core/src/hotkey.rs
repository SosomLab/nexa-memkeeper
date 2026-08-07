//! hotkey — 전역 핫키 조합의 문자열 ↔ 구조체 변환(FR-5 `hotkey_clean`, M4-2).
//! 등록/해제(`RegisterHotKey`)는 mk-app이 하고, 여기서는 **표기 규칙과 검증만** 다룬다.
//!
//! 표기: `Ctrl+Alt+M` — 수정자는 `Ctrl`·`Alt`·`Shift`·`Win`(순서·대소문자 무관), 마지막이 키.
//! 빈 문자열 = 핫키 없음. 수정자만 있거나 알 수 없는 키 이름이면 `None`(설정에서 무시).

/// 전역 핫키 조합. `vk`는 Win32 가상 키 코드.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub vk: u32,
}

impl Hotkey {
    /// 수정자가 하나도 없으면 위험하다(일반 타이핑을 가로챈다) — 등록 대상에서 제외.
    pub fn has_modifier(&self) -> bool {
        self.ctrl || self.alt || self.shift || self.win
    }
}

/// 키 이름 ↔ 가상 키 코드 표(문자·숫자·F키는 계산으로 처리하고, 나머지만 나열).
const NAMED_KEYS: &[(&str, u32)] = &[
    ("space", 0x20),
    ("insert", 0x2D),
    ("delete", 0x2E),
    ("home", 0x24),
    ("end", 0x23),
    ("pageup", 0x21),
    ("pagedown", 0x22),
    ("up", 0x26),
    ("down", 0x28),
    ("left", 0x25),
    ("right", 0x27),
    ("backspace", 0x08),
    ("tab", 0x09),
    ("enter", 0x0D),
    ("pause", 0x13),
];

/// `"Ctrl+Alt+M"` → [`Hotkey`]. 빈 문자열·수정자 없음·알 수 없는 키는 `None`.
pub fn parse_hotkey(text: &str) -> Option<Hotkey> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let mut hk = Hotkey::default();
    let mut key_seen = false;
    for part in text.split('+') {
        let p = part.trim().to_ascii_lowercase();
        if p.is_empty() {
            continue;
        }
        match p.as_str() {
            "ctrl" | "control" => hk.ctrl = true,
            "alt" => hk.alt = true,
            "shift" => hk.shift = true,
            "win" | "windows" => hk.win = true,
            _ => {
                if key_seen {
                    return None; // 키가 둘 이상 — 잘못된 표기
                }
                hk.vk = key_code(&p)?;
                key_seen = true;
            }
        }
    }
    if !key_seen || !hk.has_modifier() {
        return None; // 키가 없거나 수정자가 없으면 등록하지 않는다
    }
    Some(hk)
}

fn key_code(name: &str) -> Option<u32> {
    if name.len() == 1 {
        let c = name.chars().next()?;
        if c.is_ascii_alphabetic() {
            return Some(c.to_ascii_uppercase() as u32);
        }
        if c.is_ascii_digit() {
            return Some(c as u32);
        }
        return None;
    }
    if let Some(n) = name.strip_prefix('f') {
        if let Ok(n) = n.parse::<u32>() {
            if (1..=24).contains(&n) {
                return Some(0x70 + n - 1); // VK_F1 = 0x70
            }
        }
    }
    NAMED_KEYS
        .iter()
        .find(|(k, _)| *k == name)
        .map(|(_, vk)| *vk)
}

fn key_name(vk: u32) -> Option<String> {
    match vk {
        0x30..=0x39 => Some(((vk as u8) as char).to_string()),
        0x41..=0x5A => Some(((vk as u8) as char).to_string()),
        0x70..=0x87 => Some(format!("F{}", vk - 0x70 + 1)),
        _ => NAMED_KEYS.iter().find(|(_, c)| *c == vk).map(|(k, _)| {
            // 표기는 첫 글자만 대문자(Space·PageUp 같은 합성어는 표에서 그대로)
            let mut s = k.to_string();
            if let Some(first) = s.get_mut(0..1) {
                first.make_ascii_uppercase();
            }
            s
        }),
    }
}

/// [`Hotkey`] → `"Ctrl+Alt+M"`. 키 이름을 모르면 빈 문자열(설정 파일에 쓰레기를 남기지 않는다).
pub fn hotkey_to_string(hk: &Hotkey) -> String {
    let Some(key) = key_name(hk.vk) else {
        return String::new();
    };
    let mut parts: Vec<&str> = Vec::new();
    if hk.ctrl {
        parts.push("Ctrl");
    }
    if hk.alt {
        parts.push("Alt");
    }
    if hk.shift {
        parts.push("Shift");
    }
    if hk.win {
        parts.push("Win");
    }
    if parts.is_empty() {
        return String::new();
    }
    format!("{}+{key}", parts.join("+"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typical_combos() {
        let hk = parse_hotkey("Ctrl+Alt+M").unwrap();
        assert_eq!(
            (hk.ctrl, hk.alt, hk.shift, hk.win, hk.vk),
            (true, true, false, false, 'M' as u32)
        );
        let hk = parse_hotkey("shift+win+f5").unwrap();
        assert!(hk.shift && hk.win && !hk.ctrl);
        assert_eq!(hk.vk, 0x74); // F5
        assert_eq!(parse_hotkey("Ctrl+Space").unwrap().vk, 0x20);
        assert_eq!(parse_hotkey("ctrl+alt+7").unwrap().vk, '7' as u32);
    }

    #[test]
    fn rejects_unsafe_or_malformed() {
        assert!(parse_hotkey("").is_none()); // 빈 값 = 핫키 없음
        assert!(parse_hotkey("   ").is_none());
        assert!(parse_hotkey("M").is_none()); // 수정자 없음 — 일반 타이핑을 가로챈다
        assert!(parse_hotkey("Ctrl+Alt").is_none()); // 키가 없음
        assert!(parse_hotkey("Ctrl+뭐지").is_none()); // 알 수 없는 키
        assert!(parse_hotkey("Ctrl+A+B").is_none()); // 키가 둘
        assert!(parse_hotkey("Ctrl+F25").is_none()); // 범위 밖 F키
    }

    #[test]
    fn formats_back_to_canonical_text() {
        let hk = parse_hotkey("alt+ctrl+m").unwrap();
        assert_eq!(hotkey_to_string(&hk), "Ctrl+Alt+M"); // 수정자 순서 정규화
        assert_eq!(
            hotkey_to_string(&parse_hotkey("Ctrl+Shift+F12").unwrap()),
            "Ctrl+Shift+F12"
        );
        assert_eq!(
            hotkey_to_string(&parse_hotkey("Win+Space").unwrap()),
            "Win+Space"
        );
    }

    #[test]
    fn roundtrip_is_stable_for_canonical_text() {
        // 정규 표기(수정자 순서 Ctrl→Alt→Shift→Win)는 왕복해도 그대로여야 한다
        for text in ["Ctrl+Alt+M", "Ctrl+Shift+F1", "Alt+Win+Delete", "Ctrl+9"] {
            let hk = parse_hotkey(text).unwrap();
            assert_eq!(hotkey_to_string(&hk), text, "왕복이 깨졌다: {text}");
        }
        // 비정규 순서로 적어도 읽히고, 저장은 정규 표기로 바뀐다
        assert_eq!(
            hotkey_to_string(&parse_hotkey("Win+Alt+Delete").unwrap()),
            "Alt+Win+Delete"
        );
    }

    #[test]
    fn empty_for_unknown_vk_or_no_modifier() {
        assert_eq!(hotkey_to_string(&Hotkey::default()), ""); // vk 0 = 알 수 없음
        let no_mod = Hotkey {
            vk: 'M' as u32,
            ..Default::default()
        };
        assert_eq!(hotkey_to_string(&no_mod), "");
    }
}
