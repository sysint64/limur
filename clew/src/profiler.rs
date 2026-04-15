use clew as ui;
use clew::prelude::*;
use parking_lot::Mutex;

use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use clew_derive::Identifiable;
use rustc_hash::{FxHashMap, FxHasher};

pub const DEFAULT_MAX_SNAPSHOTS: usize = 100;

pub static PROFILER: LazyLock<Mutex<Profiler>> =
    LazyLock::new(|| Mutex::new(Profiler::new(DEFAULT_MAX_SNAPSHOTS)));

pub struct Profiler {
    snapshots: VecDeque<ProfilerSnapshot>,
    max_snapshots: usize,
    /// Incremented on every `start_cycle`; tags guards so late drops are ignored.
    generation: u64,
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_SNAPSHOTS)
    }
}

#[derive(Default)]
pub struct ProfilerSnapshot {
    entries: FxHashMap<u64, ProfilerEntry>,
    /// When `start_cycle` was called for this snapshot.
    started_at: Option<Instant>,
    /// Wall-clock duration between `start_cycle` and `end_cycle`.
    /// `None` until `end_cycle` is called.
    cycle_time: Option<Duration>,
}

#[derive(Default, Clone, Identifiable)]
pub struct ProfilerEntry {
    pub id: u64,
    pub name: String,
    pub file_name: String,
    pub line: u32,
    pub column: u32,
    /// Number of times this scope was entered (including recursive re-entries).
    pub count: u32,
    /// Number of outermost completions — used for average to avoid double-counting recursion.
    pub completed_count: u32,
    /// Duration of the most recent completed call.
    pub last_elapsed: Duration,
    /// Sum of wall-clock time across completed outermost calls.
    pub total: Duration,
    /// Current recursion depth (live guards for this entry).
    pub active: u32,
}

impl ProfilerEntry {
    /// Average duration per completed call. Returns `Duration::ZERO` if none completed.
    pub fn average(&self) -> Duration {
        if self.completed_count == 0 {
            Duration::ZERO
        } else {
            self.total / self.completed_count
        }
    }
}

#[must_use = "the profiler guard must be held for the duration of the scope; bind it to `_g` or similar"]
pub struct ProfilerEntryGuard {
    id: u64,
    snapshot_generation: u64,
    start: Instant,
}

impl Drop for ProfilerEntryGuard {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        let mut profiler = PROFILER.lock();
        profiler.leave(self, elapsed);
    }
}

impl Profiler {
    pub fn new(max_snapshots: usize) -> Self {
        assert!(max_snapshots > 0, "max_snapshots must be at least 1");
        Self {
            snapshots: VecDeque::with_capacity(max_snapshots),
            max_snapshots,
            generation: 0,
        }
    }

    pub fn set_max_snapshots(&mut self, max_snapshots: usize) {
        assert!(max_snapshots > 0, "max_snapshots must be at least 1");
        self.max_snapshots = max_snapshots;
        while self.snapshots.len() > self.max_snapshots {
            self.snapshots.pop_front();
        }
    }

    pub fn max_snapshots(&self) -> usize {
        self.max_snapshots
    }

    pub fn current_snapshot(&self) -> &ProfilerSnapshot {
        self.snapshots
            .back()
            .expect("start_cycle should be called before accessing snapshots")
    }

    pub fn previous_snapshot(&self) -> Option<&ProfilerSnapshot> {
        let len = self.snapshots.len();
        if len >= 2 {
            self.snapshots.get(len - 2)
        } else {
            None
        }
    }

    pub fn snapshots(&self) -> impl Iterator<Item = &ProfilerSnapshot> {
        self.snapshots.iter()
    }

    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    pub fn clear(&mut self) {
        self.snapshots.clear();
    }

    fn start_cycle(&mut self) {
        if self.snapshots.len() == self.max_snapshots {
            self.snapshots.pop_front();
        }

        let mut snapshot = ProfilerSnapshot::default();
        snapshot.started_at = Some(Instant::now());
        self.snapshots.push_back(snapshot);
        self.generation = self.generation.wrapping_add(1);
    }

    fn end_cycle(&mut self) {
        let snapshot = self
            .snapshots
            .back_mut()
            .expect("start_cycle should be called before ending cycle");

        if let Some(start) = snapshot.started_at {
            snapshot.cycle_time = Some(start.elapsed());
        }
    }

    #[track_caller]
    fn enter(&mut self, name: Option<&str>) -> ProfilerEntryGuard {
        let location = std::panic::Location::caller();

        let mut hasher = FxHasher::default();
        std::ptr::hash(location.file().as_ptr(), &mut hasher);
        location.line().hash(&mut hasher);
        location.column().hash(&mut hasher);

        if let Some(name) = name {
            name.hash(&mut hasher);
        }

        let id = hasher.finish();

        let generation = self.generation;
        let snapshot = self
            .snapshots
            .back_mut()
            .expect("start_cycle should be called before enter");

        let entry = snapshot.entries.entry(id).or_default();

        entry.id = id;
        entry.count += 1;
        entry.active += 1;
        entry.line = location.line();
        entry.column = location.column();
        entry.file_name = location.file().to_string();
        entry.name = name
            .map(|it| it.to_string())
            .unwrap_or_else(|| location.file().to_string());

        ProfilerEntryGuard {
            id,
            snapshot_generation: generation,
            start: Instant::now(),
        }
    }

