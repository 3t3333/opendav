//! Precomputed, distance-aligned telemetry comparisons between loaded sessions.

use std::error::Error;
use std::fmt;

use polars::prelude::Float64Chunked;

use crate::config::worksheet::{worksheet_channel_group, CacheSelector};
use crate::signals::processing::{LapData, TrackSector};
use crate::LoadedSession;

/// Plot channels backed by raw telemetry and supported by comparison caches.
pub const COMPARISON_CHANNELS: [CacheSelector; 28] = [
    CacheSelector::Speed,
    CacheSelector::RPM,
    CacheSelector::Throttle,
    CacheSelector::Brake,
    CacheSelector::Steering,
    CacheSelector::FrontHeight,
    CacheSelector::RearHeight,
    CacheSelector::Rake,
    CacheSelector::LatG,
    CacheSelector::LongG,
    CacheSelector::Gear,
    CacheSelector::Clutch,
    CacheSelector::LfTempOuter,
    CacheSelector::LfTempCenter,
    CacheSelector::LfTempInner,
    CacheSelector::RfTempInner,
    CacheSelector::RfTempCenter,
    CacheSelector::RfTempOuter,
    CacheSelector::LrTempOuter,
    CacheSelector::LrTempCenter,
    CacheSelector::LrTempInner,
    CacheSelector::RrTempInner,
    CacheSelector::RrTempCenter,
    CacheSelector::RrTempOuter,
    CacheSelector::LfShockDeflection,
    CacheSelector::RfShockDeflection,
    CacheSelector::LrShockDeflection,
    CacheSelector::RrShockDeflection,
];

/// Identifies one lap without borrowing its session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LapSelection {
    pub session_index: usize,
    pub lap_num: i32,
}

/// The pair of laps represented by a [`ComparisonCache`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComparisonSelection {
    pub primary: LapSelection,
    pub reference: LapSelection,
}

/// A clamped linear transform from a channel's raw units into plot coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearPlotScale {
    pub raw_min: f64,
    pub raw_max: f64,
    pub plot_min: f64,
    pub plot_max: f64,
}

impl LinearPlotScale {
    /// Creates a scale when both ranges are finite and ordered.
    pub fn new(raw_min: f64, raw_max: f64, plot_min: f64, plot_max: f64) -> Option<Self> {
        if !raw_min.is_finite()
            || !raw_max.is_finite()
            || !plot_min.is_finite()
            || !plot_max.is_finite()
            || raw_min > raw_max
            || plot_min > plot_max
        {
            return None;
        }

        Some(Self {
            raw_min,
            raw_max,
            plot_min,
            plot_max,
        })
    }

    /// Maps a raw value into the plot range, clamping out-of-range values.
    pub fn scale(self, raw_value: f64) -> f64 {
        let plot_midpoint = (self.plot_min + self.plot_max) * 0.5;
        let raw_span = self.raw_max - self.raw_min;
        if !raw_value.is_finite() || raw_span.abs() <= f64::EPSILON {
            return plot_midpoint;
        }

        let fraction = ((raw_value - self.raw_min) / raw_span).clamp(0.0, 1.0);
        self.plot_min + fraction * (self.plot_max - self.plot_min)
    }
}

/// A primary-session plot scale associated with its channel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChannelPlotScale {
    pub selector: CacheSelector,
    pub scale: LinearPlotScale,
}

/// Raw and primary-scaled reference values on the primary lap's absolute timeline.
#[derive(Clone, Debug)]
pub struct ComparisonChannel {
    pub selector: CacheSelector,
    pub scale: LinearPlotScale,
    pub raw_points: Vec<[f64; 2]>,
    pub scaled_points: Vec<[f64; 2]>,
}

impl ComparisonChannel {
    /// Interpolates the cached raw HUD value at an absolute primary-session time.
    pub fn raw_value_at(&self, primary_time: f64) -> Option<f64> {
        sample_points(&self.raw_points, primary_time)
    }
}

/// An allocation-owning comparison rebuilt only when either selected lap changes.
#[derive(Clone, Debug)]
pub struct ComparisonCache {
    pub selection: ComparisonSelection,
    pub primary_time_range: (f64, f64),
    pub primary_timeline: Vec<f64>,
    pub primary_distances: Vec<f64>,
    pub channels: Vec<ComparisonChannel>,
    pub custom_channels: std::collections::HashMap<String, Vec<[f64; 2]>>,
    pub sector_deltas: Vec<Option<f64>>,
}

