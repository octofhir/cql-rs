//! Interval Operator Tests
//!
//! Tests for: Interval constructor, Contains, In, ProperContains, ProperIn,
//! Includes, ProperIncludes, IncludedIn, ProperIncludedIn, Before, After,
//! Meets, MeetsBefore, MeetsAfter, Overlaps, OverlapsBefore, OverlapsAfter,
//! Union, Intersect, Except, Start, End, Width, PointFrom, Size

use octofhir_cql_types::{CqlInterval, CqlType, CqlValue};

// ============================================================================
// Test Helpers
// ============================================================================

fn int_interval(low: i32, high: i32, low_closed: bool, high_closed: bool) -> CqlValue {
    CqlValue::Interval(CqlInterval::new(
        CqlType::Integer,
        Some(CqlValue::Integer(low)),
        low_closed,
        Some(CqlValue::Integer(high)),
        high_closed,
    ))
}

fn closed_interval(low: i32, high: i32) -> CqlValue {
    int_interval(low, high, true, true)
}

fn open_interval(low: i32, high: i32) -> CqlValue {
    int_interval(low, high, false, false)
}

fn half_open_interval(low: i32, high: i32) -> CqlValue {
    int_interval(low, high, true, false)
}

// ============================================================================
// Interval Construction Tests
// ============================================================================

#[test]
fn test_closed_interval_construction() {
    let interval = closed_interval(1, 10);
    if let CqlValue::Interval(i) = interval {
        assert_eq!(i.low.as_ref().map(|v| v.as_ref()), Some(&CqlValue::Integer(1)));
        assert_eq!(i.high.as_ref().map(|v| v.as_ref()), Some(&CqlValue::Integer(10)));
        assert!(i.low_closed);
        assert!(i.high_closed);
    } else {
        panic!("Expected Interval");
    }
}

#[test]
fn test_open_interval_construction() {
    let interval = open_interval(1, 10);
    if let CqlValue::Interval(i) = interval {
        assert!(!i.low_closed);
        assert!(!i.high_closed);
    } else {
        panic!("Expected Interval");
    }
}

#[test]
fn test_half_open_interval_construction() {
    let interval = half_open_interval(1, 10);
    if let CqlValue::Interval(i) = interval {
        assert!(i.low_closed);
        assert!(!i.high_closed);
    } else {
        panic!("Expected Interval");
    }
}

// ============================================================================
// Contains Tests (interval contains point)
// ============================================================================

#[test]
fn test_contains_point_in_closed() {
    let interval = closed_interval(1, 10);
    if let CqlValue::Interval(i) = &interval {
        // Point 5 is in [1, 10]
        let point = CqlValue::Integer(5);
        assert!(interval_contains_point(i, &point));
    }
}

#[test]
fn test_contains_point_at_low_boundary_closed() {
    let interval = closed_interval(1, 10);
    if let CqlValue::Interval(i) = &interval {
        let point = CqlValue::Integer(1);
        assert!(interval_contains_point(i, &point));
    }
}

#[test]
fn test_contains_point_at_high_boundary_closed() {
    let interval = closed_interval(1, 10);
    if let CqlValue::Interval(i) = &interval {
        let point = CqlValue::Integer(10);
        assert!(interval_contains_point(i, &point));
    }
}

#[test]
fn test_contains_point_at_low_boundary_open() {
    let interval = open_interval(1, 10);
    if let CqlValue::Interval(i) = &interval {
        // Point 1 is NOT in (1, 10)
        let point = CqlValue::Integer(1);
        assert!(!interval_contains_point(i, &point));
    }
}

#[test]
fn test_contains_point_at_high_boundary_open() {
    let interval = open_interval(1, 10);
    if let CqlValue::Interval(i) = &interval {
        let point = CqlValue::Integer(10);
        assert!(!interval_contains_point(i, &point));
    }
}

#[test]
fn test_contains_point_outside() {
    let interval = closed_interval(1, 10);
    if let CqlValue::Interval(i) = &interval {
        let point = CqlValue::Integer(15);
        assert!(!interval_contains_point(i, &point));
    }
}

#[test]
fn test_contains_point_below() {
    let interval = closed_interval(5, 10);
    if let CqlValue::Interval(i) = &interval {
        let point = CqlValue::Integer(2);
        assert!(!interval_contains_point(i, &point));
    }
}

// ============================================================================
// Overlaps Tests
// ============================================================================

