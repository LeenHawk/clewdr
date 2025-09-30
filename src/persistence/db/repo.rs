use serde_json::json;
use tracing::error;

use sea_orm::{ActiveValue::Set, entity::prelude::*, TransactionTrait, QueryFilter, ColumnTrait};

use crate::config::{ClewdrConfig, CookieStatus, KeyStatus, UselessCookie, UsageBreakdown};
use crate::error::ClewdrError;

use super::{conn::ensure_conn, entities::*, metrics::*};

pub async fn bootstrap_from_db_if_enabled() -> Result<(), ClewdrError> {
    if !crate::config::CLEWDR_CONFIG.load().is_db_mode() {
        return Ok(());
    }
    let db = ensure_conn().await?;
    if let Ok(Some(row)) = EntityConfig::find_by_id("main").one(&db).await {
        match toml::from_str::<ClewdrConfig>(&row.data) {
            Ok(mut cfg) => {
                cfg = cfg.validate();
                crate::config::CLEWDR_CONFIG.store(std::sync::Arc::new(cfg));
            }
            Err(e) => {
                error!("Failed to parse config from DB: {}", e);
            }
        }
    }
    Ok(())
}

pub async fn persist_config(config: &ClewdrConfig) -> Result<(), ClewdrError> {
    if !crate::config::CLEWDR_CONFIG.load().is_db_mode() {
        return Ok(());
    }
    let db = ensure_conn().await?;
    let data = toml::to_string_pretty(config)?;
    use sea_orm::sea_query::OnConflict;
    let am = ActiveModelConfig {
        k: Set("main".to_string()),
        data: Set(data),
        updated_at: Set(Some(chrono::Utc::now().timestamp())),
    };
    let start = std::time::Instant::now();
    let res = EntityConfig::insert(am)
        .on_conflict(
            OnConflict::column(ColumnConfig::K)
                .update_columns([ColumnConfig::Data, ColumnConfig::UpdatedAt])
                .to_owned(),
        )
        .exec(&db)
        .await;
    match res {
        Ok(_) => {
            record_duration(start);
            mark_write_ok();
        }
        Err(e) => {
            record_error_msg(&e);
            mark_write_err();
            return Err(ClewdrError::Whatever {
                message: "save_config".into(),
                source: Some(Box::new(e)),
            });
        }
    }
    Ok(())
}

pub async fn persist_cookie_upsert(c: &CookieStatus) -> Result<(), ClewdrError> {
    if !crate::config::CLEWDR_CONFIG.load().is_db_mode() {
        return Ok(());
    }
    let db = ensure_conn().await?;
    use sea_orm::sea_query::OnConflict;
    let (acc, rtk, exp_at, exp_in, org) = if let Some(t) = &c.token {
        (
            Some(t.access_token.clone()),
            Some(t.refresh_token.clone()),
            Some(t.expires_at.timestamp()),
            Some(t.expires_in.as_secs() as i64),
            Some(t.organization.uuid.clone()),
        )
    } else {
        (None, None, None, None, None)
    };
    let am = ActiveModelCookie {
        cookie: Set(c.cookie.to_string()),
        reset_time: Set(c.reset_time),
        token_access: Set(acc),
        token_refresh: Set(rtk),
        token_expires_at: Set(exp_at),
        token_expires_in: Set(exp_in),
        token_org_uuid: Set(org),
        supports_claude_1m: Set(c.supports_claude_1m),
        count_tokens_allowed: Set(c.count_tokens_allowed),
        total_input_tokens: Set(None),
        total_output_tokens: Set(None),
        window_input_tokens: Set(None),
        window_output_tokens: Set(None),
        session_usage: Set(Some(serde_json::to_string(&c.session_usage).unwrap_or_else(|_| "{}".to_string()))),
        weekly_usage: Set(Some(serde_json::to_string(&c.weekly_usage).unwrap_or_else(|_| "{}".to_string()))),
        weekly_opus_usage: Set(Some(serde_json::to_string(&c.weekly_opus_usage).unwrap_or_else(|_| "{}".to_string()))),
        lifetime_usage: Set(Some(serde_json::to_string(&c.lifetime_usage).unwrap_or_else(|_| "{}".to_string()))),
        session_resets_at: Set(c.session_resets_at),
        weekly_resets_at: Set(c.weekly_resets_at),
        weekly_opus_resets_at: Set(c.weekly_opus_resets_at),
        resets_last_checked_at: Set(c.resets_last_checked_at),
        session_has_reset: Set(c.session_has_reset),
        weekly_has_reset: Set(c.weekly_has_reset),
        weekly_opus_has_reset: Set(c.weekly_opus_has_reset),
    };
    let start = std::time::Instant::now();
    let res = EntityCookie::insert(am)
        .on_conflict(
            OnConflict::column(ColumnCookie::Cookie)
                .update_columns([
                    ColumnCookie::ResetTime,
                    ColumnCookie::TokenAccess,
                    ColumnCookie::TokenRefresh,
                    ColumnCookie::TokenExpiresAt,
                    ColumnCookie::TokenExpiresIn,
                    ColumnCookie::TokenOrgUuid,
                    ColumnCookie::SupportsClaude1m,
                    ColumnCookie::CountTokensAllowed,
                    ColumnCookie::SessionUsage,
                    ColumnCookie::WeeklyUsage,
                    ColumnCookie::WeeklyOpusUsage,
                    ColumnCookie::LifetimeUsage,
                    ColumnCookie::SessionResetsAt,
                    ColumnCookie::WeeklyResetsAt,
                    ColumnCookie::WeeklyOpusResetsAt,
                    ColumnCookie::ResetsLastCheckedAt,
                    ColumnCookie::SessionHasReset,
                    ColumnCookie::WeeklyHasReset,
                    ColumnCookie::WeeklyOpusHasReset,
                ])
                .to_owned(),
        )
        .exec(&db)
        .await;
    match res {
        Ok(_) => {
            record_duration(start);
            mark_write_ok();
        }
        Err(e) => {
            record_error_msg(&e);
            mark_write_err();
            return Err(ClewdrError::Whatever {
                message: "upsert_cookie".into(),
                source: Some(Box::new(e)),
            });
        }
    }
    Ok(())
}

