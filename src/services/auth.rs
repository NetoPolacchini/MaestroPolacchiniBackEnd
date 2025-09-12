// src/services/auth.rs

use bcrypt::{hash, verify};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

use crate::{
    common::error::AppError,
    db::UserRepository,
    models::auth::{Claims, User},
};

#[derive(Clone)]
pub struct AuthService {
    user_repo: UserRepository,
    jwt_secret: String,
}

impl AuthService {
    // ASSINATURA CORRIGIDA:
    // Agora aceita as dependências explícitas, em vez do AppState inteiro.
    pub fn new(user_repo: UserRepository, jwt_secret: String) -> Self {
        Self { user_repo, jwt_secret }
    }

    pub async fn register_user(&self, email: &str, password: &str) -> Result<String, AppError> {
        let password_clone = password.to_owned();

        // Executa o hash em um thread separado para não bloquear o servidor
        let hashed_password = tokio::task::spawn_blocking(move || {
            hash(&password_clone, bcrypt::DEFAULT_COST)
        })
        .await
        .map_err(|e| anyhow::anyhow!("Falha na task de hashing: {}", e))? // Erro da task
        ?; // Erro do bcrypt, convertido automaticamente para AppError

        let new_user = self.user_repo.create_user(email, &hashed_password).await?;
        self.create_token(new_user.id)
    }

    pub async fn login_user(&self, email: &str, password: &str) -> Result<String, AppError> {
        let user = self.user_repo
            .find_by_email(email)
            .await?
            .ok_or(AppError::InvalidCredentials)?;

        let password_clone = password.to_owned();
        let password_hash_clone = user.password_hash.clone();

        // Executa a verificação em um thread separado
        let is_password_valid = tokio::task::spawn_blocking(move || {
            verify(&password_clone, &password_hash_clone)
        })
        .await
        .map_err(|e| anyhow::anyhow!("Falha na task de verificação de senha: {}", e))?
        ?;

        if !is_password_valid {
            return Err(AppError::InvalidCredentials);
        }
        
        self.create_token(user.id)
    }

    pub async fn validate_token(&self, token: &str) -> Result<User, AppError> {
        let validation = Validation::default();
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &validation,
        )
        .map_err(|_| AppError::InvalidToken)?;
        
        self.user_repo
            .find_by_id(token_data.claims.sub)
            .await?
            .ok_or(AppError::UserNotFound)
    }

    fn create_token(&self, user_id: Uuid) -> Result<String, AppError> {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::days(7);

        let claims = Claims {
            sub: user_id,
            exp: expires_at.timestamp() as usize,
            iat: now.timestamp() as usize,
        };

        // Usa '?' para um tratamento de erro mais limpo
        Ok(encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )?)
    }
}