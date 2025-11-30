-- Simple DNS domain management schema
CREATE TABLE IF NOT EXISTS domains (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    domain VARCHAR(255) UNIQUE NOT NULL,
    ip_address INET NOT NULL,
    mail_server VARCHAR(255) DEFAULT 'mail.{domain}',
    mx_priority INTEGER DEFAULT 10,
    enabled BOOLEAN DEFAULT true,
    verified BOOLEAN DEFAULT false,
    last_verified TIMESTAMP WITH TIME ZONE,
    nameservers TEXT[], -- Array of nameserver strings
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    -- Optional metadata (can be NULL)
    discord BOOLEAN DEFAULT false,
    description TEXT,
    tags TEXT[]
);

-- Index for faster lookups
CREATE INDEX IF NOT EXISTS idx_domains_domain ON domains(domain);
CREATE INDEX IF NOT EXISTS idx_domains_enabled ON domains(enabled);
CREATE INDEX IF NOT EXISTS idx_domains_verified ON domains(verified);

-- DNS records table for additional records
CREATE TABLE IF NOT EXISTS dns_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    domain_id UUID REFERENCES domains(id) ON DELETE CASCADE,
    record_type VARCHAR(10) NOT NULL, -- 'A', 'MX', 'TXT', 'CNAME', etc.
    name VARCHAR(255) NOT NULL, -- Record name (e.g., 'www', 'mail', '@')
    value TEXT NOT NULL, -- Record value
    ttl INTEGER DEFAULT 300,
    priority INTEGER DEFAULT 0, -- For MX records
    enabled BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_dns_records_domain_id ON dns_records(domain_id);
CREATE INDEX IF NOT EXISTS idx_dns_records_type ON dns_records(record_type);
CREATE INDEX IF NOT EXISTS idx_dns_records_name ON dns_records(name);

-- Simple function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Triggers to automatically update updated_at
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'update_domains_updated_at') THEN
        CREATE TRIGGER update_domains_updated_at 
            BEFORE UPDATE ON domains 
            FOR EACH ROW 
            EXECUTE FUNCTION update_updated_at_column();
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'update_dns_records_updated_at') THEN
        CREATE TRIGGER update_dns_records_updated_at 
            BEFORE UPDATE ON dns_records 
            FOR EACH ROW 
            EXECUTE FUNCTION update_updated_at_column();
    END IF;
END $$;