impl ComparisonCache {
    /// Returns cached data for a configured channel.
    pub fn channel(&self, selector: CacheSelector) -> Option<&ComparisonChannel> {
        self.channels
            .iter()
            .find(|channel| channel.selector == selector)
    }
}

/// Errors produced while validating or building a comparison cache.
#[derive(Clone, Debug, PartialEq)]
pub enum ComparisonError {
    SessionNotFound(usize),
    LapRangeNotFound(LapSelection),
    LapDataNotFound(LapSelection),
    EmptyPrimaryTimeline(LapSelection),
    InvalidLapData {
        selection: LapSelection,
        reason: &'static str,
    },
    UnsupportedChannel(CacheSelector),
    EmptySourceSeries,
    MismatchedSeriesLengths,
    NonFiniteSeries,
    NonMonotonicSeries,
}

impl fmt::Display for ComparisonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SessionNotFound(index) => {
                write!(formatter, "session index {index} was not found")
            }
            Self::LapRangeNotFound(selection) => write!(
                formatter,
                "lap {} has no time range in session {}",
                selection.lap_num, selection.session_index
            ),
            Self::LapDataNotFound(selection) => write!(
                formatter,
                "lap {} has no LapData in session {}",
                selection.lap_num, selection.session_index
            ),
            Self::EmptyPrimaryTimeline(selection) => write!(
                formatter,
                "lap {} in session {} has no cached plot samples",
                selection.lap_num, selection.session_index
            ),
            Self::InvalidLapData { selection, reason } => write!(
                formatter,
                "lap {} in session {} has invalid LapData: {reason}",
                selection.lap_num, selection.session_index
            ),
            Self::UnsupportedChannel(selector) => {
                write!(
                    formatter,
                    "channel {selector:?} has no raw comparison source"
                )
            }
            Self::EmptySourceSeries => write!(formatter, "interpolation source is empty"),
            Self::MismatchedSeriesLengths => {
                write!(
                    formatter,
                    "interpolation positions and values differ in length"
                )
            }
            Self::NonFiniteSeries => write!(formatter, "interpolation positions must be finite"),
            Self::NonMonotonicSeries => {
                write!(formatter, "interpolation positions must be non-decreasing")
            }
        }
    }
}

impl Error for ComparisonError {}

/// Builds the primary-session scaling transforms used for reference overlays.
pub fn build_primary_channel_scales(
    primary: &LoadedSession,
    selectors: &[CacheSelector],
) -> Result<Vec<ChannelPlotScale>, ComparisonError> {
    selectors
        .iter()
        .copied()
        .map(|selector| {
            Ok(ChannelPlotScale {
                selector,
                scale: primary_channel_scale(primary, selector)?,
            })
        })
        .collect()
}

