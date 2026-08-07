# 📖 Nexa MemKeeper — 문서 홈

> **처음 보는 사람을 위한 길잡이.** 프로젝트 한 줄 소개는 루트 [README](../README.md).
> **한 장 현황이 급하면 →** [STATUS](STATUS.md) · **최근 무슨 일 →** [DEVLOG](DEVLOG.md) · **기능 현황 →** [MILESTONES](MILESTONES.md).

---

## 🧭 추천 읽기 순서

1. **왜 만드나** — [00 비전](00-vision.md) : 목적(사용자 확정)·설계 원칙·예산 목표
2. **무엇을 만드나** — [05 요구사항](05-requirements.md) : FR/NFR·참조 프로그램 차용표·설정 스펙
3. **핵심 결정** — [10 결정 기록](10-decision-record.md) : DR-1~3 + ADR 색인(📐 판단 대기 3건)
4. **지금 상태** — [STATUS](STATUS.md) → [MILESTONES](MILESTONES.md)
5. **진행 경과** — [DEVLOG](DEVLOG.md)(시간 역순) → 관심 날짜 [journal/](journal/)
6. **규약** — [16 문서·커밋/푸시 규약](16-doc-git-conventions.md)

## ① 시발점 · 정체성

| 문서 | 내용 |
|---|---|
| [00 비전](00-vision.md) | ★ 목적(사용자 확정 08-07)·설계 원칙 5·예산 제안(RSS ≤5MB·exe ≤3MB) |
| [05 요구사항](05-requirements.md) | ★ FR-1~6 / NFR-1~5 / 설정 스펙·추천 기본값 / 보호 프로세스 목록 |
| [10 결정 기록](10-decision-record.md) | ★ DR-1(라이선스)·DR-2(규약)·DR-3(목적) + ADR 색인 |
| [CLAUDE.md](../CLAUDE.md) | 이식용 프로젝트 메모리 |

## ② 주요 목표 · 설계

| 문서 | 내용 |
|---|---|
| [MILESTONES](MILESTONES.md) | ★ 기능·목표 현황(✅/🚧/📐/☐) |
| [06 ADR-0001 스택](06-adr-0001-stack.md) | 📐 언어·UI 후보 5안 비교 — 권고 Rust+windows-rs(**판단 대기**) |
| [07 ADR-0002 회수 기법](07-adr-0002-memory-reclaim.md) | 📐 관찰·열거·회수 시스템 콜 매핑 + 권한 전략(**판단 대기**) |
| [08 ADR-0003 상주 구조](08-adr-0003-resident-architecture.md) | 📐 단일 프로세스+자기 트림 vs 셸/워커 분리(**판단 대기**) |
| 01 아키텍처 · 02 로드맵 | ☐ 추후 작성 — ADR 승인 후([TODO](TODO.md) P-5·P-6) |

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
| 15 개발 방법론 · 18 빌드&테스트 | ☐ 추후 작성 — 기술 기반 확정 후([TODO](TODO.md) P-5) |

---

> 문서 규약(`nexa-dir2` 계승): 진행 기록은 **일자 단위** — 상세 `journal/YYYY-MM-DD.md`(시간 역순), 요약 [DEVLOG](DEVLOG.md)·기능 [MILESTONES](MILESTONES.md). 결정은 [10](10-decision-record.md). 문서 번호(NN-)는 불변, 신규는 뒤에 append.
