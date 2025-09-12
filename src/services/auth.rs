use bcrypt::{hash, verify};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

use crate::{
    common::error::AppError,
    config::AppState,
    db::UserRepository,
    models::auth::{Claims, User},
};

// O serviço de autenticação, que orquestra a lógica de negócio
#[derive(Clone)]
pub struct AuthService {
    user_repo: UserRepository,
    jwt_secret: String,
}

impl AuthService {
    pub fn new(app_state: AppState) -> Self {
        Self {
            user_repo: UserRepository::new(app_state.db_pool),
            jwt_secret: app_state.jwt_secret,
        }
    }

    // Registra um novo usuário
    pub async fn register_user(&self, email: &str, password: &str) -> Result<String, AppError> {
        let hashed_password = hash(password, bcrypt::DEFAULT_COST)
            .map_err(|e| AppError::InternalServerError(format!("Falha ao hashear senha: {}", e)))?;

        let new_user = self.user_repo.create_user(email, &hashed_password).await?;
        self.create_token(new_user.id)
    }

    // Faz o login de um usuário
    pub async fn login_user(&self, email: &str, password: &str) -> Result<String, AppError> {
        let user = self.user_repo
            .find_by_email(email)
            .await?
            .ok_or(AppError::InvalidCredentials)?;

        let is_password_valid = verify(password, &user.hashed_password)
            .map_err(|e| AppError::InternalServerError(format!("Erro ao verificar senha: {}", e)))?;

        if !is_password_valid {
            return Err(AppError::InvalidCredentials);
        }
        
        self.create_token(user.id)
    }

    // Valida um token e retorna o usuário correspondente
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

    // Função auxiliar para criar um token JWT
    fn create_token(&self, user_id: Uuid) -> Result<String, AppError> {
        let now = chrono::Utc::now();
        let expires_at = now + chrono::Duration::days(7);

        let claims = Claims {
            sub: user_id,
            exp: expires_at.timestamp() as usize,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )
        .map_err(|e| AppError::InternalServerError(format!("Falha ao criar token: {}", e)))
    }
}