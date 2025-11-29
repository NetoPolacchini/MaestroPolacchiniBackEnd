-- Add migration script here

ALTER TYPE stock_movement_reason ADD VALUE IF NOT EXISTS 'PURCHASE';
