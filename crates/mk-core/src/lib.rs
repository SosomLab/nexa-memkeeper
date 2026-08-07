//! mk-core — 관찰·정리 엔진의 순수 로직.
//! 수집(시스템 콜)은 mk-app(Win32)에서, 계산·표시·판정은 전부 여기서(단위 테스트 대상).

/// 트레이 색 단계 임계(%) — 기본값. M4에서 settings로 이동한다(docs/05 §4-3).
pub const WARN_PERCENT: u32 = 80;
pub const DANGER_PERCENT: u32 = 90;

/// 시스템 메모리 스냅샷(바이트). `GlobalMemoryStatusEx` 결과의 플랫폼 독립 표현.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemSnapshot {
    pub total: u64,
    pub avail: u64,
}

impl MemSnapshot {
    pub fn used(&self) -> u64 {
        self.total.saturating_sub(self.avail)
    }

    /// 사용률 %(0~100, 반올림). total=0(수집 실패)이면 0.
    pub fn used_percent(&self) -> u32 {
        if self.total == 0 {
            return 0;
        }
        ((self.used() as u128 * 100 + self.total as u128 / 2) / self.total as u128) as u32
    }
}

/// 트레이 아이콘 색 단계(docs/05 §4-3: 정상 / 주의 ≥80% / 위험 ≥90%).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Normal,
    Warn,
    Danger,
}

pub fn level(used_percent: u32) -> Level {
    match used_percent {
        p if p >= DANGER_PERCENT => Level::Danger,
        p if p >= WARN_PERCENT => Level::Warn,
        _ => Level::Normal,
    }
}

/// 툴팁·알림용 용량 문자열 — MB 고정, 소수 1자리(docs/05 FR-1).
pub fn fmt_mb(bytes: u64) -> String {
    format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
}

/// 총량급 용량 문자열 — GB 고정, 소수 1자리(트레이 툴팁용).
pub fn fmt_gb(bytes: u64) -> String {
    format!("{:.1}GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
}

/// 트레이 툴팁 텍스트(FR-1) — 사용률·사용량/전체.
pub fn tray_tooltip(s: &MemSnapshot) -> String {
    format!(
        "Nexa MemKeeper — {}% ({} / {})",
        s.used_percent(),
        fmt_gb(s.used()),
        fmt_gb(s.total)
    )
}

/// 툴팁 둘째 줄(FR-1) — 마지막 정리 회수량·경과.
pub fn fmt_last_clean(mb: u32, elapsed_mins: u64) -> String {
    let ago = match elapsed_mins {
        0 => "방금".to_string(),
        m if m < 60 => format!("{m}분 전"),
        m => format!("{}시간 전", m / 60),
    };
    format!("마지막 정리 {mb}MB · {ago}")
}

// ---- 프로세스 목록 (docs/05 FR-2·FR-3) ----

/// 쓰레드 1개 — 프로세스 확장 시 표시(FR-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadInfo {
    pub tid: u32,
    /// NT 쓰레드 상태(2=Running, 5=Waiting 등) — 표시용.
    pub state: u32,
    pub context_switches: u32,
}

/// 목록 1행 — 열거 결과의 플랫폼 독립 표현(수집은 mk-app, 정렬·표시는 여기).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcInfo {
    pub pid: u32,
    /// 부모 PID(트리 구성용). 0 또는 목록에 없는 값이면 최상위로 취급.
    pub ppid: u32,
    pub name: String,
    /// 프라이빗 워킹셋(바이트) — Top N 정렬 기본 지표(docs/19 §2-2).
    pub private_ws: u64,
    pub ws: u64,
    pub threads: u32,
    pub thread_list: Vec<ThreadInfo>,
}

/// 쓰레드 상태 표시 문자열(NT `KTHREAD_STATE`).
pub fn thread_state_name(state: u32) -> &'static str {
    match state {
        0 => "초기화",
        1 => "준비",
        2 => "실행 중",
        3 => "대기(Standby)",
        4 => "종료됨",
        5 => "대기",
        6 => "전환",
        7 => "대기 중(Deferred)",
        _ => "-",
    }
}

// ---- 트리 구성 (FR-3 — 프로세스 계층 + 쓰레드 확장) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    Process,
    Thread,
}