pub async fn delete_cookie_row(c: &CookieStatus) -> Result<(), ClewdrError> {
    if !crate::config::CLEWDR_CONFIG.load().is_db_mode() {
        return Ok(());
    }
    let db = ensure_conn().await?;
    let start = std::time::Instant::now();
    let res = EntityCookie::delete_by_id(c.cookie.to_string())
        .exec(&db)
        .await;
    match res {
        Ok(_) => {
            record_duration(start);
            mark_write_ok();
        }
        Err(e) => {
            record_error_msg(&e);
            mark_write_err();
            return Err(ClewdrError::Whatever {
                message: "delete_cookie".into(),
                source: Some(Box::new(e)),
            });
        }
    }
    EntityWasted::delete_by_id(c.cookie.to_string())
        .exec(&db)
        .await
        .ok();
    Ok(())
}

pub async fn persist_wasted_upsert(u: &UselessCookie) -> Result<(), ClewdrError> {
    if !crate::config::CLEWDR_CONFIG.load().is_db_mode() {
        return Ok(());
    }
    let db = ensure_conn().await?;
    use sea_orm::sea_query::OnConflict;
    let am = ActiveModelWasted {
        cookie: Set(u.cookie.to_string()),
        reason: Set(serde_json::to_string(&u.reason).unwrap_or_else(|_| "\"Unknown\"".to_string())),
    };
    let start = std::time::Instant::now();
    let res = EntityWasted::insert(am)
        .on_conflict(
            OnConflict::column(ColumnWasted::Cookie)
                .update_columns([ColumnWasted::Reason])
                .to_owned(),
        )
        .exec(&db)
        .await;
    match res {
        Ok(_) => {
            record_duration(start);
            mark_write_ok();
        }
        Err(e) => {
            record_error_msg(&e);
            mark_write_err();
            return Err(ClewdrError::Whatever {
                message: "upsert_wasted".into(),
                source: Some(Box::new(e)),
            });
        }
    }
    Ok(())
}

pub async fn persist_keys(keys: &[KeyStatus]) -> Result<(), ClewdrError> {
    if !crate::config::CLEWDR_CONFIG.load().is_db_mode() {
        return Ok(());
    }
    use sea_orm::sea_query::OnConflict;

    let db = ensure_conn().await?;
    let start_all = std::time::Instant::now();
    let txn = db
        .begin()
        .await
        .map_err(|e| ClewdrError::Whatever { message: "txn_begin".into(), source: Some(Box::new(e)) })?;
    super::metrics::mark_tx_begin();

    // Upsert all provided keys
    let mut keep: Vec<String> = Vec::with_capacity(keys.len());
    for k in keys {
        keep.push(k.key.to_string());
        let am = ActiveModelKeyRow {
            key: Set(k.key.to_string()),
            count_403: Set(k.count_403 as i64),
        };
        if let Err(e) = EntityKeyRow::insert(am)
            .on_conflict(
                OnConflict::column(ColumnKeyRow::Key)
                    .update_columns([ColumnKeyRow::Count403])
                    .to_owned(),
            )
            .exec(&txn)
            .await
        {
            record_error_msg(&e);
            mark_write_err();
            return Err(ClewdrError::Whatever { message: "upsert_key".into(), source: Some(Box::new(e)) });
        }
    }

    // Delete extras not present in keep list
    if keep.is_empty() {
        if let Err(e) = EntityKeyRow::delete_many().exec(&txn).await {
            record_error_msg(&e);
            mark_write_err();
            return Err(ClewdrError::Whatever { message: "delete_extra_keys_all".into(), source: Some(Box::new(e)) });
        }
    } else if let Err(e) = EntityKeyRow::delete_many()
        .filter(ColumnKeyRow::Key.is_not_in(keep))
        .exec(&txn)
        .await
    {
        record_error_msg(&e);
        mark_write_err();
        return Err(ClewdrError::Whatever { message: "delete_extra_keys".into(), source: Some(Box::new(e)) });
    }

    txn
        .commit()
        .await
        .map_err(|e| ClewdrError::Whatever { message: "txn_commit".into(), source: Some(Box::new(e)) })?;
    super::metrics::mark_tx_commit();

    record_duration(start_all);
    mark_write_ok();
    Ok(())
}

