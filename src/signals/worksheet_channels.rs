use std::collections::HashMap;

use crate::config::worksheet::{
    CacheSelector, SHOCK_CHANNEL_GROUPS, STACKED_LANE_RANGES, TYRE_CHANNEL_GROUPS,
};
use crate::data::ibt_parser::IbtSession;

pub fn build_worksheet_channel_caches(
    session: &IbtSession,
    timeline: &[[f64; 2]],
    dataframe_rows: &[usize],
) -> HashMap<CacheSelector, Vec<[f64; 2]>> {
    let mut caches = HashMap::new();
    for (group, range) in TYRE_CHANNEL_GROUPS.iter().zip(STACKED_LANE_RANGES) {
        build_group(session, timeline, dataframe_rows, group, range, &mut caches);
    }
    for (group, range) in SHOCK_CHANNEL_GROUPS.iter().zip(STACKED_LANE_RANGES) {
        build_group(session, timeline, dataframe_rows, group, range, &mut caches);
    }
    caches
}

fn build_group(
    session: &IbtSession,
    timeline: &[[f64; 2]],
    dataframe_rows: &[usize],
    selectors: &[CacheSelector],
    plot_range: (f64, f64),
    caches: &mut HashMap<CacheSelector, Vec<[f64; 2]>>,
) {
    let channels: Vec<(CacheSelector, Vec<f64>)> = selectors
        .iter()
        .filter_map(|selector| {
            let (column_name, multiplier) = selector.telemetry_source()?;
            let column = session.dataframe.column(column_name).ok()?.f64().ok()?;
            let values = dataframe_rows
                .iter()
                .map(|row| column.get(*row).unwrap_or(0.0) * multiplier)
                .collect();
            Some((*selector, values))
        })
        .collect();

    let range = finite_range(
        channels
            .iter()
            .flat_map(|(_, values)| values.iter().copied()),
    );
    for (selector, values) in channels {
        let points = timeline
            .iter()
            .zip(values)
            .map(|(point, value)| [point[0], scale_to_lane(value, range, plot_range)])
            .collect();
        caches.insert(selector, points);
    }
}

fn finite_range(values: impl IntoIterator<Item = f64>) -> Option<(f64, f64)> {
    values
        .into_iter()
        .filter(|value| value.is_finite())
        .fold(None, |range, value| {
            Some(match range {
                Some((minimum, maximum)) => (minimum.min(value), maximum.max(value)),
                None => (value, value),
            })
        })
}

fn scale_to_lane(value: f64, raw_range: Option<(f64, f64)>, plot_range: (f64, f64)) -> f64 {
    let midpoint = (plot_range.0 + plot_range.1) * 0.5;
    let Some((minimum, maximum)) = raw_range else {
        return midpoint;
    };
    let span = maximum - minimum;
    if !value.is_finite() || span.abs() <= f64::EPSILON {
        return midpoint;
    }
    let padding = span * 0.05;
    let fraction = ((value - (minimum - padding)) / (span + padding * 2.0)).clamp(0.0, 1.0);
    plot_range.0 + fraction * (plot_range.1 - plot_range.0)
}

#[cfg(test)]
mod tests {
    use super::scale_to_lane;

    #[test]
    fn shared_lane_scaling_preserves_relative_channel_values() {
        let lower = scale_to_lane(80.0, Some((70.0, 100.0)), (10.0, 20.0));
        let upper = scale_to_lane(90.0, Some((70.0, 100.0)), (10.0, 20.0));

        assert!(lower < upper);
    }

    #[test]
    fn constant_channel_scaling_uses_lane_midpoint() {
        let value = scale_to_lane(42.0, Some((42.0, 42.0)), (10.0, 20.0));

        assert_eq!(value, 15.0);
    }
}
