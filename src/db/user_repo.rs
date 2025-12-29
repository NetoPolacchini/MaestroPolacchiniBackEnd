// src/user_repo.rs

use sqlx::{PgPool, Postgres, Executor};
use crate::{common::error::AppError, models::auth::User};
use crate::models::auth::{DocumentType};

// O repositório de usuários, responsável por todas as interações com a tabela 'users'
#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // Busca um usuário pelo seu e-mail
    // Usa a macro `query_as!` para segurança em tempo de compilação
    // e o operador `?` para tratamento de erro idiomático.
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let maybe_user = sqlx::query_as!(
            User,
            r#"
            SELECT
                id, email, password_hash, created_at, updated_at,
                country_code,
                -- CAST EXPLÍCITO AQUI:
                document_type as "document_type: DocumentType",
                document_number
            FROM users
            WHERE email = $1
            "#,
            email
        )
            .fetch_optional(&self.pool)
            .await?;
        Ok(maybe_user)
    }

    // Busca um usuário pelo seu ID
    pub async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<User>, AppError> {
        let maybe_user = sqlx::query_as!(
            User,
            r#"
            SELECT
                id, email, password_hash, created_at, updated_at,
                country_code,
                -- CAST EXPLÍCITO AQUI:
                document_type as "document_type: DocumentType",
                document_number
            FROM users
            WHERE id = $1
            "#,
            id
        )
            .fetch_optional(&self.pool)
            .await?;
        Ok(maybe_user)
    }

    // Cria um novo usuário no banco de dados
    // Com tratamento de erro específico para e-mails duplicados.
    // Assumi que sua coluna de senha se chama 'password_hash'. Ajuste se necessário.
    pub async fn create_user<'e, E>(
        &self,
        executor: E,
        email: &str,
        password_hash: &str,
        // Novos Argumentos
        country_code: Option<&str>,
        document_type: Option<DocumentType>,
        document_number: Option<&str>,
    ) -> Result<User, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Definindo padrões caso venha nulo
        let final_country = country_code.unwrap_or("BR");
        let final_type = document_type.unwrap_or(DocumentType::TaxId);

        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (
                email, password_hash,
                country_code, document_type, document_number
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id, email, password_hash, created_at, updated_at,
                country_code,
                document_type as "document_type: DocumentType",
                document_number
            "#,
            email,
            password_hash,
            final_country,
            final_type as DocumentType,
            document_number
        )
            .fetch_one(executor)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        if let Some(constraint) = db_err.constraint() {
                            return match constraint {
                                // O nome padrão que o Postgres cria para "UNIQUE" na coluna email
                                "users_email_key" => AppError::EmailAlreadyExists,

                                // O nome do índice que você criou na migration
                                "idx_users_global_identity" => AppError::DocumentAlreadyExists,

                                // Fallback (caso adicione outras chaves únicas no futuro)
                                _ => AppError::UniqueConstraintViolation(constraint.to_string()),
                            }
                        }
                    }
                }
                e.into()
            })?;

        Ok(user)
    }
}