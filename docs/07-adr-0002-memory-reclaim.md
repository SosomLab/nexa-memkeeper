# 07 · ADR-0002 — 메모리 관찰·회수 시스템 콜 기법

> 상태: **📐 제안 — 사용자 판단 대기.** [05 FR-4](05-requirements.md) 정리 영역 ①~⑥의 구현 기법과 근거.
> 참조 구현: Mem Reduct·Windows Memory Cleaner·RAMMap(Sysinternals)이 모두 동일 계열 API 사용 — 업계 표준 기법.

## 1. 관찰 (상주부 — 저비용이 생명)

| 기법 | 비용 | 용도 | 판정 |
| --- | --- | --- | --- |
| `GlobalMemoryStatusEx` | 매우 낮음(1콜) | 전체 사용률·트레이 아이콘 갱신 | ✅ 기본 폴링(5초) |
| `GetPerformanceInfo` (PSAPI) | 낮음 | 캐시·커밋 상세(툴팁) | ✅ 툴팁 갱신 시만 |
| `CreateMemoryResourceNotification` + 대기 | **0(이벤트)** | 저메모리 감지 | ✅ 옵션 트리거(§4-1) — 폴링 보조 |
| 성능 카운터(PDH) | 중 | — | ❌ 상주 비용 과다 |

## 2. 프로세스·쓰레드 열거 (Top 5 + 트리 뷰)

| 기법 | 특성 | 판정 |
| --- | --- | --- |
| `NtQuerySystemInformation(SystemProcessInformation)` | **1콜**로 전 프로세스+쓰레드+워킹셋·프라이빗 워킹셋까지 획득. Top 5·트리·쓰레드 뷰가 전부 이 버퍼 하나로 해결 | ✅ **권고** |
| Toolhelp32 스냅샷 | 문서화 API지만 프로세스/쓰레드/메모리를 3회 별도 순회 — 느리고 콜 수 多 | 폴백 |
| `EnumProcesses` + 프로세스별 `OpenProcess`+`GetProcessMemoryInfo` | 프로세스 수(N)만큼 핸들 오픈 — NFR-4(트리 뷰 <300ms) 위협 | ❌ |

- 미문서 리스크: `SystemProcessInformation`은 사실상 반영구 안정(작업 관리자·Process Explorer가 의존). 구조체 버전 가드 + Toolhelp32 폴백으로 방어.

## 3. 회수 기법 — 정리 영역 ①~⑥ 매핑

| 영역([05 §4-2](05-requirements.md)) | API | 권한 | 효과/부작용 |
| --- | --- | --- | --- |
| ① 프로세스 워킹셋 트림 | `OpenProcess(PROCESS_SET_QUOTA)` + `K32EmptyWorkingSet`(또는 `SetProcessWorkingSetSizeEx(-1,-1)`) 전 프로세스 순회 | 일반(자기·동일 사용자) / 관리자+`SeDebugPrivilege`(전체) | 페이지를 스탠바이로 이동 → 가용 증가. 재사용 시 소프트 폴트만 — **안전** |
| ② 우선순위 0 스탠바이 퍼지 | `NtSetSystemInformation(SystemMemoryListInformation, MemoryPurgeLowPriorityStandbyList=5)` | 관리자 + `SeProfileSingleProcessPrivilege` | 저우선 캐시만 제거 — **안전** |
| ③ 전체 스탠바이 퍼지 | 〃 `MemoryPurgeStandbyList=4` | 〃 | 가용 최대 증가, ⚠️ 파일 캐시 전멸 → 일시 체감 저하 |
| ④ 수정 페이지 플러시 | 〃 `MemoryFlushModifiedList=3` | 〃 | 더티 페이지 디스크 기록 후 해제 — ⚠️ 디스크 I/O 유발 |
| ⑤ 시스템 워킹셋 | `SetSystemFileCacheSize(-1,-1,0)` 또는 〃 `MemoryEmptyWorkingSets=2` | 관리자 + `SeIncreaseQuotaPrivilege` | 시스템 캐시 워킹셋 축소 — ⚠️ 재적재 비용 |
| ⑥ 페이지 결합 | `NtSetSystemInformation(SystemCombinePhysicalMemoryInformation)` | 관리자 | 동일 내용 페이지 병합 — 효과 완만·소요 시간 김 |