pub async fn persist_key_upsert(k: &KeyStatus) -> Result<(), ClewdrError> {
    if !crate::config::CLEWDR_CONFIG.load().is_db_mode() {
        return Ok(());
    }
    let db = ensure_conn().await?;
    use sea_orm::sea_query::OnConflict;
    let am = ActiveModelKeyRow {
        key: Set(k.key.to_string()),
        count_403: Set(k.count_403 as i64),
    };
    let start = std::time::Instant::now();
    let res = EntityKeyRow::insert(am)
        .on_conflict(
            OnConflict::column(ColumnKeyRow::Key)
                .update_columns([ColumnKeyRow::Count403])
                .to_owned(),
        )
        .exec(&db)
        .await;
    match res {
        Ok(_) => {
            record_duration(start);
            mark_write_ok();
        }
        Err(e) => {
            record_error_msg(&e);
            mark_write_err();
            return Err(ClewdrError::Whatever {
                message: "upsert_key".into(),
                source: Some(Box::new(e)),
            });
        }
    }
    Ok(())
}

pub async fn delete_key_row(k: &KeyStatus) -> Result<(), ClewdrError> {
    if !crate::config::CLEWDR_CONFIG.load().is_db_mode() {
        return Ok(());
    }
    let db = ensure_conn().await?;
    let start = std::time::Instant::now();
    let res = EntityKeyRow::delete_by_id(k.key.to_string())
        .exec(&db)
        .await;
    match res {
        Ok(_) => {
            record_duration(start);
            mark_write_ok();
        }
        Err(e) => {
            record_error_msg(&e);
            mark_write_err();
            return Err(ClewdrError::Whatever {
                message: "delete_key".into(),
                source: Some(Box::new(e)),
            });
        }
    }
    Ok(())
}

