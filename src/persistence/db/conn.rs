use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Schema, Statement};
use std::time::Duration;
use tokio::sync::OnceCell;

use crate::error::ClewdrError;

use super::entities::{
    ColumnCookie, ColumnKeyRow, EntityConfig, EntityCookie, EntityKeyRow, EntityWasted,
};

static CONN: OnceCell<DatabaseConnection> = OnceCell::const_new();

pub async fn ensure_conn() -> Result<DatabaseConnection, ClewdrError> {
    if !crate::config::CLEWDR_CONFIG.load().is_db_mode() {
        return Err(ClewdrError::Whatever {
            message: "DB mode not enabled".into(),
            source: None,
        });
    }
    let db = CONN
        .get_or_try_init(|| async {
            let cfg = crate::config::CLEWDR_CONFIG.load();
            let url = cfg.database_url().ok_or(ClewdrError::UnexpectedNone {
                msg: "Database URL not provided",
            })?;
            if url.starts_with("sqlite://")
                && !cfg.no_fs
                && let Some(parent) = std::path::Path::new(&url["sqlite://".len()..]).parent()
            {
                let _ = std::fs::create_dir_all(parent);
            }
            // configure connection pool and timeouts for stability
            let mut opt = ConnectOptions::new(url);
            let max_connections = 10u32;
            let min_connections = 1u32;
            let connect_timeout = Duration::from_secs(8);
            let acquire_timeout = Duration::from_secs(8);
            let idle_timeout = Duration::from_secs(300);
            opt.max_connections(max_connections)
                .min_connections(min_connections)
                .connect_timeout(connect_timeout)
                .acquire_timeout(acquire_timeout)
                .idle_timeout(idle_timeout)
                .sqlx_logging(false);

            // expose configured pool to metrics
            super::metrics::set_conn_config(super::metrics::ConnConfigMetrics {
                max_connections,
                min_connections,
                connect_timeout_ms: connect_timeout.as_millis() as u64,
                acquire_timeout_ms: acquire_timeout.as_millis() as u64,
                idle_timeout_ms: idle_timeout.as_millis() as u64,
            });

            let db = Database::connect(opt)
                .await
                .map_err(|e| {
                    super::metrics::mark_connect_err();
                    ClewdrError::Whatever {
                        message: "db_connect".into(),
                        source: Some(Box::new(e)),
                    }
                })?;
            super::metrics::mark_connect_ok();
            migrate(&db).await?;
            Ok::<_, ClewdrError>(db)
        })
        .await?;
    Ok(db.clone())
}

async fn migrate(db: &DatabaseConnection) -> Result<(), ClewdrError> {
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);
    let stmt: sea_orm::sea_query::TableCreateStatement =
        schema.create_table_from_entity(EntityConfig);
    db.execute(backend.build(&stmt)).await.ok();
    let stmt = schema.create_table_from_entity(EntityCookie);
    db.execute(backend.build(&stmt)).await.ok();
    let stmt = schema.create_table_from_entity(EntityWasted);
    db.execute(backend.build(&stmt)).await.ok();
    // attempt to rename legacy table names to new ones (best effort; ignore errors)
    use sea_orm::DatabaseBackend;
    let rename_stmt = match db.get_database_backend() {
        DatabaseBackend::Postgres => Some("ALTER TABLE IF EXISTS \"keys\" RENAME TO \"api_keys\"".to_string()),
        DatabaseBackend::MySql => Some("RENAME TABLE `keys` TO `api_keys`".to_string()),
        DatabaseBackend::Sqlite => Some("ALTER TABLE IF EXISTS 'keys' RENAME TO 'api_keys'".to_string()),
    };
    if let Some(sql) = rename_stmt {
        let backend = db.get_database_backend();
        let _ = db.execute(Statement::from_string(backend, sql)).await;
    }
    // ensure new table exists (idempotent create)
    let stmt = schema.create_table_from_entity(EntityKeyRow);
    db.execute(backend.build(&stmt)).await.ok();

    // indexes
    use sea_orm::sea_query::{ColumnDef, Index, TableAlterStatement};
    // cookies(token_org_uuid)
    let idx = Index::create()
        .name("idx_cookies_org_uuid")
        .table(EntityCookie)
        .col(ColumnCookie::TokenOrgUuid)
        .to_owned();
    db.execute(backend.build(&idx)).await.ok();
    // cookies(reset_time)
    let idx = Index::create()
        .name("idx_cookies_reset")
        .table(EntityCookie)
        .col(ColumnCookie::ResetTime)
        .to_owned();
    db.execute(backend.build(&idx)).await.ok();
    // keys(count_403)
    let idx = Index::create()
        .name("idx_keys_count")
        .table(EntityKeyRow)
        .col(ColumnKeyRow::Count403)
        .to_owned();
    db.execute(backend.build(&idx)).await.ok();

    // Ensure supports_claude_1m and usage/period boundary columns exist on cookies table
    let alter = TableAlterStatement::new()
        .table(EntityCookie)
        .add_column(
            ColumnDef::new(ColumnCookie::SupportsClaude1m)
                .boolean()
                .null(),
        )
        .add_column(
            ColumnDef::new(ColumnCookie::CountTokensAllowed)
                .boolean()
                .null(),
        )
        .add_column(
            ColumnDef::new(ColumnCookie::TotalInputTokens)
                .big_integer()
                .null(),
        )
        .add_column(
            ColumnDef::new(ColumnCookie::TotalOutputTokens)
                .big_integer()
                .null(),
        )
        .add_column(
            ColumnDef::new(ColumnCookie::WindowInputTokens)
                .big_integer()
                .null(),
        )
        .add_column(
            ColumnDef::new(ColumnCookie::WindowOutputTokens)
                .big_integer()
                .null(),
        )
        .add_column(
            ColumnDef::new(ColumnCookie::LifetimeUsage)
                .string()
                .null(),
        )
        .add_column(
            ColumnDef::new(ColumnCookie::SessionUsage)
                .string()
                .null(),
        )
        .add_column(
            ColumnDef::new(ColumnCookie::WeeklyUsage)
                .string()
                .null(),
        )
        .add_column(
            ColumnDef::new(ColumnCookie::WeeklyOpusUsage)
                .string()
                .null(),
        )
        .add_column(
            ColumnDef::new(ColumnCookie::SessionResetsAt)
                .big_integer()
                .null(),
        )
        .add_column(
            ColumnDef::new(ColumnCookie::WeeklyResetsAt)
                .big_integer()
                .null(),
        )
        .add_column(
            ColumnDef::new(ColumnCookie::WeeklyOpusResetsAt)
                .big_integer()
                .null(),
        )
        .add_column(
            ColumnDef::new(ColumnCookie::ResetsLastCheckedAt)
                .big_integer()
                .null(),
        )
        .add_column(
            ColumnDef::new(ColumnCookie::SessionHasReset)
                .boolean()
                .null(),
        )
        .add_column(
            ColumnDef::new(ColumnCookie::WeeklyHasReset)
                .boolean()
                .null(),
        )
        .add_column(
            ColumnDef::new(ColumnCookie::WeeklyOpusHasReset)
                .boolean()
                .null(),
        )
        .to_owned();
    db.execute(backend.build(&alter)).await.ok();
    Ok(())
}
