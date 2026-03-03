use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration, NaiveDate, NaiveTime, Utc};
use serde::Deserialize;
use sqlx::Row;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    AppState,
};

const MAX_APPS_PER_REPORT: usize = 64;
const MAX_APP_NAME_LEN: usize = 120;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/screentime/report", post(report_screentime))
        .route(
            "/parental/children/:child_id/screentime",
            get(get_child_screentime).patch(update_child_screentime),
        )
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ScreenTimeReportInput {
    recorded_date: String,
    platform: String,
    apps: Vec<AppUsageInput>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct AppUsageInput {
    app_name: String,
    duration_seconds: i32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScreenTimeQuery {
    period: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct UpdateScreenTimeSettingsInput {
    limit_minutes: i32,
    bedtime_start: Option<String>,
    bedtime_end: Option<String>,
}

async fn report_screentime(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<ScreenTimeReportInput>,
) -> AppResult<impl IntoResponse> {
    if body.apps.is_empty() {
        return Err(AppError::BadRequest("apps must not be empty".into()));
    }
    if body.apps.len() > MAX_APPS_PER_REPORT {
        return Err(AppError::BadRequest(format!(
            "apps must contain at most {MAX_APPS_PER_REPORT} items"
        )));
    }

    let recorded_date = parse_date(&body.recorded_date)?;
    let platform = parse_platform(&body.platform)?;

    let mut tx = state.db.pool().begin().await?;
    for app in &body.apps {
        let app_name = app.app_name.trim();
        if app_name.is_empty() {
            return Err(AppError::BadRequest("appName cannot be empty".into()));
        }
        if app_name.len() > MAX_APP_NAME_LEN {
            return Err(AppError::BadRequest(format!(
                "appName cannot exceed {MAX_APP_NAME_LEN} characters"
            )));
        }
        if !(0..=86_400).contains(&app.duration_seconds) {
            return Err(AppError::BadRequest(
                "durationSeconds must be between 0 and 86400".into(),
            ));
        }

        sqlx::query(
            "INSERT INTO screen_time_records (user_id, app_name, duration_seconds, platform, recorded_date)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (user_id, app_name, recorded_date, platform)
             DO UPDATE SET duration_seconds = EXCLUDED.duration_seconds, created_at = NOW()",
        )
        .bind(auth.user_id)
        .bind(app_name)
        .bind(app.duration_seconds)
        .bind(platform)
        .bind(recorded_date)
        .execute(&mut *tx)
        .await?;
    }

    refresh_daily_summary(auth.user_id, recorded_date, &mut tx).await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "status": "ok",
            "recordedDate": recorded_date.to_string(),
            "itemsUpserted": body.apps.len(),
            "platform": platform,
        })),
    ))
}