    fn leave(&mut self, guard: &ProfilerEntryGuard, elapsed: Duration) {
        if guard.snapshot_generation != self.generation {
            return;
        }
        let Some(snapshot) = self.snapshots.back_mut() else {
            return;
        };
        let Some(entry) = snapshot.entries.get_mut(&guard.id) else {
            return;
        };

        entry.last_elapsed = elapsed;
        entry.active = entry.active.saturating_sub(1);

        if entry.active == 0 {
            entry.total += elapsed;
            entry.completed_count += 1;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Total,
    Average,
    Count,
    Name,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    Descending,
    Ascending,
}

impl ProfilerSnapshot {
    /// Cycle time if ended, otherwise time since `start_cycle` (live cycle).
    pub fn elapsed(&self) -> Duration {
        match (self.cycle_time, self.started_at) {
            (Some(d), _) => d,
            (None, Some(start)) => start.elapsed(),
            _ => Duration::ZERO,
        }
    }

    /// Entry's share of the cycle, in [0.0, 1.0]. Returns 0.0 if cycle time is zero.
    pub fn share_of_cycle(&self, entry: &ProfilerEntry) -> f64 {
        let cycle = self.elapsed().as_secs_f64();
        if cycle > 0.0 {
            entry.total.as_secs_f64() / cycle
        } else {
            0.0
        }
    }

    /// Return entries sorted by the given key. Numeric keys default to descending
    /// (hottest first); `Name` defaults to ascending alphabetical.
    pub fn sorted(&self, by: SortBy) -> Vec<ProfilerEntry> {
        let default_order = match by {
            SortBy::Name => SortOrder::Ascending,
            _ => SortOrder::Descending,
        };
        self.sorted_with(by, default_order)
    }

    pub fn sorted_with(&self, by: SortBy, order: SortOrder) -> Vec<ProfilerEntry> {
        let mut out: Vec<ProfilerEntry> = self.entries.values().cloned().collect();

        out.sort_by(|a, b| {
            let ord = match by {
                SortBy::Total => a.total.cmp(&b.total),
                SortBy::Average => a.average().cmp(&b.average()),
                SortBy::Count => a.count.cmp(&b.count),
                SortBy::Name => a.name.cmp(&b.name),
            };
            match order {
                SortOrder::Ascending => ord,
                SortOrder::Descending => ord.reverse(),
            }
        });
        out
    }

    /// Convenience: top-N entries by the given key (uses default order).
    pub fn top(&self, by: SortBy, n: usize) -> Vec<ProfilerEntry> {
        let mut v = self.sorted(by);
        v.truncate(n);
        v
    }
}

pub fn start_cycle() {
    PROFILER.lock().start_cycle();
}

pub fn end_cycle() {
    PROFILER.lock().end_cycle();
}

pub fn set_max_snapshots(max: usize) {
    PROFILER.lock().set_max_snapshots(max);
}

#[track_caller]
pub fn scope() -> ProfilerEntryGuard {
    PROFILER.lock().enter(None)
}

#[track_caller]
pub fn scope_named(name: &str) -> ProfilerEntryGuard {
    PROFILER.lock().enter(Some(name))
}

pub fn profiler_overlay(ctx: &mut ui::BuildContext) {
    let (cycle, entries) = {
        let profiler = PROFILER.lock();
        let Some(snapshot) = profiler.previous_snapshot() else {
            return;
        };

        (
            snapshot.elapsed(),
            snapshot.sorted(ui::profiler::SortBy::Total),
        )
    };

    let label_color = ui::ColorRgba::from_hex(0xFFFF0000);
    let header_color = ui::ColorRgba::from_hex(0xFFFFFF00);

    ui::vstack()
        .padding(ui::EdgeInsets::all(16.))
        .background(
            ui::decoration()
                .color(ui::ColorRgba::from_hex(0xFF000000).with_opacity(0.8))
                .build(ctx),
        )
        .spacing(4.)
        .build(ctx, |ctx| {
            ui::text(&format!("Cycle Time: {:?}", cycle))
                .color(header_color)
                .build(ctx);

            ui::hstack().spacing(16.).build(ctx, |ctx| {
                // Name column
                ui::vstack().spacing(2.).build(ctx, |ctx| {
                    ui::text("Name").color(header_color).build(ctx);
                    ui::for_each(&entries).build(ctx, |ctx, it| {
                        ui::text(&it.name).color(label_color).build(ctx)
                    });
                });

                // Share column
                // ui::vstack().spacing(2.).build(ctx, |ctx| {
                //     ui::text("%").color(header_color).build(ctx);
                //     ui::for_each(&entries).build(ctx, |ctx, it| {
                //         let share = snapshot.share_of_cycle(it) * 100.0;
                //         ui::text(&format!("{:.2}%", share))
                //             .color(label_color)
                //             .build(ctx)
                //     });
                // });

                // Total column
                ui::vstack().spacing(2.).build(ctx, |ctx| {
                    ui::text("Total").color(header_color).build(ctx);
                    ui::for_each(&entries).build(ctx, |ctx, it| {
                        ui::text(&format!("{:?}", it.total))
                            .color(label_color)
                            .build(ctx)
                    });
                });

                // Average column
                ui::vstack().spacing(2.).build(ctx, |ctx| {
                    ui::text("Avg").color(header_color).build(ctx);
                    ui::for_each(&entries).build(ctx, |ctx, it| {
                        ui::text(&format!("{:?}", it.average()))
                            .color(label_color)
                            .build(ctx)
                    });
                });

                // Count column
                ui::vstack().spacing(2.).build(ctx, |ctx| {
                    ui::text("N").color(header_color).build(ctx);
                    ui::for_each(&entries).build(ctx, |ctx, it| {
                        ui::text(&format!("{}", it.completed_count))
                            .color(label_color)
                            .build(ctx)
                    });
                });
            });
        });
}