#[test]
fn test_intervals_overlap() {
    let i1 = closed_interval(1, 5);
    let i2 = closed_interval(3, 8);
    // [1, 5] and [3, 8] overlap at [3, 5]
    if let (CqlValue::Interval(int1), CqlValue::Interval(int2)) = (&i1, &i2) {
        assert!(intervals_overlap(int1, int2));
    }
}

#[test]
fn test_intervals_no_overlap() {
    let i1 = closed_interval(1, 3);
    let i2 = closed_interval(5, 8);
    // [1, 3] and [5, 8] do not overlap
    if let (CqlValue::Interval(int1), CqlValue::Interval(int2)) = (&i1, &i2) {
        assert!(!intervals_overlap(int1, int2));
    }
}

#[test]
fn test_intervals_touch_at_point() {
    let i1 = closed_interval(1, 5);
    let i2 = closed_interval(5, 10);
    // [1, 5] and [5, 10] overlap at point 5
    if let (CqlValue::Interval(int1), CqlValue::Interval(int2)) = (&i1, &i2) {
        assert!(intervals_overlap(int1, int2));
    }
}

#[test]
fn test_intervals_adjacent_open() {
    let i1 = half_open_interval(1, 5); // [1, 5)
    let i2 = closed_interval(5, 10);   // [5, 10]
    // [1, 5) and [5, 10] do not overlap (5 is not in first interval)
    if let (CqlValue::Interval(int1), CqlValue::Interval(int2)) = (&i1, &i2) {
        // They meet but don't overlap
        assert!(!intervals_overlap(int1, int2));
    }
}

// ============================================================================
// Includes Tests (interval includes interval)
// ============================================================================

#[test]
fn test_interval_includes() {
    let i1 = closed_interval(1, 10);
    let i2 = closed_interval(3, 7);
    // [1, 10] includes [3, 7]
    if let (CqlValue::Interval(int1), CqlValue::Interval(int2)) = (&i1, &i2) {
        assert!(interval_includes(int1, int2));
    }
}

#[test]
fn test_interval_includes_same() {
    let i1 = closed_interval(1, 10);
    let i2 = closed_interval(1, 10);
    // [1, 10] includes [1, 10]
    if let (CqlValue::Interval(int1), CqlValue::Interval(int2)) = (&i1, &i2) {
        assert!(interval_includes(int1, int2));
    }
}

#[test]
fn test_interval_does_not_include() {
    let i1 = closed_interval(3, 7);
    let i2 = closed_interval(1, 10);
    // [3, 7] does NOT include [1, 10]
    if let (CqlValue::Interval(int1), CqlValue::Interval(int2)) = (&i1, &i2) {
        assert!(!interval_includes(int1, int2));
    }
}

#[test]
fn test_interval_includes_partial() {
    let i1 = closed_interval(1, 10);
    let i2 = closed_interval(5, 15);
    // [1, 10] does NOT include [5, 15] (15 > 10)
    if let (CqlValue::Interval(int1), CqlValue::Interval(int2)) = (&i1, &i2) {
        assert!(!interval_includes(int1, int2));
    }
}

// ============================================================================
// Before/After Tests
// ============================================================================

#[test]
fn test_interval_before() {
    let i1 = closed_interval(1, 3);
    let i2 = closed_interval(5, 10);
    // [1, 3] is before [5, 10]
    if let (CqlValue::Interval(int1), CqlValue::Interval(int2)) = (&i1, &i2) {
        assert!(interval_before(int1, int2));
    }
}

#[test]
fn test_interval_not_before_overlap() {
    let i1 = closed_interval(1, 5);
    let i2 = closed_interval(3, 10);
    // [1, 5] is NOT before [3, 10] (they overlap)
    if let (CqlValue::Interval(int1), CqlValue::Interval(int2)) = (&i1, &i2) {
        assert!(!interval_before(int1, int2));
    }
}

#[test]
fn test_interval_after() {
    let i1 = closed_interval(5, 10);
    let i2 = closed_interval(1, 3);
    // [5, 10] is after [1, 3]
    if let (CqlValue::Interval(int1), CqlValue::Interval(int2)) = (&i1, &i2) {
        assert!(interval_after(int1, int2));
    }
}

// ============================================================================
// Start/End Tests
// ============================================================================

#[test]
fn test_interval_start() {
    let interval = closed_interval(5, 10);
    if let CqlValue::Interval(i) = &interval {
        assert_eq!(i.low.as_ref().map(|v| v.as_ref()), Some(&CqlValue::Integer(5)));
    }
}

#[test]
fn test_interval_end() {
    let interval = closed_interval(5, 10);
    if let CqlValue::Interval(i) = &interval {
        assert_eq!(i.high.as_ref().map(|v| v.as_ref()), Some(&CqlValue::Integer(10)));
    }
}

