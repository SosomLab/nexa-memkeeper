//! mk-core — 관찰·정리 엔진의 순수 로직.
//! 수집(시스템 콜)은 mk-app(Win32)에서, 계산·표시·판정은 전부 여기서(단위 테스트 대상).

pub mod hotkey;
pub mod i18n;
pub mod settings;
pub use hotkey::{hotkey_to_string, parse_hotkey, Hotkey};
pub use i18n::{lang, lang_from_setting, set_lang, tr, Lang};
pub use settings::{Areas, Settings};

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
    level_with(used_percent, WARN_PERCENT, DANGER_PERCENT)
}

/// 설정된 임계값 기준 색 단계(M4 — `settings.ini`의 warn/danger 반영).
pub fn level_with(used_percent: u32, warn: u32, danger: u32) -> Level {
    match used_percent {
        p if p >= danger => Level::Danger,
        p if p >= warn => Level::Warn,
        _ => Level::Normal,
    }
}

/// 자동 정리를 지금 실행해야 하는가(FR-4 트리거) — 임계값·쿨다운·주기를 한 곳에서 판정한다.
/// `since_last_ms` = 마지막 정리 이후 경과(ms — 설정과 같은 단위). 이력이 없으면 `u64::MAX`.
pub fn should_auto_clean(s: &Settings, used_percent: u32, since_last_ms: u64) -> bool {
    should_auto_clean_with(s, used_percent, since_last_ms, false)
}

/// 위와 같되 **저메모리 이벤트**(Windows의 `LowMemoryResourceNotification`)를 함께 본다.
///
/// 이벤트는 폴링의 보조다 — 임계값에 못 미쳐도 OS가 부족을 알리면 즉시 돈다. 다만 **쿨다운은
/// 그대로 지킨다**: 부족 상태는 한동안 계속 켜져 있어서, 안 그러면 폴링마다 정리가 반복된다.
pub fn should_auto_clean_with(
    s: &Settings,
    used_percent: u32,
    since_last_ms: u64,
    low_memory: bool,
) -> bool {
    if !s.auto_enabled {
        return false;
    }
    if since_last_ms < s.cooldown_ms as u64 {
        return false; // 쿨다운 중 — 임계값을 넘겨도 참는다(진동 방지)
    }
    if low_memory && s.low_memory_event {
        return true;
    }
    if used_percent >= s.threshold_percent {
        return true;
    }
    s.interval_ms > 0 && since_last_ms >= s.interval_ms as u64
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
        0 => tr("just now").to_string(),
        m if m < 60 => format!("{m}m ago"),
        m => format!("{}h ago", m / 60),
    };
    format!("{} {mb}MB · {ago}", tr("Last cleanup"))
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
    /// 사람이 읽는 이름 — exe 버전 정보의 `FileDescription`(작업 관리자가 쓰는 값).
    /// 없으면 `name`과 같다. 수집은 mk-app(경로별 캐시).
    pub display_name: String,
    /// 프라이빗 워킹셋(바이트) — Top N 정렬 기본 지표(docs/19 §2-2).
    pub private_ws: u64,
    pub ws: u64,
    pub threads: u32,
    pub thread_list: Vec<ThreadInfo>,
    /// 터미널 세션 ID. 0 = 서비스·시스템 구성 요소(작업 관리자의 "Windows 프로세스" 판정에 쓴다).
    pub session_id: u32,
    /// 보이는 최상위 창 제목들(있으면 "앱"으로 분류). 작업 관리자처럼 앱 아래 자식으로 보여준다.
    pub window_titles: Vec<String>,
}

/// 쓰레드 상태 표시 문자열(NT `KTHREAD_STATE`).
pub fn thread_state_name(state: u32) -> &'static str {
    match state {
        0 => tr("Initialized"),
        1 => tr("Ready"),
        2 => tr("Running"),
        3 => tr("Standby"),
        4 => tr("Terminated"),
        5 => tr("Waiting"),
        6 => tr("Transition"),
        7 => tr("Deferred Ready"),
        _ => "-",
    }
}

// ---- 트리 구성 (FR-3 — 프로세스 계층 + 쓰레드 확장) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// 분류 머리글(앱 / 백그라운드 / Windows) — 선택·조작 대상이 아니다.
    Category,
    /// 같은 실행 파일 묶음(작업 관리자의 `Chrome(12)` 형태).
    Group,
    /// 앱이 띄운 창 — 작업 관리자에서 앱 아래에 붙는 자식 행.
    Window,
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

