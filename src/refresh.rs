use crate::Frame;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateStrategy {
    #[default]
    Full,
    Changed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteRun {
    pub row: usize,
    pub column: usize,
    pub cells: Vec<u8>,
}

pub struct RefreshPlanner {
    strategy: UpdateStrategy,
    forced_refresh: Option<Duration>,
    shadow: Option<Frame>,
    last_full_refresh: Option<Instant>,
}

impl RefreshPlanner {
    pub fn new(strategy: UpdateStrategy, forced_refresh: Option<Duration>) -> Self {
        Self { strategy, forced_refresh, shadow: None, last_full_refresh: None }
    }

    pub fn plan(&self, desired: &Frame, now: Instant) -> (Vec<WriteRun>, bool, bool) {
        let forced = self.last_full_refresh.zip(self.forced_refresh)
            .is_some_and(|(last, cadence)| now.duration_since(last) >= cadence);
        let missing = self.shadow.as_ref().map_or(true, |s| s.dimensions() != desired.dimensions());
        let changed = self.shadow.as_ref() != Some(desired);
        let full = missing || forced || (self.strategy == UpdateStrategy::Full && changed);
        if full { return (full_runs(desired), true, forced); }
        if !changed { return (Vec::new(), false, false); }
        (changed_runs(self.shadow.as_ref().unwrap(), desired), false, false)
    }

    pub fn commit(&mut self, desired: Frame, now: Instant, full: bool) {
        self.shadow = Some(desired);
        if full { self.last_full_refresh = Some(now); }
    }

    pub fn invalidate(&mut self) { self.shadow = None; }
}

fn full_runs(frame: &Frame) -> Vec<WriteRun> {
    frame.lines().enumerate().map(|(row, cells)| WriteRun {
        row, column: 0, cells: cells.to_vec(),
    }).collect()
}

fn changed_runs(old: &Frame, new: &Frame) -> Vec<WriteRun> {
    let width = new.dimensions().columns;
    let mut runs = Vec::new();
    for row in 0..new.dimensions().rows {
        let before = old.row(row);
        let after = new.row(row);
        let mut col = 0;
        while col < width {
            if before[col] == after[col] { col += 1; continue; }
            let start = col;
            while col < width && before[col] != after[col] { col += 1; }
            runs.push(WriteRun { row, column: start, cells: after[start..col].to_vec() });
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_cells_are_coalesced_into_runs() {
        let d = crate::Dimensions::new(6, 1).unwrap();
        let now = Instant::now();
        let mut planner = RefreshPlanner::new(UpdateStrategy::Changed, None);
        let old = Frame::from_lines(d, ["abcdef"]);
        let (_, full, _) = planner.plan(&old, now);
        planner.commit(old, now, full);
        let new = Frame::from_lines(d, ["aXXdYf"]);
        let (runs, full, _) = planner.plan(&new, now);
        assert!(!full);
        assert_eq!(runs, vec![
            WriteRun { row: 0, column: 1, cells: b"XX".to_vec() },
            WriteRun { row: 0, column: 4, cells: b"Y".to_vec() },
        ]);
    }

    #[test]
    fn cadence_forces_full_refresh() {
        let d = crate::Dimensions::new(2, 1).unwrap();
        let now = Instant::now();
        let frame = Frame::from_lines(d, ["ok"]);
        let mut planner = RefreshPlanner::new(UpdateStrategy::Changed, Some(Duration::from_secs(5)));
        let (_, full, _) = planner.plan(&frame, now);
        planner.commit(frame.clone(), now, full);
        assert!(planner.plan(&frame, now + Duration::from_secs(4)).0.is_empty());
        let (_, full, forced) = planner.plan(&frame, now + Duration::from_secs(5));
        assert!(full && forced);
    }

    #[test]
    fn full_strategy_does_not_rewrite_an_unchanged_frame() {
        let d = crate::Dimensions::new(2, 1).unwrap();
        let now = Instant::now();
        let frame = Frame::from_lines(d, ["ok"]);
        let mut planner = RefreshPlanner::new(UpdateStrategy::Full, None);
        let (_, full, _) = planner.plan(&frame, now);
        planner.commit(frame.clone(), now, full);
        assert!(planner.plan(&frame, now).0.is_empty());
    }
}
