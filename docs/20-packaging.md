# 20 · 배포 (SSOT)

> 배포 산출물·절차의 단일 원천(DR-9). **배포 방식을 바꾼 그 커밋에서 이 문서를 함께 갱신**한다(16 규칙 4).
> 빌드·예산 게이트 자체는 [18](18-build-and-test.md), 설정 파일 위치 규칙은 [09](09-settings-schema.md).

## 1. 왜 2채널인가 (DR-9)

| 채널 | 파일 | 쓰는 상황 |
| --- | --- | --- |
| **설치형** | `NexaMemKeeper-<ver>-setup.exe` | 보통 사용자. 프로그램 목록 등록·바로 가기·**자동 실행 질문(기본 체크)** |
| **설치형 ZIP** | `NexaMemKeeper-<ver>-setup.zip` | 브라우저가 exe 다운로드를 막을 때(설치본 + 실행 안내) |
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

## 6. GitHub Actions (CI·릴리스)

| 워크플로 | 트리거 | 하는 일 |
| --- | --- | --- |
| [`ci.yml`](../.github/workflows/ci.yml) | push·PR | windows: `fmt --check` · `clippy -D warnings` · `test` · 릴리스 빌드 · **예산 게이트 + 포터블 ZIP**(같은 `pack.ps1`) / ubuntu·macos: `test`(mk-core가 플랫폼에 얽히지 않았는지) |
| [`release.yml`](../.github/workflows/release.yml) | 태그 `0.5.0`·`v0.5.0` push | 테스트 → 빌드 → `pack.ps1`(게이트·ZIP·**설치본**·`SHA256SUMS.txt`) → choco nupkg 팩 → **GitHub Release 생성·자산 첨부** → (조건부) 패키지 매니저 게시 |

- 게이트를 CI와 로컬이 **같은 스크립트**로 태운다 — 기준이 갈라지지 않는다.
- `windows-latest` 이미지에 Inno Setup 6이 들어 있어 러너에서 설치본까지 만들어진다.
- 태그 push는 **사용자 승인 필수**(16 규칙). 워크플로는 준비만 되어 있다.

## 7. 패키지 매니저 등록 (winget · Chocolatey)

두 채널 모두 **GitHub Release 자산을 URL+체크섬으로 참조**한다. 바이너리를 패키지에 동봉하지 않는
이유: 라이선스가 PolyForm Noncommercial(비-FOSS)이라 커뮤니티 저장소 재배포 규정상 **공식 URL
다운로드가 정석**이고, 그래야 배포본이 한 곳(Release)에서만 갈라진다.

### 7-1. 자동 게시는 기본 꺼짐 (사용자 지시 08-07)

| 채널 | 켜는 조건 | 끈 상태에서 하는 일 |
| --- | --- | --- |
| Chocolatey | 저장소 변수 `CHOCO_PUSH=true` **+** 시크릿 `CHOCO_API_KEY` | `.nupkg`까지 만들어 아티팩트로 남김(push 안 함) — **08-08부터 켜져 있다**: 태그를 밀면 자동 push된다 |
| winget | 저장소 변수 `WINGET_PUBLISH=true` **+** 시크릿 `WINGET_TOKEN` | 아무것도 하지 않음 |

준비되면 **Settings > Secrets and variables**에 값만 넣으면 된다(워크플로 수정 불필요).
모더레이션 승인 대기 중 이중 큐를 만들지 않으려고 기본을 꺼 뒀다.

### 7-2. 왜 `/ALLUSERS`인가

패키지 매니저는 **승격된 세션에서 조용히** 설치한다. 우리 설치기는 기본이 사용자별
(`PrivilegesRequired=lowest`)이라 그대로 두면 **관리자 계정의 `%LOCALAPPDATA%`**에 깔려 정작
사용자에게는 보이지 않는다. 그래서 `PrivilegesRequiredOverridesAllowed=commandline`을 열어 두고
choco·winget 매니페스트가 `/ALLUSERS`를 넘긴다. 자동 실행도 `/TASKS=""`로 **끄고 설치**한다 —
설치 계정과 사용할 계정이 다를 수 있어, 자동 실행은 각자 트레이 메뉴에서 켜는 편이 맞다.

### 7-3. 패키지 구성

| 파일 | 내용 |
| --- | --- |
| [`packaging/chocolatey/nexa-memkeeper/`](../packaging/chocolatey/nexa-memkeeper/) | 설치형. `Install-ChocolateyPackage` + Inno 무인 인자. 제거 시 설정 보존 |
| [`packaging/chocolatey/nexa-memkeeper.portable/`](../packaging/chocolatey/nexa-memkeeper.portable/) | 포터블. ZIP을 tools에 푼다(제거 시 설정도 함께 사라짐 — 설명에 명시) |
| [`packaging/winget/SosomLab.NexaMemKeeper/`](../packaging/winget/SosomLab.NexaMemKeeper/) | winget **설치형** 매니페스트 3종. `InstallerType: inno` · `/ALLUSERS`로 머신 전역 |
| [`packaging/winget/SosomLab.NexaMemKeeper.Portable/`](../packaging/winget/SosomLab.NexaMemKeeper.Portable/) | winget **포터블** 매니페스트 3종. `InstallerType: zip` + `NestedInstallerType: portable` — ZIP 안의 exe를 `nexamemkeeper` 별칭으로 등록(사용자 범위) |