/// 프로세스 목록 → 트리 평면화. `expanded`에 든 PID의 자식이 펼쳐진다.
/// 형제 정렬은 `key`/`asc`를 따르고, 부모가 목록에 없으면 최상위로 올린다(고아 보존).
///
/// `show_threads`(기본 off — 사용자 확정 08-07): 켜야 쓰레드 행이 함께 펼쳐진다. 작업 관리자처럼
/// **프로세스 계층만 보는 것이 기본**이고, 쓰레드는 진단이 필요할 때만 켠다.
pub fn build_rows(
    procs: &[ProcInfo],
    key: SortKey,
    asc: bool,
    expanded: &std::collections::HashSet<u32>,
    show_threads: bool,
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
    #[allow(clippy::too_many_arguments)] // 내부 재귀 헬퍼 — 인자를 묶으면 오히려 읽기 어렵다
    fn walk(
        idx: usize,
        depth: i32,
        procs: &[ProcInfo],
        children: &std::collections::HashMap<u32, Vec<usize>>,
        expanded: &std::collections::HashSet<u32>,
        show_threads: bool,
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
            label: p.display_name.clone(),
            private_ws: p.private_ws,
            ws: p.ws,
            threads: p.threads,
            expandable: has_kids || (show_threads && !p.thread_list.is_empty()),
            expanded: is_expanded,
        });
        if is_expanded {
            if let Some(kids) = kids {
                for &c in kids {
                    walk(
                        c,
                        depth + 1,
                        procs,
                        children,
                        expanded,
                        show_threads,
                        visiting,
                        out,
                    );
                }
            }
            for t in p.thread_list.iter().filter(|_| show_threads) {
                out.push(Row {
                    kind: RowKind::Thread,
                    depth: depth + 1,
                    pid: p.pid,
                    tid: t.tid,
                    label: format!(
                        "{} {} · {}",
                        tr("Thread"),
                        t.tid,
                        thread_state_name(t.state)
                    ),
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
        walk(
            r,
            0,
            procs,
            &children,
            expanded,
            show_threads,
            &mut visiting,
            &mut out,
        );
    }
    out
}

// ---- 작업 관리자식 분류·묶음 (사용자 확정 08-07 — 기본 보기) ----

/// 목록 최상위 분류. 작업 관리자의 3구획과 같은 기준으로 나눈다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Category {
    /// 보이는 창을 가진 프로그램.
    Apps,
    /// 창 없이 사용자 세션에서 도는 것.
    Background,
    /// 세션 0(서비스·시스템 구성 요소)과 커널 의사 프로세스.
    Windows,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::Apps => "Apps",
            Category::Background => "Background processes",
            Category::Windows => "Windows processes",
        }
    }
}

/// 프로세스 → 분류. 작업 관리자와 같은 기준: 보이는 창이 있으면 앱, 세션 0이면 Windows, 나머지는 백그라운드.
pub fn categorize(p: &ProcInfo) -> Category {
    if !p.window_titles.is_empty() {
        Category::Apps
    } else if p.session_id == 0 || is_pseudo(&p.name) || p.pid <= 4 {
        Category::Windows
    } else {
        Category::Background
    }
}

/// **셸**(바탕화면·시작프로그램을 대신 띄우는 프로세스)은 자식을 거느린 것으로 보지 않는다.
///
/// `explorer.exe`는 시작프로그램·바탕화면에서 실행한 모든 것의 부모가 되므로, 자식을 앱 아래로
/// 넣으면 작업 관리자와 달리 무관한 프로그램(예: `espansod.exe`)이 탐색기 밑으로 들어간다.
/// 작업 관리자도 셸의 자식은 별개 항목으로 보여준다(사용자 확인 08-07).
fn is_shell_host(p: &ProcInfo) -> bool {
    p.name.eq_ignore_ascii_case("explorer.exe")
}

/// 앱 항목이 흡수하는 자손 프로세스들(작업 관리자의 `터미널(3)` 형태).
///
/// 창을 가진 자손은 그 자신이 앱 항목이 되므로 제외하고, 셸은 아무것도 거느리지 않는다.
fn owned_children(procs: &[ProcInfo], idx: usize) -> Vec<usize> {
    if is_shell_host(&procs[idx]) {
        return Vec::new();
    }
    let kids = descendants(procs, procs[idx].pid);
    procs
        .iter()
        .enumerate()
        .filter(|(_, c)| kids.contains(&c.pid) && c.window_titles.is_empty())
        .map(|(i, _)| i)
        .collect()
}

/// 처음 볼 때 **펼쳐 둘** 앱 묶음의 키(gid) 목록 — 자식이 하나뿐이라 접혀 있을 이유가 없는 묶음.
///
/// 펼침 여부 자체는 `expanded`가 유일한 진실이다. 이 값을 **행 구성 단계에서 강제로 열어 버리면
/// 사용자가 접어도 다시 열려 축소 버튼이 먹지 않는다**(08-07 QA). 그래서 "기본값"은 호출 쪽에서
/// 묶음이 처음 등장할 때 한 번만 `expanded`에 심는다.
pub fn auto_expand_groups(procs: &[ProcInfo]) -> Vec<u32> {
    use std::collections::HashMap;
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, p) in procs.iter().enumerate() {
        if categorize(p) == Category::Apps {
            groups
                .entry(p.display_name.to_ascii_lowercase())
                .or_default()
                .push(i);
        }
    }
    groups
        .values()
        .filter_map(|members| {
            let wins: usize = members.iter().map(|&i| procs[i].window_titles.len()).sum();
            let kids: usize = members
                .iter()
                .flat_map(|&m| owned_children(procs, m))
                .filter(|i| !members.contains(i))
                .count();
            (wins + kids <= 1).then(|| members.iter().map(|&i| procs[i].pid).min().unwrap_or(0))
        })
        .collect()
}

