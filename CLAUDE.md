# CLAUDE.md — Nexa MemKeeper 프로젝트 컨텍스트 (이식용 메모리)

> 이 파일은 **다른 PC에서 clone 시 즉시 컨텍스트를 공유**하기 위한 휴대용 프로젝트 메모리다.
> **먼저 읽기:** [docs/STATUS.md](docs/STATUS.md)(현황) → [docs/10-decision-record.md](docs/10-decision-record.md)(결정).

## 1. 이 프로젝트는

**Nexa MemKeeper** = Windows **시스템 트레이 상주 초경량 메모리 최적화 도구** — 지속 관찰·회수로 여유 메모리 확보, Top 5 점유 팝업·프로세스 트리 뷰(쓰레드)·선택 종료/회수·간단 설정. **도구 자신의 점유 최소화가 정체성**(유휴 RSS ≤5MB 제안). 상세 [docs/00](docs/00-vision.md)·[docs/05](docs/05-requirements.md).
현 단계: **`0.7.1` 패치(09-29, 앱 코드 무변경 — choco 제거 스크립트 수정본 전 채널 재배포)**, 기능 기준선은 **`0.7.0`(08-08)** — 알림 Active 병기(E-1)·주기 정리 비권장 표시(E-2)·고압력 재측정(E-3)·열 폭/창 위치 기억(M3-4)·워치리스트 표적 트림(X-1)·전체화면 억제(X-2)·정리 히스토리(X-3)까지 포함. exe 0.64MB·유휴 RSS 1.34MB·테스트 57 green. **효용성 실측([21](docs/21-effectiveness-measurement.md)) 결론: 평상시·중압력 모두 주기 정리는 이득 없음 — 기본값 변경 없음.**

<!-- 이전: M0~M4 완료(08-07 11차) — 트레이 셸·정리 엔진(영역 ①~⑤)·Top 5 팝업·메인 화면·설정 창(자동 정리 포함). exe 0.51MB·상주 유휴 1.31MB·테스트 37 green. 실기 QA 후 0.5.0 승격 -->

<!-- 이전: M0~M3 완료(08-07 10차) — 트레이 셸 + 정리 엔진 + Top 5 팝업 + 메인 화면(평면/트리·검색·정렬·쓰레드/트리 종료·앱 아이콘). 상주 유휴 1.31MB·exe 0.47MB. 실기 QA 후 0.4.0 승격 -->

- 설정 SSOT = [docs/09](docs/09-settings-schema.md), 파일 = exe 옆 `data\settings.ini`(폴백 `%LOCALAPPDATA%`). 파싱·검증은 mk-core, I/O는 mk-app `config.rs`.

<!-- 이전 단계 기록: M0·M1·M2 완료(08-07 7차) — 트레이 셸 + 정리 엔진(+209MB 실측) + Top 5 팝업(열거 6.5ms) 동작, 예산 유지(exe 0.36MB·유휴 RSS 2.23MB·NFR-3 복귀 확인). 실기 QA 후 0.3.0 승격 -->

- 실행 파일명은 **`NexaMemKeeper.exe`**(DR-12) — `mk-app`은 내부 crate 이름일 뿐, 프로세스·명령에는 배포명을 쓴다.

- 조직: **SosomLab** · 개발자: Sangyong Bae · kiros33@gmail.com
- 문서·git 규약은 `SosomLab/nexa-dir2`의 검증 표준을 전면 계승([docs/16](docs/16-doc-git-conventions.md)).

## 2. 확정 결정 ([docs/10](docs/10-decision-record.md), 변경 시 새 항목으로 정정)

