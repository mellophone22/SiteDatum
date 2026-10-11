//! D5 bounded, fictional ordering model. Excluded from production.
//! SQLite durability/CAS is exercised, but signatures, protected storage, D3
//! recovery artifacts and D4 distributed admission are explicit assumptions.
//! Integers stand for key inventories, not key bytes. No pruning API exists.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};

type Result<T> = std::result::Result<T, &'static str>;
const EVENT: &str = "fictional-owner/workspace/event-1";
const TIP: u64 = 7;
const OLD_FLOOR: u32 = 8;
const NEW_FLOOR: u32 = 9;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Phase {
    Draft,
    PreservedA,
    PreservedBoth,
    PreparedA,
    PreparedBoth,
    CommitIntent,
    FloorObserved,
    AwaitingD6,
    Aborted,
    RecoveryRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum AbortCause {
    RetentionDependencyExpired,
    CloudDataDeleted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Device {
    hot: BTreeSet<u32>,
    history: BTreeSet<u32>,
    copy: BTreeSet<u32>,
    dependencies: BTreeSet<u32>,
    pin: u64,
    generation: u64,
    prepared_generation: Option<u64>,
    fence: bool,
    online: bool,
    preparation_void: bool,
    orphan_copy: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct State {
    event: String,
    phase: Phase,
    devices: [Device; 2],
    sync_writes_stopped: bool,
    abort_cause: Option<AbortCause>,
}

impl State {
    fn fictional() -> Self {
        let device = Device {
            hot: (1..=OLD_FLOOR).collect(),
            history: BTreeSet::new(),
            copy: BTreeSet::new(),
            dependencies: [1, 8].into(),
            pin: TIP,
            generation: 1,
            prepared_generation: None,
            fence: false,
            online: true,
            preparation_void: false,
            orphan_copy: false,
        };
        Self {
            event: EVENT.into(),
            phase: Phase::Draft,
            devices: [device.clone(), device],
            sync_writes_stopped: false,
            abort_cause: None,
        }
    }

    fn preserve(&mut self, index: usize) {
        // Abstract D3 oracle. This does NOT create a vault or recovery artifact.
        // Nine historical/staged versions, but the unchanged hot ring stays at eight.
        self.devices[index].history = (1..=NEW_FLOOR).collect();
        self.devices[index].copy = (1..=NEW_FLOOR).collect();
    }

    fn recheck(&self) -> Result<()> {
        if self.event != EVENT {
            return Err("EventMismatch");
        }
        for d in &self.devices {
            if !d.online {
                return Err("DeviceUnavailable");
            }
            if d.pin != TIP {
                return Err("ParticipantPinChanged");
            }
            if d.prepared_generation != Some(d.generation) || d.preparation_void {
                return Err("PreparationChanged");
            }
            let full_retained: BTreeSet<u32> = (1..=NEW_FLOOR).collect();
            if !full_retained.is_subset(&d.history)
                || !full_retained.is_subset(&d.copy)
                || !d.dependencies.is_subset(&d.history)
                || !d.dependencies.is_subset(&d.copy)
            {
                return Err("PreservationIncomplete");
            }
        }
        Ok(())
    }

    fn abort(&mut self, floor: u32, cause: AbortCause) {
        self.abort_cause = Some(cause);
        self.phase = if floor >= NEW_FLOOR {
            self.sync_writes_stopped = true;
            Phase::RecoveryRequired
        } else {
            Phase::Aborted
        };
        for d in &mut self.devices {
            d.fence = false;
            d.preparation_void = true;
            d.orphan_copy = !d.copy.is_empty();
        }
    }
}

struct Stores {
    directory: PathBuf,
    journal: Connection,
    host: Connection,
}

impl Stores {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("sitedatum-d5-fictional-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let journal = Connection::open(directory.join("journal.sqlite")).unwrap();
        let host = Connection::open(directory.join("host.sqlite")).unwrap();
        journal
            .execute_batch(
                "PRAGMA synchronous=FULL;
             CREATE TABLE journal(id INTEGER PRIMARY KEY CHECK(id=1),
               revision INTEGER NOT NULL, payload TEXT NOT NULL);",
            )
            .unwrap();
        host.execute_batch(
            "PRAGMA synchronous=FULL;
             CREATE TABLE floor(id INTEGER PRIMARY KEY CHECK(id=1),
               version INTEGER NOT NULL, event TEXT);
             INSERT INTO floor VALUES(1,8,NULL);",
        )
        .unwrap();
        let mut stores = Self {
            directory,
            journal,
            host,
        };
        stores.save(0, &State::fictional(), true).unwrap();
        stores
    }

    fn load(&self) -> Result<(i64, State)> {
        let (revision, payload): (i64, String) = self
            .journal
            .query_row("SELECT revision,payload FROM journal WHERE id=1", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .map_err(|_| "JournalUnavailable")?;
        let state: State = serde_json::from_str(&payload).map_err(|_| "JournalInvalid")?;
        if state.event != EVENT {
            return Err("EventMismatch");
        }
        Ok((revision, state))
    }

    fn save(&mut self, revision: i64, state: &State, initial: bool) -> Result<()> {
        let payload = serde_json::to_string(state).map_err(|_| "JournalInvalid")?;
        let changed = if initial {
            self.journal
                .execute("INSERT INTO journal VALUES(1,0,?1)", [payload])
        } else {
            self.journal.execute(
                "UPDATE journal SET revision=revision+1,payload=?1 WHERE id=1 AND revision=?2",
                params![payload, revision],
            )
        }
        .map_err(|_| "JournalUnavailable")?;
        if changed != 1 {
            return Err("JournalConflict");
        }
        Ok(())
    }

    fn floor(&self) -> u32 {
        self.host
            .query_row("SELECT version FROM floor WHERE id=1", [], |r| r.get(0))
            .unwrap()
    }

    fn observe_commit(&self, event: &str) -> Result<()> {
        let (floor, committed): (u32, Option<String>) = self
            .host
            .query_row("SELECT version,event FROM floor WHERE id=1", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .map_err(|_| "HostUnavailable")?;
        if floor != NEW_FLOOR || committed.as_deref() != Some(event) {
            return Err("FloorConflict");
        }
        Ok(())
    }

    fn commit_floor(&self, event: &str) -> Result<()> {
        // Abstract authenticated D4 host boundary, NOT hosted protocol code.
        let (floor, committed): (u32, Option<String>) = self
            .host
            .query_row("SELECT version,event FROM floor WHERE id=1", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .map_err(|_| "HostUnavailable")?;
        if floor == NEW_FLOOR && committed.as_deref() == Some(event) {
            return Ok(());
        }
        if floor != OLD_FLOOR || event != EVENT {
            return Err("FloorConflict");
        }
        let changed = self
            .host
            .execute(
                "UPDATE floor SET version=?1,event=?2 WHERE id=1 AND version=?3 AND event IS NULL",
                params![NEW_FLOOR, event, OLD_FLOOR],
            )
            .map_err(|_| "HostUnavailable")?;
        if changed != 1 {
            return Err("FloorConflict");
        }
        Ok(())
    }

    fn step(&mut self, lose_host_reply: bool) -> Result<()> {
        let (revision, mut state) = self.load()?;
        use Phase::*;
        match state.phase {
            Draft => {
                state.preserve(0);
                state.phase = PreservedA;
            }
            PreservedA => {
                state.preserve(1);
                state.phase = PreservedBoth;
            }
            PreservedBoth | PreparedA => {
                let index = usize::from(state.phase == PreparedA);
                let d = &mut state.devices[index];
                if !d.online || d.pin != TIP {
                    return Err("DeviceUnavailable");
                }
                d.prepared_generation = Some(d.generation);
                state.phase = if index == 0 { PreparedA } else { PreparedBoth };
            }
            PreparedBoth => {
                state.recheck()?;
                state.phase = CommitIntent;
            }
            CommitIntent => {
                state.recheck()?;
                self.commit_floor(&state.event)?;
                if lose_host_reply {
                    return Err("HostOutcomeUnknown");
                }
                state.phase = FloorObserved;
            }
            FloorObserved => {
                state.recheck()?;
                self.observe_commit(&state.event)?;
                // No pruning/finalization. D6 remains absent.
                state.phase = AwaitingD6;
            }
            AwaitingD6 | Aborted | RecoveryRequired => return Ok(()),
        }
        self.save(revision, &state, false)
    }

    fn abort(&mut self) -> Result<()> {
        self.abort_with(AbortCause::RetentionDependencyExpired)
    }

    fn abort_with(&mut self, cause: AbortCause) -> Result<()> {
        let (revision, mut state) = self.load()?;
        // Unknown host state never means uncommitted. Conservatively stop Sync.
        let floor = self
            .host
            .query_row("SELECT version FROM floor WHERE id=1", [], |r| {
                r.get::<_, u32>(0)
            })
            .unwrap_or(NEW_FLOOR);
        state.abort(floor, cause);
        self.save(revision, &state, false)
    }

    fn reopen(&mut self) {
        self.journal = Connection::open(self.directory.join("journal.sqlite")).unwrap();
        self.host = Connection::open(self.directory.join("host.sqlite")).unwrap();
    }

    fn assert_retained(&self) {
        let (_, state) = self.load().unwrap();
        for d in state.devices {
            assert_eq!(d.hot, (1..=OLD_FLOOR).collect());
            assert!(d.dependencies.is_subset(&d.hot));
        }
        assert!([OLD_FLOOR, NEW_FLOOR].contains(&self.floor()));
    }
}

impl Drop for Stores {
    fn drop(&mut self) {
        // Keep handles open only until the fictional fixture is destroyed.
        let journal = std::mem::replace(&mut self.journal, Connection::open_in_memory().unwrap());
        let host = std::mem::replace(&mut self.host, Connection::open_in_memory().unwrap());
        drop(journal);
        drop(host);
        // Only the exact uniquely created test directory, never customer paths.
        std::fs::remove_file(self.directory.join("journal.sqlite")).unwrap();
        std::fs::remove_file(self.directory.join("host.sqlite")).unwrap();
        std::fs::remove_dir(&self.directory).unwrap();
    }
}

#[test]
fn restart_at_every_ordered_boundary_retains_keys_and_resumes_exact_event() {
    for crash_after in 0..=7 {
        let mut stores = Stores::new();
        for _ in 0..crash_after {
            stores.step(false).unwrap();
        }
        stores.reopen();
        stores.assert_retained();
        for _ in 0..8 {
            stores.step(false).unwrap();
        }
        assert_eq!(stores.load().unwrap().1.phase, Phase::AwaitingD6);
        stores.assert_retained();
    }
}

#[test]
fn lost_host_response_replays_same_event_without_lowering_or_reissuing_floor() {
    let mut stores = Stores::new();
    for _ in 0..5 {
        stores.step(false).unwrap();
    }
    let before = stores.load().unwrap();
    assert_eq!(stores.step(true), Err("HostOutcomeUnknown"));
    assert_eq!(stores.load().unwrap(), before);
    assert_eq!(stores.floor(), NEW_FLOOR);
    stores.reopen();
    stores.step(false).unwrap();
    stores.step(false).unwrap();
    assert_eq!(stores.load().unwrap().1.phase, Phase::AwaitingD6);
    assert_eq!(stores.commit_floor("another-event"), Err("FloorConflict"));
    stores.assert_retained();
}

#[test]
fn abort_on_either_side_of_commit_releases_fences_without_removing_copies() {
    for steps in 0..=7 {
        let mut stores = Stores::new();
        for _ in 0..steps {
            stores.step(false).unwrap();
        }
        let floor = stores.floor();
        let (revision, mut state) = stores.load().unwrap();
        for d in &mut state.devices {
            d.fence = true;
        }
        stores.save(revision, &state, false).unwrap();
        stores.abort().unwrap();
        stores.reopen();
        let (_, state) = stores.load().unwrap();
        assert_eq!(stores.floor(), floor);
        assert_eq!(state.sync_writes_stopped, floor == NEW_FLOOR);
        for d in &state.devices {
            assert!(!d.fence && d.preparation_void);
            assert_eq!(d.orphan_copy, !d.copy.is_empty());
        }
        let terminal = stores.load().unwrap();
        stores.step(false).unwrap();
        assert_eq!(stores.load().unwrap(), terminal);
        stores.assert_retained();
    }
}

#[test]
fn changed_generation_pin_missing_copy_or_offline_device_blocks_finalization() {
    for after_commit in [false, true] {
        for damage in 0..4 {
            let mut stores = Stores::new();
            for _ in 0..if after_commit { 6 } else { 4 } {
                stores.step(false).unwrap();
            }
            let (revision, mut state) = stores.load().unwrap();
            match damage {
                0 => state.devices[1].online = false,
                1 => state.devices[1].pin += 1,
                2 => state.devices[1].generation += 1,
                _ => {
                    state.devices[1].copy.remove(&1);
                }
            }
            stores.save(revision, &state, false).unwrap();
            let before = stores.load().unwrap();
            assert!(stores.step(false).is_err());
            assert_eq!(stores.load().unwrap(), before);
            stores.assert_retained();
            stores.abort().unwrap();
            assert!(!stores.load().unwrap().1.devices[1].fence);
        }
    }
}

#[test]
fn ordinary_pull_extends_dependencies_without_granting_mutation_authority() {
    let mut stores = Stores::new();
    for _ in 0..4 {
        stores.step(false).unwrap();
    }
    let (revision, mut state) = stores.load().unwrap();
    state.devices[0].dependencies.insert(2);
    stores.save(revision, &state, false).unwrap();
    for _ in 0..3 {
        stores.step(false).unwrap();
    }
    assert_eq!(stores.load().unwrap().1.phase, Phase::AwaitingD6);
    stores.assert_retained();
}

#[test]
fn stale_journal_revision_and_corrupted_event_cannot_resume_or_overwrite() {
    let mut stores = Stores::new();
    let (revision, state) = stores.load().unwrap();
    stores.step(false).unwrap();
    assert_eq!(stores.save(revision, &state, false), Err("JournalConflict"));
    let (revision, mut state) = stores.load().unwrap();
    state.event = "other-owner/workspace/event".into();
    stores.save(revision, &state, false).unwrap();
    let floor = stores.floor();
    assert_eq!(stores.step(false), Err("EventMismatch"));
    assert_eq!(stores.floor(), floor);
}

#[test]
fn production_excludes_the_ordering_model_and_has_no_model_pruning_surface() {
    let lib = include_str!("lib.rs");
    assert!(
        lib.contains("#[cfg(test)]\nmod sync_v2_retirement_recovery_model;")
            || lib.contains("#[cfg(test)]\r\nmod sync_v2_retirement_recovery_model;")
    );
    assert!(!lib.contains("sync_v2_retirement_recovery_model::"));
}

#[test]
fn absent_or_conflicting_host_evidence_blocks_and_abort_keeps_local_work() {
    for absent in [false, true] {
        let mut stores = Stores::new();
        for _ in 0..6 {
            stores.step(false).unwrap();
        }
        if absent {
            stores.host.execute("DELETE FROM floor", []).unwrap();
        } else {
            stores
                .host
                .execute("UPDATE floor SET event='another-event'", [])
                .unwrap();
        }
        let before = stores.load().unwrap();
        assert_eq!(
            stores.step(false),
            Err(if absent {
                "HostUnavailable"
            } else {
                "FloorConflict"
            })
        );
        assert_eq!(stores.load().unwrap(), before);
        stores.abort().unwrap();
        let (_, state) = stores.load().unwrap();
        assert_eq!(state.phase, Phase::RecoveryRequired);
        assert!(state.sync_writes_stopped);
        assert!(state.devices.iter().all(|d| !d.fence && d.hot.len() == 8));
    }
}

#[test]
fn preserved_hot_ring_alone_cannot_replace_full_retained_inventory() {
    let mut stores = Stores::new();
    for _ in 0..4 {
        stores.step(false).unwrap();
    }
    let (revision, mut state) = stores.load().unwrap();
    state.devices[0].copy = state.devices[0].hot.clone();
    stores.save(revision, &state, false).unwrap();
    assert_eq!(stores.step(false), Err("PreservationIncomplete"));
    stores.assert_retained();
}

#[test]
fn two_local_connections_cannot_both_advance_the_same_journal_revision() {
    let stores = Stores::new();
    let (revision, mut state) = stores.load().unwrap();
    state.preserve(0);
    state.phase = Phase::PreservedA;
    let payload = serde_json::to_string(&state).unwrap();
    let barrier = std::sync::Barrier::new(2);
    let changed = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let path = stores.directory.join("journal.sqlite");
                let payload = payload.clone();
                let barrier = &barrier;
                scope.spawn(move || {
                    let connection = Connection::open(path).unwrap();
                    connection
                        .busy_timeout(std::time::Duration::from_secs(2))
                        .unwrap();
                    barrier.wait();
                    connection.execute(
                    "UPDATE journal SET revision=revision+1,payload=?1 WHERE id=1 AND revision=?2",
                    params![payload, revision],
                ).unwrap()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .sum::<usize>()
    });
    assert_eq!(changed, 1);
    assert_eq!(stores.load().unwrap().0, revision + 1);
    // This proves local CAS, not D4 global admission against a split provider.
    stores.assert_retained();
}

#[test]
fn malformed_journal_is_refused_without_touching_floor_or_keys() {
    let mut stores = Stores::new();
    stores
        .journal
        .execute("UPDATE journal SET payload='not-json'", [])
        .unwrap();
    assert_eq!(stores.step(false), Err("JournalInvalid"));
    assert_eq!(stores.floor(), OLD_FLOOR);
    let raw: String = stores
        .journal
        .query_row("SELECT payload FROM journal", [], |r| r.get(0))
        .unwrap();
    assert_eq!(raw, "not-json");
}

#[test]
fn retention_and_deletion_causes_are_durable_and_never_recreate_hosted_state() {
    for cause in [
        AbortCause::RetentionDependencyExpired,
        AbortCause::CloudDataDeleted,
    ] {
        for committed in [false, true] {
            let mut stores = Stores::new();
            for _ in 0..if committed { 6 } else { 4 } {
                stores.step(false).unwrap();
            }
            let floor = stores.floor();
            stores.abort_with(cause).unwrap();
            stores.reopen();
            let (_, state) = stores.load().unwrap();
            assert_eq!(state.abort_cause, Some(cause));
            assert_eq!(stores.floor(), floor);
            assert!(state.devices.iter().all(|d| !d.fence && d.preparation_void));
            stores.assert_retained();
        }
    }
}