- ②~⑥은 **미문서 NT API**지만 Win7 이후 십수 년간 안정적이며 RAMMap(MS 공식 Sysinternals)이 동일 경로 사용 — 사실상 준표준. OS 빌드 체크 + 실패 시 조용한 성공 금지(결과 통지에 실패 영역 명시).
- 종료: `WM_CLOSE` 정상 요청 → 타임아웃(기본 3초) 후 `TerminateProcess` — [05 FR-3](05-requirements.md).
- 자기 회수(NFR-3): 기능 창 파괴 후 힙 해제 + 자기 `EmptyWorkingSet` 1콜.

## 4. 권한 전략

| 모드 | 가능 범위 | **실측(08-07, 같은 PC·연속 실행)** |
| --- | --- | --- |
| 일반 실행 | 관찰 전부 · ① 자기+동일 사용자 프로세스 · 종료(동일 사용자) | 트림 **115**개 성공/378 제외 · 영역② `0xC0000061`(STATUS_PRIVILEGE_NOT_HELD) 실패 → partial · 회수 **+322MB** |
| 관리자 승격 | ①~⑥ 전부 · 타 세션 프로세스. 승격 시 `SeDebugPrivilege`·`SeProfileSingleProcessPrivilege`·`SeIncreaseQuotaPrivilege` enable | 트림 **466**개/25 제외 · 영역② NTSTATUS 0 성공 · 회수 **+847MB**(약 2.6배) |

> 📌 **관리자 권한은 필수가 아니다** — 일반 권한으로도 관찰·목록·자기 세션 트림이 모두 동작하고 실제로 회수도 된다(위 +322MB). 승격은 "더 많이 회수"를 위한 선택지이며, 권한이 없어 생략된 영역은 알림에 **명시**한다(조용한 성공 금지 — §5-1).

- 매니페스트 `asInvoker`(일반 시작) + "관리자 권한으로 재시작" 메뉴 + 시작 시 승격은 작업 스케줄러 등록([05 FR-6](05-requirements.md)).

### 4-1. 자동 시작과 권한의 관계 (08-07 정리)

| 자동 시작 방식 | 부팅 시 권한 | UAC 프롬프트 | 상태 |
| --- | --- | --- | --- |
| **`HKCU\…\Run`**(DR-11 — 현재 구현) | **일반 권한** — Windows가 Run 키 항목의 승격을 허용하지 않는다 | 없음 | ✅ 구현됨(설정 메뉴 토글) |
| **작업 스케줄러 + "가장 높은 수준의 권한으로 실행"** | **관리자** | 없음(등록 시 1회만 승격 필요) | ☐ 미구현 — 승격 자동 시작을 원할 때의 표준 해법 |
| 시작 폴더 바로가기 | 일반 권한 | 없음 | 채택 안 함(Run 키와 동등) |

- 즉 **자동 시작 = 일반 권한 시작**이 현재 동작이며, 설정 변경·목록·정리는 그대로 가능하되 정리 강도는 위 표의 "일반 실행" 수준이다.
- 승격이 필요하면 그때 트레이 메뉴 **"관리자 권한으로 재시작"**(UAC 1회)을 쓴다.

## 5. 폴백·가드

1. `NtSetSystemInformation` 실패(STATUS_PRIVILEGE_NOT_HELD 등) → 해당 영역 skip + 결과에 "권한 필요" 표기.
2. `SystemProcessInformation` 구조 이상 감지 → Toolhelp32 폴백(쓰레드 뷰 축소 동작).
3. 보호 프로세스([05 §4-4](05-requirements.md))는 열거 단계에서 플래그 — 회수·종료 경로에 진입 자체 불가.