async fn get_child_screentime(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(child_id): Path<Uuid>,
    Query(query): Query<ScreenTimeQuery>,
) -> AppResult<impl IntoResponse> {
    require_parent_of(auth.user_id, child_id, &state).await?;

    let period = query.period.unwrap_or_else(|| "today".to_string());
    let today = Utc::now().date_naive();
    let (start_date, end_date) = period_range(&period, today)?;

    let settings_row = sqlx::query(
        "SELECT daily_limit_minutes, bedtime_start, bedtime_end
         FROM screen_time_settings WHERE child_user_id = $1",
    )
    .bind(child_id)
    .fetch_optional(state.db.pool())
    .await?;

    let limit_minutes = settings_row
        .as_ref()
        .and_then(|r| r.try_get::<i32, _>("daily_limit_minutes").ok())
        .unwrap_or(180);

    let bedtime_start = settings_row
        .as_ref()
        .and_then(|r| r.try_get::<Option<NaiveTime>, _>("bedtime_start").ok())
        .flatten()
        .map(|t| t.format("%H:%M").to_string());

    let bedtime_end = settings_row
        .as_ref()
        .and_then(|r| r.try_get::<Option<NaiveTime>, _>("bedtime_end").ok())
        .flatten()
        .map(|t| t.format("%H:%M").to_string());

    let today_total_seconds = sqlx::query(
        "SELECT COALESCE(total_seconds, 0) AS total_seconds
         FROM screen_time_daily_summaries
         WHERE user_id = $1 AND summary_date = $2",
    )
    .bind(child_id)
    .bind(today)
    .fetch_optional(state.db.pool())
    .await?
    .and_then(|r| r.try_get::<i32, _>("total_seconds").ok())
    .unwrap_or(0);

    let breakdown_rows = sqlx::query(
        "SELECT app_name, SUM(duration_seconds)::INT AS total_seconds
         FROM screen_time_records
         WHERE user_id = $1 AND recorded_date BETWEEN $2 AND $3
         GROUP BY app_name
         ORDER BY total_seconds DESC
         LIMIT 8",
    )
    .bind(child_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(state.db.pool())
    .await?;

    let app_breakdown: Vec<serde_json::Value> = breakdown_rows
        .iter()
        .map(|r| {
            let app_name = r.try_get::<String, _>("app_name").unwrap_or_default();
            let minutes = r.try_get::<i32, _>("total_seconds").unwrap_or(0) / 60;
            serde_json::json!({
                "appName": app_name,
                "icon": app_icon(&app_name),
                "minutes": minutes,
            })
        })
        .collect();

    let week_start = today - Duration::days(6);
    let summary_rows = sqlx::query(
        "SELECT summary_date, total_seconds, yapper_seconds, other_seconds
         FROM screen_time_daily_summaries
         WHERE user_id = $1 AND summary_date BETWEEN $2 AND $3
         ORDER BY summary_date ASC",
    )
    .bind(child_id)
    .bind(week_start)
    .bind(today)
    .fetch_all(state.db.pool())
    .await?;

    let mut by_day = std::collections::HashMap::with_capacity(7);
    for row in summary_rows {
        let date: NaiveDate = row.try_get("summary_date")?;
        let yapper_seconds: i32 = row.try_get("yapper_seconds")?;
        let other_seconds: i32 = row.try_get("other_seconds")?;
        by_day.insert(date, (yapper_seconds, other_seconds));
    }

    let weekly_data: Vec<serde_json::Value> = (0..7)
        .map(|offset| week_start + Duration::days(offset))
        .map(|day| {
            let (yapper_seconds, other_seconds) = by_day.get(&day).copied().unwrap_or((0, 0));
            serde_json::json!({
                "day": day.format("%a").to_string(),
                "yapperMinutes": yapper_seconds / 60,
                "otherMinutes": other_seconds / 60,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "period": period,
        "rangeStart": start_date.to_string(),
        "rangeEnd": end_date.to_string(),
        "totalMinutesToday": today_total_seconds / 60,
        "limitMinutes": limit_minutes,
        "appBreakdown": app_breakdown,
        "weeklyData": weekly_data,
        "bedtimeStart": bedtime_start,
        "bedtimeEnd": bedtime_end,
    })))
}

async fn update_child_screentime(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(child_id): Path<Uuid>,
    Json(body): Json<UpdateScreenTimeSettingsInput>,
) -> AppResult<impl IntoResponse> {
    require_parent_of(auth.user_id, child_id, &state).await?;

    if !(30..=480).contains(&body.limit_minutes) {
        return Err(AppError::BadRequest(
            "limitMinutes must be between 30 and 480".into(),
        ));
    }

    let bedtime_start = parse_time_opt(body.bedtime_start)?;
    let bedtime_end = parse_time_opt(body.bedtime_end)?;

    upsert_screentime_settings(
        state.db.pool(),
        child_id,
        body.limit_minutes,
        bedtime_start,
        bedtime_end,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "childId": child_id,
        "limitMinutes": body.limit_minutes,
        "bedtimeStart": bedtime_start.map(|t| t.format("%H:%M").to_string()),
        "bedtimeEnd": bedtime_end.map(|t| t.format("%H:%M").to_string()),
    })))
}

fn parse_date(value: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("recordedDate must use YYYY-MM-DD".into()))
}

fn parse_platform(value: &str) -> AppResult<&str> {
    match value {
        "ios" | "android" | "web" | "desktop" => Ok(value),
        _ => Err(AppError::BadRequest(
            "platform must be one of: ios, android, web, desktop".into(),
        )),
    }
}

fn parse_time_opt(value: Option<String>) -> AppResult<Option<NaiveTime>> {
    let Some(raw) = value else {
        return Ok(None);
    };
    if raw.trim().is_empty() {
        return Ok(None);
    }
    NaiveTime::parse_from_str(&raw, "%H:%M")
        .map(Some)
        .map_err(|_| AppError::BadRequest("time must use HH:MM".into()))
}

