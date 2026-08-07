# CLAUDE.md — Nexa MemKeeper 프로젝트 컨텍스트 (이식용 메모리)

> 이 파일은 **다른 PC에서 clone 시 즉시 컨텍스트를 공유**하기 위한 휴대용 프로젝트 메모리다.
> **먼저 읽기:** [docs/STATUS.md](docs/STATUS.md)(현황) → [docs/10-decision-record.md](docs/10-decision-record.md)(결정).

## 1. 이 프로젝트는

**Nexa MemKeeper** = Windows **시스템 트레이 상주 초경량 메모리 최적화 도구** — 지속 관찰·회수로 여유 메모리 확보, Top 5 점유 팝업·프로세스 트리 뷰(쓰레드)·선택 종료/회수·간단 설정. **도구 자신의 점유 최소화가 정체성**(유휴 RSS ≤5MB 제안). 상세 [docs/00](docs/00-vision.md)·[docs/05](docs/05-requirements.md).
현 단계: **준비 완료(08-07 4차) — 기술 기반 확정(DR-5~8)·크레이트 골격 green, M0(트레이 상주 셸) 착수 대기.**

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

## 3. 작업 규약 (요약 — SSOT는 [docs/16](docs/16-doc-git-conventions.md))

- **문서 4층**: 진입(CLAUDE.md) / 현황(STATUS·MILESTONES·TODO) / 경과(DEVLOG·journal·BRANCHES) / 지식(NN-주제·ADR) + docs/README.md 색인.
- **journal이 원본, 나머지는 요약 + 링크.** 진행 기록은 전부 시간 역순(최신 위). 문서 번호(NN-)는 불변, 신규는 append.
- **한 작업 = 한 트랜잭션 갱신**: 커밋 → journal → DEVLOG → MILESTONES/TODO → (브랜치면) BRANCHES.
- **커밋**: Conventional Commits `type(scope): 제목` + 맥락 태그(`(사용자 요청)` 등). scope = `core`/`shell`/`worker`/`app`/`pkg`([docs/15 §2](docs/15-dev-methodology.md)). 단위 = 커밋 1개, 수직 슬라이스, main 항상 green(fmt·clippy·test — [docs/18](docs/18-build-and-test.md)).
- **브랜치·푸시**: 큰 단위 = 브랜치, 세부 = 커밋. 🔴 **push는 사용자 명시 요청 시에만**, 태그 push는 별도 승인, 파괴적 작업은 실행 전 확인. 그 외 일상 작업은 자동 진행.
- `.claude/settings.json`(권한)은 **덮어쓰기 금지, 병합만**.
- 기록에는 "왜 + 실측값". 설계 전 기존 문서·코드 먼저 확인(재발명 금지), 이식 시 출처 경로를 커밋 본문에 명기.

## 4. 새 세션 오리엔테이션

1. 이 CLAUDE.md + [docs/STATUS.md](docs/STATUS.md) → 2. [DEVLOG](docs/DEVLOG.md) 최상단 + 최신 journal → 3. 할 일 = [docs/TODO.md](docs/TODO.md)(P-1부터 순차).

## 5. 다음 단계 (2026-08-07 4차 갱신)

1. **M0-1 — 트레이 상주 셸 수직 슬라이스**([docs/02 M0](docs/02-roadmap.md)): windows crate 도입(원장 기록)·트레이 아이콘(사용률 %·색 단계)·툴팁·종료 메뉴·5초 폴링. `nexa-dir2` 트레이·창 코드 먼저 확인 후 이식.
2. M0-2 — 예산 실측 게이트([docs/18 §4](docs/18-build-and-test.md)) → `release: 0.1.0 승격`(태그 push는 별도 승인).
3. 이후 M1(정리 엔진 v1 — 워커)부터 [docs/02](docs/02-roadmap.md) 순차.