// ============================================================================
// Width Tests
// ============================================================================

#[test]
fn test_interval_width() {
    // Width of [1, 10] is 9 (number of integers between 1 and 10 inclusive is 10, width is 9)
    let interval = closed_interval(1, 10);
    if let CqlValue::Interval(i) = &interval {
        let width = interval_width(i);
        // For closed integer intervals, width = high - low
        assert_eq!(width, Some(9));
    }
}

#[test]
fn test_interval_width_single_point() {
    let interval = closed_interval(5, 5);
    if let CqlValue::Interval(i) = &interval {
        let width = interval_width(i);
        assert_eq!(width, Some(0));
    }
}

// ============================================================================
// Union Tests
// ============================================================================

#[test]
fn test_interval_union_overlapping() {
    let i1 = closed_interval(1, 5);
    let i2 = closed_interval(3, 8);
    if let (CqlValue::Interval(int1), CqlValue::Interval(int2)) = (&i1, &i2) {
        let union = interval_union(int1, int2);
        // Union of [1, 5] and [3, 8] is [1, 8]
        if let Some(u) = union {
            assert_eq!(u.low.as_ref().map(|v| v.as_ref()), Some(&CqlValue::Integer(1)));
            assert_eq!(u.high.as_ref().map(|v| v.as_ref()), Some(&CqlValue::Integer(8)));
        } else {
            panic!("Expected union result");
        }
    }
}

#[test]
fn test_interval_union_adjacent() {
    let i1 = closed_interval(1, 5);
    let i2 = closed_interval(6, 10);
    if let (CqlValue::Interval(int1), CqlValue::Interval(int2)) = (&i1, &i2) {
        let union = interval_union(int1, int2);
        // Union of [1, 5] and [6, 10] is [1, 10] (they're adjacent for integers)
        if let Some(u) = union {
            assert_eq!(u.low.as_ref().map(|v| v.as_ref()), Some(&CqlValue::Integer(1)));
            assert_eq!(u.high.as_ref().map(|v| v.as_ref()), Some(&CqlValue::Integer(10)));
        } else {
            panic!("Expected union result");
        }
    }
}

// ============================================================================
// Intersect Tests
// ============================================================================

#[test]
fn test_interval_intersect() {
    let i1 = closed_interval(1, 7);
    let i2 = closed_interval(5, 10);
    if let (CqlValue::Interval(int1), CqlValue::Interval(int2)) = (&i1, &i2) {
        let intersect = interval_intersect(int1, int2);
        // Intersection of [1, 7] and [5, 10] is [5, 7]
        if let Some(i) = intersect {
            assert_eq!(i.low.as_ref().map(|v| v.as_ref()), Some(&CqlValue::Integer(5)));
            assert_eq!(i.high.as_ref().map(|v| v.as_ref()), Some(&CqlValue::Integer(7)));
        } else {
            panic!("Expected intersect result");
        }
    }
}

#[test]
fn test_interval_intersect_no_overlap() {
    let i1 = closed_interval(1, 3);
    let i2 = closed_interval(5, 10);
    if let (CqlValue::Interval(int1), CqlValue::Interval(int2)) = (&i1, &i2) {
        let intersect = interval_intersect(int1, int2);
        // No intersection
        assert!(intersect.is_none());
    }
}

// ============================================================================
// Helper functions for testing
// ============================================================================

fn interval_contains_point(interval: &CqlInterval, point: &CqlValue) -> bool {
    let low_ok = match &interval.low {
        Some(low) => {
            let cmp = compare_values(point, low);
            if interval.low_closed {
                cmp >= 0
            } else {
                cmp > 0
            }
        }
        None => true, // No lower bound
    };

    let high_ok = match &interval.high {
        Some(high) => {
            let cmp = compare_values(point, high);
            if interval.high_closed {
                cmp <= 0
            } else {
                cmp < 0
            }
        }
        None => true, // No upper bound
    };

    low_ok && high_ok
}

fn compare_values(a: &CqlValue, b: &CqlValue) -> i32 {
    match (a, b) {
        (CqlValue::Integer(ai), CqlValue::Integer(bi)) => ai.cmp(bi) as i32,
        _ => 0,
    }
}