pub async fn import_config_from_file() -> Result<serde_json::Value, ClewdrError> {
    let text = tokio::fs::read_to_string(crate::config::CONFIG_PATH.as_path()).await?;
    let cfg: ClewdrConfig = toml::from_str(&text)?;

    let db = ensure_conn().await?;
    let txn = db
        .begin()
        .await
        .map_err(|e| ClewdrError::Whatever { message: "txn_begin".into(), source: Some(Box::new(e)) })?;
    super::metrics::mark_tx_begin();

    // Upsert config row
    use sea_orm::sea_query::OnConflict;
    let data = toml::to_string_pretty(&cfg)?;
    let amc = ActiveModelConfig {
        k: Set("main".to_string()),
        data: Set(data),
        updated_at: Set(Some(chrono::Utc::now().timestamp())),
    };
    EntityConfig::insert(amc)
        .on_conflict(
            OnConflict::column(ColumnConfig::K)
                .update_columns([ColumnConfig::Data, ColumnConfig::UpdatedAt])
                .to_owned(),
        )
        .exec(&txn)
        .await
        .map_err(|e| ClewdrError::Whatever { message: "save_config".into(), source: Some(Box::new(e)) })?;

    // classify cookies
    let mut valid: Vec<CookieStatus> = Vec::new();
    let mut exhausted: Vec<CookieStatus> = Vec::new();
    for c in cfg.cookie_array.iter().cloned() {
        if c.reset_time.is_some() { exhausted.push(c) } else { valid.push(c) }
    }
    let invalid: Vec<UselessCookie> = cfg.wasted_cookie.iter().cloned().collect();

    // Upsert cookies and delete extras
    let mut keep_cookies: Vec<String> = Vec::with_capacity(valid.len() + exhausted.len());
    for c in valid.iter().chain(exhausted.iter()) {
        keep_cookies.push(c.cookie.to_string());
        let (acc, rtk, exp_at, exp_in, org) = if let Some(t) = &c.token {
            (
                Some(t.access_token.clone()),
                Some(t.refresh_token.clone()),
                Some(t.expires_at.timestamp()),
                Some(t.expires_in.as_secs() as i64),
                Some(t.organization.uuid.clone()),
            )
        } else { (None, None, None, None, None) };
        let am = ActiveModelCookie {
            cookie: Set(c.cookie.to_string()),
            reset_time: Set(c.reset_time),
            token_access: Set(acc),
            token_refresh: Set(rtk),
            token_expires_at: Set(exp_at),
            token_expires_in: Set(exp_in),
            token_org_uuid: Set(org),
            supports_claude_1m: Set(c.supports_claude_1m),
            count_tokens_allowed: Set(c.count_tokens_allowed),
            total_input_tokens: Set(None),
            total_output_tokens: Set(None),
            window_input_tokens: Set(None),
            window_output_tokens: Set(None),
            session_usage: Set(Some(serde_json::to_string(&c.session_usage).unwrap_or_else(|_| "{}".to_string()))),
            weekly_usage: Set(Some(serde_json::to_string(&c.weekly_usage).unwrap_or_else(|_| "{}".to_string()))),
            weekly_opus_usage: Set(Some(serde_json::to_string(&c.weekly_opus_usage).unwrap_or_else(|_| "{}".to_string()))),
            lifetime_usage: Set(Some(serde_json::to_string(&c.lifetime_usage).unwrap_or_else(|_| "{}".to_string()))),
            session_resets_at: Set(c.session_resets_at),
            weekly_resets_at: Set(c.weekly_resets_at),
            weekly_opus_resets_at: Set(c.weekly_opus_resets_at),
            resets_last_checked_at: Set(c.resets_last_checked_at),
            session_has_reset: Set(c.session_has_reset),
            weekly_has_reset: Set(c.weekly_has_reset),
            weekly_opus_has_reset: Set(c.weekly_opus_has_reset),
        };
        EntityCookie::insert(am)
            .on_conflict(
                OnConflict::column(ColumnCookie::Cookie)
                    .update_columns([
                        ColumnCookie::ResetTime,
                        ColumnCookie::TokenAccess,
                        ColumnCookie::TokenRefresh,
                        ColumnCookie::TokenExpiresAt,
                        ColumnCookie::TokenExpiresIn,
                        ColumnCookie::TokenOrgUuid,
                        ColumnCookie::SupportsClaude1m,
                        ColumnCookie::CountTokensAllowed,
                        ColumnCookie::SessionUsage,
                        ColumnCookie::WeeklyUsage,
                        ColumnCookie::WeeklyOpusUsage,
                        ColumnCookie::LifetimeUsage,
                    ])
                    .to_owned(),
            )
            .exec(&txn)
            .await
            .map_err(|e| ClewdrError::Whatever { message: "upsert_cookie".into(), source: Some(Box::new(e)) })?;
    }
    if keep_cookies.is_empty() {
        EntityCookie::delete_many().exec(&txn).await.map_err(|e| ClewdrError::Whatever { message: "delete_extra_cookies_all".into(), source: Some(Box::new(e)) })?;
    } else {
        EntityCookie::delete_many()
            .filter(ColumnCookie::Cookie.is_not_in(keep_cookies))
            .exec(&txn)
            .await
            .map_err(|e| ClewdrError::Whatever { message: "delete_extra_cookies".into(), source: Some(Box::new(e)) })?;
    }

    // Upsert wasted cookies and delete extras
    let mut keep_wasted: Vec<String> = Vec::with_capacity(invalid.len());
    for u in invalid.iter() {
        keep_wasted.push(u.cookie.to_string());
        let am = ActiveModelWasted {
            cookie: Set(u.cookie.to_string()),
            reason: Set(serde_json::to_string(&u.reason).unwrap_or_else(|_| "\"Unknown\"".to_string())),
        };
        EntityWasted::insert(am)
            .on_conflict(
                OnConflict::column(ColumnWasted::Cookie)
                    .update_columns([ColumnWasted::Reason])
                    .to_owned(),
            )
            .exec(&txn)
            .await
            .map_err(|e| ClewdrError::Whatever { message: "upsert_wasted".into(), source: Some(Box::new(e)) })?;
    }
    if keep_wasted.is_empty() {
        EntityWasted::delete_many().exec(&txn).await.map_err(|e| ClewdrError::Whatever { message: "delete_extra_wasted_all".into(), source: Some(Box::new(e)) })?;
    } else {
        EntityWasted::delete_many()
            .filter(ColumnWasted::Cookie.is_not_in(keep_wasted))
            .exec(&txn)
            .await
            .map_err(|e| ClewdrError::Whatever { message: "delete_extra_wasted".into(), source: Some(Box::new(e)) })?;
    }

    // Upsert keys and delete extras
    let mut keep_keys: Vec<String> = Vec::with_capacity(cfg.gemini_keys.len());
    for k in cfg.gemini_keys.iter() {
        keep_keys.push(k.key.to_string());
        let am = ActiveModelKeyRow { key: Set(k.key.to_string()), count_403: Set(k.count_403 as i64) };
        EntityKeyRow::insert(am)
            .on_conflict(
                OnConflict::column(ColumnKeyRow::Key)
                    .update_columns([ColumnKeyRow::Count403])
                    .to_owned(),
            )
            .exec(&txn)
            .await
            .map_err(|e| ClewdrError::Whatever { message: "upsert_key".into(), source: Some(Box::new(e)) })?;
    }
    if keep_keys.is_empty() {
        EntityKeyRow::delete_many().exec(&txn).await.map_err(|e| ClewdrError::Whatever { message: "delete_extra_keys_all".into(), source: Some(Box::new(e)) })?;
    } else {
        EntityKeyRow::delete_many()
            .filter(ColumnKeyRow::Key.is_not_in(keep_keys))
            .exec(&txn)
            .await
            .map_err(|e| ClewdrError::Whatever { message: "delete_extra_keys".into(), source: Some(Box::new(e)) })?;
    }

    txn
        .commit()
        .await
        .map_err(|e| ClewdrError::Whatever { message: "txn_commit".into(), source: Some(Box::new(e)) })?;
    super::metrics::mark_tx_commit();

    Ok(json!({"status":"ok"}))
}

