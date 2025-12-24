// src/services/login

use bcrypt::{hash, verify};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;
use crate::models::auth::DocumentType;
use sqlx::PgPool; // <--- Importe o PgPool

use crate::{
    common::error::AppError,
    db::{UserRepository, CrmRepository},
    models::auth::{Claims, User},
};

#[derive(Clone)]
pub struct AuthService {
    user_repo: UserRepository,
    crm_repo: CrmRepository,
    jwt_secret: String,
    pool: PgPool,
}

impl AuthService {
    pub fn new(
        user_repo: UserRepository,
        crm_repo: CrmRepository, // <--- Novo argumento
        jwt_secret: String,
        pool: PgPool
    ) -> Self {
        Self { user_repo, crm_repo, jwt_secret, pool }
    }

    pub async fn register_user(
        &self,
        email: &str,
        password: &str,
        country_code: Option<String>,
        document_type: Option<DocumentType>,
        document_number: Option<String>,
    ) -> Result<String, AppError> {
        let password_clone = password.to_owned();

        // Executa o hash em um thread separado para não bloquear o servidor
        let hashed_password = tokio::task::spawn_blocking(move || {
            hash(&password_clone, bcrypt::DEFAULT_COST)
        })
        .await
        .map_err(|e| anyhow::anyhow!("Falha na task de hashing: {}", e))? // Erro da task
        ?; // Erro do bcrypt, convertido automaticamente para AppError

        let new_user = self.user_repo
            .create_user(
                &self.pool,
                &email,
                &hashed_password, // <--- 2. CORREÇÃO: O nome da variável correta é hashed_password
                country_code.as_deref(),
                document_type.clone(),
                document_number.as_deref()
            )
            .await?;

        // 2. O LINK MÁGICO 
        // Se o usuário informou documentos, tentamos vincular
        if let (Some(cc), Some(dt), Some(dn)) = (&country_code, &document_type, &document_number) {
            let count = self.crm_repo
                .link_user_to_existing_customers(
                    &self.pool,
                    new_user.id,
                    cc,
                    dt.clone(),
                    dn
                )
                .await?;

            if count > 0 {
                tracing::info!("🔗 Usuário {} vinculado a {} registros de cliente existentes!", new_user.id, count);
            }
        }

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