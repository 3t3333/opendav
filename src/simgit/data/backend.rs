//! Supabase cloud synchronization, RBAC models, initialization SQL, and packet compression for SimGit.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// The complete setup script required to configure a user's free-tier Supabase Postgres instance for SimGit.
pub const SUPABASE_INIT_SQL: &str = r#"-- SimGit v1.2.0 Backend Initialization SQL
-- Run this script in your Supabase SQL Editor to configure tables, RBAC roles, triggers, and RLS policies.

-- 1. Create User Role Enum
CREATE TYPE public.user_role AS ENUM ('admin', 'editor', 'viewer', 'pending');

-- 2. Create User Roles Table (maps Supabase auth users to SimGit roles)
CREATE TABLE public.user_roles (
    user_id UUID REFERENCES auth.users(id) ON DELETE CASCADE PRIMARY KEY,
    email TEXT,
    role public.user_role DEFAULT 'pending'::public.user_role NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 3. Enable Row Level Security on user_roles
ALTER TABLE public.user_roles ENABLE ROW LEVEL SECURITY;

-- 4. Create trigger function: First account automatically becomes 'admin' (never pending), subsequent accounts start as 'pending'
CREATE OR REPLACE FUNCTION public.handle_new_user()
RETURNS trigger AS $$
DECLARE
  role_assigned public.user_role;
  user_count INT;
BEGIN
  SELECT count(*) INTO user_count FROM public.user_roles;
  IF user_count = 0 THEN
    role_assigned := 'admin'::public.user_role;
  ELSE
    role_assigned := 'pending'::public.user_role;
  END IF;

  INSERT INTO public.user_roles (user_id, email, role)
  VALUES (new.id, new.email, role_assigned);
  RETURN new;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = public;

CREATE TRIGGER on_auth_user_created
  AFTER INSERT ON auth.users
  FOR EACH ROW EXECUTE PROCEDURE public.handle_new_user();

-- 4.5. Create RPC function for users to delete their own accounts
CREATE OR REPLACE FUNCTION public.delete_own_account()
RETURNS void AS $$
BEGIN
  DELETE FROM auth.users WHERE id = auth.uid();
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- 5. Create Projects Table (acts like Git Repositories for specific cars/tracks)
CREATE TABLE public.projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_by UUID REFERENCES auth.users(id),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 6. Create Telemetry Packets Metadata Table
CREATE TABLE public.telemetry_packets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID REFERENCES public.projects(id) ON DELETE CASCADE NOT NULL,
    uploaded_by UUID REFERENCES auth.users(id),
    telemetry_id TEXT NOT NULL UNIQUE,
    original_name TEXT NOT NULL,
    vehicle_name TEXT,
    venue_name TEXT,
    fastest_lap_seconds FLOAT,
    lap_count INT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 7. Create Telemetry Blobs Table (Separated from metadata for high query speed & storage efficiency)
CREATE TABLE public.telemetry_blobs (
    packet_id UUID REFERENCES public.telemetry_packets(id) ON DELETE CASCADE PRIMARY KEY,
    compressed_data BYTEA NOT NULL,
    uncompressed_size BIGINT NOT NULL,
    compressed_size BIGINT NOT NULL
);

-- 8. Create Analysis Notes Table (Synced explanations and context deltas)
CREATE TABLE public.analysis_notes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    packet_id UUID REFERENCES public.telemetry_packets(id) ON DELETE CASCADE NOT NULL,
    note_id TEXT NOT NULL UNIQUE,
    author TEXT NOT NULL,
    objective TEXT NOT NULL,
    body TEXT NOT NULL,
    color TEXT NOT NULL,
    lap_number INT,
    viewport_start FLOAT,
    viewport_end FLOAT,
    section_delta FLOAT,
    worksheet TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 9. Row Level Security Policies (configured for client-side SimGit RBAC enforcement)
ALTER TABLE public.projects ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.telemetry_packets ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.telemetry_blobs ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.analysis_notes ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Allow read for everyone" ON public.projects FOR SELECT USING (true);
CREATE POLICY "Allow read packets for everyone" ON public.telemetry_packets FOR SELECT USING (true);
CREATE POLICY "Allow read blobs for everyone" ON public.telemetry_blobs FOR SELECT USING (true);
CREATE POLICY "Allow read notes for everyone" ON public.analysis_notes FOR SELECT USING (true);
CREATE POLICY "Allow read roles for everyone" ON public.user_roles FOR SELECT USING (true);

CREATE POLICY "Allow insert projects" ON public.projects FOR INSERT WITH CHECK (true);
CREATE POLICY "Allow insert packets" ON public.telemetry_packets FOR INSERT WITH CHECK (true);
CREATE POLICY "Allow insert blobs" ON public.telemetry_blobs FOR INSERT WITH CHECK (true);
CREATE POLICY "Allow insert notes" ON public.analysis_notes FOR INSERT WITH CHECK (true);
CREATE POLICY "Allow insert roles" ON public.user_roles FOR INSERT WITH CHECK (true);

CREATE POLICY "Allow update roles" ON public.user_roles FOR UPDATE USING (true);
"#;

/// The script to wipe all SimGit data, tables, and roles from a Supabase instance.
pub const SUPABASE_WIPE_SQL: &str = r#"-- SimGit v1.2.0 Backend Wipe SQL
-- WARNING: This will delete ALL SimGit data from your database!
-- Run this script in your Supabase SQL Editor to wipe tables, triggers, and types.

DROP TRIGGER IF EXISTS on_auth_user_created ON auth.users;
DROP FUNCTION IF EXISTS public.handle_new_user();
DROP FUNCTION IF EXISTS public.delete_own_account();

DROP TABLE IF EXISTS public.analysis_notes CASCADE;
DROP TABLE IF EXISTS public.telemetry_blobs CASCADE;
DROP TABLE IF EXISTS public.telemetry_packets CASCADE;
DROP TABLE IF EXISTS public.projects CASCADE;
DROP TABLE IF EXISTS public.user_roles CASCADE;

DROP TYPE IF EXISTS public.user_role CASCADE;
"#;

/// Role assigned to an authenticated account in the SimGit system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BackendUserRole {
    #[default]
    Pending,
    Viewer,
    Editor,
    Admin,
}

