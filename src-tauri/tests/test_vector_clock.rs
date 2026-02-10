//! VectorClock unit tests

use app_lib::sync::VectorClock;

// ──────────────────────── 基础操作 ────────────────────────

#[test]
fn new_creates_clock_at_zero() {
    let clock = VectorClock::new("d1".into());
    assert_eq!(clock.clocks.get("d1"), Some(&0));
    assert_eq!(clock.clocks.len(), 1);
}

#[test]
fn empty_creates_no_entries() {
    let clock = VectorClock::empty();
    assert!(clock.clocks.is_empty());
}

#[test]
fn increment_advances_by_one() {
    let mut clock = VectorClock::new("d1".into());
    clock.increment("d1");
    assert_eq!(clock.clocks["d1"], 1);
    clock.increment("d1");
    assert_eq!(clock.clocks["d1"], 2);
}

#[test]
fn increment_unknown_device_auto_creates() {
    let mut clock = VectorClock::new("d1".into());
    clock.increment("d2"); // d2 didn't exist yet
    assert_eq!(clock.clocks["d2"], 1);
    assert_eq!(clock.clocks.len(), 2);
}

// ──────────────────────── Merge ────────────────────────

#[test]
fn merge_takes_max_per_device() {
    let mut c1 = VectorClock { clocks: [("d1".into(), 3), ("d2".into(), 1)].into() };
    let c2 = VectorClock { clocks: [("d1".into(), 2), ("d2".into(), 5)].into() };
    c1.merge(&c2);
    assert_eq!(c1.clocks["d1"], 3); // max(3,2)
    assert_eq!(c1.clocks["d2"], 5); // max(1,5)
}

#[test]
fn merge_adds_missing_devices() {
    let mut c1 = VectorClock { clocks: [("d1".into(), 1)].into() };
    let c2 = VectorClock { clocks: [("d2".into(), 4)].into() };
    c1.merge(&c2);
    assert_eq!(c1.clocks.len(), 2);
    assert_eq!(c1.clocks["d1"], 1);
    assert_eq!(c1.clocks["d2"], 4);
}

#[test]
fn merge_with_empty_is_noop() {
    let mut c1 = VectorClock { clocks: [("d1".into(), 3)].into() };
    let original = c1.clone();
    c1.merge(&VectorClock::empty());
    assert_eq!(c1, original);
}

// ──────────────────────── happened_before (因果) ────────────────────────

#[test]
fn happened_before_linear_order() {
    let c1 = VectorClock { clocks: [("d1".into(), 1)].into() };
    let c2 = VectorClock { clocks: [("d1".into(), 2)].into() };
    assert!(c1.happened_before(&c2));
    assert!(!c2.happened_before(&c1));
}

#[test]
fn happened_before_multi_device() {
    let c1 = VectorClock { clocks: [("d1".into(), 1), ("d2".into(), 1)].into() };
    let c2 = VectorClock { clocks: [("d1".into(), 2), ("d2".into(), 1)].into() };
    assert!(c1.happened_before(&c2)); // all <=, at least one <
    assert!(!c2.happened_before(&c1));
}

#[test]
fn happened_before_equal_clocks_returns_false() {
    let c1 = VectorClock { clocks: [("d1".into(), 2)].into() };
    let c2 = c1.clone();
    assert!(!c1.happened_before(&c2)); // 相等 ≠ 之前
}

#[test]
fn empty_happened_before_any_nonempty() {
    let c1 = VectorClock::empty();
    let c2 = VectorClock { clocks: [("d1".into(), 1)].into() };
    assert!(c1.happened_before(&c2));
}

// ──────────────────────── conflicts_with (并发) ────────────────────────

#[test]
fn conflicts_when_both_have_unseen_updates() {
    let c1 = VectorClock { clocks: [("d1".into(), 2), ("d2".into(), 1)].into() };
    let c2 = VectorClock { clocks: [("d1".into(), 1), ("d2".into(), 2)].into() };
    assert!(c1.conflicts_with(&c2));
    assert!(c2.conflicts_with(&c1)); // 对称性
}

#[test]
fn no_conflict_when_causal() {
    let c1 = VectorClock { clocks: [("d1".into(), 1)].into() };
    let c2 = VectorClock { clocks: [("d1".into(), 2)].into() };
    assert!(!c1.conflicts_with(&c2));
}

#[test]
fn no_conflict_after_merge() {
    let mut c1 = VectorClock { clocks: [("d1".into(), 2)].into() };
    let c2 = VectorClock { clocks: [("d2".into(), 3)].into() };
    assert!(c1.conflicts_with(&c2));

    c1.merge(&c2);
    assert!(!c1.conflicts_with(&c2)); // merge 后冲突消失
}

#[test]
fn empty_never_conflicts() {
    let c1 = VectorClock::empty();
    let c2 = VectorClock { clocks: [("d1".into(), 1)].into() };
    assert!(!c1.conflicts_with(&c2));
}

// ──────────────────────── LWW 比较 ────────────────────────

#[test]
fn lww_higher_sum_wins() {
    let c1 = VectorClock { clocks: [("d1".into(), 5)].into() };
    let c2 = VectorClock { clocks: [("d2".into(), 3)].into() };
    assert_eq!(c1.compare_for_lww(&c2, "d1", "d2"), std::cmp::Ordering::Greater);
}

#[test]
fn lww_equal_sum_breaks_tie_by_device_id() {
    let c1 = VectorClock { clocks: [("a".into(), 5), ("b".into(), 2)].into() };
    let c2 = VectorClock { clocks: [("a".into(), 4), ("b".into(), 3)].into() };
    // sum=7 vs sum=7 → 按 device_id 字典序: "alpha" < "beta"
    assert_eq!(c1.compare_for_lww(&c2, "alpha", "beta"), std::cmp::Ordering::Less);
}

#[test]
fn sum_across_all_devices() {
    let c = VectorClock { clocks: [("d1".into(), 3), ("d2".into(), 7)].into() };
    assert_eq!(c.sum(), 10);
}

#[test]
fn empty_sum_is_zero() {
    assert_eq!(VectorClock::empty().sum(), 0);
}

// ──────────────────────── 序列化 ────────────────────────

#[test]
fn serde_json_roundtrip() {
    let c = VectorClock { clocks: [("d1".into(), 5), ("d2".into(), 3)].into() };
    let json = serde_json::to_string(&c).unwrap();
    let c2: VectorClock = serde_json::from_str(&json).unwrap();
    assert_eq!(c, c2);
}

// ──────────────────────── 多设备场景 ────────────────────────

#[test]
fn three_device_conflict_when_no_direct_sync() {
    // A 同步过 B，B 同步过 C，但 A 和 C 未直接同步
    let ca = VectorClock { clocks: [("A".into(), 3), ("B".into(), 2)].into() };
    let cc = VectorClock { clocks: [("B".into(), 2), ("C".into(), 4)].into() };
    assert!(ca.conflicts_with(&cc));
}

#[test]
fn three_device_no_conflict_after_full_merge() {
    let mut ca = VectorClock { clocks: [("A".into(), 3), ("B".into(), 2)].into() };
    let cb = VectorClock { clocks: [("A".into(), 3), ("B".into(), 2), ("C".into(), 4)].into() };
    ca.merge(&cb); // A 通过 B 的汇总获得了 C 的信息
    assert!(!ca.conflicts_with(&cb));
}