/// 작업 관리자식 행 구성: **분류 → 프로그램 묶음 → 개별 프로세스 → (옵션) 쓰레드**.
///
/// 같은 실행 파일이 여러 개면 하나로 묶어 `이름 (N)`으로 보여주고 메모리를 합산한다(작업 관리자의
/// `Microsoft Edge(24)`와 같은 형태). 하나뿐이면 묶음 없이 그 프로세스를 바로 보여준다.
/// `expanded`에는 묶음 키(첫 프로세스 PID)와 프로세스 PID가 함께 들어간다.
pub fn build_grouped_rows(
    procs: &[ProcInfo],
    key: SortKey,
    asc: bool,
    expanded: &std::collections::HashSet<u32>,
    show_threads: bool,
) -> Vec<Row> {
    use std::collections::HashMap;
    // 앱이 거느린 자손은 그 앱 아래에만 둔다 — 백그라운드 구획에 중복 노출 금지
    let owned: std::collections::HashSet<u32> = procs
        .iter()
        .enumerate()
        .filter(|(_, p)| categorize(p) == Category::Apps)
        .flat_map(|(i, _)| owned_children(procs, i))
        .map(|i| procs[i].pid)
        .collect();
    let mut by_cat: HashMap<Category, HashMap<String, Vec<usize>>> = HashMap::new();
    for (i, p) in procs.iter().enumerate() {
        if owned.contains(&p.pid) && categorize(p) != Category::Apps {
            continue; // 앱 하위로 이동
        }
        by_cat
            .entry(categorize(p))
            .or_default()
            .entry(p.display_name.to_ascii_lowercase())
            .or_default()
            .push(i);
    }

    let metric = |p: &ProcInfo| -> u64 {
        match key {
            SortKey::PrivateWs => p.private_ws,
            SortKey::Ws => p.ws,
            SortKey::Threads => p.threads as u64,
            SortKey::Pid => p.pid as u64,
            SortKey::Name => 0,
        }
    };

    let mut out = Vec::with_capacity(procs.len() + 8);
    for cat in [Category::Apps, Category::Background, Category::Windows] {
        let Some(groups) = by_cat.get(&cat) else {
            continue;
        };
        let count: usize = groups.values().map(|v| v.len()).sum();
        out.push(Row {
            kind: RowKind::Category,
            depth: 0,
            pid: 0,
            tid: 0,
            label: format!("{} ({})", cat.label(), count),
            private_ws: groups
                .values()
                .flatten()
                .map(|&i| procs[i].private_ws)
                .sum(),
            ws: groups.values().flatten().map(|&i| procs[i].ws).sum(),
            threads: 0,
            expandable: false,
            expanded: true,
        });

        // 묶음 정렬 — 이름은 사전순, 나머지는 합산값 기준
        let mut keys: Vec<&String> = groups.keys().collect();
        keys.sort_by(|a, b| {
            let (ga, gb) = (&groups[*a], &groups[*b]);
            let ord = match key {
                SortKey::Name => a.cmp(b),
                _ => {
                    let sa: u64 = ga.iter().map(|&i| metric(&procs[i])).sum();
                    let sb: u64 = gb.iter().map(|&i| metric(&procs[i])).sum();
                    sa.cmp(&sb)
                }
            };
            if asc { ord } else { ord.reverse() }.then(a.cmp(b))
        });

        for k in keys {
            let members = &groups[k];
            let first = &procs[members[0]];

            // 앱 구획은 **프로그램 행 + 창 + 도우미 프로세스** 구조(작업 관리자와 동일).
            // 작업 관리자의 `터미널(3)`처럼 앱이 띄운 자손 프로세스도 앱 개수에 포함한다.
            if cat == Category::Apps {
                let wins: Vec<(u32, &String)> = members
                    .iter()
                    .flat_map(|&i| {
                        procs[i]
                            .window_titles
                            .iter()
                            .map(move |t| (procs[i].pid, t))
                    })
                    .collect();
                let kids: Vec<usize> = members
                    .iter()
                    .flat_map(|&m| owned_children(procs, m))
                    .filter(|i| !members.contains(i))
                    .collect();
                let gid = members.iter().map(|&i| procs[i].pid).min().unwrap_or(0);
                let total = members.len() + kids.len();
                let label = if total > 1 {
                    format!("{} ({})", first.display_name, total)
                } else {
                    first.display_name.clone()
                };
                let child_rows = wins.len() + kids.len();
                // 펼침은 `expanded`만 본다 — 여기서 강제로 열면 축소 버튼이 먹지 않는다(08-07 QA).
                // 자식 하나뿐인 묶음의 "처음엔 펼침"은 `auto_expand_groups`로 한 번만 심는다.
                let is_open = expanded.contains(&gid);
                let all = || members.iter().chain(kids.iter());
                out.push(Row {
                    kind: RowKind::Group,
                    depth: 1,
                    pid: gid,
                    tid: 0,
                    label,
                    private_ws: all().map(|&i| procs[i].private_ws).sum(),
                    ws: all().map(|&i| procs[i].ws).sum(),
                    threads: all().map(|&i| procs[i].threads).sum(),
                    expandable: child_rows > 0,
                    expanded: is_open,
                });
                if is_open {
                    for (pid, title) in wins {
                        out.push(Row {
                            kind: RowKind::Window,
                            depth: 2,
                            pid,
                            tid: 0,
                            label: title.clone(),
                            private_ws: 0,
                            ws: 0,
                            threads: 0,
                            expandable: false,
                            expanded: false,
                        });
                    }
                    let mut ks = kids;
                    ks.sort_by_key(|&i| std::cmp::Reverse(procs[i].private_ws));
                    for i in ks {
                        push_proc(&mut out, &procs[i], 2, expanded, show_threads);
                    }
                }
                continue;
            }

            if members.len() == 1 {
                push_proc(&mut out, first, 1, expanded, show_threads);
                continue;
            }
            let gid = members.iter().map(|&i| procs[i].pid).min().unwrap_or(0);
            let is_open = expanded.contains(&gid);
            out.push(Row {
                kind: RowKind::Group,
                depth: 1,
                pid: gid,
                tid: 0,
                label: format!("{} ({})", first.display_name, members.len()),
                private_ws: members.iter().map(|&i| procs[i].private_ws).sum(),
                ws: members.iter().map(|&i| procs[i].ws).sum(),
                threads: members.iter().map(|&i| procs[i].threads).sum(),
                expandable: true,
                expanded: is_open,
            });
            if is_open {
                let mut idx = members.clone();
                idx.sort_by(|&a, &b| {
                    let ord = metric(&procs[a]).cmp(&metric(&procs[b]));
                    if asc { ord } else { ord.reverse() }.then(procs[a].pid.cmp(&procs[b].pid))
                });
                for i in idx {
                    push_proc(&mut out, &procs[i], 2, expanded, show_threads);
                }
            }
        }
    }
    out
}