`{{VERSION}}`·`{{CHECKSUM64}}` 자리표시자는 **릴리스 워크플로가 태그 시점에 치환**한다. 손으로 채우지 않는다.

### 7-4. 포터블을 winget에서 따로 내는 이유

winget 패키지 하나에 설치형과 포터블을 함께 담을 수 없다 — 자산도 설치 방식도 범위도 다르기 때문이다.
그래서 **`SosomLab.NexaMemKeeper`(설치형, machine)**와 **`SosomLab.NexaMemKeeper.Portable`(포터블, user)**
두 패키지로 낸다. Chocolatey도 같은 이유로 `nexa-memkeeper` / `nexa-memkeeper.portable` 2패키지다.

포터블 쪽은 `InstallerType: zip` + `NestedInstallerType: portable`로, ZIP 안의 `NexaMemKeeper.exe`를
**`nexamemkeeper` 명령 별칭**으로 등록한다(승격 불필요). 릴리스 워크플로는 게시가 켜져 있을 때
두 패키지 각각에 PR을 만든다.

> ✅ **08-08 Chocolatey 등록 완료(심사 대기)** — `nexa-memkeeper` 0.7.0 · `nexa-memkeeper.portable` 0.7.0을
> push했다. 커뮤니티 저장소는 자동 검사 + 사람 모더레이션을 거치며, 통과 전에는 `choco install`로 보이지
> 않는다([큐 확인](https://ch0.co/moderation)). 이후 태그부터는 변수가 켜져 있어 **자동으로 새 버전이 올라간다**.

> ⚠️ **winget 토큰에는 `workflow` 스코프가 필요하다**(08-08 실측). `wingetcreate submit`은 토큰 계정의
> `winget-pkgs` **포크**를 통해 PR을 여는데, upstream에 워크플로 파일 변경이 포함돼 있어 동기화에
> `workflow` 권한이 요구된다. 없으면 매니페스트 검증까지는 통과하고 마지막에
> `The forked repository could not be synced with the upstream commits`로 멈춘다.
> 해결: PAT를 **`repo` + `workflow`**로 재발급 → 시크릿 `WINGET_TOKEN` 갱신 → `winget submit` 재실행.

### 7-5. 첫 등록에 필요한 사람 손 (자동화 밖)

1. **Release가 먼저** — 매니페스트가 참조할 자산 URL·해시가 있어야 한다.
2. **winget**: `microsoft/winget-pkgs`에 최초 PR 1회(계정·포크 필요). 이후는 `wingetcreate`가 갱신.
3. **Chocolatey**: 계정 + API 키 발급 → 첫 패키지는 **모더레이션 심사**(무서명 설치본은 지적이 잦다).
4. 심사 통과 후 저장소 변수 `CHOCO_PUSH` / `WINGET_PUBLISH`를 `true`로.

### 7-6. Chocolatey 지적 대응 — **같은 버전**을 다시 올린다

모더레이션이 `Waiting for Maintainer to take corrective action`이면 자동 검사가 **Requirements**를
잡은 것이다. Requirements는 반드시 고쳐야 승인된다(Notes는 안 고쳐도 승인 가능).
🔴 이때 **새 버전을 올리면 안 된다** — 규정은 *"The exact same version should be uploaded during
moderation review"*, 즉 **같은 버전 번호로 덮어쓴다**(모더레이션 중인 버전만 덮어쓸 수 있다).

태그를 새로 달지 않고 고치기 위한 수단이
[`choco-resubmit.yml`](../.github/workflows/choco-resubmit.yml)이다 —
**Actions > "choco resubmit" > Run workflow**(버전 입력, 비우면 최신 릴리스 태그).
릴리스 자산을 다시 받아 해시를 계산하므로 재제출본이 최초 제출본과 **같은 바이너리**를 가리킨다.
게시 스위치 규약은 §7-1과 같다(변수·시크릿이 없으면 nupkg만 만들고 멈춤).

**08-08 지적 이력**(2패키지 동일):

| 등급 | 내용 | 조치 |
| --- | --- | --- |
| Requirement | `An email address was found in the field Description.` — nuspec에 이메일 금지 | 설명의 상업 라이선스 문의처를 **[LICENSE.md](../LICENSE.md) 링크로 교체**(연락처 자체는 라이선스 문서에 남는다) |
| Note | `owners`(관리자)와 `authors`(제작자)가 같음 | 조치 불필요 — 실제로 동일 주체다. 모더레이터가 확인한다 |

## 8. 서명 (미정)

코드 서명 인증서는 아직 없다. 확보하면 `signtool`을 `pack.ps1`의 게이트 통과 직후에 끼워 넣고,
설치본 서명 → SmartScreen 평판 축적 순으로 간다. 그 전까지 포터블 ZIP이 실질적인 1차 채널이다.
