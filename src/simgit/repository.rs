use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: u32 = 1;
const COMPRESSION_LEVEL: i32 = 3;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TelemetryRecord {
    pub id: String,
    pub original_name: String,
    pub object_name: String,
    pub imported_at: i64,
    pub uncompressed_size: u64,
    pub compressed_size: u64,
    pub car: String,
    pub venue: String,
    pub track_id: i32,
    #[serde(default)]
    pub laps: Vec<LapSummary>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct LapSummary {
    pub lap_number: i32,
    pub duration_seconds: f64,
}

impl TelemetryRecord {
    pub fn fastest_lap(&self) -> Option<LapSummary> {
        self.laps
            .iter()
            .copied()
            .filter(|lap| lap.lap_number > 0 && lap.duration_seconds.is_finite())
            .min_by(|left, right| left.duration_seconds.total_cmp(&right.duration_seconds))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RepositoryRecordRef {
    pub project: String,
    pub telemetry_id: String,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NoteColor {
    Red,
    Yellow,
    Orange,
    #[default]
    Blue,
    Green,
    Purple,
}

impl NoteColor {
    pub const ALL: [Self; 6] = [
        Self::Red,
        Self::Yellow,
        Self::Orange,
        Self::Blue,
        Self::Green,
        Self::Purple,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Red => "Red",
            Self::Yellow => "Yellow",
            Self::Orange => "Orange",
            Self::Blue => "Blue",
            Self::Green => "Green",
            Self::Purple => "Purple",
        }
    }

    pub const fn display_color(self, is_dark: bool) -> egui::Color32 {
        match (self, is_dark) {
            (Self::Red, true) => egui::Color32::from_rgb(255, 92, 92),
            (Self::Yellow, true) => egui::Color32::from_rgb(255, 211, 74),
            (Self::Orange, true) => egui::Color32::from_rgb(255, 142, 66),
            (Self::Blue, true) => egui::Color32::from_rgb(78, 170, 255),
            (Self::Green, true) => egui::Color32::from_rgb(70, 210, 132),
            (Self::Purple, true) => egui::Color32::from_rgb(180, 123, 255),
            (Self::Red, false) => egui::Color32::from_rgb(174, 38, 38),
            (Self::Yellow, false) => egui::Color32::from_rgb(137, 99, 0),
            (Self::Orange, false) => egui::Color32::from_rgb(176, 72, 15),
            (Self::Blue, false) => egui::Color32::from_rgb(20, 88, 166),
            (Self::Green, false) => egui::Color32::from_rgb(20, 112, 62),
            (Self::Purple, false) => egui::Color32::from_rgb(92, 55, 168),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReferenceLapContext {
    pub file_name: String,
    pub repository_record: Option<RepositoryRecordRef>,
    pub lap_number: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrackMapContext {
    pub visible: bool,
    pub panel_width: f32,
    pub rotation: f64,
    pub bounds: Option<[[f64; 2]; 2]>,
}

impl TrackMapContext {
    pub fn valid_bounds(&self) -> Option<[[f64; 2]; 2]> {
        let bounds = self.bounds?;
        bounds
            .iter()
            .flatten()
            .all(|value| value.is_finite())
            .then_some(bounds)
            .filter(|bounds| bounds[1][0] > bounds[0][0] && bounds[1][1] > bounds[0][1])
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnalysisContext {
    pub cursor_seconds: Option<f64>,
    pub viewport: Option<(f64, f64)>,
    pub lap_number: Option<i32>,
    pub worksheet: String,
    #[serde(default)]
    pub cyan_reference: Option<ReferenceLapContext>,
    #[serde(default)]
    pub secondary_reference: Option<ReferenceLapContext>,
    #[serde(default)]
    pub track_map: Option<TrackMapContext>,
    #[serde(default)]
    pub section_delta: Option<f64>,
}

impl AnalysisContext {
    pub fn time_range(&self) -> Option<(f64, f64)> {
        if let Some((start, end)) = self.viewport {
            if start.is_finite() && end.is_finite() && end > start {
                return Some((start, end));
            }
        }
        let cursor = self.cursor_seconds?;
        cursor
            .is_finite()
            .then_some((cursor - 0.25, cursor + 0.25))
    }
}

pub fn calculate_section_delta_from_cache(
    time_delta_pts: &[[f64; 2]],
    start_t: f64,
    end_t: f64,
) -> Option<f64> {
    if time_delta_pts.is_empty() || end_t <= start_t {
        return None;
    }
    let start_idx = time_delta_pts
        .binary_search_by(|p| p[0].partial_cmp(&start_t).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or_else(|idx| idx.min(time_delta_pts.len().saturating_sub(1)));
    let end_idx = time_delta_pts
        .binary_search_by(|p| p[0].partial_cmp(&end_t).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or_else(|idx| idx.min(time_delta_pts.len().saturating_sub(1)));

    if start_idx >= time_delta_pts.len() || end_idx >= time_delta_pts.len() {
        return None;
    }

    let delta_start = time_delta_pts[start_idx][1];
    let delta_end = time_delta_pts[end_idx][1];
    let section_delta = delta_end - delta_start;

    if section_delta.is_finite() {
        Some(section_delta)
    } else {
        None
    }
}

pub fn format_section_delta(delta: f64) -> String {
    if delta > 0.0 {
        format!("+{:.3}s", delta)
    } else {
        format!("{:.3}s", delta)
    }
}

pub fn section_delta_color(delta: f64, is_dark: bool) -> egui::Color32 {
    if delta > 0.0001 {
        // Red for time lost (+ delta)
        if is_dark {
            egui::Color32::from_rgb(255, 92, 92)
        } else {
            egui::Color32::from_rgb(210, 40, 40)
        }
    } else if delta < -0.0001 {
        // Green for time gained (- delta)
        if is_dark {
            egui::Color32::from_rgb(70, 210, 132)
        } else {
            egui::Color32::from_rgb(20, 140, 60)
        }
    } else {
        if is_dark {
            egui::Color32::from_rgb(200, 200, 200)
        } else {
            egui::Color32::from_rgb(80, 80, 80)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnalysisNote {
    pub id: String,
    pub telemetry_id: String,
    pub author: String,
    #[serde(default)]
    pub objective: String,
    pub body: String,
    #[serde(default)]
    pub color: NoteColor,
    pub context: AnalysisContext,
    pub created_at: i64,
    pub updated_at: i64,
}

impl AnalysisNote {
    pub fn display_objective(&self) -> &str {
        let objective = self.objective.trim();
        if objective.is_empty() {
            self.body
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("Analysis note")
        } else {
            objective
        }
    }

    pub fn resolved_section_delta(&self, time_delta_pts: &[[f64; 2]]) -> Option<f64> {
        if let Some(delta) = self.context.section_delta {
            return Some(delta);
        }
        let (start_t, end_t) = self.context.time_range()?;
        calculate_section_delta_from_cache(time_delta_pts, start_t, end_t)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RepositoryManifest {
    schema_version: u32,
    name: String,
    created_at: i64,
    telemetry: Vec<TelemetryRecord>,
    notes: Vec<AnalysisNote>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportStatus {
    Imported,
    AlreadyPresent,
}

#[derive(Clone, Debug)]
pub struct ImportResult {
    pub status: ImportStatus,
    pub record: TelemetryRecord,
}

#[derive(Clone, Debug, Default)]
pub struct ImportBatchSummary {
    pub imported: usize,
    pub already_present: usize,
    pub failures: Vec<String>,
}

#[derive(Debug)]
pub enum RepositoryError {
    InvalidName,
    InvalidTelemetryFile(PathBuf),
    RecordNotFound(String),
    NoteNotFound(String),
    UnsupportedSchema(u32),
    Io(std::io::Error),
    Json(serde_json::Error),
    Telemetry(String),
    Integrity(String),
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName => write!(
                formatter,
                "repository names may contain letters, numbers, spaces, '-' and '_'"
            ),
            Self::InvalidTelemetryFile(path) => {
                write!(formatter, "{} is not an .ibt file", path.display())
            }
            Self::RecordNotFound(id) => write!(formatter, "telemetry record {id} was not found"),
            Self::NoteNotFound(id) => write!(formatter, "analysis note {id} was not found"),
            Self::UnsupportedSchema(version) => {
                write!(formatter, "repository schema {version} is not supported")
            }
            Self::Io(error) => write!(formatter, "repository I/O failed: {error}"),
            Self::Json(error) => write!(formatter, "repository metadata is invalid: {error}"),
            Self::Telemetry(error) => write!(formatter, "telemetry could not be parsed: {error}"),
            Self::Integrity(error) => write!(
                formatter,
                "repository object failed integrity validation: {error}"
            ),
        }
    }
}

impl std::error::Error for RepositoryError {}

impl From<std::io::Error> for RepositoryError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for RepositoryError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Clone, Debug)]
pub struct SimGitRepository {
    project_dir: PathBuf,
    manifest: RepositoryManifest,
}

impl SimGitRepository {
    pub fn create(root: &Path, name: &str) -> Result<Self, RepositoryError> {
        validate_repository_name(name)?;
        let project_dir = root.join(name.trim());
        fs::create_dir_all(project_dir.join("objects"))?;
        fs::create_dir_all(project_dir.join("cache"))?;
        let manifest_path = project_dir.join("repository.json");
        if manifest_path.exists() {
            return Self::open(root, name);
        }

        let repository = Self {
            project_dir,
            manifest: RepositoryManifest {
                schema_version: SCHEMA_VERSION,
                name: name.trim().to_owned(),
                created_at: unix_timestamp(),
                telemetry: Vec::new(),
                notes: Vec::new(),
            },
        };
        repository.save()?;
        Ok(repository)
    }

    pub fn open(root: &Path, name: &str) -> Result<Self, RepositoryError> {
        validate_repository_name(name)?;
        let project_dir = root.join(name.trim());
        let manifest_path = project_dir.join("repository.json");
        if !manifest_path.exists() {
            return Self::create(root, name);
        }
        let manifest: RepositoryManifest =
            serde_json::from_reader(BufReader::new(File::open(manifest_path)?))?;
        if manifest.schema_version != SCHEMA_VERSION {
            return Err(RepositoryError::UnsupportedSchema(manifest.schema_version));
        }
        fs::create_dir_all(project_dir.join("objects"))?;
        fs::create_dir_all(project_dir.join("cache"))?;
        Ok(Self {
            project_dir,
            manifest,
        })
    }

    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    pub fn telemetry(&self) -> &[TelemetryRecord] {
        &self.manifest.telemetry
    }

    pub fn notes(&self) -> &[AnalysisNote] {
        &self.manifest.notes
    }

    pub fn notes_for(&self, telemetry_id: &str) -> Vec<&AnalysisNote> {
        let mut notes: Vec<_> = self
            .manifest
            .notes
            .iter()
            .filter(|note| note.telemetry_id == telemetry_id)
            .collect();
        notes.sort_by_key(|note| std::cmp::Reverse(note.created_at));
        notes
    }

    pub fn import_ibt(&mut self, source: &Path) -> Result<ImportResult, RepositoryError> {
        if !source
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("ibt"))
        {
            return Err(RepositoryError::InvalidTelemetryFile(source.to_path_buf()));
        }

        let (id, uncompressed_size) = hash_file(source)?;
        if let Some(record) = self
            .manifest
            .telemetry
            .iter()
            .find(|record| record.id == id)
        {
            return Ok(ImportResult {
                status: ImportStatus::AlreadyPresent,
                record: record.clone(),
            });
        }

        let parsed = crate::data::ibt_parser::parse_ibt_file(source)
            .map_err(|error| RepositoryError::Telemetry(error.to_string()))?;
        let object_name = format!("{id}.ibt.zst");
        let object_path = self.project_dir.join("objects").join(&object_name);
        compress_file(source, &object_path)?;
        let compressed_size = fs::metadata(&object_path)?.len();
        let record = TelemetryRecord {
            id,
            original_name: source
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            object_name,
            imported_at: unix_timestamp(),
            uncompressed_size,
            compressed_size,
            car: parsed.car,
            venue: parsed.venue,
            track_id: parsed.track_id,
            laps: parsed
                .lap_times
                .iter()
                .map(|(lap_number, duration_seconds)| LapSummary {
                    lap_number: *lap_number,
                    duration_seconds: *duration_seconds,
                })
                .collect(),
        };
        self.manifest.telemetry.push(record.clone());
        if let Err(error) = self.save() {
            let _ = fs::remove_file(object_path);
            return Err(error);
        }
        Ok(ImportResult {
            status: ImportStatus::Imported,
            record,
        })
    }

    pub fn resolve_ibt(&self, telemetry_id: &str) -> Result<PathBuf, RepositoryError> {
        let record = self
            .manifest
            .telemetry
            .iter()
            .find(|record| record.id == telemetry_id)
            .ok_or_else(|| RepositoryError::RecordNotFound(telemetry_id.to_owned()))?;
        let cache_path = self
            .project_dir
            .join("cache")
            .join(format!("{}.ibt", record.id));
        if cache_path.exists() {
            let (hash, size) = hash_file(&cache_path)?;
            if hash == record.id && size == record.uncompressed_size {
                return Ok(cache_path);
            }
            fs::remove_file(&cache_path)?;
        }

        let object_path = self.project_dir.join("objects").join(&record.object_name);
        decompress_file(&object_path, &cache_path)?;
        let (hash, size) = hash_file(&cache_path)?;
        if hash != record.id || size != record.uncompressed_size {
            let _ = fs::remove_file(&cache_path);
            return Err(RepositoryError::Integrity(record.id.clone()));
        }
        Ok(cache_path)
    }

    pub fn ensure_lap_summaries(
        &mut self,
        telemetry_id: &str,
    ) -> Result<Vec<LapSummary>, RepositoryError> {
        let position = self
            .manifest
            .telemetry
            .iter()
            .position(|record| record.id == telemetry_id)
            .ok_or_else(|| RepositoryError::RecordNotFound(telemetry_id.to_owned()))?;
        if !self.manifest.telemetry[position].laps.is_empty() {
            return Ok(self.manifest.telemetry[position].laps.clone());
        }
        let path = self.resolve_ibt(telemetry_id)?;
        let parsed = crate::data::ibt_parser::parse_ibt_file(path)
            .map_err(|error| RepositoryError::Telemetry(error.to_string()))?;
        let laps: Vec<_> = parsed
            .lap_times
            .iter()
            .map(|(lap_number, duration_seconds)| LapSummary {
                lap_number: *lap_number,
                duration_seconds: *duration_seconds,
            })
            .collect();
        self.manifest.telemetry[position].laps = laps.clone();
        self.save()?;
        Ok(laps)
    }

    pub fn compressed_object_path(&self, telemetry_id: &str) -> Result<PathBuf, RepositoryError> {
        let record = self
            .manifest
            .telemetry
            .iter()
            .find(|record| record.id == telemetry_id)
            .ok_or_else(|| RepositoryError::RecordNotFound(telemetry_id.to_owned()))?;
        Ok(self.project_dir.join("objects").join(&record.object_name))
    }

    pub fn install_compressed_record(
        &mut self,
        record: TelemetryRecord,
        compressed_source: &Path,
    ) -> Result<(), RepositoryError> {
        if self
            .manifest
            .telemetry
            .iter()
            .any(|item| item.id == record.id)
        {
            return Ok(());
        }
        let object_path = self.project_dir.join("objects").join(&record.object_name);
        copy_file_atomically(compressed_source, &object_path)?;
        let cache_path = self
            .project_dir
            .join("cache")
            .join(format!("{}.ibt", record.id));
        decompress_file(&object_path, &cache_path)?;
        let (hash, size) = hash_file(&cache_path)?;
        if hash != record.id || size != record.uncompressed_size {
            let _ = fs::remove_file(object_path);
            let _ = fs::remove_file(cache_path);
            return Err(RepositoryError::Integrity(record.id));
        }
        self.manifest.telemetry.push(record);
        self.save()
    }

    pub fn remove_telemetry(&mut self, telemetry_id: &str) -> Result<(), RepositoryError> {
        let position = self
            .manifest
            .telemetry
            .iter()
            .position(|record| record.id == telemetry_id)
            .ok_or_else(|| RepositoryError::RecordNotFound(telemetry_id.to_owned()))?;
        let record = self.manifest.telemetry.remove(position);
        self.manifest
            .notes
            .retain(|note| note.telemetry_id != telemetry_id);
        self.save()?;
        let _ = fs::remove_file(self.project_dir.join("objects").join(record.object_name));
        let _ = fs::remove_file(
            self.project_dir
                .join("cache")
                .join(format!("{}.ibt", record.id)),
        );
        Ok(())
    }

    pub fn add_note(
        &mut self,
        telemetry_id: &str,
        author: &str,
        objective: &str,
        body: &str,
        color: NoteColor,
        context: AnalysisContext,
    ) -> Result<AnalysisNote, RepositoryError> {
        if !self
            .manifest
            .telemetry
            .iter()
            .any(|record| record.id == telemetry_id)
        {
            return Err(RepositoryError::RecordNotFound(telemetry_id.to_owned()));
        }
        let body = body.trim();
        if body.is_empty() {
            return Err(RepositoryError::Integrity(
                "analysis notes cannot be empty".to_owned(),
            ));
        }
        let objective = objective.trim();
        if objective.is_empty() {
            return Err(RepositoryError::Integrity(
                "analysis notes require an objective".to_owned(),
            ));
        }
        let timestamp = unix_timestamp();
        let id = blake3::hash(
            format!(
                "{telemetry_id}:{timestamp}:{}:{objective}:{body}",
                self.manifest.notes.len()
            )
            .as_bytes(),
        )
        .to_hex()
        .to_string();
        let author = author.trim();
        let note = AnalysisNote {
            id,
            telemetry_id: telemetry_id.to_owned(),
            author: if author.is_empty() { "Driver" } else { author }.to_owned(),
            objective: objective.to_owned(),
            body: body.to_owned(),
            color,
            context,
            created_at: timestamp,
            updated_at: timestamp,
        };
        self.manifest.notes.push(note.clone());
        self.save()?;
        Ok(note)
    }

    pub fn remove_note(&mut self, note_id: &str) -> Result<(), RepositoryError> {
        let previous_len = self.manifest.notes.len();
        self.manifest.notes.retain(|note| note.id != note_id);
        if self.manifest.notes.len() == previous_len {
            return Err(RepositoryError::NoteNotFound(note_id.to_owned()));
        }
        self.save()
    }

    fn save(&self) -> Result<(), RepositoryError> {
        let bytes = serde_json::to_vec_pretty(&self.manifest)?;
        write_atomically(&self.project_dir.join("repository.json"), &bytes)
    }
}

fn validate_repository_name(name: &str) -> Result<(), RepositoryError> {
    let name = name.trim();
    if name.is_empty()
        || name == "."
        || name == ".."
        || !name
            .chars()
            .all(|character| character.is_alphanumeric() || matches!(character, ' ' | '-' | '_'))
    {
        return Err(RepositoryError::InvalidName);
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<(String, u64), RepositoryError> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut hasher = blake3::Hasher::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        size += read as u64;
    }
    Ok((hasher.finalize().to_hex().to_string(), size))
}

fn compress_file(source: &Path, destination: &Path) -> Result<(), RepositoryError> {
    let temporary = destination.with_extension("zst.importing");
    let mut reader = BufReader::new(File::open(source)?);
    let writer = BufWriter::new(File::create(&temporary)?);
    let mut encoder = zstd::stream::Encoder::new(writer, COMPRESSION_LEVEL)?;
    std::io::copy(&mut reader, &mut encoder)?;
    encoder.finish()?.flush()?;
    replace_file(&temporary, destination)
}

fn decompress_file(source: &Path, destination: &Path) -> Result<(), RepositoryError> {
    let temporary = destination.with_extension("ibt.importing");
    let reader = BufReader::new(File::open(source)?);
    let mut decoder = zstd::stream::Decoder::new(reader)?;
    let mut writer = BufWriter::new(File::create(&temporary)?);
    std::io::copy(&mut decoder, &mut writer)?;
    writer.flush()?;
    replace_file(&temporary, destination)
}

fn copy_file_atomically(source: &Path, destination: &Path) -> Result<(), RepositoryError> {
    let temporary = destination.with_extension("zst.importing");
    fs::copy(source, &temporary)?;
    replace_file(&temporary, destination)
}

fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), RepositoryError> {
    let temporary = path.with_extension("json.next");
    let mut file = File::create(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    replace_file(&temporary, path)
}

fn replace_file(source: &Path, destination: &Path) -> Result<(), RepositoryError> {
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    fs::rename(source, destination)?;
    Ok(())
}

fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::{
        compress_file, decompress_file, hash_file, AnalysisContext, AnalysisNote, LapSummary,
        NoteColor, ReferenceLapContext, RepositoryError, RepositoryRecordRef, SimGitRepository,
        TelemetryRecord, TrackMapContext,
    };
    use std::fs;
    use std::path::PathBuf;

    fn temporary_directory(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "opendav-simgit-{name}-{}-{}",
            std::process::id(),
            super::unix_timestamp()
        ))
    }

    #[test]
    fn compressed_object_round_trip_preserves_bytes() {
        let root = temporary_directory("compression");
        fs::create_dir_all(&root).expect("temporary directory should be created");
        let source = root.join("source.ibt");
        let compressed = root.join("source.ibt.zst");
        let restored = root.join("restored.ibt");
        let payload = vec![42_u8; 128 * 1024];
        fs::write(&source, &payload).expect("source should be written");

        compress_file(&source, &compressed).expect("source should compress");
        decompress_file(&compressed, &restored).expect("object should decompress");

        assert_eq!(
            fs::read(restored).expect("restored file should read"),
            payload
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn repository_rejects_path_like_names() {
        let root = temporary_directory("invalid-name");

        let error = SimGitRepository::create(&root, "../team").expect_err("name must fail");

        assert!(matches!(error, RepositoryError::InvalidName));
    }

    #[test]
    fn analysis_notes_persist_with_telemetry_context() {
        let root = temporary_directory("notes");
        let mut repository =
            SimGitRepository::create(&root, "Race Team").expect("repository should be created");
        repository.manifest.telemetry.push(TelemetryRecord {
            id: "telemetry-id".to_owned(),
            original_name: "lap.ibt".to_owned(),
            object_name: "telemetry-id.ibt.zst".to_owned(),
            imported_at: 1,
            uncompressed_size: 100,
            compressed_size: 50,
            car: "GT3".to_owned(),
            venue: "Spa".to_owned(),
            track_id: 1,
            laps: vec![LapSummary {
                lap_number: 3,
                duration_seconds: 90.0,
            }],
        });
        repository.save().expect("record should persist");

        repository
            .add_note(
                "telemetry-id",
                "Driver",
                "Reduce entry rotation",
                "Rear instability at entry",
                NoteColor::Orange,
                AnalysisContext {
                    cursor_seconds: Some(42.5),
                    viewport: Some((40.0, 45.0)),
                    lap_number: Some(3),
                    worksheet: "Vehicle".to_owned(),
                    cyan_reference: Some(ReferenceLapContext {
                        file_name: "reference.ibt".to_owned(),
                        repository_record: Some(RepositoryRecordRef {
                            project: "Race Team".to_owned(),
                            telemetry_id: "reference-id".to_owned(),
                        }),
                        lap_number: 4,
                    }),
                    secondary_reference: None,
                    track_map: Some(TrackMapContext {
                        visible: true,
                        panel_width: 420.0,
                        rotation: 1.25,
                        bounds: Some([[-30.0, -20.0], [40.0, 50.0]]),
                    }),
                    section_delta: None,
                },
            )
            .expect("note should be added");
        let reopened =
            SimGitRepository::open(&root, "Race Team").expect("repository should reopen");

        assert_eq!(reopened.notes()[0].context.cursor_seconds, Some(42.5));
        assert_eq!(reopened.notes()[0].objective, "Reduce entry rotation");
        assert_eq!(reopened.notes()[0].color, NoteColor::Orange);
        assert_eq!(
            reopened.notes()[0]
                .context
                .track_map
                .as_ref()
                .and_then(TrackMapContext::valid_bounds),
            Some([[-30.0, -20.0], [40.0, 50.0]])
        );
        assert_eq!(
            reopened.notes()[0]
                .context
                .cyan_reference
                .as_ref()
                .map(|reference| reference.lap_number),
            Some(4)
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_notes_receive_color_and_reference_defaults() {
        let note: AnalysisNote = serde_json::from_str(
            r#"{
                "id":"note", "telemetry_id":"telemetry", "author":"Driver",
                "body":"Legacy", "created_at":1, "updated_at":1,
                "context":{"cursor_seconds":2.0,"viewport":null,"lap_number":1,"worksheet":"Driver"}
            }"#,
        )
        .expect("legacy note should deserialize");

        assert_eq!(note.color, NoteColor::Blue);
        assert!(note.objective.is_empty());
        assert_eq!(note.display_objective(), "Legacy");
        assert!(note.context.cyan_reference.is_none());
        assert!(note.context.secondary_reference.is_none());
        assert!(note.context.track_map.is_none());
    }

    #[test]
    fn legacy_telemetry_records_receive_empty_lap_summaries() {
        let record: TelemetryRecord = serde_json::from_str(
            r#"{
                "id":"telemetry", "original_name":"legacy.ibt",
                "object_name":"telemetry.ibt.zst", "imported_at":1,
                "uncompressed_size":100, "compressed_size":50,
                "car":"GT3", "venue":"Spa", "track_id":1
            }"#,
        )
        .expect("legacy telemetry should deserialize");

        assert!(record.laps.is_empty());
    }

    #[test]
    fn fastest_lap_ignores_invalid_summaries() {
        let record = TelemetryRecord {
            id: "telemetry".to_owned(),
            original_name: "laps.ibt".to_owned(),
            object_name: "telemetry.ibt.zst".to_owned(),
            imported_at: 1,
            uncompressed_size: 100,
            compressed_size: 50,
            car: "GT3".to_owned(),
            venue: "Spa".to_owned(),
            track_id: 1,
            laps: vec![
                LapSummary {
                    lap_number: 0,
                    duration_seconds: 1.0,
                },
                LapSummary {
                    lap_number: 4,
                    duration_seconds: f64::NAN,
                },
                LapSummary {
                    lap_number: 2,
                    duration_seconds: 91.2,
                },
                LapSummary {
                    lap_number: 3,
                    duration_seconds: 89.7,
                },
            ],
        };

        assert_eq!(record.fastest_lap().map(|lap| lap.lap_number), Some(3));
    }

    #[test]
    fn received_compressed_record_is_verified_and_openable() {
        let root = temporary_directory("received-record");
        fs::create_dir_all(&root).expect("temporary directory should be created");
        let source = root.join("remote.ibt");
        let compressed = root.join("remote.ibt.zst");
        let payload = vec![7_u8; 64 * 1024];
        fs::write(&source, &payload).expect("source should be written");
        compress_file(&source, &compressed).expect("source should compress");
        let (id, uncompressed_size) = hash_file(&source).expect("source should hash");
        let record = TelemetryRecord {
            object_name: format!("{id}.ibt.zst"),
            id: id.clone(),
            original_name: "remote.ibt".to_owned(),
            imported_at: 1,
            uncompressed_size,
            compressed_size: fs::metadata(&compressed)
                .expect("compressed metadata should read")
                .len(),
            car: "GT3".to_owned(),
            venue: "Spa".to_owned(),
            track_id: 1,
            laps: Vec::new(),
        };
        let mut repository =
            SimGitRepository::create(&root, "Team").expect("repository should be created");

        repository
            .install_compressed_record(record, &compressed)
            .expect("record should install");
        let restored = repository.resolve_ibt(&id).expect("record should resolve");

        assert_eq!(
            fs::read(restored).expect("restored file should read"),
            payload
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn calculate_section_delta_from_cache_computes_difference() {
        let pts = vec![
            [10.0, 0.100],
            [15.0, 0.250],
            [20.0, 0.400],
            [25.0, 0.150],
        ];
        // From 10.0s to 20.0s: delta goes from +0.100 to +0.400 => lost 0.300s
        let section_delta = super::calculate_section_delta_from_cache(&pts, 10.0, 20.0);
        assert!((section_delta.unwrap() - 0.3).abs() < 1e-5);
        assert_eq!(super::format_section_delta(section_delta.unwrap()), "+0.300s");

        // From 20.0s to 25.0s: delta goes from +0.400 to +0.150 => gained 0.250s (-0.250s)
        let section_delta_gained = super::calculate_section_delta_from_cache(&pts, 20.0, 25.0);
        assert!((section_delta_gained.unwrap() - (-0.25)).abs() < 1e-5);
        assert_eq!(super::format_section_delta(section_delta_gained.unwrap()), "-0.250s");
    }
}
