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
    // Usa a macro `query_as!` para segurança em tempo de compilação
    // e o operador `?` para tratamento de erro idiomático.
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let maybe_user = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE email = $1",
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(maybe_user)
    }

    // Busca um usuário pelo seu ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AppError> {
        let maybe_user = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(maybe_user)
    }

    // Cria um novo usuário no banco de dados
    // Com tratamento de erro específico para e-mails duplicados.
    // Assumi que sua coluna de senha se chama 'password_hash'. Ajuste se necessário.
    pub async fn create_user(&self, email: &str, password_hash: &str) -> Result<User, AppError> {
        let result = sqlx::query_as!(
            User,
            "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING *",
            email,
            password_hash
        )
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(user) => Ok(user),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                // Se o erro for de violação de chave única, sabemos que é o e-mail.
                Err(AppError::EmailAlreadyExists)
            }
            Err(e) => {
                // Para todos os outros erros, usamos a conversão automática de 'thiserror'.
                Err(e.into())
            }
        }
    }
}