pub async fn export_current_config() -> Result<serde_json::Value, ClewdrError> {
    // Reconstruct latest runtime config from DB rows using a transaction for snapshot consistency
    let db = ensure_conn().await?;
    let txn = db
        .begin()
        .await
        .map_err(|e| ClewdrError::Whatever { message: "txn_begin".into(), source: Some(Box::new(e)) })?;
    super::metrics::mark_tx_begin();
    // base config from DB row or current
    let mut cfg = if let Ok(Some(row)) = EntityConfig::find_by_id("main").one(&txn).await {
        toml::from_str::<ClewdrConfig>(&row.data)
            .unwrap_or_else(|_| crate::config::CLEWDR_CONFIG.load().as_ref().clone())
    } else {
        crate::config::CLEWDR_CONFIG.load().as_ref().clone()
    };
    // cookies
    let cookie_rows = EntityCookie::find().all(&txn).await.unwrap_or_default();
    cfg.cookie_array.clear();
    for r in cookie_rows {
        let mut c = CookieStatus::new(&r.cookie, r.reset_time).unwrap_or_default();
        if let Some(acc) = r.token_access {
            let expires_at = r
                .token_expires_at
                .and_then(|s| chrono::DateTime::from_timestamp(s, 0))
                .unwrap_or_else(chrono::Utc::now);
            let expires_in =
                std::time::Duration::from_secs(r.token_expires_in.unwrap_or_default() as u64);
            c.token = Some(crate::config::TokenInfo {
                access_token: acc,
                refresh_token: r.token_refresh.unwrap_or_default(),
                organization: crate::config::Organization {
                    uuid: r.token_org_uuid.unwrap_or_default(),
                },
                expires_at,
                expires_in,
            });
        }
        c.supports_claude_1m = r.supports_claude_1m;
        c.count_tokens_allowed = r.count_tokens_allowed;
        c.session_resets_at = r.session_resets_at;
        c.weekly_resets_at = r.weekly_resets_at;
        c.weekly_opus_resets_at = r.weekly_opus_resets_at;
        c.resets_last_checked_at = r.resets_last_checked_at;
        c.session_has_reset = r.session_has_reset;
        c.weekly_has_reset = r.weekly_has_reset;
        c.weekly_opus_has_reset = r.weekly_opus_has_reset;
        if let Some(s) = r.session_usage.as_ref() {
            if let Ok(v) = serde_json::from_str::<UsageBreakdown>(s) {
                c.session_usage = v;
            }
        }
        if let Some(s) = r.weekly_usage.as_ref() {
            if let Ok(v) = serde_json::from_str::<UsageBreakdown>(s) {
                c.weekly_usage = v;
            }
        }
        if let Some(s) = r.weekly_opus_usage.as_ref() {
            if let Ok(v) = serde_json::from_str::<UsageBreakdown>(s) {
                c.weekly_opus_usage = v;
            }
        }
        if let Some(s) = r.lifetime_usage.as_ref() {
            if let Ok(v) = serde_json::from_str::<UsageBreakdown>(s) {
                c.lifetime_usage = v;
            }
        }
        cfg.cookie_array.insert(c);
    }
    // wasted
    let wasted_rows = EntityWasted::find().all(&txn).await.unwrap_or_default();
    cfg.wasted_cookie.clear();
    for r in wasted_rows {
        if let Ok(reason) = serde_json::from_str(&r.reason)
            && let Ok(cc) = <crate::config::ClewdrCookie as std::str::FromStr>::from_str(&r.cookie)
        {
            cfg.wasted_cookie.insert(UselessCookie::new(cc, reason));
        }
    }
    // keys
    let key_rows = EntityKeyRow::find().all(&txn).await.unwrap_or_default();
    cfg.gemini_keys.clear();
    for r in key_rows {
        cfg.gemini_keys.insert(KeyStatus {
            key: r.key.into(),
            count_403: r.count_403 as u32,
        });
    }

    txn
        .commit()
        .await
        .map_err(|e| ClewdrError::Whatever { message: "txn_commit".into(), source: Some(Box::new(e)) })?;
    super::metrics::mark_tx_commit();

    if crate::config::CLEWDR_CONFIG.load().no_fs {
        return Err(ClewdrError::Whatever {
            message: "File export disabled when no_fs is enabled".into(),
            source: None,
        });
    }
    let toml = toml::to_string_pretty(&cfg)?;
    Ok(json!({"toml": toml}))
}

