use crate::profile::{Row, RowAction, MIN_MS};

/// Per-row timing memory, parallel to the row list.
#[derive(Debug, Clone, Default)]
pub struct RowClock {
    pub last_fire_ms: Option<u64>,
}

/// Indices of `Press` rows that are due at `now_ms`.
///
/// A row that has never fired is due immediately, so a freshly armed profile
/// starts working on the first tick. `Hold` rows are edge-triggered elsewhere
/// and are never returned here. A missing clock entry counts as never-fired,
/// which keeps a shorter clock list from panicking.
pub fn due_rows(rows: &[Row], clocks: &[RowClock], now_ms: u64) -> Vec<usize> {
    let mut due = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        if row.action != RowAction::Press || !row.enabled {
            continue;
        }
        let interval = u64::from(row.ms.max(MIN_MS));
        let last = clocks.get(i).and_then(|c| c.last_fire_ms);
        let ready = match last {
            None => true,
            Some(last) => now_ms.saturating_sub(last) >= interval,
        };
        if ready {
            due.push(i);
        }
    }
    due
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{Row, RowAction, MIN_MS};

    fn row(key: &str, ms: u32, enabled: bool, action: RowAction) -> Row {
        Row {
            id: 0,
            key: key.into(),
            action,
            ms,
            enabled,
            condition: None,
        }
    }

    #[test]
    fn a_never_fired_row_is_due_immediately() {
        let rows = vec![row("1", 1000, true, RowAction::Press)];
        let clocks = vec![RowClock { last_fire_ms: None }];
        assert_eq!(due_rows(&rows, &clocks, 5_000), vec![0]);
    }

    #[test]
    fn a_row_is_not_due_before_its_interval_elapses() {
        let rows = vec![row("1", 1000, true, RowAction::Press)];
        let clocks = vec![RowClock {
            last_fire_ms: Some(1000),
        }];
        assert!(due_rows(&rows, &clocks, 1500).is_empty());
        assert_eq!(due_rows(&rows, &clocks, 2000), vec![0]);
    }

    #[test]
    fn disabled_rows_are_never_due() {
        let rows = vec![row("1", 1000, false, RowAction::Press)];
        let clocks = vec![RowClock { last_fire_ms: None }];
        assert!(due_rows(&rows, &clocks, 100_000).is_empty());
    }

    #[test]
    fn hold_rows_are_never_due() {
        let rows = vec![row("Mouse Left", 1000, true, RowAction::Hold)];
        let clocks = vec![RowClock { last_fire_ms: None }];
        assert!(due_rows(&rows, &clocks, 100_000).is_empty());
    }

    #[test]
    fn an_interval_below_the_floor_is_clamped() {
        let rows = vec![row("1", 0, true, RowAction::Press)];
        let clocks = vec![RowClock {
            last_fire_ms: Some(0),
        }];
        assert!(due_rows(&rows, &clocks, u64::from(MIN_MS) - 1).is_empty());
        assert_eq!(due_rows(&rows, &clocks, u64::from(MIN_MS)), vec![0]);
    }

    #[test]
    fn a_short_clocks_list_is_treated_as_never_fired() {
        let rows = vec![
            row("1", 1000, true, RowAction::Press),
            row("2", 1000, true, RowAction::Press),
        ];
        let clocks = vec![RowClock {
            last_fire_ms: Some(5000),
        }];
        assert_eq!(due_rows(&rows, &clocks, 5000), vec![1]);
    }
}
