-- Garante que a extensão pgcrypto, que provê gen_random_uuid(), esteja disponível
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    -- CORREÇÃO: Usar TEXT para garantir que o hash da senha sempre caiba.
    hashed_password TEXT NOT NULL,
    created_at TIMESTPTZ NOT NULL DEFAULT NOW()
);