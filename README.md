# Nexa MemKeeper — 트레이 상주 초경량 메모리 최적화 도구

> Ultra-lightweight tray-resident memory optimizer for Windows.

**Nexa MemKeeper**는 시스템 트레이에 상주하며 메모리를 지속 관찰하고, 필요 없어진 메모리를 회수해 여유분을 늘려주는 Windows 도구입니다.
**도구 자신의 점유 최소화가 정체성** — 상주부는 최소로, 기능(Top 5 점유 팝업·프로세스 트리 뷰·선택 종료/회수)은 수행 시에만 점유하고 종료 시 스스로 회수합니다.

## 설계 원칙

1. **성능·동작 안정성 우선** — 개발·배포 편의보다 앞선다(DR-4). 관리 런타임·GC 없는 올 러스트 + Win32 직접 호출.
2. **예산이 정체성** — 유휴 상주 RSS ≤5MB · 단일 exe ≤3MB · 외부 DLL 0(OS 인박스 제외). 마일스톤마다 실측 게이트.
3. **상주 셸 / 온디맨드 워커 분리** — 무거운 기능은 별도 프로세스에서 수행하고 종료 시 OS가 전량 회수.
4. **안전한 회수 우선** — 시스템·보호 프로세스는 건드리지 않고, 캐시를 버리는 공격적 정리는 기본 off(부작용 명시 후 opt-in).

## 현재 상태

- 단계: **M0~M4 완료**(트레이 상주 셸 + 정리 엔진 + Top 5 팝업 + 메인 화면 + 설정) — 실기 QA 후 `0.5.0` 승격 예정. 상세 → [docs/STATUS.md](docs/STATUS.md).
- 실측: exe **0.51MB** ≤3 · 상주 유휴 RSS **1.32MB** ≤5 · 임포트 **OS 인박스만** · 정리 1회 **+847MB** 회수(관리자 권한·트림 466 프로세스) · 프로세스 열거 **500개 7.5ms** · 테스트 **38 green**.

## 지금 할 수 있는 것

| 위치 | 기능 |
| --- | --- |
| 트레이 아이콘 | 사용률 % 실시간 표시(색 단계 80/90%) · 툴팁(사용량·마지막 정리) |
| 좌클릭 | **Top 5 점유 팝업**(2초 자동 갱신) · [지금 정리] [목록 보기…] · 항목 우클릭으로 회수/종료 |
| 우클릭 | 지금 정리 · 목록 보기 · **설정** · 관리자 권한으로 재시작 · 종료 |
| 메인 화면 | 프로세스 **평면 목록/트리 전환** · **타이핑 즉시 검색** · 헤더 클릭 정렬 · 쓰레드 펼치기 · **프로세스/트리/쓰레드 종료** · 자동 실행 토글 |
| 설정 창 | **임계값 자동 정리**(기본 90%·쿨다운 300000ms) · 주기 정리 · **정리 영역 ①~⑤ 선택** · 알림·표시 옵션. 저장 즉시 반영, `data\settings.ini`(포터블) |

> ⏱ 설정의 시간 값은 **모두 밀리초(ms)** 입니다.

문서·git 규약과 라이선스 정책은 [`SosomLab/nexa-dir2`](https://github.com/SosomLab/nexa-dir2)의 검증 표준을 계승합니다.

## 문서 — 📖 [문서 홈](docs/README.md)에서 시작

바로가기: [현황 STATUS](docs/STATUS.md) · [기능·마일스톤](docs/MILESTONES.md) · [진행 DEVLOG](docs/DEVLOG.md) · [비전 00](docs/00-vision.md) · [결정 기록 10](docs/10-decision-record.md) · [메모리 측정·회수 해설 19](docs/19-memory-measurement-and-reclaim.md) · [이식 메모리](CLAUDE.md)

## 프로젝트 정보 / 라이선스

| 항목 | 내용 |
| --- | --- |
| 조직 | **SosomLab** — <https://sosomlab.com> |
| 저장소 | <https://github.com/SosomLab/nexa-memkeeper> |
| 개발자 | Sangyong Bae — kiros33@gmail.com |

**PolyForm Noncommercial 1.0.0** ([LICENSE.md](LICENSE.md) · 한글 [LICENSE.ko.md](LICENSE.ko.md)) — 개인·비상업 무료, 상업 사용은 유료 라이선스(문의 kiros33@sosomlab.com).
