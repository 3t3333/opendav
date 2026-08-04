//! REST client for Supabase cloud synchronization, implementing RBAC verification and data pushes/pulls.

use std::io::Read;
use serde::{Deserialize, Serialize};
use crate::simgit::data::backend::{
    BackendUserRole, UserRoleRecord, compress_cloud_payload, decompress_cloud_payload,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RemoteProject {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RemotePacketMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub project_id: String,
    pub telemetry_id: String,
    pub original_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vehicle_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub venue_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fastest_lap_seconds: Option<f64>,
    pub lap_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uploaded_by: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RemoteAnalysisNote {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_id: Option<String>,
    pub note_id: String,
    pub author: String,
    pub objective: String,
    pub body: String,
    pub color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lap_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewport_start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewport_end: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_delta: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worksheet: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BlobRecord {
    pub packet_id: String,
    pub compressed_data: String,
    pub uncompressed_size: i64,
    pub compressed_size: i64,
}

/// Helper to encode raw byte arrays into Postgres BYTEA hex representation (\x...).
pub fn encode_bytea(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(2 + bytes.len() * 2);
    s.push_str("\\x");
    for &b in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}

/// Helper to decode Postgres BYTEA hex representation back into raw byte vectors.
pub fn decode_bytea(hex_str: &str) -> Result<Vec<u8>, String> {
    let stripped = hex_str.strip_prefix("\\x").unwrap_or(hex_str);
    if stripped.len() % 2 != 0 {
        return Err("Invalid bytea hex length (must be even)".to_string());
    }
    (0..stripped.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&stripped[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

/// Client for interacting with an individual user's configured Supabase instance via PostgREST.
pub struct SupabaseClient {
    pub url: String,
    pub anon_key: String,
    pub access_token: Option<String>,
    pub cached_role: BackendUserRole,
}

impl SupabaseClient {
    pub fn new(url: &str, anon_key: &str, access_token: Option<String>) -> Self {
        let clean_url = url.trim_end_matches('/').to_string();
        Self {
            url: clean_url,
            anon_key: anon_key.trim().to_string(),
            access_token,
            cached_role: BackendUserRole::Pending,
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.url.is_empty() && !self.anon_key.is_empty()
    }

    fn auth_headers<T>(&self, req: ureq::RequestBuilder<T>) -> ureq::RequestBuilder<T> {
        let auth_val = if let Some(token) = &self.access_token {
            format!("Bearer {}", token)
        } else {
            format!("Bearer {}", self.anon_key)
        };
        req.header("apikey", &self.anon_key)
            .header("Authorization", &auth_val)
            .header("Accept", "application/json")
    }

    fn read_body_to_string(res: ureq::http::Response<ureq::Body>) -> Result<String, String> {
        let mut s = String::new();
        Read::read_to_string(&mut res.into_body().into_reader(), &mut s)
            .map_err(|e| format!("Failed to read HTTP response body: {}", e))?;
        Ok(s)
    }

    pub fn sign_in(&mut self, email: &str, password: &str) -> Result<(String, String), String> {
        if !self.is_configured() {
            return Err("Supabase cloud synchronization credentials are not configured in Settings.".to_string());
        }
        let endpoint = format!("{}/auth/v1/token?grant_type=password", self.url);
        let payload = format!(r#"{{"email":"{}","password":"{}"}}"#, email, password);
        let req = self.auth_headers(ureq::post(&endpoint))
            .header("Content-Type", "application/json");
        let response = req.send(payload.as_bytes()).map_err(|e| format!("Sign in failed: {}", e))?;
        let body = Self::read_body_to_string(response)?;
        
        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| format!("Parse error: {}", e))?;
        let token = parsed["access_token"].as_str().ok_or("No access_token in response")?.to_string();
        let user_id = parsed["user"]["id"].as_str().ok_or("No user id in response")?.to_string();
        Ok((token, user_id))
    }

    pub fn sign_up(&mut self, email: &str, password: &str) -> Result<(String, String), String> {
        if !self.is_configured() {
            return Err("Supabase cloud synchronization credentials are not configured in Settings.".to_string());
        }
        let endpoint = format!("{}/auth/v1/signup", self.url);
        let payload = format!(r#"{{"email":"{}","password":"{}"}}"#, email, password);
        let req = self.auth_headers(ureq::post(&endpoint))
            .header("Content-Type", "application/json");
        let response = req.send(payload.as_bytes()).map_err(|e| format!("Sign up failed: {}", e))?;
        let body = Self::read_body_to_string(response)?;
        
        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| format!("Parse error: {}", e))?;
        let token = parsed["access_token"].as_str().ok_or("Signup successful! However, you must disable 'Confirm email' in your Supabase Auth Providers settings, or confirm your email to sign in.")?.to_string();
        let user_id = parsed["user"]["id"].as_str().ok_or("No user id in response")?.to_string();
        Ok((token, user_id))
    }

    pub fn delete_account(&self) -> Result<(), String> {
        if !self.is_configured() {
            return Err("Supabase cloud synchronization credentials are not configured in Settings.".to_string());
        }
        let endpoint = format!("{}/rest/v1/rpc/delete_own_account", self.url);
        let req = self.auth_headers(ureq::post(&endpoint))
            .header("Content-Type", "application/json");
        let _response = req.send("".as_bytes()).map_err(|e| format!("Delete account failed: {}", e))?;
        Ok(())
    }

    /// Verifies connectivity and retrieves the user's role from the Supabase user_roles table.
    pub fn check_connection_and_role(&mut self, user_id: &str) -> Result<BackendUserRole, String> {
        if !self.is_configured() {
            return Err("Supabase cloud synchronization credentials are not configured in Settings.".to_string());
        }

        let endpoint = format!("{}/rest/v1/user_roles?user_id=eq.{}&select=*", self.url, urlencoding_simple(user_id));
        let req = self.auth_headers(ureq::get(&endpoint));
        let response = req.call().map_err(|e| format!("Failed to connect to Supabase: {}", e))?;
        let body = Self::read_body_to_string(response)?;

        let records: Vec<UserRoleRecord> = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse user role JSON response: {}", e))?;

        if let Some(record) = records.first() {
            self.cached_role = record.role;
            Ok(self.cached_role)
        } else {
            // User exists in auth or hasn't been added to user_roles yet; default to pending
            self.cached_role = BackendUserRole::Pending;
            Ok(self.cached_role)
        }
    }

    /// Fetches all user role records from the database for admin team management.
    pub fn fetch_all_users(&self) -> Result<Vec<UserRoleRecord>, String> {
        if !self.cached_role.can_manage_team() {
            return Err("User role is not Admin and lacks permission to manage team accounts.".to_string());
        }
        let endpoint = format!("{}/rest/v1/user_roles?select=*&order=created_at.asc", self.url);
        let req = self.auth_headers(ureq::get(&endpoint));
        let response = req.call().map_err(|e| format!("Failed to fetch user directory: {}", e))?;
        let body = Self::read_body_to_string(response)?;
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse user list JSON: {}", e))
    }

    /// Updates the access role of a designated user account.
    pub fn update_user_role(&self, target_user_id: &str, new_role: BackendUserRole) -> Result<(), String> {
        if !self.cached_role.can_manage_team() {
            return Err("Only team Administrators can modify account roles or approve pending members.".to_string());
        }
        let endpoint = format!("{}/rest/v1/user_roles?user_id=eq.{}", self.url, target_user_id);
        let role_str = match new_role {
            BackendUserRole::Admin => "admin",
            BackendUserRole::Editor => "editor",
            BackendUserRole::Viewer => "viewer",
            BackendUserRole::Pending => "pending",
        };
        let payload_json = format!("{{\"role\":\"{}\"}}", role_str);
        let req = self
            .auth_headers(ureq::patch(&endpoint))
            .header("Content-Type", "application/json");
        let res = req
            .send(payload_json.as_bytes())
            .map_err(|e| format!("Failed to update role for user {}: {}", target_user_id, e))?;
        let _ = Self::read_body_to_string(res);
        Ok(())
    }

    /// Fetches available collaborative projects on the remote server.
    pub fn fetch_remote_projects(&self) -> Result<Vec<RemoteProject>, String> {
        if !self.cached_role.can_pull() {
            return Err("User role lacks permission to read remote projects.".to_string());
        }
        let endpoint = format!("{}/rest/v1/projects?select=*", self.url);
        let req = self.auth_headers(ureq::get(&endpoint));
        let response = req.call().map_err(|e| format!("Failed to fetch projects: {}", e))?;
        let body = Self::read_body_to_string(response)?;
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse projects JSON: {}", e))
    }

    /// Finds a project UUID by name, or creates it if it doesn't exist.
    pub fn ensure_project(&self, project_name: &str) -> Result<String, String> {
        let projects = self.fetch_remote_projects()?;
        if let Some(p) = projects.iter().find(|p| p.name == project_name) {
            return p.id.clone().ok_or_else(|| "Project found but missing ID".to_string());
        }
        
        if !self.cached_role.can_push() {
            return Err("Cannot create new remote project: lacks Editor/Admin role.".to_string());
        }
        
        let new_project = RemoteProject {
            id: None,
            name: project_name.to_string(),
            description: None,
        };
        
        let url = format!("{}/rest/v1/projects", self.url);
        let json = serde_json::to_string(&new_project).map_err(|e| e.to_string())?;
        let req = self.auth_headers(ureq::post(&url))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation");
        let res = req.send(json.as_bytes()).map_err(|e| format!("Failed to create project: {}", e))?;
        let body = Self::read_body_to_string(res)?;
        let returned: Vec<RemoteProject> = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse inserted project: {}", e))?;
        
        returned.first().and_then(|p| p.id.clone())
            .ok_or_else(|| "Failed to retrieve project ID after creation".to_string())
    }

    /// Fetches telemetry packet metadata from the remote server for a specific project.
    pub fn fetch_remote_packets(&self, project_id: &str) -> Result<Vec<RemotePacketMetadata>, String> {
        if !self.cached_role.can_pull() {
            return Err("User role lacks permission to pull telemetry packets.".to_string());
        }
        let endpoint = format!("{}/rest/v1/telemetry_packets?project_id=eq.{}&select=*", self.url, project_id);
        let req = self.auth_headers(ureq::get(&endpoint));
        let response = req.call().map_err(|e| format!("Failed to fetch telemetry packets: {}", e))?;
        let body = Self::read_body_to_string(response)?;
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse telemetry packets JSON: {}", e))
    }

    /// Pushes a telemetry packet (metadata, compressed binary blob, and analysis notes) to the cloud.
    pub fn push_packet(
        &self,
        metadata: &RemotePacketMetadata,
        raw_telemetry_payload: &[u8],
        notes: &[RemoteAnalysisNote],
    ) -> Result<(), String> {
        if !self.cached_role.can_push() {
            return Err("User role lacks Editor or Admin permission to push telemetry packets to the team repository.".to_string());
        }

        // 1. Insert Metadata
        let meta_url = format!("{}/rest/v1/telemetry_packets", self.url);
        let meta_json = serde_json::to_string(metadata).map_err(|e| e.to_string())?;
        let meta_req = self.auth_headers(ureq::post(&meta_url))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation");
        
        let meta_res = meta_req.send(meta_json.as_bytes()).map_err(|e| format!("Failed to insert metadata: {}", e))?;
        let meta_body = Self::read_body_to_string(meta_res)?;
        let returned_meta: Vec<RemotePacketMetadata> = serde_json::from_str(&meta_body)
            .map_err(|e| format!("Failed to parse inserted packet metadata response: {}", e))?;
        
        let packet_id = returned_meta.first()
            .and_then(|m| m.id.clone())
            .ok_or_else(|| "No packet ID returned after metadata insert".to_string())?;

        // 2. Compress payload and insert into telemetry_blobs
        let compressed = compress_cloud_payload(raw_telemetry_payload)
            .map_err(|e| format!("Compression failed: {}", e))?;
        let hex_data = encode_bytea(&compressed);

        let blob_record = BlobRecord {
            packet_id: packet_id.clone(),
            compressed_data: hex_data,
            uncompressed_size: raw_telemetry_payload.len() as i64,
            compressed_size: compressed.len() as i64,
        };

        let blob_url = format!("{}/rest/v1/telemetry_blobs", self.url);
        let blob_json = serde_json::to_string(&blob_record).map_err(|e| e.to_string())?;
        let blob_req = self.auth_headers(ureq::post(&blob_url))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=minimal");
        let _ = blob_req.send(blob_json.as_bytes()).map_err(|e| format!("Failed to upload telemetry binary blob (HTTP {}): {}", e, e.to_string()))?;
        // 3. Insert analysis notes if any exist
        if !notes.is_empty() {
            let mut notes_to_insert = Vec::new();
            for n in notes {
                let mut note = n.clone();
                note.packet_id = Some(packet_id.clone());
                notes_to_insert.push(note);
            }
            let notes_url = format!("{}/rest/v1/analysis_notes", self.url);
            let notes_json = serde_json::to_string(&notes_to_insert).map_err(|e| e.to_string())?;
            let notes_req = self.auth_headers(ureq::post(&notes_url))
                .header("Content-Type", "application/json")
                .header("Prefer", "return=minimal");
            let _ = notes_req.send(notes_json.as_bytes()).map_err(|e| format!("Failed to upload associated analysis notes: {}", e))?;
        }

        Ok(())
    }

    /// Pulls a telemetry packet by ID, decompressing its binary data and fetching associated notes.
    pub fn pull_packet(&self, packet_id: &str) -> Result<(Vec<u8>, Vec<RemoteAnalysisNote>), String> {
        if !self.cached_role.can_pull() {
            return Err("User role lacks permission to pull telemetry data.".to_string());
        }

        // 1. Fetch compressed blob
        let blob_url = format!("{}/rest/v1/telemetry_blobs?packet_id=eq.{}&select=*", self.url, packet_id);
        let blob_req = self.auth_headers(ureq::get(&blob_url));
        let blob_res = blob_req.call().map_err(|e| format!("Failed to download telemetry blob: {}", e))?;
        let blob_body = Self::read_body_to_string(blob_res)?;
        let blobs: Vec<BlobRecord> = serde_json::from_str(&blob_body)
            .map_err(|e| format!("Failed to parse downloaded blob JSON: {}", e))?;
        
        let blob = blobs.first().ok_or_else(|| format!("No telemetry blob found for packet_id {}", packet_id))?;
        let compressed = decode_bytea(&blob.compressed_data)?;
        let decompressed = decompress_cloud_payload(&compressed)
            .map_err(|e| format!("Decompression failed: {}", e))?;

        // 2. Fetch associated notes
        let notes = self.fetch_remote_notes(packet_id)?;
        Ok((decompressed, notes))
    }

    /// Fetches analysis notes associated with a specific remote packet ID.
    pub fn fetch_remote_notes(&self, packet_id: &str) -> Result<Vec<RemoteAnalysisNote>, String> {
        if !self.cached_role.can_pull() {
            return Err("User role lacks permission to read analysis notes.".to_string());
        }
        let notes_url = format!("{}/rest/v1/analysis_notes?packet_id=eq.{}&select=*", self.url, packet_id);
        let notes_req = self.auth_headers(ureq::get(&notes_url));
        let notes_res = notes_req.call().map_err(|e| format!("Failed to fetch notes: {}", e))?;
        let notes_body = Self::read_body_to_string(notes_res)?;
        serde_json::from_str(&notes_body).map_err(|e| format!("Failed to parse notes JSON: {}", e))
    }
}

/// Minimal URL query string encoder for email addresses.
fn urlencoding_simple(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                use std::fmt::Write as _;
                let _ = write!(&mut out, "%{:02X}", b);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytea_encoding_and_decoding_roundtrip() {
        let test_bytes = vec![0x00, 0x01, 0xFF, 0x42, 0xAA, 0x55];
        let encoded = encode_bytea(&test_bytes);
        assert_eq!(encoded, "\\x0001ff42aa55");

        let decoded = decode_bytea(&encoded).expect("Failed to decode valid bytea string");
        assert_eq!(decoded, test_bytes);
    }

    #[test]
    fn test_simple_url_encoding() {
        let email = "arturo.driver+team@opendav.com";
        let encoded = urlencoding_simple(email);
        assert_eq!(encoded, "arturo.driver%2Bteam%40opendav.com");
    }

    #[test]
    fn test_admin_permissions_required_for_user_management() {
        let mut client = SupabaseClient::new("https://test.supabase.co", "key", None);
        client.cached_role = BackendUserRole::Editor;
        assert!(client.fetch_all_users().is_err());
        assert!(client.update_user_role("uuid-123", BackendUserRole::Viewer).is_err());

        client.cached_role = BackendUserRole::Viewer;
        assert!(client.fetch_all_users().is_err());
        assert!(client.update_user_role("uuid-123", BackendUserRole::Editor).is_err());
    }
}