/// 화면 1행(트리 평면화 결과) — 그리기는 이 구조만 보면 된다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub kind: RowKind,
    /// 들여쓰기 단계(0 = 최상위).
    pub depth: i32,
    pub pid: u32,
    /// 쓰레드 행일 때만 유효.
    pub tid: u32,
    pub label: String,
    pub private_ws: u64,
    pub ws: u64,
    pub threads: u32,
    /// 확장 버튼 표시 여부(자식 프로세스 또는 쓰레드 보유).
    pub expandable: bool,
    pub expanded: bool,
}

/// 프로세스 목록 → 트리 평면화. `expanded`에 든 PID의 자식·쓰레드가 펼쳐진다.
/// 형제 정렬은 `key`/`asc`를 따르고, 부모가 목록에 없으면 최상위로 올린다(고아 보존).
pub fn build_rows(
    procs: &[ProcInfo],
    key: SortKey,
    asc: bool,
    expanded: &std::collections::HashSet<u32>,
) -> Vec<Row> {
    use std::collections::{HashMap, HashSet};
    let present: HashSet<u32> = procs.iter().map(|p| p.pid).collect();
    let mut children: HashMap<u32, Vec<usize>> = HashMap::new();
    let mut roots: Vec<usize> = Vec::new();
    for (i, p) in procs.iter().enumerate() {
        if p.ppid != 0 && p.ppid != p.pid && present.contains(&p.ppid) {
            children.entry(p.ppid).or_default().push(i);
        } else {
            roots.push(i);
        }
    }
    let cmp = |a: &usize, b: &usize| {
        let (x, y) = (&procs[*a], &procs[*b]);
        let ord = match key {
            SortKey::Name => x
                .name
                .to_ascii_lowercase()
                .cmp(&y.name.to_ascii_lowercase()),
            SortKey::Pid => x.pid.cmp(&y.pid),
            SortKey::PrivateWs => x.private_ws.cmp(&y.private_ws),
            SortKey::Ws => x.ws.cmp(&y.ws),
            SortKey::Threads => x.threads.cmp(&y.threads),
        };
        if asc { ord } else { ord.reverse() }.then(x.pid.cmp(&y.pid))
    };
    roots.sort_by(cmp);
    for v in children.values_mut() {
        v.sort_by(cmp);
    }

    let mut out = Vec::with_capacity(procs.len());
    let mut visiting: HashSet<u32> = HashSet::new(); // 순환 보호(비정상 ppid 대비)
    fn walk(
        idx: usize,
        depth: i32,
        procs: &[ProcInfo],
        children: &std::collections::HashMap<u32, Vec<usize>>,
        expanded: &std::collections::HashSet<u32>,
        visiting: &mut std::collections::HashSet<u32>,
        out: &mut Vec<Row>,
    ) {
        let p = &procs[idx];
        if !visiting.insert(p.pid) {
            return;
        }
        let kids = children.get(&p.pid);
        let has_kids = kids.is_some_and(|k| !k.is_empty());
        let is_expanded = expanded.contains(&p.pid);
        out.push(Row {
            kind: RowKind::Process,
            depth,
            pid: p.pid,
            tid: 0,
            label: p.name.clone(),
            private_ws: p.private_ws,
            ws: p.ws,
            threads: p.threads,
            expandable: has_kids || !p.thread_list.is_empty(),
            expanded: is_expanded,
        });
        if is_expanded {
            if let Some(kids) = kids {
                for &c in kids {
                    walk(c, depth + 1, procs, children, expanded, visiting, out);
                }
            }
            for t in &p.thread_list {
                out.push(Row {
                    kind: RowKind::Thread,
                    depth: depth + 1,
                    pid: p.pid,
                    tid: t.tid,
                    label: format!("쓰레드 {} · {}", t.tid, thread_state_name(t.state)),
                    private_ws: 0,
                    ws: 0,
                    threads: t.context_switches,
                    expandable: false,
                    expanded: false,
                });
            }
        }
        visiting.remove(&p.pid);
    }
    for r in roots {
        walk(r, 0, procs, &children, expanded, &mut visiting, &mut out);
    }
    out
}

/// `pid`의 모든 하위 프로세스 PID(자기 자신 제외) — 트리 종료용. 순환에 안전.
pub fn descendants(procs: &[ProcInfo], pid: u32) -> Vec<u32> {
    use std::collections::HashSet;
    let mut out = Vec::new();
    let mut seen: HashSet<u32> = HashSet::from([pid]);
    let mut queue = vec![pid];
    while let Some(cur) = queue.pop() {
        for p in procs.iter().filter(|p| p.ppid == cur && p.pid != cur) {
            if seen.insert(p.pid) {
                out.push(p.pid);
                queue.push(p.pid);
            }
        }
    }
    out
}

