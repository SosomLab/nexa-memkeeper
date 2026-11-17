# DEVLOG — 진행 요약

> 시간 역순(최신 위), 항목당 1~2줄. 상세는 [journal/](journal/)이 원본, 여기는 요약 + 링크만.

## 2026-11-18

- 14차: **`0.7.2` 릴리스**(`08c03da`) — release run 44191276028 success · 자산 4종 · choco 2건 `Pending Automated Review` · winget [#482747](https://github.com/microsoft/winget-pkgs/pull/482747)·[#482748](https://github.com/microsoft/winget-pkgs/pull/482748). 사전 검증 V1 60 green · exe 0.678MB · 유휴 WS 1.41MB · DLL 0. → [journal/2026-11-18](journal/2026-11-18.md)

- 13차: push `f9a2b16..04a87a4`(9커밋) · CI success(windows·ubuntu·macos) · [DR-17](10-decision-record.md) 사용자 확정. → [journal/2026-11-18](journal/2026-11-18.md)

- 12차: main ff 병합(`7d8c64d`) · `fix/console-guard`·`fix/single-instance` 삭제. push는 사용자 요청 대기.

- 11차: **S-1 중복 실행 방어(`32eb2d5`)** — 세션당 셸 하나, 두 번째 실행은 기존 팝업을 열고 종료(시험: 3초 내 종료·팝업 표시·워커 bench 정상). exe 0.678MB · 유휴 WS 1.41MB · 60 green. 방식은 [DR-17](10-decision-record.md)(사용자 확정 11-18). → [journal/2026-11-18](journal/2026-11-18.md)

- 10차: **C-1~C-3 콘솔 창 방어(`3ee60f8`) · L-1 언어 저장(`f89c94e`) · X-5 지연 시작(`b28ebc2`)** — V1 60 green · exe 0.677MB(+2KB) · DLL 0 · 유휴 WS 1.43MB/Private 1.54MB · CPU ~0%. 설치본 복귀. 육안 확인은 사용자 QA. → [journal/2026-11-18](journal/2026-11-18.md)

- 9차: 🔴 **언어 설정 저장 누락 결함 확정** — `to_ini`에 `lang` 줄이 없어 설정 창·목록 창 저장 때마다 지워진다(코드 판정, 시험 공백 포함). TODO L-1. → [journal/2026-11-18](journal/2026-11-18.md)
- 8차: push `8803a62..f9a2b16` · CI 확인 · 사용자 결정 2건(언어 결함 이번에 수정 · 단일 인스턴스는 X-5 다음) · DR-16 두 항목 사용자 확정.
- 6~7차: main ff 병합(`89f3770`)·브랜치 2개 삭제 · X-5 설계 결정(트레이 즉시, 자동 정리만 보류, 기본 30초, `start_delay_ms`).

- 5차: **0.7.1 심사 종료 확인** — choco 2건 09-29 Approved(moderator virtualex) · winget #443189·#443190 09-29 MERGED. 대기 항목 닫음, 남은 건 M5-3 설치 왕복 QA. → [journal/2026-11-18](journal/2026-11-18.md)
- 4차: 사용자 확정 진행 순서 — ① `fix/tray-header-version` 마무리 → ② 콘솔 창 방어 수정(C-1~C-3) → ③ X-5 지연 시작.
- 3차: **e9d09cf 검증** — fmt·clippy·test 57 green · exe 708,096B · DLL 0 · CPU ~0%. 유휴 RSS 4.4MB는 설치본 A/B로 **측정 조건 차이** 판정(Private 두 빌드 모두 2.5~2.6MB, WS는 1.4~4.8MB 출렁임).
- 2차: "시작 시 콘솔 창" 점검 — 이 프로그램이 원인일 가능성 낮음(GUI 서브시스템·인자 없는 Run 값). 방어할 잠재 위험 3건 TODO 등재.
- 1차: 저장소 최신화(원격 변경 0건) · **세션 분담·자동화 개발 규칙 [22](22-session-and-automation-rules.md) 신설**(`50ae7dc`·`6be3373`) → [DR-16](10-decision-record.md).

## 2026-10-12

- 1차: **트레이 메뉴 머리줄 `0.0.1` 고정 버그 수정** — 버전 리소스와 같은 출처(build.rs `PRODUCT` + `CARGO_PKG_VERSION`)로 컴파일 시 생성. About·툴팁·창 제목은 점검 결과 이상 없음. → [journal/2026-10-12](journal/2026-10-12.md)

## 2026-09-29

- 2차: 진행사항 최신화 — 심사 재확인(choco `Pending Automated Review`·winget OPEN, 변화 없음) + 패키징 SSOT에 **[20 §7-10](20-packaging.md) "승인된 버전은 패치로"** 신설. → [journal/2026-09-29](journal/2026-09-29.md)
- 1차: 저장소 최신화(변경 없음) + 배포 필요성 검토 — winget 2·choco 2 **모두 승인·게시**됐으나, 승인된 choco 설치형이 08-11본이라 **제거 버그가 그대로 실린 것**을 공개 nupkg 파싱으로 확인 → **`0.7.1` 패치 배포**(앱 코드 무변경). choco 2건 push·winget PR 2건(#443189·#443190) 제출, release.yml에 포크 동기화 추가. → [journal/2026-09-29](journal/2026-09-29.md)

## 2026-08-21

- 2차: 승인 가능성 재검토 — 모더레이션 기준·validator 76규칙과 대조. **1차 수정이 문서화된 규칙("UTF-8 with BOM") 위반이었음을 확인**, 이름 구조 우려는 기우로 판명, 설치형의 `portable` 태그 제거. 남은 저지 요소 없음([20 §7-9](20-packaging.md)). → [journal/2026-08-21](journal/2026-08-21.md)
- 1차: 저장소 최신화(변경 없음) + 심사 재점검 — winget·choco 모두 대기 상태 그대로지만, **choco 검증 로그에서 `choco uninstall`이 항상 실패하는 버그**를 발견(`"$var:" 보간 → PS 5.1 파싱 실패). 수정 + 패키징 `.ps1` BOM 통일 + **CI 구문·BOM 게이트** 추가. → [journal/2026-08-21](journal/2026-08-21.md)

## 2026-08-11

- 2차: winget 재확인 — 상태 변화 없음(승인 대기). PR 목록의 **빨간 `✗ 1/19`은 winget-pkgs 자체 봇 job 취소**일 뿐 매니페스트 문제가 아님을 실측 확인(병합된 남의 PR에도 동일). 큐 순번 238건. → [journal/2026-08-11](journal/2026-08-11.md)
- 1차: 심사 2차 점검 — **choco 2건이 "유지관리자 조치 대기"**(nuspec 설명의 이메일 = Requirement 위반) → 이메일 제거 + **동일 버전 재제출 워크플로**(`choco-resubmit.yml`)로 0.7.0 재제출 완료(`Pending automated review` 복귀). winget 2건은 검증 통과·승인 대기. → [journal/2026-08-11](journal/2026-08-11.md)

## 2026-08-08

- 12차: 패키지 심사 상태 첫 점검 — winget PR 2건 **자동 검증 통과**(사람 승인 대기), choco 2건 피드 미등재(심사 중). → [journal/2026-08-08](journal/2026-08-08.md)
- 11차: **winget PR 2건 제출 완료**(#413829 설치형 · #413830 포터블) — 토큰 스코프 갱신으로 해소. → [journal/2026-08-08](journal/2026-08-08.md)
- 10차: winget 최초 등록 워크플로 추가 — 매니페스트 검증은 통과, **토큰 `workflow` 스코프 부족으로 PR 생성 실패**(사용자 조치 필요). → [journal/2026-08-08](journal/2026-08-08.md)
- 9차: **Chocolatey 등록** — 설치형·포터블 2패키지 0.7.0 push 완료(모더레이션 대기), 이후 태그는 자동 게시. → [journal/2026-08-08](journal/2026-08-08.md)
- 8차: **`0.7.0` 배포** — 08-08 기능 7건(E-1~E-3·M3-4·X-1~X-3) 묶음 릴리스. → [journal/2026-08-08](journal/2026-08-08.md)
- 7차: **X-3 정리 히스토리**(`history.csv` + 계기 기록, 메뉴에서 열기). → [journal/2026-08-08](journal/2026-08-08.md)
- 6차: **X-1 워치리스트 표적 트림**(지정 앱만 임계 초과 시 트림, 꺼져 있으면 열거 0). → [journal/2026-08-08](journal/2026-08-08.md)
- 5차: **X-2 전체화면 억제**(게임·발표 중 자동 정리 보류) + 유휴 복귀 재확인(11.2 → 2.7MB, 회귀 아님). → [journal/2026-08-08](journal/2026-08-08.md)
- 4차: **E-3 — 고압력 재측정**(중간 압력 1.5~3.0GB에서도 개선 없음, +7%/+14% 느림). 첫 시도는 압력이 풀려 폐기·재측정. → [journal/2026-08-08](journal/2026-08-08.md)
- 3차: **M3-4 — 열 폭 조절 · 창 위치·보기 모드 기억**(ini 연동, 모니터 존재 확인 후 복원). → [journal/2026-08-08](journal/2026-08-08.md)
- 2차: **E-2 — 설정 창 주기 정리에 비권장 근거 문구**(잘린 문구 2건 함께 수정). → [journal/2026-08-08](journal/2026-08-08.md)
- 1차: **E-1 — 알림에 Active 감소분 병기**(표시 +935MB vs 실제 −5519MB). 종료 코드를 15비트 2값으로 분할. → [journal/2026-08-08](journal/2026-08-08.md)

## 2026-08-07

- 38차: **효용성 실측([21](21-effectiveness-measurement.md))** — 실험 5종. 정리 효과는 진짜(Active −3.9GB)지만 **반감기 14초**·4분이면 96% 원위치. 주기 정리는 Available +688MB 대가로 기동 +4.3%·읽기 +12.6% 악화 → **평상시 주기 정리는 손해**. ③ 기본 off는 실측으로 정당화(읽기 4.4배 악화). → [journal/2026-08-07](journal/2026-08-07.md)
- 37차: 0.6.1 설치본 ZIP 누락 수정(워크플로 첨부 목록) · winget 포터블 패키지 추가(2패키지 체계). → [journal/2026-08-07](journal/2026-08-07.md)
- 36차: `0.6.1` 재배포 — 설치본 ZIP까지 워크플로가 자동 생성·첨부하는 경로 검증(코드 변경 없음). → [journal/2026-08-07](journal/2026-08-07.md)
- 35차: `0.6.0` 배포 완료 · 설치본 ZIP 자산 추가 · 스크린샷 촬영 스크립트 · **Wiki 7쪽 공개**. → [journal/2026-08-07](journal/2026-08-07.md)
- 34차: `0.6.0` 승격·태그 릴리스(자동 게시는 변수 off로 차단). 언어 전역 상태 때문에 간헐 실패하던 i18n 테스트를 락으로 직렬화. → [journal/2026-08-07](journal/2026-08-07.md)
- 33차: GitHub Actions(ci·release) + winget·choco 패키지 자산. 게시는 저장소 변수로 차단(기본 off). DLL 게이트 실제 동작 확인 — 외부 DLL 0. → [journal/2026-08-07](journal/2026-08-07.md)
- 32차: M5-1 — 영역 ⑥ 페이지 결합(일반 권한에서도 성공, +365MB 실측) · 저메모리 이벤트 트리거(폴링에 질의 1콜) · 설정 창 항목 추가. 테스트 55 green. → [journal/2026-08-07](journal/2026-08-07.md)
- 31차: M5-2 배포 2채널 — Inno 설치형 스크립트 + 포터블 ZIP + `pack.ps1`(예산 게이트 내장) + [20 배포](20-packaging.md). ZIP 355KB 생성 확인, 설치본은 Inno 설치 후 검증 예정. → [journal/2026-08-07](journal/2026-08-07.md)
- 30차: 메뉴의 검색 지우기가 검색칸을 비우지 않던 문제 수정(입력 상태 단일화, set_filter 제거). → [journal/2026-08-07](journal/2026-08-07.md)
- 29차: 검색 지우기 단추를 직접 그리는 창으로 교체(클릭 불통·네모난 모양 해결, 호버 강조). → [journal/2026-08-07](journal/2026-08-07.md)
- 28차: 묶음 축소 버튼이 먹지 않던 문제 수정(기본 펼침은 1회만 심는다) · 검색 중 임시 펼침 · 낡은 mk-app.exe 재기동 함정 문서화. 테스트 54 green. → [journal/2026-08-07](journal/2026-08-07.md)
- 27차: 작업 관리자식 이름(FileDescription)·앱의 도우미 프로세스 합산(`터미널(3)`)·셸(explorer) 예외·창 제목 검색. 테스트 54 green. → [journal/2026-08-07](journal/2026-08-07.md)
- 26차: 작업 관리자식 목록 구성(앱/백그라운드/Windows 분류 + 프로그램별 묶음, 기본 보기) · 알림 6초 자동 닫힘. 테스트 51 green. → [journal/2026-08-07](journal/2026-08-07.md)
- 25차: 기본 보기를 프로세스 트리로(자동 펼침) · 쓰레드는 보기 옵션(기본 off) · 설정 키 show_threads. 테스트 51 green. → [journal/2026-08-07](journal/2026-08-07.md)
- 24차: 프로세스(트리) 종료 시 UI 멈춤 수정 — 회수·종료를 백그라운드 스레드로 이동, 진행 중 표시. → [journal/2026-08-07](journal/2026-08-07.md)
- 23차: 하단 검색창(실시간·PID 검색·지우기 버튼·placeholder) + 문서 최신화. 테스트 50 green. → [journal/2026-08-07](journal/2026-08-07.md)
- 22차: 메뉴 토글 시 목록 창 즉시 종료 수정(사용자 QA) — RefCell 이중 대여 패닉, 헬퍼가 상태를 인자로 받도록 변경. → [journal/2026-08-07](journal/2026-08-07.md)
- 21차: M4-2 전역 핫키(등록·해제·설정 UI) · 그리드 헤더 렌더 수정(폭 실측·화살표 분리·비례 축소) · **다국어 처리(기본 영어, DR-13)**. 테스트 49 green. → [journal/2026-08-07](journal/2026-08-07.md)
- 20차: 진행사항 최신화 — README(M0~M4·설정 창 행·ms 표기)·STATUS(다음 단계)·BRANCHES 정정 후 병합·push. → [journal/2026-08-07](journal/2026-08-07.md)
- 19차: 설정 창 라벨 줄바꿈 수정(사용자 QA) — 라벨/입력 규격 통일(218/232/96), GetTextExtentPoint32W로 필요 폭 실측 검증. → [journal/2026-08-07](journal/2026-08-07.md)
- 18차: 설정 시간 단위를 전부 ms로 통일(poll_ms·cooldown_ms·interval_ms) + 구 키 1회 마이그레이션. 실기로 구 파일 → ms 재작성 확인. 테스트 38 green. → [journal/2026-08-07](journal/2026-08-07.md)
- 17차: M4 설정 구현(`feat/m4-settings`) — settings.ini 파싱/검증·설정 창·자동 정리(임계값/쿨다운/주기)·영역 ③④⑤·앱 매니페스트. 설정 창 크기 QA 수정(AdjustWindowRect). 테스트 37 green. → [journal/2026-08-07](journal/2026-08-07.md)
- 16차: 진행사항 최신화 — README·CLAUDE·MILESTONES·TODO·05·09의 불일치 6건 정정 후 브랜치 병합·push. → [journal/2026-08-07](journal/2026-08-07.md)
- 15차: 트리 기본 보기 회귀 해결(부모 접힘으로 자식이 안 보이던 문제) — 기본 평면 목록·보기 메뉴·타이핑 검색. 앱 아이콘 신규(nexa-dir2 계열 + 80% 게이지·RAM·회수 화살표) exe 임베드. → [journal/2026-08-07](journal/2026-08-07.md)
- 14차: 도움말 메뉴에 홈페이지·GitHub 링크 추가(정보 창 재작성, 앱 홈페이지 자리 예약), 트리 셰브론을 nexa-dir2 MDL2 규약(E76C/E70D·9DIP)으로 교체. → [journal/2026-08-07](journal/2026-08-07.md)
- 13차: 권한별 회수량 실측(일반 +322MB/115개 vs 관리자 +847MB/466개 — 관리자는 선택)·자동 시작 권한 관계 정리, 의사 프로세스 숨김 옵션(기본 on) 구현. 재기동 절차를 중지→빌드→실행으로 정정. → [journal/2026-08-07](journal/2026-08-07.md)
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
