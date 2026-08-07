# 18 · 빌드 & 테스트 (SSOT)

> 빌드·테스트·예산 실측 절차의 단일 원천. **절차를 바꾼 그 커밋에서 이 문서를 함께 갱신**한다(16 규칙 4).

## 1. 환경

- Rust **stable** + `rustfmt`·`clippy`([rust-toolchain.toml](../rust-toolchain.toml)). Windows: MSVC 타깃(`x86_64-pc-windows-msvc`).
- 실행·QA·예산 실측은 **Windows 실기**가 신뢰 원천. mk-core 단위 테스트는 플랫폼 무관.

## 2. 일상 검증 (커밋 전 — 전부 green이어야 main 병합 가능)

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- 비Windows 경로 검증(nexa-dir2 08-02 교훈 계승 — cfg 소거 경로 미검증 방지, 크로스 체크 가능 환경에서):
  `cargo check --workspace --all-targets --target x86_64-unknown-linux-gnu`

## 3. 릴리스 빌드

```powershell
cargo build --release
# 산출물: target\release\mk-app.exe (배포명 NexaMemKeeper.exe로 rename — pkg 단계)
```

- 프로파일([Cargo.toml](../Cargo.toml)): `opt-level=3`·`lto=fat`·`codegen-units=1`·`panic=abort`·`strip=symbols` — `nexa-dir2` 검증 설정 계승(성능 우선, DR-4).

### 3-1. 코드 수정 후 재기동 (사용자 확정 08-07 — 매 수정 사이클의 마지막 단계)

**코드를 고쳤으면 릴리스 빌드 → 실행 중이면 중지 → 재실행**까지 한 번에 수행한다. 사용자가 트레이에서 곧바로 실기 확인을 하므로, 예전 바이너리가 상주해 있으면 안 된다(디버그 빌드는 콘솔이 뜨고 예산 수치도 달라 실기 확인 기준이 될 수 없다). 문서만 고친 경우는 불필요.

**순서 주의 — 중지가 먼저다.** 실행 중인 exe는 Windows가 파일을 잠그기 때문에, 빌드를 먼저 하면 `액세스가 거부되었습니다(os error 5)`로 링크에 실패한다(08-07 실측). 더 나쁜 경우는 **빌드 실패를 못 보고 옛 바이너리를 그대로 재실행**하는 것이다.

```powershell
Stop-Process -Name NexaMemKeeper -Confirm:$false -ErrorAction SilentlyContinue
cargo build --release
Start-Process target\release\NexaMemKeeper.exe
```

> 산출물 이름은 **`NexaMemKeeper.exe`**(DR-12 — `[[bin]] name`). 버전 리소스는 `build.rs`가 Windows SDK `rc.exe`로 임베드하며, rc.exe가 없으면 경고만 남기고 건너뛴다(빌드는 성공).

## 4. 예산 실측 (DR-8 게이트 — 마일스톤 완료 전 필수)

| 게이트 | 기준 | 측정 |
| --- | --- | --- |
| exe 크기 | ≤ 3MB | `(Get-Item target\release\mk-app.exe).Length / 1MB` |
| 유휴 RSS | ≤ 5MB | 릴리스 exe 실행 → **1분 방치** 후 `Get-Process mk-app \| Select WorkingSet64` (작업 관리자 "메모리" 열과 교차 확인) |
| 외부 DLL 0 | OS 인박스만 | `dumpbin /dependents`(또는 Dependencies 도구)로 임포트 목록 확인 |
| 유휴 CPU | ~0% | 작업 관리자 1분 관찰 — 0.0~0.1% |
| NFR-3 유휴 복귀 | 기능 창 닫은 후 RSS ≤ 유휴+2MB | 팝업/트리 뷰 열고 닫기 3회 → 30초 후 RSS 재측정 |

- 실측값은 **journal에 기록**(수치 없는 "통과" 금지 — 16 규칙 6). 마일스톤 게이트 결과는 MILESTONES에 요약.

## 4-1. ⚠️ 측정 함정 (08-07 실측 교훈)

- **PowerShell로 릴리스 exe의 종료 코드를 재지 마라** — 릴리스는 GUI 서브시스템(`windows_subsystem`)이라 `& $exe`가 **대기하지 않고 즉시 0을 반환**한다. 워커 종료 코드 확인은 **디버그 빌드**(콘솔 있음)로 하거나 `Start-Process -Wait` 사용. 셸의 `Command::status()`는 서브시스템과 무관하게 정상 대기한다.
- 워커 진단은 stderr 1줄(`clean: trimmed=… purge_status=… avail …`) — 디버그 빌드를 콘솔에서 실행할 때만 보인다.
- **트레이 클릭 시뮬레이션은 `WM_APP+1 = 0x8001`** — `WM_USER+1`(0x0401)로 보내면 조용히 무시된다(08-07 M2에서 오진 유발). lParam = `WM_LBUTTONUP`(0x0202) / `WM_RBUTTONUP`(0x0205).
- 열거 성능 측정: `mk-app.exe --worker bench` — 5회 반복해 프로세스 수·소요를 stderr에 출력(NFR-4 게이트).
- 창 핸들은 `FindWindowW`보다 **EnumWindows + GetClassNameW**로 찾는 편이 확실하다(클래스 `NexaMemKeeperShell`·`NexaMemKeeperPopup`).

## 5. 실기 QA

- 시나리오는 마일스톤 완료 기준([02](02-roadmap.md) 표) 그대로 수행. 발견 건은 `(사용자 QA)` 태그 커밋으로 수정, journal에 재현 조건 기록.
