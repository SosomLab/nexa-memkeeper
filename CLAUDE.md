# CLAUDE.md — Nexa MemKeeper 프로젝트 컨텍스트 (이식용 메모리)

> 이 파일은 **다른 PC에서 clone 시 즉시 컨텍스트를 공유**하기 위한 휴대용 프로젝트 메모리다.
> **먼저 읽기:** [docs/STATUS.md](docs/STATUS.md)(현황) → [docs/10-decision-record.md](docs/10-decision-record.md)(결정).

## 1. 이 프로젝트는

**Nexa MemKeeper** = Windows **시스템 트레이 상주 초경량 메모리 최적화 도구** — 지속 관찰·회수로 여유 메모리 확보, Top 5 점유 팝업·프로세스 트리 뷰(쓰레드)·선택 종료/회수·간단 설정. **도구 자신의 점유 최소화가 정체성**(유휴 RSS ≤5MB 제안). 상세 [docs/00](docs/00-vision.md)·[docs/05](docs/05-requirements.md).
현 단계: **`0.7.2` 릴리스(11-18, winget 게시 완료 · choco 사람 심사 대기)** — `0.7.0`(08-08) 기능 묶음(E-1~E-3·M3-4·X-1~X-3) 위에 콘솔 창 방어(C-1~C-3)·언어 저장 수정(L-1)·시작 후 자동 작업 보류(X-5)·중복 실행 방어(S-1)·트레이 머리줄 버전 수정을 더했다. exe 0.678MB·유휴 WS 1.41MB·테스트 60 green. **효용성 실측([21](docs/21-effectiveness-measurement.md)) 결론: 평상시·중압력 모두 주기 정리는 이득 없음 — 기본값 변경 없음.**

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
| DR-16 | **세션 분담·자동화 개발 규칙 = [docs/22](docs/22-session-and-automation-rules.md)**(11-18) — 개발 Fable × 협업 Opus, 파일 단위 세마포어, push는 개발 세션이 사용자 요청 때만, 자동/고지/질문 3단 경계. DR-13~15는 [docs/10](docs/10-decision-record.md) 참조 |
| DR-17 | **중복 실행 방어 = 세션당 셸 하나**(사용자 확정 11-18) — 두 번째 실행은 조용히 종료하고 기존 셸의 팝업을 연다. 설치본·포터블·개발 빌드 공통 표식, 승격 재시작은 `--restart`로 최대 5초 대기, 워커는 검사하지 않음 |

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

## 5. 다음 단계 (2026-11-18 갱신)

1. **0.7.2 릴리스 완료 · choco 심사 대기**(11-18, 태그 `0.7.2` → `08c03da`, release run 44191276028 success, [journal/2026-11-18](docs/journal/2026-11-18.md)) — GitHub Release 자산 4종 게시 · choco `nexa-memkeeper`·`.portable` 0.7.2 `Ready for review`(자동 검사 통과 · 사람 모더레이션 대기, Scan Note는 무서명 오탐) · winget [#482747](https://github.com/microsoft/winget-pkgs/pull/482747)·[#482748](https://github.com/microsoft/winget-pkgs/pull/482748) **MERGED**(게시 완료). **choco 심사가 끝날 때까지 새 태그를 만들지 않는다** — choco가 지적하면 `choco-resubmit.yml`로 같은 버전을 재제출한다([20 §7-6](docs/20-packaging.md)). 직전 0.7.1은 전 채널 게시 완료(choco Approved · winget #443189·#443190 MERGED).
2. **M5-3 — 설치본 실검증**(사용자 QA): 설치 → 상주 → 제거 왕복([20 §5](docs/20-packaging.md)). P0인데 아직 미확인 — GitHub Release의 `NexaMemKeeper-0.7.2-setup.exe`로 바로 할 수 있고, choco 승인 뒤에는 `choco install/uninstall nexa-memkeeper`로 한다(이 PC에는 Inno Setup이 없어 로컬에서는 설치본을 만들지 못한다).
3. 🔴 **심사 상태 읽는 법**([20 §7-6·§7-7](docs/20-packaging.md)): choco는 피드가 아니라 **패키지 페이지**(`community.chocolatey.org/packages/<id>`) — 피드 조회로는 "조치 대기"를 놓친다(08-08 실수). winget은 체크 아이콘이 아니라 **라벨**(`Validation-Completed`) — 빨간 `✗ 1/19`은 저장소 자체 봇 job 취소일 뿐이다(08-11 확인).
4. 🔴 **패키징 수정은 커밋만으로 끝나지 않는다** — 제출·승인된 버전에 반영됐는지 공개 nupkg로 확인할 것(08-21 수정이 09-02 승인본에 빠졌던 교훈).
5. **0.7.2에 실린 변경**(V1·V2 통과 — exe 0.678MB · 유휴 WS 1.4MB · 외부 DLL 0 · 테스트 60):
   - **콘솔 창 방어**(`3ee60f8`): C-1 워커 생성을 `worker_cmd()` 하나로(디버그 빌드에서 부모 콘솔이 없을 때만 창 없음 플래그) · C-2 디버그/`target\` 경로는 자동 실행 등록 거부 · C-3 choco 포터블 `.gui`(0.7.2 제출 nupkg에 실린 것 확인 — 승인 뒤 공개 nupkg로 한 번 더 대조). "윈도우 시작 시 콘솔 창" 증상 자체는 이 프로그램 원인이 아닐 가능성이 높다(설치본·워커 모두 GUI 서브시스템 실측, 로그온 재현은 미확인).
   - **L-1 언어 저장 누락 수정**(`f89c94e`): `to_ini`가 `lang`을 쓰지 않아 저장 때마다 언어가 기본값으로 돌아갔다.
   - **X-5 시작 후 자동 작업 보류**(`b28ebc2`, 사용자 결정): 트레이는 즉시, 자동 정리·워치리스트 트림만 보류 · 매 시작마다 · 기본 30초 · `[general] start_delay_ms`(0~120000, 0=끔) · 설정 창 '일반'.
   - **S-1 중복 실행 방어**(`32eb2d5`, 사용자 확정 11-18 — DR-17): 세션당 셸 하나(`Local\NexaMemKeeper.Shell`), 두 번째 실행은 기존 팝업을 열고 종료, 승격 재시작은 `--restart`로 최대 5초 대기. **상주본이 떠 있으면 개발 빌드는 곧바로 종료된다**([18 §3-1](docs/18-build-and-test.md) — 중지가 먼저).
   - **사용자 QA 대기**(육안·UAC, 0.7.2로 확인): 설정 창 '시작 지연' 입력란 배치 · 자동 실행 거부 안내 · 관리자 권한 재시작 · 트레이 머리줄 버전 · 팝업 닫은 뒤 RSS 복귀(NFR-3).
6. 다음 할 일: **choco 0.7.2 승인 확인**(패키지 페이지 — 아래 3번 방식, 승인 뒤 공개 nupkg로 `.gui` 대조)과 사용자 QA. 기능 후보(사용자가 고른 것만 착수): 워치리스트 편집 UI(설정 창 자체는 M4에 있음) · X-4(배터리 인식) · X-6(트레이 표시 모드) — [05 §5](docs/05-requirements.md).
7. **유휴 RSS 읽는 법**: 기록값 1.3MB는 "시작 트림 직후 1분" 조건이다. 워킹셋은 5MB 아래에서 1.4~4.8MB로 출렁이고(자기 트림은 5MB 초과 때만) Private는 2.5MB로 일정하다 — 수치가 달라 보이면 같은 조건의 설치본과 비교한다(11-18 A/B).
