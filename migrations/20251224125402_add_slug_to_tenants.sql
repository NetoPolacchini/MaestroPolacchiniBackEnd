-- 20251224125402_add_slug_to_tenants.sql

-- 1. Adiciona a coluna com um valor padrão temporário (para não falhar em registros existentes)
ALTER TABLE tenants
    ADD COLUMN slug VARCHAR(255) NOT NULL DEFAULT 'store-' || gen_random_uuid();

-- 2. Remove o valor padrão (daqui pra frente, quem criar loja tem que mandar o slug ou geramos no backend)
ALTER TABLE tenants
    ALTER COLUMN slug DROP DEFAULT;

-- 3. Garante que o slug seja único no sistema (dois tenants não podem ter o mesmo link)
CREATE UNIQUE INDEX idx_tenants_slug ON tenants(slug);