pub async fn persist_cookies(
    valid: &[CookieStatus],
    exhausted: &[CookieStatus],
    invalid: &[UselessCookie],
) -> Result<(), ClewdrError> {
    if !crate::config::CLEWDR_CONFIG.load().is_db_mode() {
        return Ok(());
    }
    let db = ensure_conn().await?;
    let start_all = std::time::Instant::now();
    let txn = db
        .begin()
        .await
        .map_err(|e| ClewdrError::Whatever { message: "txn_begin".into(), source: Some(Box::new(e)) })?;
    // Upsert cookies, then delete extras
    use sea_orm::sea_query::OnConflict;
    let mut keep_cookies: Vec<String> = Vec::with_capacity(valid.len() + exhausted.len());
    for c in valid.iter().chain(exhausted.iter()) {
        keep_cookies.push(c.cookie.to_string());
        let (acc, rtk, exp_at, exp_in, org) = if let Some(t) = &c.token {
            (
                Some(t.access_token.clone()),
                Some(t.refresh_token.clone()),
                Some(t.expires_at.timestamp()),
                Some(t.expires_in.as_secs() as i64),
                Some(t.organization.uuid.clone()),
            )
        } else {
            (None, None, None, None, None)
        };
        let am = ActiveModelCookie {
            cookie: Set(c.cookie.to_string()),
            reset_time: Set(c.reset_time),
            token_access: Set(acc),
            token_refresh: Set(rtk),
            token_expires_at: Set(exp_at),
            token_expires_in: Set(exp_in),
            token_org_uuid: Set(org),
            supports_claude_1m: Set(c.supports_claude_1m),
            count_tokens_allowed: Set(c.count_tokens_allowed),
            total_input_tokens: Set(None),
            total_output_tokens: Set(None),
            window_input_tokens: Set(None),
            window_output_tokens: Set(None),
            session_usage: Set(Some(serde_json::to_string(&c.session_usage).unwrap_or_else(|_| "{}".to_string()))),
            weekly_usage: Set(Some(serde_json::to_string(&c.weekly_usage).unwrap_or_else(|_| "{}".to_string()))),
            weekly_opus_usage: Set(Some(serde_json::to_string(&c.weekly_opus_usage).unwrap_or_else(|_| "{}".to_string()))),
            lifetime_usage: Set(Some(serde_json::to_string(&c.lifetime_usage).unwrap_or_else(|_| "{}".to_string()))),
            session_resets_at: Set(c.session_resets_at),
            weekly_resets_at: Set(c.weekly_resets_at),
            weekly_opus_resets_at: Set(c.weekly_opus_resets_at),
            resets_last_checked_at: Set(c.resets_last_checked_at),
            session_has_reset: Set(c.session_has_reset),
            weekly_has_reset: Set(c.weekly_has_reset),
            weekly_opus_has_reset: Set(c.weekly_opus_has_reset),
        };
        if let Err(e) = EntityCookie::insert(am)
            .on_conflict(
                OnConflict::column(ColumnCookie::Cookie)
                    .update_columns([
                        ColumnCookie::ResetTime,
                        ColumnCookie::TokenAccess,
                        ColumnCookie::TokenRefresh,
                        ColumnCookie::TokenExpiresAt,
                        ColumnCookie::TokenExpiresIn,
                        ColumnCookie::TokenOrgUuid,
                        ColumnCookie::SupportsClaude1m,
                        ColumnCookie::CountTokensAllowed,
                        ColumnCookie::SessionUsage,
                        ColumnCookie::WeeklyUsage,
                        ColumnCookie::WeeklyOpusUsage,
                        ColumnCookie::LifetimeUsage,
                        ColumnCookie::SessionResetsAt,
                        ColumnCookie::WeeklyResetsAt,
                        ColumnCookie::WeeklyOpusResetsAt,
                        ColumnCookie::ResetsLastCheckedAt,
                        ColumnCookie::SessionHasReset,
                        ColumnCookie::WeeklyHasReset,
                        ColumnCookie::WeeklyOpusHasReset,
                    ])
                    .to_owned(),
            )
            .exec(&txn)
            .await
        {
            record_error_msg(&e);
            mark_write_err();
            return Err(ClewdrError::Whatever { message: "upsert_cookie".into(), source: Some(Box::new(e)) });
        }
    }

    // Delete extra cookies
    if keep_cookies.is_empty() {
        if let Err(e) = EntityCookie::delete_many().exec(&txn).await {
            record_error_msg(&e);
            mark_write_err();
            return Err(ClewdrError::Whatever { message: "delete_extra_cookies_all".into(), source: Some(Box::new(e)) });
        }
    } else if let Err(e) = EntityCookie::delete_many()
        .filter(ColumnCookie::Cookie.is_not_in(keep_cookies))
        .exec(&txn)
        .await
    {
        record_error_msg(&e);
        mark_write_err();
        return Err(ClewdrError::Whatever { message: "delete_extra_cookies".into(), source: Some(Box::new(e)) });
    }

    // Upsert wasted cookies, then delete extras
    let mut keep_wasted: Vec<String> = Vec::with_capacity(invalid.len());
    for u in invalid {
        keep_wasted.push(u.cookie.to_string());
        let am = ActiveModelWasted {
            cookie: Set(u.cookie.to_string()),
            reason: Set(serde_json::to_string(&u.reason).unwrap_or_else(|_| "\"Unknown\"".to_string())),
        };
        if let Err(e) = EntityWasted::insert(am)
            .on_conflict(
                OnConflict::column(ColumnWasted::Cookie)
                    .update_columns([ColumnWasted::Reason])
                    .to_owned(),
            )
            .exec(&txn)
            .await
        {
            record_error_msg(&e);
            mark_write_err();
            return Err(ClewdrError::Whatever { message: "upsert_wasted".into(), source: Some(Box::new(e)) });
        }
    }

    if keep_wasted.is_empty() {
        if let Err(e) = EntityWasted::delete_many().exec(&txn).await {
            record_error_msg(&e);
            mark_write_err();
            return Err(ClewdrError::Whatever { message: "delete_extra_wasted_all".into(), source: Some(Box::new(e)) });
        }
    } else if let Err(e) = EntityWasted::delete_many()
        .filter(ColumnWasted::Cookie.is_not_in(keep_wasted))
        .exec(&txn)
        .await
    {
        record_error_msg(&e);
        mark_write_err();
        return Err(ClewdrError::Whatever { message: "delete_extra_wasted".into(), source: Some(Box::new(e)) });
    }

    txn
        .commit()
        .await
        .map_err(|e| ClewdrError::Whatever { message: "txn_commit".into(), source: Some(Box::new(e)) })?;

    record_duration(start_all);
    mark_write_ok();
    Ok(())
}