| # | 결정 |
| --- | --- |
| DR-1 | 라이선스 = **PolyForm Noncommercial 1.0.0** — 개인·비상업 무료, 상업은 유료(kiros33@sosomlab.com). 영문 정본 [LICENSE.md](LICENSE.md) + 한글본 [LICENSE.ko.md](LICENSE.ko.md) |
| DR-2 | 문서·git 규약 = `nexa-dir2` 표준 계승 — SSOT는 [docs/16](docs/16-doc-git-conventions.md) |
| DR-3 | 프로그램 목적 확정(사용자 확정 08-07) — 위 §1. 스펙 SSOT = [docs/05](docs/05-requirements.md) |
| DR-4 | **판단 우선순위 = 성능·동작 안정성 > 개발·배포 편의성**(사용자 확정 08-07) — 모든 ADR 트레이드오프 판정 기준 |
| DR-5 | 스택 = **Rust + windows-rs, Win32 직접·커스텀 드로잉**(ADR-0001). 관리 런타임·GC 금지, 외부 crate 0 지향(windows 계열 제외 — 추가는 docs/10 §3 원장) |
| DR-6 | 관찰·회수 기법 = ADR-0002 — 열거 `NtQuerySystemInformation` 1콜, 회수 영역 ①~⑥, asInvoker+온디맨드 승격 |
| DR-7 | 상주 구조 = **하이브리드** — 셸(트레이·관찰·Top 5 팝업) + 온디맨드 워커(`--worker`, 트리 뷰·정리 엔진 — 100% 회수·크래시 격리) |
| DR-8 | **예산 게이트**: 유휴 RSS ≤5MB · exe ≤3MB · 외부 DLL 0 · 유휴 CPU ~0% — 초과 시 main 병합 금지([docs/18 §4](docs/18-build-and-test.md)) |
| DR-9 | **배포 2채널**: 설치형 exe(Inno — 프로그램 목록 등록·자동 실행 질문 **기본 체크**) + 포터블 ZIP(무서명 오탐 회피). 구현 M5 |
| DR-10 | **목록 창 = 메인 화면** — 메뉴바 + 트리 그리드(확장·정렬) + 프로세스/트리/쓰레드 종료. 구조는 온디맨드 워커 유지(DR-7) |
| DR-11 | **자동 실행 = `HKCU\…\Run`** — 포터블 원칙의 명시적 예외(사용자가 메뉴로 켤 때만 값 1개, 끄면 삭제) |
| DR-12 | **exe = `NexaMemKeeper.exe` + 버전 리소스**(`FileDescription=Nexa MemKeeper`) — 알림·작업 관리자에 내부명 노출 금지 |

## 3. 작업 규약 (요약 — SSOT는 [docs/16](docs/16-doc-git-conventions.md))

- **문서 4층**: 진입(CLAUDE.md) / 현황(STATUS·MILESTONES·TODO) / 경과(DEVLOG·journal·BRANCHES) / 지식(NN-주제·ADR) + docs/README.md 색인.
- **journal이 원본, 나머지는 요약 + 링크.** 진행 기록은 전부 시간 역순(최신 위). 문서 번호(NN-)는 불변, 신규는 append.
- **한 작업 = 한 트랜잭션 갱신**: 커밋 → journal → DEVLOG → MILESTONES/TODO → (브랜치면) BRANCHES.
- **커밋**: Conventional Commits `type(scope): 제목` + 맥락 태그(`(사용자 요청)` 등). scope = `core`/`shell`/`worker`/`app`/`pkg`([docs/15 §2](docs/15-dev-methodology.md)). 단위 = 커밋 1개, 수직 슬라이스, main 항상 green(fmt·clippy·test — [docs/18](docs/18-build-and-test.md)).
- **브랜치·푸시**: 큰 단위 = 브랜치, 세부 = 커밋. 🔴 **push는 사용자 명시 요청 시에만**, 태그 push는 별도 승인, 파괴적 작업은 실행 전 확인. 그 외 일상 작업은 자동 진행.
- `.claude/settings.json`(권한)은 **덮어쓰기 금지, 병합만**.
- 기록에는 "왜 + 실측값". 설계 전 기존 문서·코드 먼저 확인(재발명 금지), 이식 시 출처 경로를 커밋 본문에 명기.
- **코드 수정 후에는 릴리스 빌드 → 실행 중이면 중지 → 재실행**(사용자 확정 08-07 — 실기 확인용, 절차 [docs/18 §3-1](docs/18-build-and-test.md)).
- **세션 분담**(SSOT [docs/22](docs/22-session-and-automation-rules.md) Ⅰ부 · 전역 규칙 사본 [docs/claude-global-rules.md](docs/claude-global-rules.md) · 원천 nexa-sql `docs/102` d526d23e, 2026-11-18 정리): 개발 세션(Fable) = 설계 · 원인 분석 · 다층 변경 · 릴리스, 소유 `crates/**` · `Cargo.*` · `.github/**` · `packaging/**` · `scripts/**` · CLAUDE.md · docs/22. 협업 세션(Opus) = 상태 점검 · 조사 · 빌드/시험/실측 **실행** · 기록 · 간단 수정, 소유 `docs/**` · README · Wiki. 한 파일은 한 세션만 고친다(`[SEM 요청]` → 승인 → 해제). 위임 첫 줄 = `[P0~P3]`. 검증 V0(개발 `cargo check`) → V1(협업 fmt · clippy · test) → V2(협업 릴리스 빌드 · 재기동 · 예산 실측) → V3(릴리스 전). 같은 `target/` 동시 사용 금지 · HEAD를 바꾸기 전 상대에게 통지 · **push는 개발 세션이 사용자 요청 때만**(협업은 commit까지).
- **자동화 개발 규칙**(SSOT [docs/22](docs/22-session-and-automation-rules.md) Ⅱ부): 프로젝트 내부 작업은 묻지 않고 진행하고, **push · 태그 · 패키지 제출 · 이력 파괴 · 프로젝트 밖 사용자 리소스(레지스트리 · 설치본) · 다른 앱 프로세스 · DR/예산 기준 변경은 먼저 묻는다**. 질문 하나 = 결정 하나 + 권장안. 검증 체인은 `&&`만 · 게이트를 낮춰 통과시키지 않는다 · 같은 실패 두 번이면 원인 분석으로. 새 기능 후보는 사용자가 고른 것만 착수 · 범위 확장 금지. 무인 구간은 사용자가 기간을 명시했을 때만.

