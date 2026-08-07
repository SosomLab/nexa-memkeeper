# 20 · 배포 (SSOT)

> 배포 산출물·절차의 단일 원천(DR-9). **배포 방식을 바꾼 그 커밋에서 이 문서를 함께 갱신**한다(16 규칙 4).
> 빌드·예산 게이트 자체는 [18](18-build-and-test.md), 설정 파일 위치 규칙은 [09](09-settings-schema.md).

## 1. 왜 2채널인가 (DR-9)

| 채널 | 파일 | 쓰는 상황 |
| --- | --- | --- |
| **설치형** | `NexaMemKeeper-<ver>-setup.exe` | 보통 사용자. 프로그램 목록 등록·바로 가기·**자동 실행 질문(기본 체크)** |
| **포터블** | `NexaMemKeeper-<ver>-portable.zip` | 설치가 막힌 환경·USB 휴대. 압축을 풀고 바로 실행 |

무서명 exe는 SmartScreen이 **설치 단계에서** 막는 일이 잦다. 포터블 ZIP은 그 단계를 통째로 건너뛰므로,
서명 인증서를 마련하기 전까지 **차단당한 사용자의 탈출구**가 된다. 두 채널 모두 같은 `NexaMemKeeper.exe`
하나를 담는다 — 산출물이 갈라지지 않는다.

## 2. 만들기

```powershell
pwsh packaging\pack.ps1                 # 릴리스 빌드 → 예산 게이트 → ZIP (+ Inno 있으면 설치본)
pwsh packaging\pack.ps1 -SkipBuild      # 이미 빌드된 산출물로 포장만
pwsh packaging\pack.ps1 -SkipInstaller  # ZIP만
```

- 결과물은 `target\pack\`에 떨어진다(빌드 산출물이라 git에 올리지 않는다).
- **버전은 워크스페이스 `Cargo.toml` 한 곳**에서만 읽는다. 파일명·설치 정보·버전 리소스가 여기서 갈라진다.
- 게이트(DR-8)를 통과하지 못하면 **포장하지 않고 중단**한다 — exe > 3MB, 또는 OS 기본이 아닌 DLL 의존이
  발견되면 실패. DLL 검사는 `dumpbin`이 있을 때만 수행하므로, 릴리스 전에는 **Developer PowerShell**에서 한 번 돌린다.
- 설치본은 [Inno Setup 6](https://jrsoftware.org/isdl.php)의 `ISCC.exe`가 있을 때만 만들어진다. 없으면
  경고만 남기고 ZIP까지 완료한다(배포 자체는 성립).

## 3. 설치형이 하는 일 ([packaging/NexaMemKeeper.iss](../packaging/NexaMemKeeper.iss))

- **사용자별 설치**(`PrivilegesRequired=lowest`) — 앱이 asInvoker로 돌고(DR-6) 자동 실행도 HKCU(DR-11)라
  승격이 필요 없다. UAC 창이 뜨지 않고, 제거 목록에는 사용자 항목으로 등록된다.
- 설치 위치가 `%LOCALAPPDATA%\Programs\Nexa MemKeeper`라 **exe 옆 `data\settings.ini`가 그대로 쓰인다**
  (포터블과 같은 경로 규칙 — `config.rs`). Program Files였다면 `%LOCALAPPDATA%` 폴백을 탔을 것이다.
- **자동 실행 질문은 기본 체크**(DR-9). 켜면 `HKCU\…\Run`에 값 **1개**, 제거하면 그 값도 지운다.
  앱의 [자동 실행] 메뉴가 쓰는 값과 **같은 이름**이라 설치 후 메뉴로 끄면 그대로 해제된다.
- `AppId` GUID는 **절대 바꾸지 않는다** — 바뀌면 갱신 설치가 안 되고 목록에 중복 등록된다.
- 제거 시 `taskkill`로 상주 인스턴스를 먼저 내린다(파일 잠금 방지).

## 4. 포터블이 하는 일

`NexaMemKeeper.exe` + `README.md` + `LICENSE.md` + `LICENSE.ko.md` + `PORTABLE.txt`.
설정은 exe 옆 `data\settings.ini`에 생기므로 **폴더째 옮기면 설정도 따라온다**. 읽기 전용 위치면
`%LOCALAPPDATA%\NexaMemKeeper`로 폴백한다. 자동 실행은 꺼진 상태이며, 트레이 메뉴에서 켤 수 있다.

## 5. 릴리스 체크리스트

1. `cargo fmt` · `cargo clippy -- -D warnings` · `cargo test` (main green — 18 §2)
2. **Developer PowerShell**에서 `pwsh packaging\pack.ps1` — 예산 게이트 + DLL 검사 통과 확인
3. 설치본 왕복 검증: 설치 → 트레이 상주 → 자동 실행 값 확인 → 제거 → **값·바로 가기 잔여 0**
   (설정 `data\settings.ini`는 일부러 남긴다. 완전 삭제하려면 설치 폴더의 `data`를 지운다)
4. 포터블 왕복 검증: 압축 해제 → 실행 → 폴더 이동 후에도 설정 유지
5. 태그·릴리스 노트 — 태그 push는 **사용자 승인 필수**(16 규칙)

## 6. 서명 (미정)

코드 서명 인증서는 아직 없다. 확보하면 `signtool`을 `pack.ps1`의 게이트 통과 직후에 끼워 넣고,
설치본 서명 → SmartScreen 평판 축적 순으로 간다. 그 전까지 포터블 ZIP이 실질적인 1차 채널이다.
