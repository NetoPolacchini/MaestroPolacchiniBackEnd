pub mod user_repo;
pub mod inventory_repo;
pub mod tenancy_repo;
pub mod crm_repo;
pub mod rbac_repo;
mod operations_repo;

pub use user_repo::UserRepository;
pub use inventory_repo::InventoryRepository;
pub use tenancy_repo::TenantRepository;
pub use crm_repo::CrmRepository;
pub use rbac_repo::RbacRepository;