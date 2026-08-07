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
pub fn load() -> Settings {
    match std::fs::read_to_string(settings_path()) {
        Ok(text) => Settings::parse(&text),
        Err(_) => Settings::default(),
    }
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
