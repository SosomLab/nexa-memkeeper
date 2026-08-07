# 📖 Nexa MemKeeper — 문서 홈

> **처음 보는 사람을 위한 길잡이.** 프로젝트 한 줄 소개는 루트 [README](../README.md).
> **한 장 현황이 급하면 →** [STATUS](STATUS.md) · **최근 무슨 일 →** [DEVLOG](DEVLOG.md) · **기능 현황 →** [MILESTONES](MILESTONES.md).

---

## 🧭 추천 읽기 순서

1. **핵심 결정** — [10 결정 기록](10-decision-record.md) : DR 표 + ADR 색인
2. **지금 상태** — [STATUS](STATUS.md) → [MILESTONES](MILESTONES.md)
3. **진행 경과** — [DEVLOG](DEVLOG.md)(시간 역순) → 관심 날짜 [journal/](journal/)
4. **규약** — [16 문서·커밋/푸시 규약](16-doc-git-conventions.md)

> 비전(00)·요구사항(05)·아키텍처(01)·로드맵(02)은 프로젝트 정의 확정 후 작성 예정 — 작성되면 이 순서 맨 앞에 추가한다.

## ① 시발점 · 정체성

| 문서 | 내용 |
|---|---|
| [10 결정 기록](10-decision-record.md) | ★ DR-1(라이선스)·DR-2(규약 계승) + ADR 색인 |
| [CLAUDE.md](../CLAUDE.md) | 이식용 프로젝트 메모리 |
| 00 비전 · 05 요구사항 | ☐ 추후 작성 — 개발 목표 정리 시([TODO](TODO.md) P-3) |

## ② 주요 목표 · 설계

| 문서 | 내용 |
|---|---|
| [MILESTONES](MILESTONES.md) | ★ 기능·목표 현황(✅/🚧/📐/☐) |
| 01 아키텍처 · 02 로드맵 | ☐ 추후 작성 — 구성 방식·기술 기반 정리 시([TODO](TODO.md) P-2·P-4) |

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
