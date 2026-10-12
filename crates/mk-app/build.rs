//! 빌드 스크립트 — Windows 버전 리소스(VERSIONINFO) 임베드.
//!
//! **왜 필요한가**: 트레이 풍선 알림의 앱 이름·작업 관리자 설명·파일 속성은 exe의
//! `FileDescription`에서 온다. 없으면 Windows가 파일명(`mk-app.exe`)을 그대로 노출한다.
//!
//! **왜 crate를 쓰지 않는가**: DR-5(외부 crate 0 지향). Windows SDK의 `rc.exe`를 직접 찾아
//! 컴파일한다. rc.exe가 없는 환경(비Windows·SDK 미설치 CI)에서는 **조용히 건너뛰고**
//! 빌드는 성공시킨다 — 버전 정보는 배포 빌드 품질 항목이지 컴파일 조건이 아니다.

use std::path::{Path, PathBuf};

const PRODUCT: &str = "Nexa MemKeeper";
const COMPANY: &str = "SosomLab";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // 제품명을 앱 코드에도 넘긴다(`env!("MK_PRODUCT_NAME")`) — 트레이 메뉴 머리줄이 버전 리소스와
    // 같은 출처를 쓰게 하려는 것. 플랫폼 분기 전에 내보내야 비Windows `cargo check`도 통과한다.
    println!("cargo:rustc-env=MK_PRODUCT_NAME={PRODUCT}");
    if std::env::var("CARGO_CFG_WINDOWS").is_err() {
        return;
    }
    let Some(rc) = find_rc() else {
        println!("cargo:warning=rc.exe를 찾지 못해 버전 정보를 넣지 않았습니다(기능 영향 없음).");
        return;
    };
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let ver = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into());
    let quad: Vec<&str> = ver.split('.').collect();
    let (a, b, c) = (
        quad.first().copied().unwrap_or("0"),
        quad.get(1).copied().unwrap_or("0"),
        quad.get(2).copied().unwrap_or("0"),
    );

    // 앱 아이콘(assets/nexa-memkeeper.ico — scripts/make-icon.ps1이 생성). rc는 역슬래시를
    // 이스케이프해야 하므로 슬래시로 통일한다. 파일이 없으면 아이콘 줄을 빼고 버전 정보만 넣는다.
    let icon_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("nexa-memkeeper.ico");
    println!("cargo:rerun-if-changed={}", icon_path.display());
    let icon_line = if icon_path.is_file() {
        format!(
            "1 ICON \"{}\"\n",
            icon_path.display().to_string().replace('\\', "/")
        )
    } else {
        println!("cargo:warning=앱 아이콘(assets/nexa-memkeeper.ico)이 없어 임베드를 건너뜁니다.");
        String::new()
    };

    // 애플리케이션 매니페스트(RT_MANIFEST=24, ID 1) — 공용 컨트롤 v6·asInvoker.
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("app.manifest");
    println!("cargo:rerun-if-changed={}", manifest_path.display());
    let manifest_line = if manifest_path.is_file() {
        format!(
            "1 24 \"{}\"\n",
            manifest_path.display().to_string().replace('\\', "/")
        )
    } else {
        String::new()
    };

    let rc_src = out.join("version.rc");
    let res = out.join("version.res");
    std::fs::write(
        &rc_src,
        format!(
            r#"{icon_line}{manifest_line}1 VERSIONINFO
FILEVERSION {a},{b},{c},0
PRODUCTVERSION {a},{b},{c},0
FILEOS 0x4
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "{COMPANY}"
      VALUE "FileDescription", "{PRODUCT}"
      VALUE "FileVersion", "{ver}"
      VALUE "InternalName", "NexaMemKeeper"
      VALUE "LegalCopyright", "Copyright (c) 2026 {COMPANY} - PolyForm Noncommercial 1.0.0"
      VALUE "OriginalFilename", "NexaMemKeeper.exe"
      VALUE "ProductName", "{PRODUCT}"
      VALUE "ProductVersion", "{ver}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
        ),
    )
    .expect("version.rc 작성 실패");

    let status = std::process::Command::new(&rc)
        .args(["/nologo", "/fo"])
        .arg(&res)
        .arg(&rc_src)
        .status();
    match status {
        Ok(s) if s.success() => println!("cargo:rustc-link-arg-bins={}", res.display()),
        _ => println!("cargo:warning=rc.exe 실행 실패 — 버전 정보를 건너뜁니다."),
    }
}

/// Windows SDK에서 최신 rc.exe 탐색(x64 호스트 우선).
fn find_rc() -> Option<PathBuf> {
    let roots = [
        r"C:\Program Files (x86)\Windows Kits\10\bin",
        r"C:\Program Files\Windows Kits\10\bin",
    ];
    let mut best: Option<PathBuf> = None;
    for root in roots {
        let Ok(entries) = std::fs::read_dir(Path::new(root)) else {
            continue;
        };
        for e in entries.flatten() {
            let cand = e.path().join("x64").join("rc.exe");
            if cand.is_file() {
                // 디렉터리 이름이 SDK 버전 — 사전순 최대가 최신
                if best.as_ref().is_none_or(|b| b < &cand) {
                    best = Some(cand);
                }
            }
        }
    }
    best
}