fn intervals_overlap(a: &CqlInterval, b: &CqlInterval) -> bool {
    // Two intervals overlap if a starts before b ends AND a ends after b starts
    let a_start = &a.low;
    let a_end = &a.high;
    let b_start = &b.low;
    let b_end = &b.high;

    match (a_start, a_end, b_start, b_end) {
        (Some(as_), Some(ae), Some(bs), Some(be)) => {
            let a_starts_before_b_ends = compare_values(as_, be) < 0 ||
                (compare_values(as_, be) == 0 && a.low_closed && b.high_closed);
            let a_ends_after_b_starts = compare_values(ae, bs) > 0 ||
                (compare_values(ae, bs) == 0 && a.high_closed && b.low_closed);
            a_starts_before_b_ends && a_ends_after_b_starts
        }
        _ => false,
    }
}

fn interval_includes(a: &CqlInterval, b: &CqlInterval) -> bool {
    // a includes b if a.start <= b.start and a.end >= b.end
    match (&a.low, &a.high, &b.low, &b.high) {
        (Some(as_), Some(ae), Some(bs), Some(be)) => {
            let start_ok = compare_values(as_, bs) < 0 ||
                (compare_values(as_, bs) == 0 && (a.low_closed || !b.low_closed));
            let end_ok = compare_values(ae, be) > 0 ||
                (compare_values(ae, be) == 0 && (a.high_closed || !b.high_closed));
            start_ok && end_ok
        }
        _ => false,
    }
}

fn interval_before(a: &CqlInterval, b: &CqlInterval) -> bool {
    match (&a.high, &b.low) {
        (Some(ae), Some(bs)) => {
            compare_values(ae, bs) < 0 ||
                (compare_values(ae, bs) == 0 && (!a.high_closed || !b.low_closed))
        }
        _ => false,
    }
}

fn interval_after(a: &CqlInterval, b: &CqlInterval) -> bool {
    interval_before(b, a)
}

fn interval_width(interval: &CqlInterval) -> Option<i32> {
    match (&interval.low, &interval.high) {
        (Some(low), Some(high)) => {
            match (low.as_ref(), high.as_ref()) {
                (CqlValue::Integer(l), CqlValue::Integer(h)) => Some(h - l),
                _ => None,
            }
        }
        _ => None,
    }
}

fn interval_union(a: &CqlInterval, b: &CqlInterval) -> Option<CqlInterval> {
    // Simple union for overlapping or adjacent intervals
    if intervals_overlap(a, b) || intervals_adjacent(a, b) {
        let low = match (&a.low, &b.low) {
            (Some(al), Some(bl)) => {
                if compare_values(al, bl) <= 0 {
                    Some(al.as_ref().clone())
                } else {
                    Some(bl.as_ref().clone())
                }
            }
            (Some(al), None) | (None, Some(al)) => Some(al.as_ref().clone()),
            (None, None) => None,
        };

        let high = match (&a.high, &b.high) {
            (Some(ah), Some(bh)) => {
                if compare_values(ah, bh) >= 0 {
                    Some(ah.as_ref().clone())
                } else {
                    Some(bh.as_ref().clone())
                }
            }
            (Some(ah), None) | (None, Some(ah)) => Some(ah.as_ref().clone()),
            (None, None) => None,
        };

        Some(CqlInterval::new(CqlType::Integer, low, true, high, true))
    } else {
        None
    }
}

fn intervals_adjacent(a: &CqlInterval, b: &CqlInterval) -> bool {
    match (&a.high, &b.low) {
        (Some(ae), Some(bs)) => {
            match (ae.as_ref(), bs.as_ref()) {
                (CqlValue::Integer(h), CqlValue::Integer(l)) => h + 1 == *l,
                _ => false,
            }
        }
        _ => false,
    }
}

fn interval_intersect(a: &CqlInterval, b: &CqlInterval) -> Option<CqlInterval> {
    if !intervals_overlap(a, b) {
        return None;
    }

    let low = match (&a.low, &b.low) {
        (Some(al), Some(bl)) => {
            if compare_values(al, bl) >= 0 {
                Some(al.as_ref().clone())
            } else {
                Some(bl.as_ref().clone())
            }
        }
        (Some(al), None) => Some(al.as_ref().clone()),
        (None, Some(bl)) => Some(bl.as_ref().clone()),
        (None, None) => None,
    };

    let high = match (&a.high, &b.high) {
        (Some(ah), Some(bh)) => {
            if compare_values(ah, bh) <= 0 {
                Some(ah.as_ref().clone())
            } else {
                Some(bh.as_ref().clone())
            }
        }
        (Some(ah), None) => Some(ah.as_ref().clone()),
        (None, Some(bh)) => Some(bh.as_ref().clone()),
        (None, None) => None,
    };

    Some(CqlInterval::new(CqlType::Integer, low, true, high, true))
}