/// Builds a distance-aligned reference cache on the selected primary lap timeline.
pub fn build_comparison_cache(
    sessions: &[LoadedSession],
    selection: ComparisonSelection,
    selectors: &[CacheSelector],
    custom_channels_list: &[String],
) -> Result<ComparisonCache, ComparisonError> {
    let primary =
        sessions
            .get(selection.primary.session_index)
            .ok_or(ComparisonError::SessionNotFound(
                selection.primary.session_index,
            ))?;
    let reference =
        sessions
            .get(selection.reference.session_index)
            .ok_or(ComparisonError::SessionNotFound(
                selection.reference.session_index,
            ))?;

    let primary_range = find_lap_range(primary, selection.primary)?;
    let primary_lap = find_lap_data(primary, selection.primary)?;
    let reference_lap = find_lap_data(reference, selection.reference)?;
    validate_lap_data(primary_lap, selection.primary)?;
    validate_lap_data(reference_lap, selection.reference)?;

    let (cache_start, cache_end) =
        point_range(&primary.speed_pts_cache, primary_range.0, primary_range.1);
    if cache_start == cache_end {
        return Err(ComparisonError::EmptyPrimaryTimeline(selection.primary));
    }

    let primary_points = &primary.speed_pts_cache[cache_start..cache_end];
    let mut primary_timeline = Vec::with_capacity(primary_points.len());
    let mut primary_relative_times = Vec::with_capacity(primary_points.len());
    for point in primary_points {
        primary_timeline.push(point[0]);
        primary_relative_times.push(point[0] - primary_range.0);
    }
    let primary_distances = align_series(
        &primary_relative_times,
        &primary_lap.time,
        &primary_lap.dist,
    )?;

    let reference_rows: Vec<usize> = reference
        .session
        .laps
        .iter()
        .enumerate()
        .filter_map(|(index, lap_num)| (*lap_num == selection.reference.lap_num).then_some(index))
        .collect();
    if reference_rows.len() != reference_lap.dist.len() {
        return Err(ComparisonError::InvalidLapData {
            selection: selection.reference,
            reason: "raw row count does not match the distance series",
        });
    }

    let weights = interpolation_weights(&primary_distances, &reference_lap.dist)?;
    let scales = build_primary_channel_scales(primary, selectors)?;
    let mut channels = Vec::with_capacity(scales.len());

    for channel_scale in scales {
        let source = raw_channel(reference, channel_scale.selector)?;
        let reference_values: Vec<f64> = reference_rows
            .iter()
            .map(|row| source.value(*row))
            .collect();
        let aligned_values = apply_weights(&reference_values, &weights)?;
        let mut raw_points = Vec::with_capacity(primary_timeline.len());
        let mut scaled_points = Vec::with_capacity(primary_timeline.len());

        for (&time, &raw_value) in primary_timeline.iter().zip(&aligned_values) {
            raw_points.push([time, raw_value]);
            scaled_points.push([time, channel_scale.scale.scale(raw_value)]);
        }

        channels.push(ComparisonChannel {
            selector: channel_scale.selector,
            scale: channel_scale.scale,
            raw_points,
            scaled_points,
        });
    }

    let mut custom_channels = std::collections::HashMap::new();
    for custom_name in custom_channels_list {
        let source = dataframe_channel(reference, custom_name, 1.0);
        let reference_values: Vec<f64> = reference_rows
            .iter()
            .map(|row| source.value(*row))
            .collect();
        if let Ok(aligned_values) = apply_weights(&reference_values, &weights) {
            let mut raw_points = Vec::with_capacity(primary_timeline.len());
            for (&time, &raw_value) in primary_timeline.iter().zip(&aligned_values) {
                raw_points.push([time, raw_value]);
            }
            custom_channels.insert(custom_name.clone(), raw_points);
        }
    }

    let sector_deltas = calculate_sector_deltas(primary_lap, reference_lap, &primary.sectors);

    Ok(ComparisonCache {
        selection,
        primary_time_range: primary_range,
        primary_timeline,
        primary_distances,
        channels,
        custom_channels,
        sector_deltas,
    })
}

/// Linearly maps source values at source distances onto target distances.
pub fn align_series_by_distance(
    target_distances: &[f64],
    source_distances: &[f64],
    source_values: &[f64],
) -> Result<Vec<f64>, ComparisonError> {
    align_series(target_distances, source_distances, source_values)
}

/// Calculates active-minus-reference sector times from independent lap caches.
pub fn calculate_sector_deltas(
    active_lap: &LapData,
    reference_lap: &LapData,
    sectors: &[TrackSector],
) -> Vec<Option<f64>> {
    if !valid_series(&active_lap.dist, &active_lap.time)
        || !valid_series(&reference_lap.dist, &reference_lap.time)
    {
        return vec![None; sectors.len()];
    }

    sectors
        .iter()
        .map(|sector| {
            let active_start =
                sample_sorted_series(&active_lap.dist, &active_lap.time, sector.start_dist)?;
            let active_end =
                sample_sorted_series(&active_lap.dist, &active_lap.time, sector.end_dist)?;
            let reference_start =
                sample_sorted_series(&reference_lap.dist, &reference_lap.time, sector.start_dist)?;
            let reference_end =
                sample_sorted_series(&reference_lap.dist, &reference_lap.time, sector.end_dist)?;
            let active_time = active_end - active_start;
            let reference_time = reference_end - reference_start;

            (active_time.is_finite()
                && reference_time.is_finite()
                && active_time > 0.0
                && reference_time > 0.0)
                .then_some(active_time - reference_time)
        })
        .collect()
}