fn period_range(period: &str, today: NaiveDate) -> AppResult<(NaiveDate, NaiveDate)> {
    let dates = match period {
        "today" => (today, today),
        "week" => (today - Duration::days(6), today),
        "month" => (today - Duration::days(29), today),
        _ => {
            return Err(AppError::BadRequest(
                "period must be one of: today, week, month".into(),
            ))
        }
    };
    Ok(dates)
}

async fn require_parent_of(parent_id: Uuid, child_id: Uuid, state: &AppState) -> AppResult<()> {
    require_parent_of_pool(parent_id, child_id, state.db.pool()).await
}

async fn require_parent_of_pool(parent_id: Uuid, child_id: Uuid, pool: &PgPool) -> AppResult<()> {
    let row = sqlx::query(
        "SELECT 1 FROM parent_child_relationships
         WHERE parent_user_id = $1 AND child_user_id = $2",
    )
    .bind(parent_id)
    .bind(child_id)
    .fetch_optional(pool)
    .await?;

    if row.is_none() {
        return Err(AppError::Forbidden);
    }

    Ok(())
}

async fn upsert_screentime_settings(
    pool: &PgPool,
    child_id: Uuid,
    limit_minutes: i32,
    bedtime_start: Option<NaiveTime>,
    bedtime_end: Option<NaiveTime>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO screen_time_settings (child_user_id, daily_limit_minutes, bedtime_start, bedtime_end, updated_at)
         VALUES ($1, $2, $3, $4, NOW())
         ON CONFLICT (child_user_id)
         DO UPDATE SET
            daily_limit_minutes = EXCLUDED.daily_limit_minutes,
            bedtime_start = EXCLUDED.bedtime_start,
            bedtime_end = EXCLUDED.bedtime_end,
            updated_at = NOW()",
    )
    .bind(child_id)
    .bind(limit_minutes)
    .bind(bedtime_start)
    .bind(bedtime_end)
    .execute(pool)
    .await?;

    Ok(())
}

async fn refresh_daily_summary(
    user_id: Uuid,
    summary_date: NaiveDate,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO screen_time_daily_summaries
            (user_id, summary_date, total_seconds, yapper_seconds, other_seconds, updated_at)
         SELECT
            $1,
            $2,
            COALESCE(SUM(duration_seconds), 0),
            COALESCE(SUM(CASE WHEN LOWER(app_name) = 'yapper' THEN duration_seconds ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN LOWER(app_name) <> 'yapper' THEN duration_seconds ELSE 0 END), 0),
            NOW()
         FROM screen_time_records
         WHERE user_id = $1 AND recorded_date = $2
         ON CONFLICT (user_id, summary_date)
         DO UPDATE SET
            total_seconds = EXCLUDED.total_seconds,
            yapper_seconds = EXCLUDED.yapper_seconds,
            other_seconds = EXCLUDED.other_seconds,
            updated_at = NOW()",
    )
    .bind(user_id)
    .bind(summary_date)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

