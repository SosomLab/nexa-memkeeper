# assets — 앱 아이콘

`scripts/make-icon.ps1`이 생성하는 **재현 가능한** 산출물. 손으로 고치지 말고 스크립트를 고친 뒤 다시 실행한다.

| 파일 | 용도 |
| --- | --- |
| `nexa-memkeeper.ico` | exe 임베드용 다중 해상도 ico(16·20·24·32·40·48·64·128·256) — `build.rs`가 `1 ICON`으로 삽입 |
| `nexa-memkeeper-1024.png` | 1024×1024 마스터(문서·스토어·재생성 확인용) |

## 디자인 근거

`nexa-dir2`의 앱 아이콘 계열을 계승한다(`../../../nexa-dir2/crates/nexa-app/assets/nexa-dir-1024.png`):
**둥근 사각 컨테이너 + 파랑 외곽선(#3B82F6) + 남색 세로 그라데이션 바디 + 초록 글리프(#22C55E)**.

이 앱의 정체성은 글리프로 구분한다(nexa-dir은 폴더+`>_` 터미널 프롬프트):

- **80% 도넛 게이지** — 사용률 표시가 이 앱의 핵심 UI이고, 80%는 트레이 "주의" 임계와 같은 값([docs/05 §4-3](../../../docs/05-requirements.md)).
- **RAM 모듈**(노치·핀 포함) — 대상이 "메모리"임을 한눈에.
- **아래 방향 화살표** — 회수(reclaim) 동작.

## 재생성

```powershell
pwsh -File scripts/make-icon.ps1
```

외부 도구·crate 없이 `System.Drawing`만 사용한다(DR-5 외부 의존 0 지향). 아이콘 파일이 없으면
`build.rs`는 경고만 남기고 임베드를 건너뛰므로 빌드는 계속 성공한다.