/// Looks up laps in separate session slices and calculates their sector deltas.
pub fn calculate_cross_session_sector_deltas(
    active_laps: &[LapData],
    active: LapSelection,
    reference_laps: &[LapData],
    reference: LapSelection,
    sectors: &[TrackSector],
) -> Result<Vec<Option<f64>>, ComparisonError> {
    let active_lap = active_laps
        .iter()
        .find(|lap| lap.lap_num == active.lap_num)
        .ok_or(ComparisonError::LapDataNotFound(active))?;
    let reference_lap = reference_laps
        .iter()
        .find(|lap| lap.lap_num == reference.lap_num)
        .ok_or(ComparisonError::LapDataNotFound(reference))?;

    validate_lap_data(active_lap, active)?;
    validate_lap_data(reference_lap, reference)?;
    Ok(calculate_sector_deltas(active_lap, reference_lap, sectors))
}

fn primary_channel_scale(
    primary: &LoadedSession,
    selector: CacheSelector,
) -> Result<LinearPlotScale, ComparisonError> {
    if let Some((group, plot_range)) = worksheet_channel_group(selector) {
        return dynamic_channel_group_scale(primary, group, 0.05, plot_range.0, plot_range.1);
    }
    let scale = match selector {
        CacheSelector::Throttle | CacheSelector::Brake | CacheSelector::Clutch => {
            linear_scale(0.0, 100.0, 28.0, 48.0)
        }
        CacheSelector::Gear => linear_scale(-1.0, 8.0, 52.0, 70.0),
        CacheSelector::FrontHeight => {
            let range = finite_range(primary.cache_to_df_index.iter().flat_map(|index| {
                [
                    primary.session.front_raw.get(*index).copied(),
                    primary.session.front_smooth.get(*index).copied(),
                ]
                .into_iter()
                .flatten()
            }));
            padded_scale(range, 0.02, 45.5, 74.5)
        }
        CacheSelector::RearHeight => {
            let range = finite_range(primary.cache_to_df_index.iter().flat_map(|index| {
                [
                    primary.session.rear_raw.get(*index).copied(),
                    primary.session.rear_smooth.get(*index).copied(),
                ]
                .into_iter()
                .flatten()
            }));
            padded_scale(range, 0.02, 45.5, 74.5)
        }
        CacheSelector::LatG | CacheSelector::LongG => {
            let lateral = raw_channel(primary, CacheSelector::LatG)?;
            let longitudinal = raw_channel(primary, CacheSelector::LongG)?;
            let range = finite_range(
                primary
                    .cache_to_df_index
                    .iter()
                    .flat_map(|index| [lateral.value(*index), longitudinal.value(*index)]),
            );
            padded_scale(range, 0.1, 10.0, 40.0)
        }
        CacheSelector::Speed => dynamic_channel_scale(primary, selector, 0.1, 76.0, 98.0)?,
        CacheSelector::RPM => dynamic_channel_scale(primary, selector, 0.1, 52.0, 72.0)?,
        CacheSelector::Steering => dynamic_channel_scale(primary, selector, 0.1, 10.0, 24.0)?,
        CacheSelector::Rake => dynamic_channel_scale(primary, selector, 0.1, 45.5, 74.5)?,
        CacheSelector::DistanceDelta | CacheSelector::TimeDelta => {
            return Err(ComparisonError::UnsupportedChannel(selector));
        }
        _ => return Err(ComparisonError::UnsupportedChannel(selector)),
    };

    Ok(scale)
}

fn dynamic_channel_scale(
    primary: &LoadedSession,
    selector: CacheSelector,
    padding: f64,
    plot_min: f64,
    plot_max: f64,
) -> Result<LinearPlotScale, ComparisonError> {
    let source = raw_channel(primary, selector)?;
    let range = finite_range(
        primary
            .cache_to_df_index
            .iter()
            .map(|index| source.value(*index)),
    );
    Ok(padded_scale(range, padding, plot_min, plot_max))
}

fn dynamic_channel_group_scale(
    primary: &LoadedSession,
    selectors: &[CacheSelector],
    padding: f64,
    plot_min: f64,
    plot_max: f64,
) -> Result<LinearPlotScale, ComparisonError> {
    let sources: Vec<RawChannel<'_>> = selectors
        .iter()
        .copied()
        .map(|selector| raw_channel(primary, selector))
        .collect::<Result<_, _>>()?;
    let range = finite_range(
        primary
            .cache_to_df_index
            .iter()
            .flat_map(|index| sources.iter().map(|source| source.value(*index))),
    );
    Ok(padded_scale(range, padding, plot_min, plot_max))
}