fn push_proc(
    out: &mut Vec<Row>,
    p: &ProcInfo,
    depth: i32,
    expanded: &std::collections::HashSet<u32>,
    show_threads: bool,
) {
    let is_open = expanded.contains(&p.pid);
    let label = p.display_name.clone();
    out.push(Row {
        kind: RowKind::Process,
        depth,
        pid: p.pid,
        tid: 0,
        label,
        private_ws: p.private_ws,
        ws: p.ws,
        threads: p.threads,
        expandable: show_threads && !p.thread_list.is_empty(),
        expanded: is_open,
    });
    if is_open && show_threads {
        for t in &p.thread_list {
            out.push(Row {
                kind: RowKind::Thread,
                depth: depth + 1,
                pid: p.pid,
                tid: t.tid,
                label: format!(
                    "{} {} · {}",
                    tr("Thread"),
                    t.tid,
                    thread_state_name(t.state)
                ),
                private_ws: 0,
                ws: 0,
                threads: t.context_switches,
                expandable: false,
                expanded: false,
            });
        }
    }
}

/// **평면 목록** — 계층 없이 모든 프로세스를 한 단계로 나열(작업 관리자 "자세히"와 같은 형태).
/// 트리에서는 부모가 접혀 있으면 자식이 보이지 않아 특정 프로그램을 찾기 어렵다(08-07 사용자 사례:
/// `NexaMarkdownViewer.exe`가 `explorer.exe` 아래 접혀 있었다) → 기본 보기는 이쪽.
/// 확장 버튼은 **쓰레드 전개**로만 쓰인다.
pub fn build_rows_flat(
    procs: &[ProcInfo],
    key: SortKey,
    asc: bool,
    expanded: &std::collections::HashSet<u32>,
    show_threads: bool,
) -> Vec<Row> {
    let mut idx: Vec<usize> = (0..procs.len()).collect();
    idx.sort_by(|a, b| {
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
    });
    let mut out = Vec::with_capacity(procs.len());
    for i in idx {
        let p = &procs[i];
        let is_expanded = expanded.contains(&p.pid);
        out.push(Row {
            kind: RowKind::Process,
            depth: 0,
            pid: p.pid,
            tid: 0,
            label: p.display_name.clone(),
            private_ws: p.private_ws,
            ws: p.ws,
            threads: p.threads,
            expandable: show_threads && !p.thread_list.is_empty(),
            expanded: is_expanded,
        });
        if is_expanded && show_threads {
            for t in &p.thread_list {
                out.push(Row {
                    kind: RowKind::Thread,
                    depth: 1,
                    pid: p.pid,
                    tid: t.tid,
                    label: format!(
                        "{} {} · {}",
                        tr("Thread"),
                        t.tid,
                        thread_state_name(t.state)
                    ),
                    private_ws: 0,
                    ws: 0,
                    threads: t.context_switches,
                    expandable: false,
                    expanded: false,
                });
            }
        }
    }
    out
}

