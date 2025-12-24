-- 20251212184418_add_global_identity_to_users.sql

-- 1. Criação do Enum de Tipos de Documento
-- TAX_ID = CPF/CNPJ (Brasil), NIF (Portugal), SSN (EUA)
-- ID_CARD = RG (Brasil), Cartão Cidadão
CREATE TYPE document_type AS ENUM ('TAX_ID', 'ID_CARD', 'PASSPORT', 'DRIVER_LICENSE', 'OTHER');

-- 2. Alteração na tabela users
ALTER TABLE users
    -- Padrão ISO 3166-1 alpha-2 (BR, US, PT). Default BR para facilitar migração.
    ADD COLUMN country_code CHAR(2) NOT NULL DEFAULT 'BR',

    -- Tipo do documento
    ADD COLUMN document_type document_type NOT NULL DEFAULT 'TAX_ID',

    -- O número. Pode ser NULL se o usuário se cadastrar só com email inicialmente.
    ADD COLUMN document_number VARCHAR(50);

-- 3. Índice Único Global
-- Garante que não exista o mesmo CPF/Passaporte cadastrado duas vezes no sistema inteiro.
CREATE UNIQUE INDEX idx_users_global_identity
    ON users (country_code, document_type, document_number)
    WHERE document_number IS NOT NULL;