fn padded_scale(
    range: Option<(f64, f64)>,
    padding: f64,
    plot_min: f64,
    plot_max: f64,
) -> LinearPlotScale {
    let (raw_min, raw_max) = range.unwrap_or((0.0, 0.0));
    let pad = (raw_max - raw_min) * padding;
    linear_scale(raw_min - pad, raw_max + pad, plot_min, plot_max)
}

fn linear_scale(raw_min: f64, raw_max: f64, plot_min: f64, plot_max: f64) -> LinearPlotScale {
    LinearPlotScale {
        raw_min,
        raw_max,
        plot_min,
        plot_max,
    }
}

fn finite_range(values: impl IntoIterator<Item = f64>) -> Option<(f64, f64)> {
    let mut range: Option<(f64, f64)> = None;
    for value in values.into_iter().filter(|value| value.is_finite()) {
        range = Some(match range {
            Some((minimum, maximum)) => (minimum.min(value), maximum.max(value)),
            None => (value, value),
        });
    }
    range
}

#[derive(Clone, Copy)]
enum RawChannel<'a> {
    Column(&'a Float64Chunked, f64),
    Values(&'a [f64]),
    Zero,
}

impl RawChannel<'_> {
    fn value(self, row: usize) -> f64 {
        let value = match self {
            Self::Column(column, multiplier) => column.get(row).unwrap_or(0.0) * multiplier,
            Self::Values(values) => values.get(row).copied().unwrap_or(0.0),
            Self::Zero => 0.0,
        };
        if value.is_finite() {
            value
        } else {
            0.0
        }
    }
}

fn raw_channel(
    loaded: &LoadedSession,
    selector: CacheSelector,
) -> Result<RawChannel<'_>, ComparisonError> {
    if let Some((name, multiplier)) = selector.telemetry_source() {
        return Ok(dataframe_channel(loaded, name, multiplier));
    }
    let source = match selector {
        CacheSelector::Speed => dataframe_channel(loaded, "Speed", 3.6),
        CacheSelector::RPM => dataframe_channel(loaded, "RPM", 1.0),
        CacheSelector::Throttle => dataframe_channel(loaded, "Throttle", 100.0),
        CacheSelector::Brake => dataframe_channel(loaded, "Brake", 100.0),
        CacheSelector::Steering => dataframe_channel(loaded, "SteeringWheelAngle", 57.2958),
        CacheSelector::FrontHeight => RawChannel::Values(&loaded.session.front_raw),
        CacheSelector::RearHeight => RawChannel::Values(&loaded.session.rear_raw),
        CacheSelector::Rake => RawChannel::Values(&loaded.session.rake),
        CacheSelector::LatG => dataframe_channel(loaded, "LatAccel", 1.0 / 9.80665),
        CacheSelector::LongG => dataframe_channel(loaded, "LongAccel", 1.0 / 9.80665),
        CacheSelector::Gear => dataframe_channel(loaded, "Gear", 1.0),
        CacheSelector::Clutch => dataframe_channel(loaded, "ClutchRaw", 100.0),
        CacheSelector::DistanceDelta | CacheSelector::TimeDelta => {
            return Err(ComparisonError::UnsupportedChannel(selector));
        }
        _ => return Err(ComparisonError::UnsupportedChannel(selector)),
    };
    Ok(source)
}

fn dataframe_channel<'a>(loaded: &'a LoadedSession, name: &str, multiplier: f64) -> RawChannel<'a> {
    loaded
        .session
        .dataframe
        .column(name)
        .ok()
        .and_then(|column| column.f64().ok())
        .map_or(RawChannel::Zero, |column| {
            RawChannel::Column(column, multiplier)
        })
}

fn find_lap_range(
    loaded: &LoadedSession,
    selection: LapSelection,
) -> Result<(f64, f64), ComparisonError> {
    loaded
        .lap_ranges
        .iter()
        .find(|range| range.0 == selection.lap_num)
        .map(|range| (range.1, range.2))
        .ok_or(ComparisonError::LapRangeNotFound(selection))
}

