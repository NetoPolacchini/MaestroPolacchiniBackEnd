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
        // 1. Hashing (Isso pode ficar fora da transação, pois não toca no banco)
        let password_clone = password.to_owned();
        let hashed_password = tokio::task::spawn_blocking(move || {
            hash(&password_clone, bcrypt::DEFAULT_COST)
        })
            .await
            .map_err(|e| anyhow::anyhow!("Falha na task de hashing: {}", e))?
            ?;

        // --- INÍCIO DA TRANSAÇÃO ---
        // Iniciamos uma transação no pool de conexões
        let mut tx = self.pool.begin().await?;

        // Note que passamos `&mut *tx` (o executor) em vez de `&self.pool`

        // 2. Cria Usuário (Passando a transação)
        let new_user = self.user_repo
            .create_user(
                &mut *tx, // <--- AQUI A MÁGICA
                &email,
                &hashed_password,
                country_code.as_deref(),
                document_type.clone(),
                document_number.as_deref()
            )
            .await?; // Se falhar aqui, o tx sofre rollback automático ao sair do escopo (drop)

        // 3. Link no CRM (Passando a mesma transação)
        if let (Some(cc), Some(dt), Some(dn)) = (&country_code, &document_type, &document_number) {
            let count = self.crm_repo
                .link_user_to_existing_customers(
                    &mut *tx, // <--- AQUI TAMBÉM
                    new_user.id,
                    cc,
                    dt.clone(),
                    dn
                )
                .await?; // Se falhar aqui, o usuário criado acima é desfeito!

            if count > 0 {
                tracing::info!("🔗 Usuário vinculado a {} clientes na transação.", count);
            }
        }

        // 4. Se chegou aqui, deu tudo certo. "Commita" a transação.
        tx.commit().await?;
        // --- FIM DA TRANSAÇÃO ---

        // 5. Gera o token (Isso não precisa de transação de banco)
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