/// 점유 상위 N개 — `private_ws` 내림차순, 동률은 PID 오름차순(표시 안정성).
pub fn top_n(mut procs: Vec<ProcInfo>, n: usize) -> Vec<ProcInfo> {
    procs.sort_by(|a, b| b.private_ws.cmp(&a.private_ws).then(a.pid.cmp(&b.pid)));
    procs.truncate(n);
    procs
}

/// 목록 정렬 기준(그리드 헤더 클릭 — FR-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Pid,
    PrivateWs,
    Ws,
    Threads,
}

/// 목록 정렬. 이름은 대소문자 무시, 모든 기준에서 동률은 PID 오름차순으로 안정화한다.
pub fn sort_procs(v: &mut [ProcInfo], key: SortKey, asc: bool) {
    v.sort_by(|a, b| {
        let ord = match key {
            SortKey::Name => a
                .name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase()),
            SortKey::Pid => a.pid.cmp(&b.pid),
            SortKey::PrivateWs => a.private_ws.cmp(&b.private_ws),
            SortKey::Ws => a.ws.cmp(&b.ws),
            SortKey::Threads => a.threads.cmp(&b.threads),
        };
        if asc { ord } else { ord.reverse() }.then(a.pid.cmp(&b.pid))
    });
}

/// 목록 행 용량 표기 — 1GB 미만은 MB 정수, 이상은 GB 소수 1자리.
pub fn fmt_size(bytes: u64) -> String {
    const GB: u64 = 1 << 30;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else {
        format!("{} MB", bytes / (1 << 20))
    }
}

// ---- 보호 프로세스 (docs/05 §4-4 — 종료·트림 하드 제외, 해제 불가) ----

/// 이미지 이름 기준 고정 보호 목록(소문자). PID 0/4·자기 자신·PPL은 호출측에서 추가 차단.
const PROTECTED: &[&str] = &[
    "csrss.exe",
    "wininit.exe",
    "winlogon.exe",
    "services.exe",
    "lsass.exe",
    "smss.exe",
    "dwm.exe",
    "fontdrvhost.exe",
    // 커널 의사 프로세스 — 커널이 주는 이름과 작업 관리자 표기가 달라 둘 다 등록(08-07 실측:
    // NtQuerySystemInformation은 "Memory Compression"으로 준다).
    "memcompression",
    "memory compression",
    "registry",
    "secure system",
    "system",
    "system idle process",
];

/// 보호 프로세스 여부(대소문자 무시). 트림·종료 경로는 이 판정을 반드시 통과해야 한다(15 §4).
pub fn is_protected(image_name: &str) -> bool {
    let lower = image_name.to_ascii_lowercase();
    PROTECTED.contains(&lower.as_str())
}

/// 커널 의사(pseudo) 프로세스 — 실행 이미지가 없는 최소 프로세스. 작업 관리자도 목록에서 숨긴다
/// (숨김 근거·정체는 docs/19 §4-5). 조작 불가이므로 기본은 숨김, 옵션으로 표시.
const PSEUDO: &[&str] = &[
    "memcompression",
    "memory compression",
    "registry",
    "secure system",
];

pub fn is_pseudo(image_name: &str) -> bool {
    let lower = image_name.to_ascii_lowercase();
    PSEUDO.contains(&lower.as_str())
}

/// 표시 목록 필터 — `hide_pseudo`면 의사 프로세스를 제외한다(기본 동작).
pub fn filter_pseudo(procs: Vec<ProcInfo>, hide_pseudo: bool) -> Vec<ProcInfo> {
    if !hide_pseudo {
        return procs;
    }
    procs.into_iter().filter(|p| !is_pseudo(&p.name)).collect()
}

// ---- 워커 결과 프로토콜 (DR-7 — 결과 = 종료 코드 1개) ----

/// 종료 코드 비트 31: 일부 영역이 권한 부족으로 생략됨(관리자 필요).
pub const WORKER_FLAG_PARTIAL: u32 = 0x8000_0000;
/// 워커 내부 오류(예약값 — 회수량으로 해석 금지).
pub const WORKER_ERR: u32 = 0xFFFF_FFFF;
const WORKER_MB_MAX: u32 = 0x7FFF_FFFE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleanOutcome {
    /// 회수량(MB, Avail 전후 차 — docs/19 §4-3).
    pub mb: u32,
    /// true = 영역 일부 생략(승격 필요).
    pub partial: bool,
}