fn find_lap_data(
    loaded: &LoadedSession,
    selection: LapSelection,
) -> Result<&LapData, ComparisonError> {
    loaded
        .lap_data_cache
        .iter()
        .find(|lap| lap.lap_num == selection.lap_num)
        .ok_or(ComparisonError::LapDataNotFound(selection))
}

fn validate_lap_data(lap: &LapData, selection: LapSelection) -> Result<(), ComparisonError> {
    if lap.dist.is_empty() || lap.time.is_empty() {
        return Err(ComparisonError::InvalidLapData {
            selection,
            reason: "distance and time series must not be empty",
        });
    }
    if lap.dist.len() != lap.time.len() {
        return Err(ComparisonError::InvalidLapData {
            selection,
            reason: "distance and time series differ in length",
        });
    }
    validate_positions(&lap.dist).map_err(|error| ComparisonError::InvalidLapData {
        selection,
        reason: match error {
            ComparisonError::NonFiniteSeries => "distance series contains a non-finite value",
            _ => "distance series is not non-decreasing",
        },
    })?;
    validate_positions(&lap.time).map_err(|error| ComparisonError::InvalidLapData {
        selection,
        reason: match error {
            ComparisonError::NonFiniteSeries => "time series contains a non-finite value",
            _ => "time series is not non-decreasing",
        },
    })
}

fn point_range(points: &[[f64; 2]], start: f64, end: f64) -> (usize, usize) {
    let start_index = points.partition_point(|point| point[0] < start);
    let end_index = points.partition_point(|point| point[0] <= end);
    (start_index, end_index)
}

#[derive(Clone, Copy, Debug)]
struct InterpolationWeight {
    lower: usize,
    upper: usize,
    fraction: f64,
}

fn align_series(
    targets: &[f64],
    source_positions: &[f64],
    source_values: &[f64],
) -> Result<Vec<f64>, ComparisonError> {
    if source_positions.len() != source_values.len() {
        return Err(ComparisonError::MismatchedSeriesLengths);
    }
    let weights = interpolation_weights(targets, source_positions)?;
    apply_weights(source_values, &weights)
}

fn interpolation_weights(
    targets: &[f64],
    source_positions: &[f64],
) -> Result<Vec<InterpolationWeight>, ComparisonError> {
    validate_positions(source_positions)?;
    if targets.iter().any(|target| !target.is_finite()) {
        return Err(ComparisonError::NonFiniteSeries);
    }

    let last_index = source_positions.len() - 1;
    let mut weights = Vec::with_capacity(targets.len());
    for &target in targets {
        if target <= source_positions[0] {
            weights.push(InterpolationWeight {
                lower: 0,
                upper: 0,
                fraction: 0.0,
            });
            continue;
        }
        if target >= source_positions[last_index] {
            weights.push(InterpolationWeight {
                lower: last_index,
                upper: last_index,
                fraction: 0.0,
            });
            continue;
        }

        let upper = source_positions.partition_point(|position| *position < target);
        if source_positions[upper] == target {
            weights.push(InterpolationWeight {
                lower: upper,
                upper,
                fraction: 0.0,
            });
            continue;
        }

        let lower = upper - 1;
        let span = source_positions[upper] - source_positions[lower];
        let fraction = if span.abs() <= f64::EPSILON {
            1.0
        } else {
            (target - source_positions[lower]) / span
        };
        weights.push(InterpolationWeight {
            lower,
            upper,
            fraction,
        });
    }
    Ok(weights)
}

fn apply_weights(
    source_values: &[f64],
    weights: &[InterpolationWeight],
) -> Result<Vec<f64>, ComparisonError> {
    if weights
        .iter()
        .any(|weight| weight.upper >= source_values.len())
    {
        return Err(ComparisonError::MismatchedSeriesLengths);
    }

    Ok(weights
        .iter()
        .map(|weight| {
            let lower = source_values[weight.lower];
            lower + (source_values[weight.upper] - lower) * weight.fraction
        })
        .collect())
}

fn validate_positions(positions: &[f64]) -> Result<(), ComparisonError> {
    if positions.is_empty() {
        return Err(ComparisonError::EmptySourceSeries);
    }
    if positions.iter().any(|position| !position.is_finite()) {
        return Err(ComparisonError::NonFiniteSeries);
    }
    if positions.windows(2).any(|pair| pair[0] > pair[1]) {
        return Err(ComparisonError::NonMonotonicSeries);
    }
    Ok(())
}

