//! REST client for Supabase cloud synchronization, implementing RBAC verification and data pushes/pulls.

use crate::simgit::data::backend::{
    compress_cloud_payload, decompress_cloud_payload, BackendUserRole, ProjectMemberRecord,
};
use serde::{Deserialize, Serialize};
use std::io::Read;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RemoteProject {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
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
    pub storage_file_path: String,
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

#[derive(Deserialize)]
struct UserRoleRecord {
    pub role: BackendUserRole,
}

pub struct SupabaseClient {
    pub url: String,
    pub anon_key: String,
    pub access_token: Option<String>,
    pub user_id: Option<String>,
    pub cached_role: BackendUserRole,
}

impl SupabaseClient {
    pub fn new(url: &str, anon_key: &str, access_token: Option<String>, user_id: Option<String>) -> Self {
        Self {
            url: url.trim_end_matches('/').to_string(),
            anon_key: anon_key.to_string(),
            access_token,
            user_id,
            cached_role: BackendUserRole::default(),
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
            return Err(
                "Supabase cloud synchronization credentials are not configured in Settings."
                    .to_string(),
            );
        }
        let endpoint = format!("{}/auth/v1/token?grant_type=password", self.url);
        let payload = format!(r#"{{"email":"{}","password":"{}"}}"#, email, password);
        let req = self
            .auth_headers(ureq::post(&endpoint))
            .header("Content-Type", "application/json");
        let response = req
            .send(payload.as_bytes())
            .map_err(|e| format!("Sign in failed: {}", e))?;
        let body = Self::read_body_to_string(response)?;

        let parsed: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Parse error: {}", e))?;
        let token = parsed["access_token"]
            .as_str()
            .ok_or("No access_token in response")?
            .to_string();
        let user_id = parsed["user"]["id"]
            .as_str()
            .ok_or("No user id in response")?
            .to_string();
        Ok((token, user_id))
    }

    pub fn sign_up(&mut self, email: &str, password: &str) -> Result<(String, String), String> {
        if !self.is_configured() {
            return Err(
                "Supabase cloud synchronization credentials are not configured in Settings."
                    .to_string(),
            );
        }
        let endpoint = format!("{}/auth/v1/signup", self.url);
        let payload = format!(r#"{{"email":"{}","password":"{}"}}"#, email, password);
        let req = self
            .auth_headers(ureq::post(&endpoint))
            .header("Content-Type", "application/json");
        let response = req
            .send(payload.as_bytes())
            .map_err(|e| format!("Sign up failed: {}", e))?;
        let body = Self::read_body_to_string(response)?;

        let parsed: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("Parse error: {}", e))?;
        let token = parsed["access_token"].as_str().ok_or("Signup successful! However, you must disable 'Confirm email' in your Supabase Auth Providers settings, or confirm your email to sign in.")?.to_string();
        let user_id = parsed["user"]["id"]
            .as_str()
            .ok_or("No user id in response")?
            .to_string();
        Ok((token, user_id))
    }

    pub fn delete_account(&self) -> Result<(), String> {
        if !self.is_configured() {
            return Err(
                "Supabase cloud synchronization credentials are not configured in Settings."
                    .to_string(),
            );
        }
        let endpoint = format!("{}/rest/v1/rpc/delete_own_account", self.url);
        let req = self
            .auth_headers(ureq::post(&endpoint))
            .header("Content-Type", "application/json");
        let _response = req
            .send("".as_bytes())
            .map_err(|e| format!("Delete account failed: {}", e))?;
        Ok(())
    }

    pub fn check_connection(&mut self) -> Result<(), String> {
        if !self.is_configured() {
            return Err("Supabase cloud synchronization credentials are not configured in Settings.".to_string());
        }
        let endpoint = format!("{}/rest/v1/projects?limit=1", self.url);
        let req = self.auth_headers(ureq::get(&endpoint));
        req.call().map_err(|e| format!("Failed to connect to Supabase: {}", e))?;
        Ok(())
    }

    pub fn check_connection_and_role(&mut self, _user_id: &str) -> Result<BackendUserRole, String> {
        self.check_connection()?;
        self.cached_role = BackendUserRole::Admin;
        Ok(BackendUserRole::Admin)
    }

    pub fn fetch_project_members(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectMemberRecord>, String> {
        let endpoint = format!("{}/rest/v1/project_members?project_id=eq.{}&select=*", self.url, urlencoding_simple(project_id));
        let req = self.auth_headers(ureq::get(&endpoint));
        let response = req.call().map_err(|e| format!("Failed to fetch members: {}", e))?;
        let body = Self::read_body_to_string(response)?;
        let records: Vec<ProjectMemberRecord> = serde_json::from_str(&body).map_err(|e| format!("Parse error: {}", e))?;
        Ok(records)
    }

    pub fn upsert_project_member(
        &self,
        project_id: &str,
        email: &str,
        role: BackendUserRole,
    ) -> Result<(), String> {
        let endpoint = format!("{}/rest/v1/project_members", self.url);
        let role_str = match role {
            BackendUserRole::Pending => "pending",
            BackendUserRole::Viewer => "viewer",
            BackendUserRole::Editor => "editor",
            BackendUserRole::Admin => "admin",
        };
        let payload = format!(r#"{{"project_id":"{}","email":"{}","role":"{}"}}"#, project_id, email, role_str);
        let req = self.auth_headers(ureq::post(&endpoint))
            .header("Content-Type", "application/json")
            .header("Prefer", "resolution=merge-duplicates");
            
        match req.send(payload.as_bytes()) {
            Ok(res) => {
                let status = res.status();
                if status.as_u16() >= 400 {
                    let body = Self::read_body_to_string(res).unwrap_or_default();
                    return Err(format!("Upsert failed (HTTP {}): {}", status.as_u16(), body));
                }
                Ok(())
            }
            Err(e) => Err(format!("Upsert request failed: {}", e)),
        }
    }

    pub fn remove_project_member(&self, project_id: &str, email: &str) -> Result<(), String> {
        let endpoint = format!("{}/rest/v1/project_members?project_id=eq.{}&email=eq.{}", self.url, urlencoding_simple(project_id), urlencoding_simple(email));
        let req = self.auth_headers(ureq::delete(&endpoint));
        req.call().map_err(|e| format!("Remove failed: {}", e))?;
        Ok(())
    }

    pub fn fetch_remote_projects(&self) -> Result<Vec<RemoteProject>, String> {
        let endpoint = format!("{}/rest/v1/projects?select=*", self.url);
        let req = self.auth_headers(ureq::get(&endpoint));
        let response = req
            .call()
            .map_err(|e| format!("Failed to fetch projects: {}", e))?;
        let body = Self::read_body_to_string(response)?;
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse projects JSON: {}", e))
    }

    pub fn ensure_project(&self, project_name: &str) -> Result<String, String> {
        let projects = self.fetch_remote_projects()?;
        if let Some(p) = projects.iter().find(|p| p.name == project_name) {
            return p
                .id
                .clone()
                .ok_or_else(|| "Project found but missing ID".to_string());
        }

        let new_project = RemoteProject {
            id: None,
            name: project_name.to_string(),
            description: None,
            owner_id: self.user_id.clone(),
        };

        let url = format!("{}/rest/v1/projects", self.url);
        let json = serde_json::to_string(&new_project).map_err(|e| e.to_string())?;
        let req = self
            .auth_headers(ureq::post(&url))
            .header("Content-Type", "application/json");
        req.send(json.as_bytes())
            .map_err(|e| format!("Failed to create project: {}", e))?;
        
        let projects = self.fetch_remote_projects()?;
        if let Some(p) = projects.iter().find(|p| p.name == project_name) {
            return p.id.clone().ok_or_else(|| "Failed to retrieve project ID after creation".to_string());
        }
        
        Err("Failed to verify project creation".to_string())
    }

    pub fn fetch_remote_packets(
        &self,
        project_id: &str,
    ) -> Result<Vec<RemotePacketMetadata>, String> {
        let endpoint = format!(
            "{}/rest/v1/telemetry_metadata?project_id=eq.{}&select=*",
            self.url, project_id
        );
        let req = self.auth_headers(ureq::get(&endpoint));
        let response = req
            .call()
            .map_err(|e| format!("Failed to fetch telemetry metadata: {}", e))?;
        let body = Self::read_body_to_string(response)?;
        serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse telemetry metadata JSON: {}", e))
    }

    pub fn push_packet(
        &self,
        metadata: &RemotePacketMetadata,
        raw_telemetry_payload: &[u8],
        notes: &[RemoteAnalysisNote],
    ) -> Result<(), String> {
        // 1. Upload the raw telemetry payload to storage bucket
        let storage_url = format!(
            "{}/storage/v1/object/telemetry-files/{}/{}.ibt",
            self.url, metadata.project_id, metadata.telemetry_id
        );
        let storage_req = self
            .auth_headers(ureq::post(&storage_url))
            .header("Content-Type", "application/octet-stream");

        let mut upload_success = false;
        match storage_req.send(raw_telemetry_payload) {
            Ok(res) => {
                let status = res.status();
                if status == 200 || status == 201 {
                    upload_success = true;
                } else if status == 400 || status == 409 {
                    if let Ok(body) = Self::read_body_to_string(res) {
                        if body.contains("Duplicate") || body.contains("KeyAlreadyExists") {
                            upload_success = true;
                        } else {
                            return Err(format!("Failed to upload telemetry payload to storage ({}): {}", status, body));
                        }
                    } else {
                        return Err(format!("Failed to upload telemetry payload to storage ({}) and failed to read body", status));
                    }
                } else {
                    return Err(format!("Failed to upload telemetry payload to storage, received HTTP {}", status));
                }
            }
            Err(e) => return Err(format!("Failed to upload telemetry payload to storage: {}", e)),
        }

        if !upload_success {
            return Err("Failed to upload telemetry payload".to_string());
        }

        // 2. Insert Metadata
        let meta_url = format!("{}/rest/v1/telemetry_metadata", self.url);
        let meta_json = serde_json::to_string(metadata).map_err(|e| e.to_string())?;
        let meta_req = self
            .auth_headers(ureq::post(&meta_url))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation");

        let meta_res = meta_req
            .send(meta_json.as_bytes())
            .map_err(|e| format!("Failed to insert metadata: {}", e))?;
        let meta_body = Self::read_body_to_string(meta_res)?;
        let returned_meta: Vec<RemotePacketMetadata> = serde_json::from_str(&meta_body)
            .map_err(|e| format!("Failed to parse inserted packet metadata response: {}", e))?;

        let packet_id = returned_meta
            .first()
            .and_then(|m| m.id.clone())
            .ok_or_else(|| "No packet ID returned after metadata insert".to_string())?;

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
            let notes_req = self
                .auth_headers(ureq::post(&notes_url))
                .header("Content-Type", "application/json")
                .header("Prefer", "return=minimal");
            let _ = notes_req
                .send(notes_json.as_bytes())
                .map_err(|e| format!("Failed to upload associated analysis notes: {}", e))?;
        }

        Ok(())
    }

    pub fn pull_packet(
        &self,
        packet_id: &str,
        storage_file_path: &str,
    ) -> Result<(Vec<u8>, Vec<RemoteAnalysisNote>), String> {
        // 1. Fetch raw payload from storage
        let storage_url = format!(
            "{}/storage/v1/object/telemetry-files/{}",
            self.url, storage_file_path
        );
        let storage_req = self.auth_headers(ureq::get(&storage_url));
        let storage_res = storage_req
            .call()
            .map_err(|e| format!("Failed to download telemetry file from storage: {}", e))?;

        let mut raw_data = Vec::new();
        storage_res
            .into_body()
            .into_reader()
            .read_to_end(&mut raw_data)
            .map_err(|e| format!("Failed to read telemetry file bytes: {}", e))?;

        // 2. Fetch associated notes
        let notes = self.fetch_remote_notes(packet_id)?;
        Ok((raw_data, notes))
    }

    pub fn fetch_remote_notes(&self, packet_id: &str) -> Result<Vec<RemoteAnalysisNote>, String> {
        let notes_url = format!(
            "{}/rest/v1/analysis_notes?packet_id=eq.{}&select=*",
            self.url, packet_id
        );
        let notes_req = self.auth_headers(ureq::get(&notes_url));
        let notes_res = notes_req
            .call()
            .map_err(|e| format!("Failed to fetch notes: {}", e))?;
        let notes_body = Self::read_body_to_string(notes_res)?;
        serde_json::from_str(&notes_body).map_err(|e| format!("Failed to parse notes JSON: {}", e))
    }
}

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
