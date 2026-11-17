//! config — 설정 파일 위치 결정과 읽기/쓰기(FR-5).
//! **포터블 우선**: exe 옆 `data\settings.ini`. 그 위치에 쓸 수 없으면(Program Files 설치 등)
//! `%LOCALAPPDATA%\NexaMemKeeper\settings.ini`로 폴백한다.
//! 쓰기는 셸·설정 창만, 워커는 읽기 전용(DR-7 — IPC 최소 원칙).

use std::path::PathBuf;

use mk_core::Settings;

/// 실제 사용할 설정 파일 경로. 이미 존재하는 파일이 있으면 그쪽을 우선한다.
pub fn settings_path() -> PathBuf {
    let portable = portable_path();
    if portable.is_file() {
        return portable;
    }
    let fallback = fallback_path();
    if fallback.is_file() {
        return fallback;
    }
    // 둘 다 없으면 새로 쓸 곳 — 포터블 위치에 쓸 수 있는지 실제로 확인
    if can_write(&portable) {
        portable
    } else {
        fallback
    }
}

fn portable_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.join("data").join("settings.ini")))
        .unwrap_or_else(|| PathBuf::from("data/settings.ini"))
}

fn fallback_path() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_default();
    PathBuf::from(base)
        .join("NexaMemKeeper")
        .join("settings.ini")
}

/// 디렉터리를 만들 수 있고 쓰기 가능한지 — 실제로 만들어 보고 판단(권한만 봐서는 알 수 없다).
fn can_write(path: &std::path::Path) -> bool {
    let Some(dir) = path.parent() else {
        return false;
    };
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".mk-write-test");
    match std::fs::write(&probe, b"") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// 설정 로드 — 파일이 없거나 읽기 실패면 기본값(첫 실행 정상 경로).
/// 로드할 때마다 **UI 언어도 함께 적용**한다(프로세스마다 각자 설정을 읽으므로 여기가 유일한 관문).
pub fn load() -> Settings {
    let s = match std::fs::read_to_string(settings_path()) {
        Ok(text) => Settings::parse(&text),
        Err(_) => Settings::default(),
    };
    apply_lang(&s.lang);
    s
}

/// 설정 문자열 → UI 언어(기본 `en`). `auto`면 **OS UI 언어**를 따르되 한국어가 아니면 영어.
fn apply_lang(setting: &str) {
    let lang = mk_core::lang_from_setting(setting).unwrap_or_else(|| {
        // 0x0412 = ko-KR. 하위 워드의 기본 언어 ID만 본다.
        let ui = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
        if ui & 0x3FF == 0x12 {
            mk_core::Lang::Ko
        } else {
            mk_core::Lang::En
        }
    });
    mk_core::set_lang(lang);
}

/// 설정 저장 — 디렉터리 생성 포함. 실패 시 false(호출자가 사용자에게 알린다).
pub fn save(s: &Settings) -> bool {
    let path = settings_path();
    if let Some(dir) = path.parent() {
        if std::fs::create_dir_all(dir).is_err() {
            return false;
        }
    }
    std::fs::write(path, s.to_ini()).is_ok()
}

/// 정리 기록 파일(X-3) — 설정 파일과 **같은 폴더**에 둔다(포터블이면 같이 따라간다).
pub fn history_path() -> std::path::PathBuf {
    settings_path().with_file_name("history.csv")
}

/// 정리 1회를 기록에 남긴다(X-3). 실패해도 조용히 넘어간다 — 기록은 부가 기능이고,
/// 이것 때문에 정리 알림이 늦어지거나 막히면 안 된다.
///
/// 최근 **200줄만** 남긴다. 상주 도구가 파일을 무한정 키우면 그 자체가 민폐다.
pub fn append_history(line: &str) {
    let path = history_path();
    if let Some(dir) = path.parent() {
        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
    }
    let header = "time,trigger,avail_mb,active_mb,partial";
    let mut lines: Vec<String> = std::fs::read_to_string(&path)
        .map(|t| t.lines().map(str::to_string).collect())
        .unwrap_or_default();
    if lines.first().map(|l| l.as_str()) != Some(header) {
        lines.clear();
        lines.push(header.to_string());
    }
    lines.push(line.to_string());
    // 머리글 1줄 + 기록 200줄
    if lines.len() > 201 {
        let cut = lines.len() - 200;
        lines.drain(1..=cut);
    }
    // CRLF로 쓴다 — 메모장으로 열어 보는 사람이 있다(CSV는 그 자리에서 읽혀야 한다)
    let mut text = lines.join("\r\n");
    text.push_str("\r\n");
    let _ = std::fs::write(path, text);
}