pub fn encode_clean_result(mb: u64, partial: bool) -> u32 {
    let mb = (mb.min(WORKER_MB_MAX as u64)) as u32;
    if partial {
        mb | WORKER_FLAG_PARTIAL
    } else {
        mb
    }
}

/// 워커 종료 코드 해석 — `None` = 오류(크래시·예약값).
pub fn decode_clean_result(code: u32) -> Option<CleanOutcome> {
    if code == WORKER_ERR {
        return None;
    }
    Some(CleanOutcome {
        mb: code & !WORKER_FLAG_PARTIAL,
        partial: code & WORKER_FLAG_PARTIAL != 0,
    })
}

/// 정리 결과 알림 (제목, 본문) — 부풀리기 금지 규칙(docs/19 §4-3): 생략 영역 명시.
pub fn clean_report(outcome: Option<CleanOutcome>) -> (String, String) {
    match outcome {
        Some(o) => {
            let mut body = format!("{}MB 회수", o.mb);
            if o.partial {
                body.push_str(" · 일부 영역 생략(관리자 권한 필요)");
            }
            ("메모리 정리 완료".to_string(), body)
        }
        None => (
            "메모리 정리 실패".to_string(),
            "워커가 비정상 종료했습니다. 다시 시도해 주세요.".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn used_percent_rounds_half_up() {
        // 8GB 중 4GB 사용 = 50%
        let s = MemSnapshot {
            total: 8 << 30,
            avail: 4 << 30,
        };
        assert_eq!(s.used_percent(), 50);
        // 반올림 경계: 1000 중 995 사용 = 99.5% → 100
        let s = MemSnapshot {
            total: 1000,
            avail: 5,
        };
        assert_eq!(s.used_percent(), 100);
        // 1000 중 994 사용 = 99.4% → 99
        let s = MemSnapshot {
            total: 1000,
            avail: 6,
        };
        assert_eq!(s.used_percent(), 99);
    }

    #[test]
    fn used_percent_zero_total_is_zero() {
        let s = MemSnapshot { total: 0, avail: 0 };
        assert_eq!(s.used_percent(), 0);
    }

    #[test]
    fn used_saturates_when_avail_exceeds_total() {
        let s = MemSnapshot {
            total: 100,
            avail: 200,
        };
        assert_eq!(s.used(), 0);
    }

    #[test]
    fn level_boundaries() {
        assert_eq!(level(0), Level::Normal);
        assert_eq!(level(79), Level::Normal);
        assert_eq!(level(80), Level::Warn);
        assert_eq!(level(89), Level::Warn);
        assert_eq!(level(90), Level::Danger);
        assert_eq!(level(100), Level::Danger);
    }

    #[test]
    fn fmt_mb_one_decimal() {
        assert_eq!(fmt_mb(0), "0.0MB");
        assert_eq!(fmt_mb(1024 * 1024), "1.0MB");
        assert_eq!(fmt_mb(1536 * 1024), "1.5MB");
        assert_eq!(fmt_mb(16 * 1024 * 1024 * 1024), "16384.0MB");
    }

    #[test]
    fn fmt_gb_one_decimal() {
        assert_eq!(fmt_gb(16 << 30), "16.0GB");
        assert_eq!(fmt_gb((10 << 30) + (1 << 29)), "10.5GB");
    }

    #[test]
    fn tray_tooltip_shape() {
        let s = MemSnapshot {
            total: 16 << 30,
            avail: 8 << 30,
        };
        assert_eq!(tray_tooltip(&s), "Nexa MemKeeper — 50% (8.0GB / 16.0GB)");
    }

    #[test]
    fn fmt_last_clean_ago() {
        assert_eq!(fmt_last_clean(12, 0), "마지막 정리 12MB · 방금");
        assert_eq!(fmt_last_clean(340, 3), "마지막 정리 340MB · 3분 전");
        assert_eq!(fmt_last_clean(5, 120), "마지막 정리 5MB · 2시간 전");
    }

    fn p(pid: u32, name: &str, private_ws: u64) -> ProcInfo {
        ProcInfo {
            pid,
            ppid: 0,
            name: name.to_string(),
            private_ws,
            ws: private_ws * 2,
            threads: 1,
            thread_list: Vec::new(),
        }
    }

    fn child(pid: u32, ppid: u32, name: &str, private_ws: u64) -> ProcInfo {
        ProcInfo {
            ppid,
            ..p(pid, name, private_ws)
        }
    }

    #[test]
    fn build_rows_nests_children_when_expanded() {
        let procs = vec![
            p(100, "parent.exe", 500),
            child(200, 100, "child.exe", 300),
            child(300, 200, "grand.exe", 100),
        ];
        // 접힌 상태 = 최상위만
        let rows = build_rows(&procs, SortKey::PrivateWs, false, &HashSet::new());
        assert_eq!(rows.len(), 1);
        assert!(rows[0].expandable && !rows[0].expanded);

        // parent만 펼침 → child가 depth 1로
        let rows = build_rows(&procs, SortKey::PrivateWs, false, &HashSet::from([100]));
        assert_eq!(
            rows.iter().map(|r| (r.pid, r.depth)).collect::<Vec<_>>(),
            vec![(100, 0), (200, 1)]
        );
        // 전부 펼침 → 3단계
        let rows = build_rows(
            &procs,
            SortKey::PrivateWs,
            false,
            &HashSet::from([100, 200]),
        );
        assert_eq!(
            rows.iter().map(|r| (r.pid, r.depth)).collect::<Vec<_>>(),
            vec![(100, 0), (200, 1), (300, 2)]
        );
    }

    #[test]
    fn build_rows_lists_threads_after_children() {
        let mut parent = p(10, "a.exe", 100);
        parent.thread_list = vec![
            ThreadInfo {
                tid: 11,
                state: 2,
                context_switches: 5,
            },
            ThreadInfo {
                tid: 12,
                state: 5,
                context_switches: 7,
            },
        ];
        parent.threads = 2;
        let procs = vec![parent, child(20, 10, "b.exe", 50)];
        let rows = build_rows(&procs, SortKey::PrivateWs, false, &HashSet::from([10]));
        let kinds: Vec<_> = rows.iter().map(|r| (r.kind, r.tid)).collect();
        assert_eq!(
            kinds,
            vec![
                (RowKind::Process, 0),
                (RowKind::Process, 0), // 자식 프로세스 먼저
                (RowKind::Thread, 11),
                (RowKind::Thread, 12),
            ]
        );
    }

    #[test]
    fn build_rows_keeps_orphans_at_top_level() {
        // 부모(999)가 목록에 없는 고아 — 최상위로 올라와야 사라지지 않는다
        let procs = vec![child(50, 999, "orphan.exe", 10)];
        let rows = build_rows(&procs, SortKey::PrivateWs, false, &HashSet::new());
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].pid, rows[0].depth), (50, 0));
    }

    #[test]
    fn build_rows_survives_parent_cycle() {
        // 비정상 데이터: 서로가 부모 — 무한 재귀 없이 끝나야 한다
        let procs = vec![child(1, 2, "a.exe", 1), child(2, 1, "b.exe", 1)];
        let rows = build_rows(&procs, SortKey::PrivateWs, false, &HashSet::from([1, 2]));
        assert!(rows.len() <= 4);
    }

    #[test]
    fn descendants_collects_all_levels() {
        let procs = vec![
            p(1, "root.exe", 1),
            child(2, 1, "a.exe", 1),
            child(3, 2, "b.exe", 1),
            child(4, 1, "c.exe", 1),
            p(9, "other.exe", 1),
        ];
        let mut d = descendants(&procs, 1);
        d.sort();
        assert_eq!(d, vec![2, 3, 4]);
        assert!(descendants(&procs, 9).is_empty());
    }

    #[test]
    fn top_n_sorts_desc_and_truncates() {
        let v = vec![
            p(10, "a.exe", 100),
            p(20, "b.exe", 300),
            p(30, "c.exe", 200),
        ];
        let top = top_n(v, 2);
        assert_eq!(
            top.iter().map(|x| x.pid).collect::<Vec<_>>(),
            vec![20, 30] // 300, 200
        );
    }

    #[test]
    fn top_n_ties_break_by_pid() {
        let v = vec![p(30, "a.exe", 100), p(10, "b.exe", 100)];
        assert_eq!(
            top_n(v, 2).iter().map(|x| x.pid).collect::<Vec<_>>(),
            vec![10, 30]
        );
    }

    #[test]
    fn top_n_handles_fewer_than_requested() {
        assert_eq!(top_n(vec![p(1, "a.exe", 5)], 5).len(), 1);
        assert!(top_n(Vec::new(), 5).is_empty());
    }

    #[test]
    fn sort_procs_by_each_key() {
        let mut v = vec![
            p(30, "Beta.exe", 100),
            p(10, "alpha.exe", 300),
            p(20, "Gamma.exe", 200),
        ];
        sort_procs(&mut v, SortKey::PrivateWs, false);
        assert_eq!(
            v.iter().map(|x| x.pid).collect::<Vec<_>>(),
            vec![10, 20, 30]
        );
        sort_procs(&mut v, SortKey::PrivateWs, true);
        assert_eq!(
            v.iter().map(|x| x.pid).collect::<Vec<_>>(),
            vec![30, 20, 10]
        );
        // 이름은 대소문자 무시 — alpha < Beta < Gamma
        sort_procs(&mut v, SortKey::Name, true);
        assert_eq!(
            v.iter().map(|x| x.pid).collect::<Vec<_>>(),
            vec![10, 30, 20]
        );
        sort_procs(&mut v, SortKey::Pid, true);
        assert_eq!(
            v.iter().map(|x| x.pid).collect::<Vec<_>>(),
            vec![10, 20, 30]
        );
    }

    #[test]
    fn sort_procs_ties_stay_pid_ascending() {
        let mut v = vec![
            p(30, "a.exe", 100),
            p(10, "b.exe", 100),
            p(20, "c.exe", 100),
        ];
        sort_procs(&mut v, SortKey::PrivateWs, false);
        assert_eq!(
            v.iter().map(|x| x.pid).collect::<Vec<_>>(),
            vec![10, 20, 30]
        );
    }

    #[test]
    fn fmt_size_switches_unit_at_1gb() {
        assert_eq!(fmt_size(0), "0 MB");
        assert_eq!(fmt_size(512 << 20), "512 MB");
        assert_eq!(fmt_size(1 << 30), "1.0 GB");
        assert_eq!(fmt_size((3 << 30) + (512 << 20)), "3.5 GB");
    }

    #[test]
    fn protected_matches_case_insensitive() {
        assert!(is_protected("LSASS.exe"));
        assert!(is_protected("dwm.exe"));
        assert!(is_protected("MemCompression"));
        // 실측 이름(08-07): 커널은 "Memory Compression"으로 보고한다
        assert!(is_protected("Memory Compression"));
        assert!(is_protected("System"));
        assert!(is_protected("System Idle Process"));
        assert!(!is_protected("notepad.exe"));
        assert!(!is_protected("chrome.exe"));
    }

    #[test]
    fn pseudo_processes_hidden_by_default() {
        assert!(is_pseudo("Memory Compression"));
        assert!(is_pseudo("MemCompression"));
        assert!(is_pseudo("Registry"));
        assert!(is_pseudo("Secure System"));
        // 작업 관리자도 표시하는 실제 프로세스는 의사 프로세스가 아니다
        assert!(!is_pseudo("System"));
        assert!(!is_pseudo("System Idle Process"));
        assert!(!is_pseudo("chrome.exe"));

        let v = vec![
            p(1, "Memory Compression", 900),
            p(2, "chrome.exe", 500),
            p(3, "Registry", 100),
        ];
        let hidden = filter_pseudo(v.clone(), true);
        assert_eq!(hidden.iter().map(|x| x.pid).collect::<Vec<_>>(), vec![2]);
        assert_eq!(filter_pseudo(v, false).len(), 3);
    }

    #[test]
    fn worker_result_roundtrip() {
        let c = decode_clean_result(encode_clean_result(340, false)).unwrap();
        assert_eq!((c.mb, c.partial), (340, false));
        let c = decode_clean_result(encode_clean_result(0, true)).unwrap();
        assert_eq!((c.mb, c.partial), (0, true));
        // 오류 예약값
        assert_eq!(decode_clean_result(WORKER_ERR), None);
        // 클램프: 예약값과 충돌하지 않음
        let c = decode_clean_result(encode_clean_result(u64::MAX, true)).unwrap();
        assert_eq!((c.mb, c.partial), (0x7FFF_FFFE, true));
    }

    #[test]
    fn clean_report_texts() {
        let (t, b) = clean_report(Some(CleanOutcome {
            mb: 340,
            partial: false,
        }));
        assert_eq!(t, "메모리 정리 완료");
        assert_eq!(b, "340MB 회수");
        let (_, b) = clean_report(Some(CleanOutcome {
            mb: 12,
            partial: true,
        }));
        assert!(b.contains("관리자 권한 필요"));
        let (t, _) = clean_report(None);
        assert_eq!(t, "메모리 정리 실패");
    }
}