// Read helpers used by background sync
pub async fn load_all_keys() -> Result<Vec<KeyStatus>, ClewdrError> {
    let db = ensure_conn().await?;
    let rows = EntityKeyRow::find()
        .all(&db)
        .await
        .map_err(|e| ClewdrError::Whatever {
            message: "load_keys".into(),
            source: Some(Box::new(e)),
        })?;
    Ok(rows
        .into_iter()
        .map(|r| KeyStatus {
            key: r.key.into(),
            count_403: r.count_403 as u32,
        })
        .collect())
}

pub async fn load_all_cookies()
-> Result<(Vec<CookieStatus>, Vec<CookieStatus>, Vec<UselessCookie>), ClewdrError> {
    let db = ensure_conn().await?;
    let mut valid = Vec::new();
    let mut exhausted = Vec::new();
    let mut invalid = Vec::new();
    let rows = EntityCookie::find()
        .all(&db)
        .await
        .map_err(|e| ClewdrError::Whatever {
            message: "load_cookies".into(),
            source: Some(Box::new(e)),
        })?;
    for r in rows {
        let mut c = CookieStatus::new(&r.cookie, r.reset_time).unwrap_or_default();
        if let Some(acc) = r.token_access {
            let expires_at = r
                .token_expires_at
                .and_then(|s| chrono::DateTime::from_timestamp(s, 0))
                .unwrap_or_else(chrono::Utc::now);
            let expires_in =
                std::time::Duration::from_secs(r.token_expires_in.unwrap_or_default() as u64);
            c.token = Some(crate::config::TokenInfo {
                access_token: acc,
                refresh_token: r.token_refresh.unwrap_or_default(),
                organization: crate::config::Organization {
                    uuid: r.token_org_uuid.unwrap_or_default(),
                },
                expires_at,
                expires_in,
            });
        }
        c.supports_claude_1m = r.supports_claude_1m;
        c.count_tokens_allowed = r.count_tokens_allowed;
        // Legacy totals/windows removed; ignore DB columns if present
        if let Some(s) = r.session_usage.as_ref() {
            if let Ok(v) = serde_json::from_str::<UsageBreakdown>(s) {
                c.session_usage = v;
            }
        }
        if let Some(s) = r.weekly_usage.as_ref() {
            if let Ok(v) = serde_json::from_str::<UsageBreakdown>(s) {
                c.weekly_usage = v;
            }
        }
        if let Some(s) = r.weekly_opus_usage.as_ref() {
            if let Ok(v) = serde_json::from_str::<UsageBreakdown>(s) {
                c.weekly_opus_usage = v;
            }
        }
        if let Some(s) = r.lifetime_usage.as_ref() {
            if let Ok(v) = serde_json::from_str::<UsageBreakdown>(s) {
                c.lifetime_usage = v;
            }
        }
        if c.reset_time.is_some() {
            exhausted.push(c);
        } else {
            valid.push(c);
        }
    }
    let wasted = EntityWasted::find()
        .all(&db)
        .await
        .map_err(|e| ClewdrError::Whatever {
            message: "load_wasted".into(),
            source: Some(Box::new(e)),
        })?;
    for r in wasted {
        if let Ok(reason) = serde_json::from_str(&r.reason)
            && let Ok(cc) = <crate::config::ClewdrCookie as std::str::FromStr>::from_str(&r.cookie)
        {
            invalid.push(UselessCookie::new(cc, reason));
        }
    }
    Ok((valid, exhausted, invalid))
}

