# 09 · 설정 스키마 — `data\settings.ini` (M4 구현 SSOT)

> ⏱ **시간 단위는 전부 밀리초(ms)로 통일**(사용자 확정 08-07) — 설정 창·ini 파일·코드가 같은 단위를 쓴다.
> 구 키(`poll_secs`·`cooldown_secs`·`interval_mins`)는 읽을 때 자동 환산되고 저장 시 새 키로 대체된다(1회 마이그레이션).
>
> **구현 상태(08-07, M4)**: 파싱·검증(mk-core `settings.rs`)·파일 I/O(mk-app `config.rs`)·설정 창 완료.
> **08-07(M5-1)**: `[auto] low_memory_event`·`[areas] combine`(⑥)까지 구현 완료.
> 미구현: `[watch]`(X-1 후보), `[exclude]`. UI에 없는 키도 파일에 쓰면 보존되며, 구현 시 그대로 살아난다.

> [05 §4](05-requirements.md) 설정 스펙의 구현 상세: 키 이름·형·기본값·범위·UI 배치·적용 시점.
> 저장 위치: exe 옆 `data\settings.ini`(포터블), 쓰기 불가 시 `%LOCALAPPDATA%\NexaMemKeeper\`(FR-5).
> **쓰기는 셸만, 워커는 읽기 전용**(DR-7 — IPC 최소 원칙). 저장 즉시 적용(재시작 불요).
> ⚠️ `후보` 표시 키는 [05 §5](05-requirements.md) 추가 기능 후보(X-n) 채택 시에만 구현.

## 1. `[general]` — 일반

| 키 | 형 | 기본 | 범위/값 | UI |
| --- | --- | --- | --- | --- |
| `poll_ms` | u32 | `5000` | 2000~30000 | 설정 창·일반 |
| `tray_display` | enum | `percent` | `percent` \| `free_mb` \| `used_mb` (후보 X-6) | 〃 |
| `warn_percent` | u32 | `80` | 50~95, danger 미만 | 〃 |
| `danger_percent` | u32 | `90` | warn 초과~99 | 〃 |
| `notify` | bool | `true` | 정리 완료 토스트(회수량 MB) | 트레이 토글 + 설정 창 |
| `hotkey_clean` | str | `""`(off) | 예 `Ctrl+Alt+M` — 수정자 필수. 설정 창에서 **조합을 직접 눌러** 지정(M4-2 구현) | 설정 창·일반 |
| `lang` | enum | **`en`** | `en`(기본) \| `ko` \| `auto`(OS 언어) — 설정 창 콤보. 변경은 재시작 후 적용(DR-13) | 〃 |
| `start_delay_secs` | u32 | `0` | 0~120 — 자동 시작 시 부팅 경쟁 회피(후보 X-5, 설치형 기본 30 권장) | 〃 |

## 2. `[auto]` — 자동 정리 (FR-4 트리거)

| 키 | 형 | 기본 | 범위/값 | UI |
| --- | --- | --- | --- | --- |
| `enabled` | bool | `true` | 임계값 자동 정리 마스터 스위치 | 트레이 토글 + 설정 창 |
| `threshold_percent` | u32 | `90` | 50~95, 5 단위 | 설정 창·자동 정리 |
| `cooldown_ms` | u32 | `300000` | 60000~1800000 — 연속 실행 방지 | 〃 |
| `interval_ms` | u32 | `0`(off) | 0 \| 600000~14400000 — 주기 정리(임계값과 독립) | 〃 |
| `low_memory_event` | bool | **`true`** | Windows의 저메모리 알림(`LowMemoryResourceNotification`)이 켜지면 임계값 미만이어도 정리. **쿨다운은 그대로 적용**(M5-1 — [07 §2](07-adr-0002-memory-reclaim.md)) | 〃 |
| `suppress_fullscreen` | bool | `true` | 전체화면(게임·발표) 중 자동 정리 보류 — `SHQueryUserNotificationState`(후보 X-2) | 〃 |
| `on_battery` | enum | `normal` | `normal` \| `pause`(배터리 시 자동 정리 보류) (후보 X-4) | 〃 |

## 3. `[areas]` — 정리 영역 (FR-4 ①~⑥, 기본 = 안전 우선)

| 키 | 기본 | 대응 영역([07 §3](07-adr-0002-memory-reclaim.md)) |
| --- | --- | --- |
| `working_sets` | `true` | ① 프로세스 워킹셋 트림 |
| `standby_low` | `true` | ② 우선순위 0 스탠바이 퍼지 |
| `standby_all` | `false` | ③ 전체 스탠바이 퍼지 ⚠️ |
| `modified` | `false` | ④ 수정 페이지 플러시 ⚠️ |
| `system_ws` | `false` | ⑤ 시스템 워킹셋 ⚠️ |
| `combine` | `false` | ⑥ 페이지 결합 |

- UI: 설정 창·정리 영역 그룹 — 체크박스 6개, ⚠️ 항목은 부작용 한 줄 표기(05 §4-2 문구).

## 4. `[exclude]` — 제외 목록 (FR-4·§4-4)

| 키 | 형 | 기본 | 비고 |
| --- | --- | --- | --- |
| `list` | str | `""` | `;` 구분 exe 이름(예 `devenv.exe;sqlservr.exe`) — 트림·자동 정리 대상 제외 |

- **보호 고정 목록([05 §4-4](05-requirements.md))은 코드 내장 — ini로 해제 불가**(안전 규율 15 §4).

## 5. `[watch]` — 프로세스 워치리스트 자동 트림 (후보 X-1 채택 시)

| 키 | 형 | 기본 | 비고 |
| --- | --- | --- | --- |
| `enabled` | bool | `false` | |
| `entries` | str | `""` | `이름:임계MB` 목록(예 `chrome.exe:1500;msedge.exe:1200`) — 초과 시 해당 프로세스만 트림 |

## 6. `[ui]` — 표시

| 키 | 형 | 기본 | 범위 |
| --- | --- | --- | --- |
| `topn` | u32 | `5` | 3~10 |
| `topn_metric` | enum | `private_ws` | `private_ws` \| `ws` \| `commit` |
| `list_refresh_ms` | u32 | `2000` | 500~10000 — Top 5 팝업·트리 뷰 열려 있는 동안 사용량 자동 갱신 주기(사용자 확정 08-07). 설정 창·일반 그룹 노출 |
| `view_mode` | enum | **`grouped`** | `grouped`(작업 관리자식 분류·묶음, 기본) \| `tree`(PPID 계층) \| `flat`(전체 나열). 보기 메뉴에서 전환. 구 키 `view_flat`은 1회 마이그레이션(DR-14) |
| `show_threads` | bool | `false` | 확장 시 쓰레드 행 표시 — 기본 off(작업 관리자와 동일). 보기 메뉴에서 토글 |
| `hide_pseudo` | bool | `true` | 의사 프로세스(Memory Compression·Registry·Secure System) 숨김 — 작업 관리자와 같은 기본값(사용자 확정 08-07). 메인 화면 설정 메뉴에서 토글, 해제하면 목록에 표시(여전히 🔒 조작 불가) |

## 7. 규칙

1. **알 수 없는 키는 보존**(왕복 시 유실 금지), 잘못된 값은 기본값 폴백 + 무시 로그 없음(조용히 교정).
2. 범위 밖 값은 경계로 클램프. `warn ≥ danger`처럼 모순이면 두 키 모두 기본값 복원.
3. ini 파싱·직렬화는 mk-core(순수 로직 — 단위 테스트), 파일 I/O·감시는 셸.
4. 스키마 변경은 이 문서와 같은 커밋에서 갱신(16 규칙 4). 키 개명 금지 — 신규 키 추가 + 구키 1회 마이그레이션.
