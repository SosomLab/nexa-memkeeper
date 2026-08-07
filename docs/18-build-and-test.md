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

## 4. 예산 실측 (DR-8 게이트 — 마일스톤 완료 전 필수)

| 게이트 | 기준 | 측정 |
| --- | --- | --- |
| exe 크기 | ≤ 3MB | `(Get-Item target\release\mk-app.exe).Length / 1MB` |
| 유휴 RSS | ≤ 5MB | 릴리스 exe 실행 → **1분 방치** 후 `Get-Process mk-app \| Select WorkingSet64` (작업 관리자 "메모리" 열과 교차 확인) |
| 외부 DLL 0 | OS 인박스만 | `dumpbin /dependents`(또는 Dependencies 도구)로 임포트 목록 확인 |
| 유휴 CPU | ~0% | 작업 관리자 1분 관찰 — 0.0~0.1% |
| NFR-3 유휴 복귀 | 기능 창 닫은 후 RSS ≤ 유휴+2MB | 팝업/트리 뷰 열고 닫기 3회 → 30초 후 RSS 재측정 |

- 실측값은 **journal에 기록**(수치 없는 "통과" 금지 — 16 규칙 6). 마일스톤 게이트 결과는 MILESTONES에 요약.

## 5. 실기 QA

- 시나리오는 마일스톤 완료 기준([02](02-roadmap.md) 표) 그대로 수행. 발견 건은 `(사용자 QA)` 태그 커밋으로 수정, journal에 재현 조건 기록.