fn app_icon(app_name: &str) -> &'static str {
    match app_name.to_lowercase().as_str() {
        "yapper" => "🟣",
        "youtube" => "🔴",
        "instagram" => "🟠",
        "tiktok" => "⬛",
        "discord" => "🟦",
        _ => "📱",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    async fn test_pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .ok()?;

        if sqlx::migrate!("./migrations").run(&pool).await.is_err() {
            return None;
        }

        Some(pool)
    }

    async fn insert_user(pool: &PgPool, email: &str, username: &str, account_type: &str) -> Uuid {
        let row = sqlx::query(
            "INSERT INTO users (email, username, display_name, account_type, parental_controls_enabled)
             VALUES ($1, $2, $3, $4, FALSE)
             RETURNING id",
        )
        .bind(email)
        .bind(username)
        .bind(username)
        .bind(account_type)
        .fetch_one(pool)
        .await
        .expect("insert user should succeed");

        row.try_get("id").expect("id should exist")
    }

    #[test]
    fn parse_platform_rejects_unknown() {
        let result = parse_platform("xbox");
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn period_range_rejects_invalid() {
        let today = Utc::now().date_naive();
        let result = period_range("year", today);
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn require_parent_of_pool_allows_linked_parent() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let suffix = Uuid::new_v4().simple().to_string();
        let parent = insert_user(
            &pool,
            &format!("parent1+{suffix}@example.com"),
            &format!("parent1_{suffix}"),
            "parent",
        )
        .await;
        let child = insert_user(
            &pool,
            &format!("child1+{suffix}@example.com"),
            &format!("child1_{suffix}"),
            "child",
        )
        .await;

        sqlx::query(
            "INSERT INTO parent_child_relationships (parent_user_id, child_user_id)
             VALUES ($1, $2)",
        )
        .bind(parent)
        .bind(child)
        .execute(&pool)
        .await
        .expect("relationship insert should succeed");

        let result = require_parent_of_pool(parent, child, &pool).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn require_parent_of_pool_denies_unlinked_parent() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let suffix = Uuid::new_v4().simple().to_string();
        let parent = insert_user(
            &pool,
            &format!("parent2+{suffix}@example.com"),
            &format!("parent2_{suffix}"),
            "parent",
        )
        .await;
        let child = insert_user(
            &pool,
            &format!("child2+{suffix}@example.com"),
            &format!("child2_{suffix}"),
            "child",
        )
        .await;

        let result = require_parent_of_pool(parent, child, &pool).await;
        assert!(matches!(result, Err(AppError::Forbidden)));
    }

    #[tokio::test]
    async fn refresh_daily_summary_aggregates_by_app_type() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let suffix = Uuid::new_v4().simple().to_string();
        let user = insert_user(
            &pool,
            &format!("kid1+{suffix}@example.com"),
            &format!("kid1_{suffix}"),
            "child",
        )
        .await;
        let date = Utc::now().date_naive();

        sqlx::query(
            "INSERT INTO screen_time_records (user_id, app_name, duration_seconds, platform, recorded_date)
             VALUES ($1, 'Yapper', 3600, 'ios', $2),
                    ($1, 'YouTube', 1200, 'ios', $2),
                    ($1, 'TikTok', 1800, 'ios', $2)",
        )
        .bind(user)
        .bind(date)
        .execute(&pool)
        .await
        .expect("screen_time_records insert should succeed");

        let mut tx = pool.begin().await.expect("tx should start");
        refresh_daily_summary(user, date, &mut tx)
            .await
            .expect("summary refresh should succeed");
        tx.commit().await.expect("tx should commit");

        let row = sqlx::query(
            "SELECT total_seconds, yapper_seconds, other_seconds
             FROM screen_time_daily_summaries
             WHERE user_id = $1 AND summary_date = $2",
        )
        .bind(user)
        .bind(date)
        .fetch_one(&pool)
        .await
        .expect("summary row should exist");

        let total_seconds: i32 = row.try_get("total_seconds").expect("total_seconds present");
        let yapper_seconds: i32 = row.try_get("yapper_seconds").expect("yapper_seconds present");
        let other_seconds: i32 = row.try_get("other_seconds").expect("other_seconds present");

        assert_eq!(total_seconds, 6600);
        assert_eq!(yapper_seconds, 3600);
        assert_eq!(other_seconds, 3000);
    }

    #[tokio::test]
    async fn upsert_screentime_settings_updates_existing_row() {
        let Some(pool) = test_pool().await else {
            return;
        };
        let suffix = Uuid::new_v4().simple().to_string();
        let child = insert_user(
            &pool,
            &format!("child3+{suffix}@example.com"),
            &format!("child3_{suffix}"),
            "child",
        )
        .await;

        upsert_screentime_settings(
            &pool,
            child,
            180,
            NaiveTime::from_hms_opt(22, 0, 0),
            NaiveTime::from_hms_opt(7, 0, 0),
        )
        .await
        .expect("first upsert should succeed");

        upsert_screentime_settings(
            &pool,
            child,
            240,
            NaiveTime::from_hms_opt(21, 30, 0),
            NaiveTime::from_hms_opt(6, 30, 0),
        )
        .await
        .expect("second upsert should succeed");

        let row = sqlx::query(
            "SELECT daily_limit_minutes, bedtime_start, bedtime_end
             FROM screen_time_settings WHERE child_user_id = $1",
        )
        .bind(child)
        .fetch_one(&pool)
        .await
        .expect("settings row should exist");

        let limit: i32 = row.try_get("daily_limit_minutes").expect("limit should exist");
        let bedtime_start: Option<NaiveTime> = row
            .try_get("bedtime_start")
            .expect("bedtime_start should exist");
        let bedtime_end: Option<NaiveTime> = row
            .try_get("bedtime_end")
            .expect("bedtime_end should exist");

        assert_eq!(limit, 240);
        assert_eq!(bedtime_start, NaiveTime::from_hms_opt(21, 30, 0));
        assert_eq!(bedtime_end, NaiveTime::from_hms_opt(6, 30, 0));
    }
}