/// 검색 필터 — **이름 부분 일치(대소문자 무시) 또는 PID 일치**. 빈 질의는 전체 반환.
///
/// PID는 숫자만 입력했을 때 의미가 있으므로, 질의가 전부 숫자면 PID **부분 일치**도 허용한다
/// (`33` → 33, 336, 1330 …). 이름에 숫자가 든 프로세스(`msedgewebview2.exe`)도 함께 걸린다.
pub fn filter_procs(procs: Vec<ProcInfo>, query: &str) -> Vec<ProcInfo> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return procs;
    }
    let numeric = q.chars().all(|c| c.is_ascii_digit());
    procs
        .into_iter()
        .filter(|p| {
            p.name.to_ascii_lowercase().contains(&q) || (numeric && p.pid.to_string().contains(&q))
        })
        .collect()
}

/// **묶음 보기 검색 필터** — 이름·PID뿐 아니라 **창 제목**도 찾고, 구조를 유지한다.
///
/// 창 제목만 일치하면 그 프로세스를 남기되 **일치한 창만** 자식으로 남긴다. 그래서 `2026`을 찾으면
/// `Slack.exe`(앱 행) + `2026-… - Slack`(창 행) 두 줄이 나온다(사용자 확정 08-07).
pub fn filter_grouped(procs: Vec<ProcInfo>, query: &str) -> Vec<ProcInfo> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return procs;
    }
    let numeric = q.chars().all(|c| c.is_ascii_digit());
    procs
        .into_iter()
        .filter_map(|mut p| {
            let by_name = p.name.to_ascii_lowercase().contains(&q)
                || p.display_name.to_ascii_lowercase().contains(&q)
                || (numeric && p.pid.to_string().contains(&q));
            if by_name {
                return Some(p);
            }
            let hits: Vec<String> = p
                .window_titles
                .iter()
                .filter(|t| t.to_ascii_lowercase().contains(&q))
                .cloned()
                .collect();
            if hits.is_empty() {
                None
            } else {
                p.window_titles = hits; // 일치한 창만 보여준다
                Some(p)
            }
        })
        .collect()
}

