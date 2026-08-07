# DEVLOG — 진행 요약

> 시간 역순(최신 위), 항목당 1~2줄. 상세는 [journal/](journal/)이 원본, 여기는 요약 + 링크만.

## 2026-08-07

- 12차: 메인 화면 완성(M3) — 메뉴바·트리 그리드·헤더 정렬·쓰레드 확장·프로세스/트리/쓰레드 종료·자동 실행(DR-10~12). 알림 이름 `Nexa MemKeeper`로 수정. Memory Compression 문의 조사([19 §4-5](19-memory-measurement-and-reclaim.md)). → [journal/2026-08-07](journal/2026-08-07.md)
- 11차: M2 Top 5 팝업 완료(`feat/m2-top5-popup`→main) — 열거 482프로세스 6.5ms·2000ms 자동 갱신·항목 트림/종료. NFR-3·NFR-4 통과. 15 §4 정정(읽기 전용 NT 조회는 셸 허용). → [journal/2026-08-07](journal/2026-08-07.md)
- 10차: 문서 최신화(README 현재 상태·설계 원칙, MILESTONES M2~M5 보강) 후 **최초 원격 push**(커밋 10건). → [journal/2026-08-07](journal/2026-08-07.md)
- 9차: M1 정리 엔진 v1 완료(`feat/m1-clean-engine`→main) — 워커 정리(트림 461건·퍼지)·회수 +209MB 실측·풍선 알림·승격 재시작. 예산 유지(exe 0.34MB·RSS 2.20MB). → [journal/2026-08-07](journal/2026-08-07.md)
- 8차: 메모리 측정·대상 선정·회수 원리 해설서(19) 작성 — mermaid 다이어그램·수식·매트릭스 포함. → [journal/2026-08-07](journal/2026-08-07.md)
- 7차: 기능 후보 X-1~X-7(05 §5) + 설정 스키마 09(settings.ini 6섹션, M4 SSOT) 정리 — 후보는 사용자 판단 대기. → [journal/2026-08-07](journal/2026-08-07.md)
- 6차: M0-1 트레이 상주 셸 완료(`feat/m0-tray-shell`→main) — 실측 exe 0.23MB·유휴 RSS 1.56MB·DLL 0·CPU ~0.03%, 전 게이트 통과. DR-9 배포 2채널(설치형+ZIP·자동 실행 질문 기본 on) 확정. → [journal/2026-08-07](journal/2026-08-07.md)
- 5차: 스택 재검토(사용자 요청 — 시스템 콜 관점 Rust vs C++) → ADR-0001 §6 append, DR-5 유지(호출 비용 동일·RSS 차 <1MB·안정성 Rust 우위). → [journal/2026-08-07](journal/2026-08-07.md)
- 4차: ADR 3건 승인 → DR-5~8 승격(스택 Rust·회수 기법·하이브리드·예산 게이트). 로드맵 02(M0~M5)·방법론 15·빌드&테스트 18 작성 + 크레이트 골격(mk-core/mk-app, 테스트 5 green). → [journal/2026-08-07](journal/2026-08-07.md)
- 3차: 판단 우선순위 확정(DR-4 — 성능·안정성 > 편의성, 사용자 확정) → ADR-0003 권고를 하이브리드(셸+온디맨드 워커)로 정정, ADR-0001 논거 재정리. → [journal/2026-08-07](journal/2026-08-07.md)
- 2차: 개발 목표·기술 기반 문서화 — 목적 확정(DR-3)·비전 00·요구사항 05(설정 스펙 포함)·ADR-0001~0003(스택·회수 기법·상주 구조, 📐 사용자 판단 대기). → [journal/2026-08-07](journal/2026-08-07.md)
- 1차: 저장소 부트스트랩 — `nexa-dir2` 문서·git 규약 + PolyForm NC 라이선스 이식, 4층 문서 골격 생성(DR-1·DR-2). → [journal/2026-08-07](journal/2026-08-07.md)