impl BackendUserRole {
    pub const fn can_pull(self) -> bool {
        matches!(self, Self::Viewer | Self::Editor | Self::Admin)
    }

    pub const fn can_push(self) -> bool {
        matches!(self, Self::Editor | Self::Admin)
    }

    pub const fn can_manage_team(self) -> bool {
        matches!(self, Self::Admin)
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Pending => "Pending Approval",
            Self::Viewer => "Viewer (Read Only)",
            Self::Editor => "Editor (Push & Pull)",
            Self::Admin => "Admin (Full Access)",
        }
    }
}

/// User profile and access role retrieved from Supabase `user_roles` table.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UserRoleRecord {
    pub user_id: String,
    pub email: Option<String>,
    pub role: BackendUserRole,
}

/// High-compression level for network transmission and free-tier storage optimization.
const CLOUD_COMPRESSION_LEVEL: i32 = 15;

/// Compresses a binary telemetry payload for upload to Supabase using high-ratio Zstd compression.
pub fn compress_cloud_payload(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), CLOUD_COMPRESSION_LEVEL)?;
    encoder.write_all(data)?;
    encoder.finish()
}

/// Decompresses a binary telemetry blob pulled from Supabase back into raw bytes.
pub fn decompress_cloud_payload(compressed_data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut decoder = zstd::stream::read::Decoder::new(compressed_data)?;
    let mut buffer = Vec::new();
    decoder.read_to_end(&mut buffer)?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_permissions_and_hierarchies() {
        assert!(!BackendUserRole::Pending.can_pull());
        assert!(!BackendUserRole::Pending.can_push());
        assert!(!BackendUserRole::Pending.can_manage_team());

        assert!(BackendUserRole::Viewer.can_pull());
        assert!(!BackendUserRole::Viewer.can_push());
        assert!(!BackendUserRole::Viewer.can_manage_team());

        assert!(BackendUserRole::Editor.can_pull());
        assert!(BackendUserRole::Editor.can_push());
        assert!(!BackendUserRole::Editor.can_manage_team());

        assert!(BackendUserRole::Admin.can_pull());
        assert!(BackendUserRole::Admin.can_push());
        assert!(BackendUserRole::Admin.can_manage_team());
    }

    #[test]
    fn test_cloud_payload_compression_roundtrip() {
        // Create dummy telemetry payload resembling repeated sensor outputs
        let mut original_payload = Vec::with_capacity(50_000);
        for i in 0..5000 {
            original_payload.extend_from_slice(&(i as f32 * 0.1).to_le_bytes());
            original_payload.extend_from_slice(&(98.6_f32).to_le_bytes());
            original_payload.extend_from_slice(&(42_u32).to_le_bytes());
        }

        let compressed = compress_cloud_payload(&original_payload)
            .expect("Failed to compress cloud payload");

        // Asserts significant compression achieved for repetitive telemetry streams
        assert!(compressed.len() < original_payload.len() / 2, "Compression ratio should be greater than 2x for test payload");

        let decompressed = decompress_cloud_payload(&compressed)
            .expect("Failed to decompress cloud payload");

        assert_eq!(original_payload, decompressed);
    }

    #[test]
    fn test_supabase_init_sql_contains_key_elements() {
        assert!(SUPABASE_INIT_SQL.contains("CREATE TYPE user_role AS ENUM"));
        assert!(SUPABASE_INIT_SQL.contains("CREATE TABLE public.user_roles"));
        assert!(SUPABASE_INIT_SQL.contains("CREATE TABLE public.telemetry_blobs"));
        assert!(SUPABASE_INIT_SQL.contains("ENABLE ROW LEVEL SECURITY"));
        assert!(SUPABASE_INIT_SQL.contains("compressed_data BYTEA NOT NULL"));
        assert!(SUPABASE_INIT_SQL.contains("role_assigned := 'admin'::user_role;"));
    }
}