## 4. 새 세션 오리엔테이션

0. 역할(개발 / 협업) 확인 → `ListAgents`로 짝 세션과 역할 · 파일 · 작업 트리 경로 교환 → `bash scripts/install-global-rules.sh --check`([docs/22 §6](docs/22-session-and-automation-rules.md)).
1. 이 CLAUDE.md + [docs/STATUS.md](docs/STATUS.md) → 2. [DEVLOG](docs/DEVLOG.md) 최상단 + 최신 journal → 3. 할 일 = [docs/TODO.md](docs/TODO.md)(P-1부터 순차).

## 5. 다음 단계 (2026-09-29 갱신)

1. **0.7.1 심사 대기(할 일 없음)** — choco `nexa-memkeeper`·`.portable` 0.7.1(`Pending Automated Review`) · winget [#443189](https://github.com/microsoft/winget-pkgs/pull/443189)·[#443190](https://github.com/microsoft/winget-pkgs/pull/443190). 0.7.0은 전 채널 게시 완료(winget 09-01, choco 09-02·03). 단 **choco 설치형 0.7.0은 제거 스크립트가 깨진 08-11본** — 0.7.1이 대체한다([journal/2026-09-29](docs/journal/2026-09-29.md)).
2. **M5-3 — 설치본 실검증**: [최신 릴리스](https://github.com/SosomLab/nexa-memkeeper/releases/latest) 자산으로 설치 → 상주 → 제거 왕복([20 §5](docs/20-packaging.md)). P0인데 아직 미확인 — choco 0.7.1 승인 후 `choco install/uninstall`로 하면 제거 수정도 함께 확인된다.
3. 🔴 **심사 상태 읽는 법**([20 §7-6·§7-7](docs/20-packaging.md)): choco는 피드가 아니라 **패키지 페이지**(`community.chocolatey.org/packages/<id>`) — 피드 조회로는 "조치 대기"를 놓친다(08-08 실수). winget은 체크 아이콘이 아니라 **라벨**(`Validation-Completed`) — 빨간 `✗ 1/19`은 저장소 자체 봇 job 취소일 뿐이다(08-11 확인).
4. 🔴 **패키징 수정은 커밋만으로 끝나지 않는다** — 제출·승인된 버전에 반영됐는지 공개 nupkg로 확인할 것(08-21 수정이 09-02 승인본에 빠졌던 교훈).
5. 남은 기능 후보: X-1 **설정 UI**(현재 파일 설정만) · X-4(배터리 인식) · X-5(부팅 후 지연 시작) · X-6(트레이 표시 모드) — [05 §5](docs/05-requirements.md).