pub async fn status_json() -> Result<serde_json::Value, ClewdrError> {
    use sea_orm::{DatabaseBackend, Statement};
    use std::sync::atomic::Ordering;

    let mut healthy = false;
    let mut err_str: Option<String> = None;
    let mut latency_ms: Option<u128> = None;
    match ensure_conn().await {
        Ok(db) => {
            let backend = db.get_database_backend();
            let stmt = match backend {
                DatabaseBackend::Postgres => Statement::from_string(backend, "SELECT 1"),
                DatabaseBackend::MySql => Statement::from_string(backend, "SELECT 1"),
                DatabaseBackend::Sqlite => Statement::from_string(backend, "SELECT 1"),
            };
            let start = std::time::Instant::now();
            match db.execute(stmt).await {
                Ok(_) => {
                    healthy = true;
                    latency_ms = Some(start.elapsed().as_millis());
                }
                Err(e) => {
                    err_str = Some(e.to_string());
                }
            }
        }
        Err(e) => {
            err_str = Some(e.to_string());
        }
    }
    let total = TOTAL_WRITES.load(Ordering::Relaxed);
    let errors = WRITE_ERROR_COUNT.load(Ordering::Relaxed);
    let nanos = TOTAL_WRITE_NANOS.load(Ordering::Relaxed);
    let avg_ms = if total > 0 {
        (nanos as f64 / total as f64) / 1_000_000.0
    } else {
        0.0
    };
    let ratio = if total > 0 {
        errors as f64 / total as f64
    } else {
        0.0
    };
    let last_error = LAST_ERROR.lock().ok().and_then(|g| g.clone());
    let tx_begin = TX_BEGIN_COUNT.load(Ordering::Relaxed);
    let tx_commit = TX_COMMIT_COUNT.load(Ordering::Relaxed);
    let tx_rollback = TX_ROLLBACK_COUNT.load(Ordering::Relaxed);
    let tx_open = tx_begin.saturating_sub(tx_commit + tx_rollback);
    let last_tx_ts = LAST_TX_TS.load(Ordering::Relaxed);
    let last_conn_ts = LAST_CONN_TS.load(Ordering::Relaxed);
    let conn_ok = CONN_SUCCESS_COUNT.load(Ordering::Relaxed);
    let conn_err = CONN_ERROR_COUNT.load(Ordering::Relaxed);
    let conn_cfg = CONN_CONFIG.lock().ok().and_then(|g| g.clone());
    Ok(json!({
        "enabled": true,
        "mode": "db",
        "healthy": healthy,
        "latency_ms": latency_ms,
        "last_write_ts": LAST_WRITE_TS.load(Ordering::Relaxed),
        "write_error_count": errors,
        "total_writes": total,
        "avg_write_ms": avg_ms,
        "failure_ratio": ratio,
        "error": err_str,
        "last_error": last_error,
        "tx": {
            "begin": tx_begin,
            "commit": tx_commit,
            "rollback": tx_rollback,
            "open": tx_open,
            "last_tx_ts": last_tx_ts,
        },
        "conn": {
            "last_conn_ts": last_conn_ts,
            "success": conn_ok,
            "error": conn_err,
            "pool_config": conn_cfg.map(|c| json!({
                "max_connections": c.max_connections,
                "min_connections": c.min_connections,
                "connect_timeout_ms": c.connect_timeout_ms,
                "acquire_timeout_ms": c.acquire_timeout_ms,
                "idle_timeout_ms": c.idle_timeout_ms,
            })),
        },
    }))
}
