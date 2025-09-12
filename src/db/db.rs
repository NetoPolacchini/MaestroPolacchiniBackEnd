use sqlx::PgPool;
use uuid::Uuid;
use crate::{common::error::AppError, models::auth::User};

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
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::InternalServerError(e.to_string()))
    }

    // Busca um usuário pelo seu ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AppError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::InternalServerError(e.to_string()))
    }

    // Cria um novo usuário no banco de dados
    pub async fn create_user(&self, email: &str, hashed_password: &str) -> Result<User, AppError> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (email, hashed_password) VALUES ($1, $2) RETURNING *",
        )
        .bind(email)
        .bind(hashed_password)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            // Converte erro de violação de chave única em um erro mais amigável
            if let Some(db_err) = e.as_database_error() {
                if db_err.is_unique_violation() {
                    return AppError::EmailAlreadyExists;
                }
            }
            AppError::InternalServerError(e.to_string())
        })
    }
}