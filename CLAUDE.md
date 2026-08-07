# CLAUDE.md — Nexa MemKeeper 프로젝트 컨텍스트 (이식용 메모리)

> 이 파일은 **다른 PC에서 clone 시 즉시 컨텍스트를 공유**하기 위한 휴대용 프로젝트 메모리다.
> **먼저 읽기:** [docs/STATUS.md](docs/STATUS.md)(현황) → [docs/10-decision-record.md](docs/10-decision-record.md)(결정).

## 1. 이 프로젝트는

**Nexa MemKeeper** — SosomLab의 신규 프로젝트. **정의(목적·구성·기술 기반)는 정리 중** — 확정 시 이 절과 docs/00·05를 갱신한다.
현 단계: **준비 — 기본 문서 골격 완료(08-07), 프로젝트 구성 방식 정리 대기.**

- 조직: **SosomLab** · 개발자: Sangyong Bae · kiros33@gmail.com
- 문서·git 규약은 `SosomLab/nexa-dir2`의 검증 표준을 전면 계승([docs/16](docs/16-doc-git-conventions.md)).

## 2. 확정 결정 ([docs/10](docs/10-decision-record.md), 변경 시 새 항목으로 정정)

| # | 결정 |
| --- | --- |
| DR-1 | 라이선스 = **PolyForm Noncommercial 1.0.0** — 개인·비상업 무료, 상업은 유료(kiros33@sosomlab.com). 영문 정본 [LICENSE.md](LICENSE.md) + 한글본 [LICENSE.ko.md](LICENSE.ko.md) |
| DR-2 | 문서·git 규약 = `nexa-dir2` 표준 계승 — SSOT는 [docs/16](docs/16-doc-git-conventions.md) |

## 3. 작업 규약 (요약 — SSOT는 [docs/16](docs/16-doc-git-conventions.md))

- **문서 4층**: 진입(CLAUDE.md) / 현황(STATUS·MILESTONES·TODO) / 경과(DEVLOG·journal·BRANCHES) / 지식(NN-주제·ADR) + docs/README.md 색인.
- **journal이 원본, 나머지는 요약 + 링크.** 진행 기록은 전부 시간 역순(최신 위). 문서 번호(NN-)는 불변, 신규는 append.
- **한 작업 = 한 트랜잭션 갱신**: 커밋 → journal → DEVLOG → MILESTONES/TODO → (브랜치면) BRANCHES.
- **커밋**: Conventional Commits `type(scope): 제목` + 맥락 태그(`(사용자 요청)` 등). 단위 = 커밋 1개, 수직 슬라이스, main 항상 green.
- **브랜치·푸시**: 큰 단위 = 브랜치, 세부 = 커밋. 🔴 **push는 사용자 명시 요청 시에만**, 태그 push는 별도 승인, 파괴적 작업은 실행 전 확인. 그 외 일상 작업은 자동 진행.
- `.claude/settings.json`(권한)은 **덮어쓰기 금지, 병합만**.
- 기록에는 "왜 + 실측값". 설계 전 기존 문서·코드 먼저 확인(재발명 금지), 이식 시 출처 경로를 커밋 본문에 명기.

## 4. 새 세션 오리엔테이션

1. 이 CLAUDE.md + [docs/STATUS.md](docs/STATUS.md) → 2. [DEVLOG](docs/DEVLOG.md) 최상단 + 최신 journal → 3. 할 일 = [docs/TODO.md](docs/TODO.md)(P-1부터 순차).

## 5. 다음 단계 (2026-08-07 갱신)

1. **프로젝트 구성 방식 정리**(P-2) — 디렉터리·모듈 구조, commit scope 규약 확정.
2. **개발 목표 정리**(P-3) — 비전(docs/00)·요구사항(docs/05) 작성.
3. **기술 기반 확정**(P-4) — 스택 ADR(06-adr-0001) + DR 추가 → 이후 15·18 작성(P-5).
