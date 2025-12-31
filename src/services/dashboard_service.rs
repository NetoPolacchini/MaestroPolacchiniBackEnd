// src/services/dashboard_service.rs

use sqlx::{Postgres, Executor, Acquire};
use uuid::Uuid;
use crate::{
    common::error::AppError,
    db::DashboardRepository,
    models::dashboard::{DashboardSummary, SalesChartEntry, TopProductEntry},
};

#[derive(Clone)]
pub struct DashboardService {
    repo: DashboardRepository,
}

impl DashboardService {
    pub fn new(repo: DashboardRepository) -> Self {
        Self { repo }
    }

    pub async fn get_summary<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<DashboardSummary, AppError>
    where
        E: Executor<'e, Database = Postgres> + Acquire<'e, Database = Postgres>,
    {
        self.repo.get_summary(executor, tenant_id).await
    }

    pub async fn get_sales_chart<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<SalesChartEntry>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.repo.get_sales_last_30_days(executor, tenant_id).await
    }

    pub async fn get_top_products<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Vec<TopProductEntry>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        self.repo.get_top_products(executor, tenant_id).await
    }
}