# 📖 Nexa MemKeeper — 문서 홈

> **처음 보는 사람을 위한 길잡이.** 프로젝트 한 줄 소개는 루트 [README](../README.md).
> **한 장 현황이 급하면 →** [STATUS](STATUS.md) · **최근 무슨 일 →** [DEVLOG](DEVLOG.md) · **기능 현황 →** [MILESTONES](MILESTONES.md).

---

## 🧭 추천 읽기 순서

1. **왜 만드나** — [00 비전](00-vision.md) : 목적(사용자 확정)·설계 원칙·예산 목표
2. **무엇을 만드나** — [05 요구사항](05-requirements.md) : FR/NFR·참조 프로그램 차용표·설정 스펙
3. **핵심 결정** — [10 결정 기록](10-decision-record.md) : DR-1~8 + ADR 색인
4. **어떻게 짓나** — [02 로드맵](02-roadmap.md)(M0~M5) → ADR 06·07·08
5. **지금 상태** — [STATUS](STATUS.md) → [MILESTONES](MILESTONES.md)
6. **진행 경과** — [DEVLOG](DEVLOG.md)(시간 역순) → 관심 날짜 [journal/](journal/)
7. **규약** — [16 문서·커밋/푸시 규약](16-doc-git-conventions.md) → [22 세션 분담 & 자동화](22-session-and-automation-rules.md) → [15 방법론](15-dev-methodology.md) → [18 빌드&테스트](18-build-and-test.md)

## ① 시발점 · 정체성

| 문서 | 내용 |
|---|---|
| [00 비전](00-vision.md) | ★ 목적(사용자 확정 08-07)·설계 원칙 5·예산 제안(RSS ≤5MB·exe ≤3MB) |
| [05 요구사항](05-requirements.md) | ★ FR-1~6 / NFR-1~5 / 설정 스펙·추천 기본값 / 보호 프로세스 목록 |
| [10 결정 기록](10-decision-record.md) | ★ DR-1~16(라이선스·규약·목적·우선순위·스택·기법·구조·예산) + ADR 색인·crate 원장 |
| [CLAUDE.md](../CLAUDE.md) | 이식용 프로젝트 메모리 |

## ② 주요 목표 · 설계

| 문서 | 내용 |
|---|---|
| [MILESTONES](MILESTONES.md) | ★ 기능·목표 현황(✅/🚧/📐/☐) |
| [02 로드맵](02-roadmap.md) | ★ M0~M5 계획 원안(완료 기준·예산 게이트) |
| [06 ADR-0001 스택](06-adr-0001-stack.md) | ✅ 언어·UI = Rust+windows-rs(후보 5안 비교 → DR-5) |
| [07 ADR-0002 회수 기법](07-adr-0002-memory-reclaim.md) | ✅ 관찰·열거·회수 시스템 콜 매핑 + 권한 전략(→ DR-6) |
| [08 ADR-0003 상주 구조](08-adr-0003-resident-architecture.md) | ✅ 하이브리드 — 셸+온디맨드 워커(DR-4 기준 정정 → DR-7) |
| [09 설정 스키마](09-settings-schema.md) | `data\settings.ini` 키·기본값·범위·UI 배치(M4 구현 SSOT) |
| [19 메모리 측정·회수 해설](19-memory-measurement-and-reclaim.md) | 📖 페이지 상태·측정 지표·대상 선정 파이프라인·회수 원리(다이어그램) |
| [20 배포](20-packaging.md) | 📦 배포 2채널(설치형 Inno·포터블 ZIP)·`pack.ps1`·릴리스 체크리스트 |
| [21 효용성 실측](21-effectiveness-measurement.md) | 🔬 **"주기적 정리가 성능에 도움이 되나"** — 판정 기준 4축 + 실험 5종 실측·판정·권고 |
| 01 아키텍처 | ☐ 추후 작성 — M0~M1 구현으로 실체 확정 후 기록 |

## ③ 진행 경과 · 할 일

| 문서 | 내용 |
|---|---|
| [DEVLOG](DEVLOG.md) | ★ 진행 시간순(최신 위) · [journal/](journal/) 일자 상세 |
| [BRANCHES](BRANCHES.md) | 브랜치 생성/작업/병합/삭제 이력 |
| [STATUS](STATUS.md) | ★ 현재 상태 한 장 |
| [TODO](TODO.md) | 순차 백로그(P-1~) |

## ④ 개발 · 기여

| 문서 | 내용 |
|---|---|
| [16 문서·커밋/푸시 규약](16-doc-git-conventions.md) | ★ 문서 4층 체계·커밋/브랜치/푸시 규칙 — **타 프로젝트 이식용 지시문 포함** |
| [15 개발 방법론](15-dev-methodology.md) | 수직 슬라이스·commit scope 규약·크레이트 구조·안전 규율 |
| [18 빌드 & 테스트](18-build-and-test.md) | ★ 빌드·테스트·예산 실측 절차(SSOT) |
| [22 세션 분담 & 자동화 개발 규칙](22-session-and-automation-rules.md) | ★ 개발(Fable)·협업(Opus) 세션 분담·파일 소유·SEM, 자동·고지·질문 3단 경계(SSOT) |
| [claude-global-rules.md](claude-global-rules.md) | 전역 규칙(`~/.claude/CLAUDE.md`) 이식용 사본 — 설치·일치 판정 `scripts/install-global-rules.sh` |

---

> 문서 규약(`nexa-dir2` 계승): 진행 기록은 **일자 단위** — 상세 `journal/YYYY-MM-DD.md`(시간 역순), 요약 [DEVLOG](DEVLOG.md)·기능 [MILESTONES](MILESTONES.md). 결정은 [10](10-decision-record.md). 문서 번호(NN-)는 불변, 신규는 뒤에 append.