/// **트리 검색 필터** — 일치한 프로세스와 **그 조상들**만 남겨 계층을 유지한다(사용자 확정 08-07).
///
/// - 부모가 일치하면 부모만 남는다(자식은 각자 일치해야 남는다).
/// - 자식이 일치하면 자식 + 조상 전부가 남아 **구조가 보인다**.
pub fn filter_tree(procs: Vec<ProcInfo>, query: &str) -> Vec<ProcInfo> {
    use std::collections::{HashMap, HashSet};
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return procs;
    }
    let numeric = q.chars().all(|c| c.is_ascii_digit());
    let parent: HashMap<u32, u32> = procs.iter().map(|p| (p.pid, p.ppid)).collect();

    let mut keep: HashSet<u32> = HashSet::new();
    for p in &procs {
        let hit =
            p.name.to_ascii_lowercase().contains(&q) || (numeric && p.pid.to_string().contains(&q));
        if !hit {
            continue;
        }
        keep.insert(p.pid);
        // 조상 추가(순환 대비 방문 검사)
        let mut cur = p.ppid;
        let mut guard = 0;
        while cur != 0 && keep.insert(cur) && guard < 64 {
            cur = parent.get(&cur).copied().unwrap_or(0);
            guard += 1;
        }
    }
    procs
        .into_iter()
        .filter(|p| keep.contains(&p.pid))
        .collect()
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
            let mut body = format!("{}{}", o.mb, tr("MB reclaimed"));
            if o.partial {
                body.push_str(tr(" · some areas skipped (administrator rights required)"));
            }
            (tr("Memory cleanup complete").to_string(), body)
        }
        None => (
            tr("Memory cleanup failed").to_string(),
            tr("The worker exited abnormally. Please try again.").to_string(),
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
    fn auto_clean_respects_threshold_cooldown_and_interval() {
        // 단위는 전부 ms(08-07 통일) — 기본 임계 90%, 쿨다운 300000ms, 주기 off
        let mut s = Settings::default();
        // 임계 미만 → 실행 안 함
        assert!(!should_auto_clean(&s, 85, u64::MAX));
        // 임계 이상 + 쿨다운 지남 → 실행
        assert!(should_auto_clean(&s, 90, 300_001));
        // 임계 이상이어도 쿨다운 중이면 참는다(진동 방지)
        assert!(!should_auto_clean(&s, 99, 120_000));
        // 자동 정리 off면 무조건 안 함
        s.auto_enabled = false;
        assert!(!should_auto_clean(&s, 99, u64::MAX));
        // 주기 정리: 임계 미만이어도 경과 시간이 주기를 넘으면 실행
        s.auto_enabled = true;
        s.interval_ms = 1_800_000; // 30분
        assert!(!should_auto_clean(&s, 10, 1_799_000));
        assert!(should_auto_clean(&s, 10, 1_800_000));
    }

    #[test]
    fn level_with_uses_configured_thresholds() {
        assert_eq!(level_with(70, 60, 80), Level::Warn);
        assert_eq!(level_with(80, 60, 80), Level::Danger);
        assert_eq!(level_with(59, 60, 80), Level::Normal);
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
        // 기본 언어 = 영어(i18n). 한국어는 i18n 표에서 검증한다.
        let _guard = crate::i18n::LANG_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        set_lang(Lang::En);
        assert_eq!(fmt_last_clean(12, 0), "Last cleanup 12MB · just now");
        assert_eq!(fmt_last_clean(340, 3), "Last cleanup 340MB · 3m ago");
        assert_eq!(fmt_last_clean(5, 120), "Last cleanup 5MB · 2h ago");
    }

    fn p(pid: u32, name: &str, private_ws: u64) -> ProcInfo {
        ProcInfo {
            pid,
            ppid: 0,
            name: name.to_string(),
            display_name: name.to_string(),
            private_ws,
            ws: private_ws * 2,
            threads: 1,
            thread_list: Vec::new(),
            session_id: 1,
            window_titles: Vec::new(),
        }
    }

    fn child(pid: u32, ppid: u32, name: &str, private_ws: u64) -> ProcInfo {
        ProcInfo {
            ppid,
            ..p(pid, name, private_ws)
        }
    }

    fn app(pid: u32, ppid: u32, name: &str, title: &str) -> ProcInfo {
        ProcInfo {
            ppid,
            window_titles: vec![title.to_string()],
            ..p(pid, name, 100)
        }
    }

    #[test]
    fn low_memory_event_triggers_but_respects_cooldown() {
        // 08-07 M5-1: OS가 부족을 알리면 임계값 미만이어도 돈다. 단 쿨다운 중에는 참는다
        // (부족 상태는 한동안 켜져 있어 폴링마다 반복 실행될 수 있다).
        let mut s = Settings::default();
        s.threshold_percent = 90;
        s.cooldown_ms = 300_000;
        assert!(!should_auto_clean_with(&s, 40, u64::MAX, false));
        assert!(should_auto_clean_with(&s, 40, u64::MAX, true));
        assert!(
            !should_auto_clean_with(&s, 40, 120_000, true),
            "쿨다운 우선"
        );
        s.low_memory_event = false;
        assert!(
            !should_auto_clean_with(&s, 40, u64::MAX, true),
            "옵션 off면 무시"
        );
    }

    #[test]
    fn grouped_filter_matches_window_titles() {
        // 08-07 사용자 확정: 창 제목으로 찾으면 **앱 행 + 일치한 창** 두 줄이 나와야 한다.
        let procs = vec![
            app(10, 0, "Slack.exe", "2026-sebang-snop-fp - Slack"),
            app(11, 0, "Code.exe", "readme.md - Code"),
        ];
        let kept = filter_grouped(procs, "2026");
        assert_eq!(kept.len(), 1, "제목이 걸린 프로세스만 남는다");
        assert_eq!(kept[0].window_titles.len(), 1, "일치한 창만 남는다");

        // 자식이 하나뿐인 묶음은 기본 펼침 대상 — 목록 창이 `expanded`에 심어 준 상태를 흉내낸다.
        let open: std::collections::HashSet<u32> = auto_expand_groups(&kept).into_iter().collect();
        assert!(open.contains(&10), "자식 하나뿐인 묶음은 기본 펼침 대상");
        let rows = build_grouped_rows(&kept, SortKey::PrivateWs, false, &open, false);
        let body: Vec<&RowKind> = rows
            .iter()
            .map(|r| &r.kind)
            .filter(|k| **k != RowKind::Category)
            .collect();
        assert_eq!(
            body,
            vec![&RowKind::Group, &RowKind::Window],
            "앱 행과 창 행 두 줄"
        );

        // 08-07 QA: 접으면 접힌 채로 있어야 한다(행 구성이 다시 펼치면 축소 버튼이 먹지 않는다).
        let collapsed = build_grouped_rows(
            &kept,
            SortKey::PrivateWs,
            false,
            &std::collections::HashSet::new(),
            false,
        );
        assert!(
            !collapsed.iter().any(|r| r.kind == RowKind::Window),
            "접힌 묶음은 자식을 그리지 않는다"
        );
    }

    #[test]
    fn shell_children_stay_separate() {
        // 08-07 사용자 확인: 작업 관리자처럼 탐색기(셸)가 띄운 프로그램은 탐색기 밑에 넣지 않는다.
        // 반대로 일반 앱의 도우미 프로세스는 앱 항목에 합산한다(`터미널(3)` 형태).
        let procs = vec![
            app(1, 0, "explorer.exe", "Desktop"),
            child(2, 1, "espansod.exe", 50),
            app(3, 0, "WindowsTerminal.exe", "pwsh"),
            child(4, 3, "OpenConsole.exe", 20),
            child(5, 4, "pwsh.exe", 30),
        ];
        let rows = build_grouped_rows(
            &procs,
            SortKey::PrivateWs,
            false,
            &std::collections::HashSet::new(),
            false,
        );
        let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
        assert!(
            labels.iter().any(|l| l.starts_with("espansod.exe")),
            "셸의 자식은 따로 보인다: {labels:?}"
        );
        assert!(
            labels.contains(&"WindowsTerminal.exe (3)"),
            "앱의 도우미 프로세스는 앱 개수에 포함된다: {labels:?}"
        );
        assert!(
            !labels.iter().any(|l| l.starts_with("OpenConsole.exe (")),
            "도우미는 백그라운드에 중복 노출되지 않는다: {labels:?}"
        );
    }

    #[test]
    fn build_rows_nests_children_when_expanded() {
        let procs = vec![
            p(100, "parent.exe", 500),
            child(200, 100, "child.exe", 300),
            child(300, 200, "grand.exe", 100),
        ];
        // 접힌 상태 = 최상위만
        let rows = build_rows(&procs, SortKey::PrivateWs, false, &HashSet::new(), true);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].expandable && !rows[0].expanded);

        // parent만 펼침 → child가 depth 1로
        let rows = build_rows(
            &procs,
            SortKey::PrivateWs,
            false,
            &HashSet::from([100]),
            true,
        );
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
            true,
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
        let rows = build_rows(
            &procs,
            SortKey::PrivateWs,
            false,
            &HashSet::from([10]),
            true,
        );
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
    fn threads_hidden_unless_enabled() {
        // 기본은 프로세스 계층만(사용자 확정 08-07) — 쓰레드는 옵션을 켜야 보인다
        let mut a = p(10, "a.exe", 100);
        a.thread_list = vec![ThreadInfo {
            tid: 11,
            state: 2,
            context_switches: 1,
        }];
        a.threads = 1;
        let procs = vec![a];
        let expanded = HashSet::from([10]);

        let off = build_rows(&procs, SortKey::PrivateWs, false, &expanded, false);
        assert_eq!(off.len(), 1, "쓰레드가 기본으로 보이면 안 된다");
        assert!(
            !off[0].expandable,
            "쓰레드만 있는 프로세스는 확장 버튼도 없어야 한다"
        );

        let on = build_rows(&procs, SortKey::PrivateWs, false, &expanded, true);
        assert_eq!(on.len(), 2);
        assert_eq!(on[1].kind, RowKind::Thread);

        // 평면 목록도 같은 규칙
        assert_eq!(
            build_rows_flat(&procs, SortKey::PrivateWs, false, &expanded, false).len(),
            1
        );
    }

    #[test]
    fn build_rows_keeps_orphans_at_top_level() {
        // 부모(999)가 목록에 없는 고아 — 최상위로 올라와야 사라지지 않는다
        let procs = vec![child(50, 999, "orphan.exe", 10)];
        let rows = build_rows(&procs, SortKey::PrivateWs, false, &HashSet::new(), true);
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].pid, rows[0].depth), (50, 0));
    }

    #[test]
    fn build_rows_survives_parent_cycle() {
        // 비정상 데이터: 서로가 부모 — 무한 재귀 없이 끝나야 한다
        let procs = vec![child(1, 2, "a.exe", 1), child(2, 1, "b.exe", 1)];
        let rows = build_rows(
            &procs,
            SortKey::PrivateWs,
            false,
            &HashSet::from([1, 2]),
            true,
        );
        assert!(rows.len() <= 4);
    }

    #[test]
    fn flat_rows_show_every_process_regardless_of_parent() {
        // 08-07 사용자 사례 재현: 자식이 접힌 부모 아래 숨지 않아야 한다
        let procs = vec![
            p(100, "explorer.exe", 500),
            child(200, 100, "NexaMarkdownViewer.exe", 300),
            child(300, 200, "child.exe", 100),
        ];
        let rows = build_rows_flat(&procs, SortKey::PrivateWs, false, &HashSet::new(), true);
        assert_eq!(
            rows.iter().map(|r| (r.pid, r.depth)).collect::<Vec<_>>(),
            vec![(100, 0), (200, 0), (300, 0)]
        );
        // 트리 보기에서는 접힌 부모 아래라 최상위 1개만 보인다(대비 확인)
        assert_eq!(
            build_rows(&procs, SortKey::PrivateWs, false, &HashSet::new(), true).len(),
            1
        );
    }

    #[test]
    fn flat_rows_expand_threads_only() {
        let mut a = p(10, "a.exe", 100);
        a.thread_list = vec![ThreadInfo {
            tid: 11,
            state: 2,
            context_switches: 1,
        }];
        let procs = vec![a, child(20, 10, "b.exe", 50)];
        let rows = build_rows_flat(
            &procs,
            SortKey::PrivateWs,
            false,
            &HashSet::from([10]),
            true,
        );
        assert_eq!(
            rows.iter()
                .map(|r| (r.kind, r.pid, r.tid))
                .collect::<Vec<_>>(),
            vec![
                (RowKind::Process, 10, 0),
                (RowKind::Thread, 10, 11),
                (RowKind::Process, 20, 0), // 자식도 같은 단계로 나열
            ]
        );
    }

    #[test]
    fn filter_matches_name_case_insensitively() {
        let procs = vec![
            p(1, "NexaMarkdownViewer.exe", 10),
            p(2, "explorer.exe", 20),
            p(3, "Code.exe", 30),
        ];
        assert_eq!(filter_procs(procs.clone(), "markdown").len(), 1);
        assert_eq!(filter_procs(procs.clone(), "EXE").len(), 3);
        assert_eq!(filter_procs(procs.clone(), "  ").len(), 3); // 공백만 = 전체
        assert!(filter_procs(procs, "zzz").is_empty());
    }

    #[test]
    fn tree_filter_keeps_ancestors() {
        // 08-07 사용자 확정: 자식이 걸리면 조상까지 남아 구조가 보이고,
        // 부모가 걸리면 부모만 남는다(자식은 각자 일치해야 남음).
        let procs = vec![
            p(1, "explorer.exe", 10),
            child(2, 1, "app.exe", 20),
            child(3, 2, "helper.exe", 30),
            p(9, "other.exe", 40),
        ];
        let mut kept: Vec<u32> = filter_tree(procs.clone(), "helper")
            .iter()
            .map(|x| x.pid)
            .collect();
        kept.sort();
        assert_eq!(kept, vec![1, 2, 3], "자식이 걸리면 조상까지 남아야 한다");

        let kept: Vec<u32> = filter_tree(procs.clone(), "explorer")
            .iter()
            .map(|x| x.pid)
            .collect();
        assert_eq!(kept, vec![1], "부모가 걸리면 부모만 남는다");

        assert_eq!(filter_tree(procs, "").len(), 4); // 빈 질의 = 전체
    }

    #[test]
    fn filter_matches_pid_when_query_is_numeric() {
        let procs = vec![
            p(4772, "Slack.exe", 10),
            p(33608, "msedge.exe", 20),
            p(360, "svchost.exe", 30),
        ];
        // 정확한 PID
        assert_eq!(
            filter_procs(procs.clone(), "4772")
                .iter()
                .map(|x| x.pid)
                .collect::<Vec<_>>(),
            vec![4772]
        );
        // 부분 일치 — 336(33608)과 360 둘 다
        let mut hit: Vec<u32> = filter_procs(procs.clone(), "36")
            .iter()
            .map(|x| x.pid)
            .collect();
        hit.sort();
        assert_eq!(hit, vec![360, 33608]);
        // 숫자가 아니면 PID는 보지 않는다(이름만)
        assert!(filter_procs(procs, "47a").is_empty());
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
        set_lang(Lang::En);
        let (t, b) = clean_report(Some(CleanOutcome {
            mb: 340,
            partial: false,
        }));
        assert_eq!(t, "Memory cleanup complete");
        assert_eq!(b, "340MB reclaimed");
        let (_, b) = clean_report(Some(CleanOutcome {
            mb: 12,
            partial: true,
        }));
        assert!(b.contains("administrator rights required"));
        let (t, _) = clean_report(None);
        assert_eq!(t, "Memory cleanup failed");
    }

    #[test]
    fn report_follows_selected_language() {
        set_lang(Lang::Ko);
        let (t, _) = clean_report(Some(CleanOutcome {
            mb: 1,
            partial: false,
        }));
        assert_eq!(t, "메모리 정리 완료");
        set_lang(Lang::En); // 다른 테스트에 영향 주지 않게 복귀
    }
}