fn valid_series(positions: &[f64], values: &[f64]) -> bool {
    positions.len() == values.len()
        && validate_positions(positions).is_ok()
        && values.iter().all(|value| value.is_finite())
}

fn sample_sorted_series(positions: &[f64], values: &[f64], target: f64) -> Option<f64> {
    if !target.is_finite() {
        return None;
    }
    if target <= positions[0] {
        return Some(values[0]);
    }
    let last = positions.len() - 1;
    if target >= positions[last] {
        return Some(values[last]);
    }

    let upper = positions.partition_point(|position| *position < target);
    if positions[upper] == target {
        return Some(values[upper]);
    }
    let lower = upper - 1;
    let span = positions[upper] - positions[lower];
    if span.abs() <= f64::EPSILON {
        return Some(values[upper]);
    }

    let fraction = (target - positions[lower]) / span;
    Some(values[lower] + (values[upper] - values[lower]) * fraction)
}

pub fn sample_points(points: &[[f64; 2]], target: f64) -> Option<f64> {
    if points.is_empty() || target < points[0][0] || target > points[points.len() - 1][0] {
        return None;
    }
    if target <= points[0][0] {
        return Some(points[0][1]);
    }
    let last = points.len() - 1;
    if target >= points[last][0] {
        return Some(points[last][1]);
    }

    let upper = points.partition_point(|point| point[0] < target);
    if points[upper][0] == target {
        return Some(points[upper][1]);
    }
    let lower = upper - 1;
    let span = points[upper][0] - points[lower][0];
    if span.abs() <= f64::EPSILON {
        return Some(points[upper][1]);
    }
    let fraction = (target - points[lower][0]) / span;
    Some(points[lower][1] + (points[upper][1] - points[lower][1]) * fraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lap(lap_num: i32, dist: &[f64], time: &[f64]) -> LapData {
        LapData {
            lap_num,
            dist: dist.to_vec(),
            time: time.to_vec(),
            x: vec![0.0; dist.len()],
            y: vec![0.0; dist.len()],
        }
    }

    #[test]
    fn linear_plot_scale_maps_and_clamps_raw_values() {
        let scale = LinearPlotScale::new(0.0, 100.0, 20.0, 40.0).unwrap();
        let result = [scale.scale(-10.0), scale.scale(50.0), scale.scale(120.0)];

        assert_eq!(result, [20.0, 30.0, 40.0]);
    }

    #[test]
    fn align_series_by_distance_interpolates_between_reference_samples() {
        let result =
            align_series_by_distance(&[50.0, 150.0], &[0.0, 100.0, 200.0], &[0.0, 10.0, 30.0])
                .unwrap();

        assert_eq!(result, vec![5.0, 20.0]);
    }

    #[test]
    fn align_series_by_distance_rejects_mismatched_source_lengths() {
        let error = align_series_by_distance(&[50.0], &[0.0, 100.0], &[1.0]).unwrap_err();

        assert_eq!(error, ComparisonError::MismatchedSeriesLengths);
    }

    #[test]
    fn calculate_sector_deltas_uses_laps_from_independent_caches() {
        let active = lap(4, &[0.0, 100.0, 200.0], &[0.0, 11.0, 23.0]);
        let reference = lap(7, &[0.0, 100.0, 200.0], &[0.0, 10.0, 20.0]);
        let sectors = [TrackSector {
            name: "S1".to_owned(),
            start_dist: 0.0,
            end_dist: 100.0,
        }];

        let result = calculate_sector_deltas(&active, &reference, &sectors);

        assert_eq!(result, vec![Some(1.0)]);
    }

    #[test]
    fn calculate_cross_session_sector_deltas_reports_missing_reference_lap() {
        let active = [lap(4, &[0.0, 100.0], &[0.0, 10.0])];
        let reference = [lap(6, &[0.0, 100.0], &[0.0, 9.0])];
        let reference_selection = LapSelection {
            session_index: 1,
            lap_num: 7,
        };

        let error = calculate_cross_session_sector_deltas(
            &active,
            LapSelection {
                session_index: 0,
                lap_num: 4,
            },
            &reference,
            reference_selection,
            &[],
        )
        .unwrap_err();

        assert_eq!(error, ComparisonError::LapDataNotFound(reference_selection));
    }
}
