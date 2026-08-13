//! Supabase cloud synchronization, RBAC models, initialization SQL, and packet compression for SimGit.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// The complete setup script required to configure a user's Supabase Postgres instance for SimGit.
pub const SUPABASE_INIT_SQL: &str = r#"-- SimGit v2.0.0 Multi-Tenant Backend Initialization SQL
-- Run this script in your Supabase SQL Editor to configure tables, RBAC roles, triggers, and RLS policies.

-- 1. Create User Role Enum
CREATE TYPE public.user_role AS ENUM ('admin', 'editor', 'viewer', 'pending');

-- 2. Create Projects Table (Acts as isolated multi-tenant repositories)
CREATE TABLE public.projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_by UUID REFERENCES auth.users(id) DEFAULT auth.uid(),
    owner_id UUID REFERENCES auth.users(id) DEFAULT auth.uid(),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 3. Create Project Members Table (Multi-Tenant RBAC)
CREATE TABLE public.project_members (
    project_id UUID REFERENCES public.projects(id) ON DELETE CASCADE,
    user_id UUID REFERENCES auth.users(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    role public.user_role DEFAULT 'viewer'::public.user_role NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (project_id, email)
);

-- 4. Triggers to auto-link invited emails to Supabase Auth UUIDs
CREATE OR REPLACE FUNCTION public.handle_new_user()
RETURNS trigger AS $$
BEGIN
  UPDATE public.project_members SET user_id = new.id WHERE email = new.email;
  RETURN new;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = public;

CREATE TRIGGER on_auth_user_created
  AFTER INSERT ON auth.users
  FOR EACH ROW EXECUTE PROCEDURE public.handle_new_user();

CREATE OR REPLACE FUNCTION public.handle_new_member_invite()
RETURNS trigger AS $$
DECLARE
  found_id UUID;
BEGIN
  IF new.user_id IS NULL AND new.email IS NOT NULL THEN
    SELECT id INTO found_id FROM auth.users WHERE email = new.email LIMIT 1;
    IF found_id IS NOT NULL THEN
      new.user_id := found_id;
    END IF;
  END IF;
  RETURN new;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = public;

CREATE TRIGGER on_project_member_invited
  BEFORE INSERT OR UPDATE ON public.project_members
  FOR EACH ROW EXECUTE PROCEDURE public.handle_new_member_invite();

CREATE OR REPLACE FUNCTION public.handle_new_project()
RETURNS trigger AS $$
DECLARE
  creator_email TEXT;
BEGIN
  IF new.owner_id IS NOT NULL THEN
    SELECT email INTO creator_email FROM auth.users WHERE id = new.owner_id;
    IF creator_email IS NOT NULL THEN
      INSERT INTO public.project_members (project_id, user_id, email, role)
      VALUES (new.id, new.owner_id, creator_email, 'admin'::public.user_role)
      ON CONFLICT DO NOTHING;
    END IF;
  END IF;
  RETURN new;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = public;

CREATE TRIGGER on_project_created
  AFTER INSERT ON public.projects
  FOR EACH ROW EXECUTE PROCEDURE public.handle_new_project();

-- 4.5. Create RPC function for users to delete their own accounts
CREATE OR REPLACE FUNCTION public.delete_own_account()
RETURNS void AS $$
BEGIN
  DELETE FROM auth.users WHERE id = auth.uid();
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- 5. Create Telemetry Packets Metadata Table
CREATE TABLE public.telemetry_packets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID REFERENCES public.projects(id) ON DELETE CASCADE NOT NULL,
    uploaded_by UUID REFERENCES auth.users(id) DEFAULT auth.uid(),
    telemetry_id TEXT NOT NULL UNIQUE,
    original_name TEXT NOT NULL,
    vehicle_name TEXT,
    venue_name TEXT,
    fastest_lap_seconds FLOAT,
    lap_count INT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 6. Create Telemetry Blobs Table (Separated from metadata for high query speed & storage efficiency)
CREATE TABLE public.telemetry_blobs (
    packet_id UUID REFERENCES public.telemetry_packets(id) ON DELETE CASCADE PRIMARY KEY,
    compressed_data TEXT NOT NULL,
    uncompressed_size BIGINT NOT NULL,
    compressed_size BIGINT NOT NULL
);

-- 7. Create Analysis Notes Table (Synced explanations and context deltas)
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

-- 8. Row Level Security Policies (Multi-Tenant Isolation)
GRANT USAGE ON SCHEMA public TO anon, authenticated;
GRANT ALL ON ALL TABLES IN SCHEMA public TO anon, authenticated;
GRANT ALL ON ALL ROUTINES IN SCHEMA public TO anon, authenticated;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO anon, authenticated;

-- Security Definer Functions to avoid RLS nested evaluation (PostgREST 500 fix)
CREATE OR REPLACE FUNCTION public.simgit_can_read_project(proj_id UUID)
RETURNS boolean AS $$
BEGIN
  RETURN EXISTS (
    SELECT 1 FROM public.projects p WHERE p.id = proj_id AND (p.owner_id = auth.uid() OR p.created_by = auth.uid())
  ) OR EXISTS (
    SELECT 1 FROM public.project_members m WHERE m.project_id = proj_id AND (m.user_id = auth.uid() OR m.email = (auth.jwt()->>'email'))
  );
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

CREATE OR REPLACE FUNCTION public.simgit_can_read_packet(proj_id UUID)
RETURNS boolean AS $$
BEGIN
  RETURN EXISTS (
    SELECT 1 FROM public.projects p WHERE p.id = proj_id AND (p.owner_id = auth.uid() OR p.created_by = auth.uid())
  ) OR EXISTS (
    SELECT 1 FROM public.project_members m WHERE m.project_id = proj_id AND (m.user_id = auth.uid() OR m.email = (auth.jwt()->>'email')) AND m.role != 'pending'::public.user_role
  );
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

CREATE OR REPLACE FUNCTION public.simgit_can_write_packet(proj_id UUID)
RETURNS boolean AS $$
BEGIN
  RETURN EXISTS (
    SELECT 1 FROM public.projects p WHERE p.id = proj_id AND (p.owner_id = auth.uid() OR p.created_by = auth.uid())
  ) OR EXISTS (
    SELECT 1 FROM public.project_members m WHERE m.project_id = proj_id AND (m.user_id = auth.uid() OR m.email = (auth.jwt()->>'email')) AND m.role IN ('admin'::public.user_role, 'editor'::public.user_role)
  );
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

ALTER TABLE public.projects ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.project_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.telemetry_packets ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.telemetry_blobs ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.analysis_notes ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Select projects" ON public.projects FOR SELECT 
USING (public.simgit_can_read_project(id));

CREATE POLICY "Insert projects" ON public.projects FOR INSERT WITH CHECK (auth.uid() IS NOT NULL);

CREATE POLICY "Select project members" ON public.project_members FOR SELECT USING (true);

CREATE POLICY "Insert project members" ON public.project_members FOR INSERT WITH CHECK (
    public.simgit_can_write_packet(project_id)
);

CREATE POLICY "Update project members" ON public.project_members FOR UPDATE USING (
    public.simgit_can_write_packet(project_id)
);

CREATE POLICY "Delete project members" ON public.project_members FOR DELETE USING (
    project_id IN (SELECT id FROM public.projects WHERE owner_id = auth.uid() OR created_by = auth.uid()) OR
    project_id IN (SELECT project_id FROM public.project_members WHERE user_id = auth.uid() AND role = 'admin'::public.user_role)
);

CREATE POLICY "Select packets" ON public.telemetry_packets FOR SELECT 
USING (public.simgit_can_read_packet(project_id));

CREATE POLICY "Insert packets" ON public.telemetry_packets FOR INSERT WITH CHECK (
    public.simgit_can_write_packet(project_id)
);

CREATE POLICY "Select blobs" ON public.telemetry_blobs FOR SELECT 
USING (packet_id IN (SELECT id FROM public.telemetry_packets));

CREATE POLICY "Insert blobs" ON public.telemetry_blobs FOR INSERT WITH CHECK (
    packet_id IN (SELECT id FROM public.telemetry_packets)
);

CREATE POLICY "Select notes" ON public.analysis_notes FOR SELECT 
USING (packet_id IN (SELECT id FROM public.telemetry_packets));

CREATE POLICY "Insert notes" ON public.analysis_notes FOR INSERT WITH CHECK (
    packet_id IN (SELECT id FROM public.telemetry_packets)
);
"#;

/// Automated storage management script for OpenDav Free Cloud & BYOD (30-day retention + 250MB repo limit).
pub const SUPABASE_PRUNE_SQL: &str = r#"-- SimGit v2.0.0 Automated Storage Pruning SQL
-- Requires pg_cron extension enabled on Supabase.
CREATE EXTENSION IF NOT EXISTS pg_cron;

-- Job 1: Auto-delete telemetry older than 30 days (runs daily at midnight)
SELECT cron.schedule('delete_old_telemetry', '0 0 * * *', $$
  DELETE FROM public.telemetry_packets WHERE created_at < NOW() - INTERVAL '30 days';
$$);

-- Job 2: Enforce 250MB limit per repository (runs hourly)
CREATE OR REPLACE FUNCTION public.enforce_per_project_limit() RETURNS void AS $func$
DECLARE
    proj RECORD;
    total_size BIGINT;
    oldest_packet_id UUID;
BEGIN
    FOR proj IN SELECT id FROM public.projects LOOP
        SELECT COALESCE(SUM(b.compressed_size), 0) INTO total_size 
        FROM public.telemetry_blobs b
        JOIN public.telemetry_packets p ON b.packet_id = p.id
        WHERE p.project_id = proj.id;

        -- While project storage exceeds 250MB (262,144,000 bytes), prune oldest packet
        WHILE total_size > 262144000 LOOP
            SELECT id INTO oldest_packet_id FROM public.telemetry_packets 
            WHERE project_id = proj.id ORDER BY created_at ASC LIMIT 1;
            
            IF oldest_packet_id IS NULL THEN EXIT; END IF;
            
            DELETE FROM public.telemetry_packets WHERE id = oldest_packet_id;
            
            SELECT COALESCE(SUM(b.compressed_size), 0) INTO total_size 
            FROM public.telemetry_blobs b
            JOIN public.telemetry_packets p ON b.packet_id = p.id
            WHERE p.project_id = proj.id;
        END LOOP;
    END LOOP;
END;
$func$ LANGUAGE plpgsql SECURITY DEFINER;

SELECT cron.schedule('enforce_repo_limits', '0 * * * *', $$
  SELECT public.enforce_per_project_limit();
$$);
"#;

/// The script to wipe all SimGit data, tables, and roles from a Supabase instance.
pub const SUPABASE_WIPE_SQL: &str = r#"-- SimGit v2.0.0 Backend Wipe SQL
-- WARNING: This will delete ALL SimGit data from your database!
-- Run this script in your Supabase SQL Editor to wipe tables, triggers, and types.

DROP TRIGGER IF EXISTS on_auth_user_created ON auth.users;
DROP TRIGGER IF EXISTS on_project_member_invited ON public.project_members;
DROP TRIGGER IF EXISTS on_project_created ON public.projects;
DROP FUNCTION IF EXISTS public.handle_new_user();
DROP FUNCTION IF EXISTS public.handle_new_member_invite();
DROP FUNCTION IF EXISTS public.handle_new_project();
DROP FUNCTION IF EXISTS public.delete_own_account();

DROP TABLE IF EXISTS public.analysis_notes CASCADE;
DROP TABLE IF EXISTS public.telemetry_blobs CASCADE;
DROP TABLE IF EXISTS public.telemetry_packets CASCADE;
DROP TABLE IF EXISTS public.project_members CASCADE;
DROP TABLE IF EXISTS public.projects CASCADE;

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

/// User profile and access role retrieved from Supabase `project_members` table.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProjectMemberRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub email: String,
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

        let compressed =
            compress_cloud_payload(&original_payload).expect("Failed to compress cloud payload");

        // Asserts significant compression achieved for repetitive telemetry streams
        assert!(
            compressed.len() < original_payload.len() / 2,
            "Compression ratio should be greater than 2x for test payload"
        );

        let decompressed =
            decompress_cloud_payload(&compressed).expect("Failed to decompress cloud payload");

        assert_eq!(original_payload, decompressed);
    }

    #[test]
    fn test_supabase_init_sql_contains_key_elements() {
        assert!(SUPABASE_INIT_SQL.contains("CREATE TYPE public.user_role AS ENUM"));
        assert!(SUPABASE_INIT_SQL.contains("CREATE TABLE public.project_members"));
        assert!(SUPABASE_INIT_SQL.contains("CREATE TABLE public.telemetry_blobs"));
        assert!(SUPABASE_INIT_SQL.contains("ENABLE ROW LEVEL SECURITY"));
        assert!(SUPABASE_INIT_SQL.contains("compressed_data BYTEA NOT NULL"));
        assert!(SUPABASE_INIT_SQL.contains("CREATE TABLE public.projects"));
    }

    #[test]
    fn test_supabase_prune_sql_contains_key_elements() {
        assert!(SUPABASE_PRUNE_SQL.contains("CREATE EXTENSION IF NOT EXISTS pg_cron;"));
        assert!(SUPABASE_PRUNE_SQL.contains("INTERVAL '30 days'"));
        assert!(SUPABASE_PRUNE_SQL.contains("262144000")); // 250MB limit in bytes
    }
}
