# TODO — 순차 백로그

> 목표순. ID·우선(P0~P2)·규모(소/중/대)·의존·상태. 완료 항목은 상태만 바꾸고 지우지 않는다.
>
> **남은 일(11-18 기준)**
> - 개발(사용자 확정 순서): ~~C-1~C-3 → L-1 → X-5~~ ✅ 11-18(`fix/console-guard`) → ~~S-1 · L-2~~ ✅ 11-18(main `7d8c64d`, push 11-18). DR-17 사용자 확정 11-18. 남은 것: 사용자 QA(설정 창 입력란·안내 문구·트레이 머리줄·`--restart`)
> - 사용자: **M5-3**(설치 왕복 QA, P0 — choco 0.7.1이 승인돼 `choco install/uninstall`로 제거 수정도 함께 확인 가능) · **M3-3**(앱 홈페이지 URL 확정 시 한 줄) · 트레이 머리줄 버전 육안 확인
> - 대기: 없음 — **0.7.1** 전 채널 게시 완료(choco 2건 09-29 Approved · winget [#443189](https://github.com/microsoft/winget-pkgs/pull/443189)·[#443190](https://github.com/microsoft/winget-pkgs/pull/443190) 09-29 MERGED)
> - 코드 후보(사용자 선택 대기): X-1 설정 UI · X-4(배터리) · X-6(트레이 표시 모드)

| ID | 항목 | 우선 | 규모 | 의존 | 상태 |
| --- | --- | --- | --- | --- | --- |
| P-1 | 기본 문서 골격(규약·라이선스 이식) | P0 | 소 | — | ✅ 08-07 |
| P-2 | 개발 목표 정리 — 목적 확정(DR-3)·비전([00](00-vision.md))·요구사항([05](05-requirements.md)) | P0 | 중 | P-1 | ✅ 08-07 |
| P-3 | 기술 기반 문서화 — [ADR-0001](06-adr-0001-stack.md)·[ADR-0002](07-adr-0002-memory-reclaim.md)·[ADR-0003](08-adr-0003-resident-architecture.md) 작성 | P0 | 중 | P-2 | ✅ 08-07 |
| P-4 | ADR 3건 + 예산 수치 사용자 판단 → DR-5~8 승격("다음 진행" 승인) | P0 | 소 | P-3 | ✅ 08-07 |
| P-5 | 15 방법론(scope 규약) · 18 빌드&테스트 SSOT · 크레이트 골격(mk-core/mk-app green) | P1 | 소 | P-4 | ✅ 08-07 |
| P-6 | 로드맵(02) 작성 — M0~M5 정의 | P0 | 소 | P-5 | ✅ 08-07 |
| M0-1 | 트레이 상주 셸 수직 슬라이스 — windows crate 도입·트레이 아이콘·5초 폴링·종료 메뉴([02 M0](02-roadmap.md)) | P0 | 중 | P-6 | ✅ 08-07 |
| M0-2 | 예산 실측 게이트 ✅(08-07) → 단계별 승격은 **`0.6.0` 배포로 흡수**(중간 버전 태그 생략) | P0 | 소 | M0-1 | ✅ 08-07 |
| M1-1 | 정리 엔진 v1 — `--worker` 정리 실행(영역 ①②)·회수량 알림·승격 재시작([02 M1](02-roadmap.md)) | P0 | 중 | M0-2 | ✅ 08-07 |
| M1-2 | M0·M1 실기 QA → **`0.6.0`에 포함되어 배포**. 24h 상주 관찰은 릴리스 후 계속 | P0 | 소 | M1-1 | ✅ 08-07 |
| M2-1 | Top 5 팝업(셸 내) — `NtQuerySystemInformation` 열거·2000ms 자동 갱신·항목 트림/종료([02 M2](02-roadmap.md)) | P0 | 중 | M1-1 | ✅ 08-07 |
| M3-1 | 메인 화면 — 메뉴바·트리 그리드·헤더 정렬·쓰레드 확장·프로세스/트리/쓰레드 종료·자동 실행(DR-10~12) | P0 | 대 | M2-1 | ✅ 08-07 |
| M3-2 | 메인 화면 보강 — 평면/트리 전환·**이름 검색**·의사 프로세스 숨김·도움말 링크·앱 아이콘 | P0 | 중 | M3-1 | ✅ 08-07 |
| M3-4 | 메인 화면 잔여 — 열 폭 조절·창 위치/보기 모드 기억(M4 ini와 연동) | P1 | 중 | M4-1 | ✅ 08-08 |
| M3-3 | **앱 홈페이지 URL 반영** — 주소 확정 시 `listwnd.rs`의 `URL_APP` 한 줄 채우면 메뉴·정보 창 자동 노출 | P2 | 소 | 사용자 | ☐ **사용자 대기** |
| M4-1 | 설정 — `settings.ini` 저장/로드([09](09-settings-schema.md))·설정 창·임계값/주기 자동 정리·영역 ①~⑤·알림 토글 | P0 | 대 | M3-1 | ✅ 08-07 |
| M4-2 | 전역 핫키(`hotkey_clean`) 등록·해제 + 설정 창 입력 UI | P1 | 중 | M4-1 | ✅ 08-07 |
| M4-3 | 다국어(i18n) — 기본 영어·한국어 표·언어 설정(DR-13) | P1 | 중 | M4-1 | ✅ 08-07 |
| M4-4 | 하단 검색창 — 실시간·이름/PID·지우기 버튼·placeholder | P1 | 중 | M3-1 | ✅ 08-07 |
| M5-1 | 영역 ⑥ 페이지 결합 + 저메모리 이벤트 트리거([07 §3](07-adr-0002-memory-reclaim.md)) | P1 | 중 | M4-1 | ✅ 08-07 |
| M5-2 | 배포 2채널(DR-9) — 설치형 Inno(프로그램 목록·자동 실행 질문 기본 체크) + 포터블 ZIP · [20](20-packaging.md) | P0 | 대 | M4-2 | ✅ 08-07(설치본 생성은 Inno 설치 후 실검증) |
| M5-4 | CI/CD — GitHub Actions(ci·release) + winget·choco 패키지 자산([20 §6·§7](20-packaging.md)) | P0 | 중 | M5-2 | ✅ 08-07(게시는 저장소 변수로 차단) |
| M5-6 | Wiki 유지 — UI 변경 시 `scripts/capture-shots.ps1` 재실행 + 해당 페이지 갱신 | P2 | 소 | — | ♻️ 상시 |
| M5-5a | **Chocolatey 등록** — `nexa-memkeeper`·`nexa-memkeeper.portable` 0.7.0 push 완료, 자동 검사·모더레이션 대기([큐](https://ch0.co/moderation)) | P1 | 중 | M5-4 | ✅ 08-08(심사 대기) |
| M5-5b | **winget 등록** — PR 2건 제출 완료([#413829 설치형](https://github.com/microsoft/winget-pkgs/pull/413829) · [#413830 포터블](https://github.com/microsoft/winget-pkgs/pull/413830)), **검증 통과**(`Validation-Completed`) · 사람 승인 대기 | P1 | 소 | M5-4 | ✅ 08-08(승인 대기) |
| M5-5c | **choco 지적 대응** — nuspec 이메일 제거 + `choco resubmit`으로 0.7.0 동일 버전 재제출([20 §7-6](20-packaging.md)). 페이지 상태 `Pending automated review`로 복귀 | P1 | 소 | M5-5a | ✅ 08-11(재심사 대기) |
| M5-5d | 🔴 **choco 제거 스크립트 버그 수정** — `"$packageName:"` 보간이 PS 5.1에서 파싱 실패해 `choco uninstall`이 항상 exit -1([20 §7-8](20-packaging.md)). 수정·BOM 통일·CI 게이트·태그 정리 완료. 승인 기준 전면 대조 결과 **남은 저지 요소 없음**([20 §7-9](20-packaging.md)) — 승인된 0.7.0에는 수정 전 스크립트가 실려(09-29 공개 nupkg 확인) **`0.7.1` 패치로 전 채널 재배포** | P0 | 소 | M5-5c | ✅ 09-29(0.7.1 심사 중) |
| M5-3 | 설치본 실검증 — [최신 릴리스 자산](https://github.com/SosomLab/nexa-memkeeper/releases/latest)으로 설치/제거 왕복 확인([20 §5](20-packaging.md)). 로컬 생성은 Inno 설치 시 |  P0 | 소 | M5-2 | ☐ **사용자 QA** |
| X-3 | **정리 히스토리** — `history.csv`(최근 200줄, 계기 포함) + 메인 화면 메뉴에서 열기 | P2 | 소 | E-1 | ✅ 08-08 |
| X-1 | **워치리스트 표적 트림** — `[watch] enabled/entries`, 프로세스별 쿨다운·보호 목록 준수([09 §5](09-settings-schema.md)) | P1 | 중 | M4-1 | ✅ 08-08 |
| X-2 | **전체화면 억제** — 게임·발표 전체화면 중 자동 정리 보류(`suppress_fullscreen`, 기본 on) | P1 | 소 | M4-1 | ✅ 08-08 |
| E-1 | **알림 회수량에 `Active` 감소분 병기** — 현재 Available 차만 표기해 실제의 1/5에 못 미치게 설명(+679MB vs Active −3,870MB). 정직성 규칙([21 §2-1](21-effectiveness-measurement.md)) | P1 | 소 | — | ✅ 08-08 |
| E-2 | 설정 창 **주기 정리 항목에 비권장 근거 문구** — 회복 반감기 14초라 어떤 주기 설정보다 짧다([21 §8](21-effectiveness-measurement.md)) | P2 | 소 | E-1 | ✅ 08-08 |
| F-1 | **트레이 메뉴 머리줄 버전 하드코딩 제거**(`e9d09cf`) — `0.0.1` 고정 → 버전 리소스와 같은 출처. 검증 57 green·exe 0.675MB([journal/11-18](journal/2026-11-18.md)) · 육안 확인은 사용자 QA | P1 | 소 | — | ✅ 11-18(main ff `89f3770`) |
| C-1 | **워커 생성 헬퍼 `worker_cmd()`** — 생성 5곳 통합, 디버그 빌드에서 부모 콘솔이 없을 때 `CREATE_NO_WINDOW`(콘솔 창 방어 ②) · 개발 | P1 | 소 | — | ✅ 11-18 `3ee60f8` |
| C-2 | **개발 빌드 자동 실행 등록 거부** — `is_dev_exe()`: 디버그이거나 `…\target\{release,debug}\` 바로 아래 exe면 등록 거부 + 안내(③, ①과 함께 막음) · 개발 | P1 | 소 | — | ✅ 11-18 `3ee60f8` |
| C-3 | **choco 포터블 `NexaMemKeeper.exe.gui` 생성** — 콘솔 shim 여지 차단(④). 실제 설치 효과는 다음 choco 릴리스 때 확인 · 개발 | P1 | 소 | — | ✅ 11-18 `3ee60f8` |
| L-1 | **언어 설정 저장 누락 수정** — `to_ini`에 `lang` 출력 + 시험 `lang_survives_roundtrip`(결함 판정 [journal](journal/2026-11-18.md) 9차) · 개발 | P1 | 소 | C-1~C-3 | ✅ 11-18 `f89c94e` |
| L-2 | **기본 언어 주석 정리** — 주석이 `auto`(`settingswnd.rs:321·323`, `config.rs:71`)와 `en`(`settings.rs:56·108`, [09](09-settings-schema.md))으로 갈림 → `en`으로 통일 · 개발 | P2 | 소 | — | ✅ 11-18 `32eb2d5`(주석 3줄) |
| X-5 | **부팅 후 지연 시작** — 로그온 직후 부하 회피([05 §5](05-requirements.md)). 사용자 결정(11-18): 트레이는 즉시, **자동 정리·워치리스트 트림만 보류** · 매 시작마다 · 기본 30초 · 설정 창 '일반'에 입력란 · 키 `[general] start_delay_ms`(09 후보 `start_delay_secs`를 ms로 통일, 09·05는 구현 커밋 때 갱신) · 개발 | P2 | 중 | L-1 | ✅ 11-18 `b28ebc2`(V1 60 green · V2 exe 0.677MB · 유휴 WS 1.43MB) |
| S-1 | **중복 실행 방어(단일 인스턴스)** — 뮤텍스 `Local\NexaMemKeeper.Shell`, 두 번째 실행은 기존 팝업을 열고 종료, 승격 재시작 `--restart` 5초 대기([DR-17](10-decision-record.md), 사용자 확정 11-18). V1 60 green · exe 0.678MB · 유휴 WS 1.41MB · 시험: 두 번째 실행 3초 내 종료(코드 0)·팝업 표시·워커 bench 정상(0). `--restart`는 사용자 QA · 개발 | P2 | 소 | X-5 | ✅ 11-18 `32eb2d5` |
| E-3 | 고압력 구간 재측정 → **중간 압력(1.5~3.0GB)까지 확인, 결론 동일**([21 §11](21-effectiveness-measurement.md)). 수백 MB 스래싱은 장치 한계로 미검증 — 재도전은 별도 장비 필요 | P2 | 중 | — | ✅ 08-08(부분) |
