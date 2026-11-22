//! User accounts: creation on first sign-in, profile reads and settings.

use sqlx::{PgConnection, PgExecutor, PgPool};
use uuid::Uuid;

use crate::{
    domain::minutes,
    error::{AppError, AppResult},
    providers::google::GoogleIdentity,
};

/// Explanation languages. `en` means "explain in English".
pub const L1_VALUES: &[&str] = &["ur", "hi", "bn", "ar", "en"];
pub const LEVEL_VALUES: &[&str] = &["a1", "a2", "b1", "b2", "c1", "c2"];
pub const GOAL_VALUES: &[&str] = &["ielts", "interview", "work", "abroad", "daily"];

#[derive(Debug, sqlx::FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub l1: String,
    pub level: Option<String>,
    pub goal: Option<String>,
    pub country: Option<String>,
    pub timezone: String,
}

/// Find the user with this phone or create one. Returns `(id, created)`.
/// New users receive the one-time free minutes in the same transaction.
pub async fn upsert_by_phone(conn: &mut PgConnection, phone: &str) -> AppResult<(Uuid, bool)> {
    // `xmax = 0` is true only for a freshly inserted row, so one statement
    // tells us whether the user is new, with no race between "find" and "create".
    let (id, created): (Uuid, bool) = sqlx::query_as(
        "INSERT INTO users (phone) VALUES ($1)
         ON CONFLICT (phone) DO UPDATE SET updated_at = now()
         RETURNING id, (xmax = 0) AS created",
    )
    .bind(phone)
    .fetch_one(&mut *conn)
    .await?;
    if created {
        minutes::grant_free(conn, id).await?;
    }
    Ok((id, created))
}

/// Sign in with Google: match by Google account id, else link to an existing
/// account with the same *verified* email, else create a new account.
pub async fn find_or_create_google(
    conn: &mut PgConnection,
    ident: &GoogleIdentity,
) -> AppResult<(Uuid, bool)> {
    if let Some(id) = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE google_sub = $1")
        .bind(&ident.sub)
        .fetch_optional(&mut *conn)
        .await?
    {
        return Ok((id, false));
    }

    let verified_email = ident.email.as_deref().filter(|_| ident.email_verified);

    if let Some(email) = verified_email {
        let existing: Option<(Uuid, Option<String>)> = sqlx::query_as(
            "SELECT id, google_sub FROM users WHERE lower(email) = lower($1) FOR UPDATE",
        )
        .bind(email)
        .fetch_optional(&mut *conn)
        .await?;
        match existing {
            Some((id, None)) => {
                sqlx::query("UPDATE users SET google_sub = $2, updated_at = now() WHERE id = $1")
                    .bind(id)
                    .bind(&ident.sub)
                    .execute(&mut *conn)
                    .await?;
                return Ok((id, false));
            }
            Some((_, Some(_))) => {
                return Err(AppError::Conflict(
                    "this email is linked to a different Google account".into(),
                ));
            }
            None => {}
        }
    }

    // Unverified emails are not stored: they must not claim an address.
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (google_sub, email, name) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(&ident.sub)
    .bind(verified_email)
    .bind(&ident.name)
    .fetch_one(&mut *conn)
    .await?;
    minutes::grant_free(conn, id).await?;
    Ok((id, true))
}

pub async fn get<'e, E: PgExecutor<'e>>(ex: E, id: Uuid) -> AppResult<UserRow> {
    sqlx::query_as(
        "SELECT id, phone, email, name, l1, level, goal, country::text AS country, timezone
         FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(ex)
    .await?
    .ok_or(AppError::NotFound)
}

/// The plan currently granting access, or "free". Only `entitlements` decides.
pub async fn current_plan<'e, E: PgExecutor<'e>>(ex: E, id: Uuid) -> AppResult<String> {
    let plan: Option<String> = sqlx::query_scalar(
        "SELECT plan FROM entitlements
         WHERE user_id = $1 AND status = 'active'
           AND starts_at <= now() AND (ends_at IS NULL OR ends_at > now())
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(id)
    .fetch_optional(ex)
    .await?;
    Ok(plan.unwrap_or_else(|| "free".to_owned()))
}

pub async fn streak<'e, E: PgExecutor<'e>>(ex: E, id: Uuid) -> AppResult<(i32, i32)> {
    let row: Option<(i32, i32)> =
        sqlx::query_as("SELECT current_days, best_days FROM streaks WHERE user_id = $1")
            .bind(id)
            .fetch_optional(ex)
            .await?;
    Ok(row.unwrap_or((0, 0)))
}

/// Partial update; `None` leaves a field unchanged.
#[derive(Debug, Default)]
pub struct Settings {
    pub name: Option<String>,
    pub l1: Option<String>,
    pub level: Option<String>,
    pub goal: Option<String>,
}

impl Settings {
    pub fn validate(&mut self) -> AppResult<()> {
        fn check(value: &mut Option<String>, allowed: &[&str], field: &str) -> AppResult<()> {
            if let Some(v) = value {
                *v = v.trim().to_lowercase();
                if !allowed.contains(&v.as_str()) {
                    return Err(AppError::BadRequest(format!(
                        "{field} must be one of: {}",
                        allowed.join(", ")
                    )));
                }
            }
            Ok(())
        }
        check(&mut self.l1, L1_VALUES, "l1")?;
        check(&mut self.level, LEVEL_VALUES, "level")?;
        check(&mut self.goal, GOAL_VALUES, "goal")?;
        if let Some(name) = &mut self.name {
            *name = name.trim().to_owned();
            if name.is_empty() || name.chars().count() > 80 {
                return Err(AppError::BadRequest("name must be 1-80 characters".into()));
            }
        }
        Ok(())
    }
}

pub async fn update_settings(db: &PgPool, id: Uuid, s: &Settings) -> AppResult<()> {
    let res = sqlx::query(
        "UPDATE users SET
            name  = COALESCE($2, name),
            l1    = COALESCE($3, l1),
            level = COALESCE($4, level),
            goal  = COALESCE($5, goal),
            updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(&s.name)
    .bind(&s.l1)
    .bind(&s.level)
    .bind(&s.goal)
    .execute(db)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_normalised_and_checked() {
        let mut ok = Settings {
            l1: Some(" UR ".into()),
            level: Some("B1".into()),
            ..Default::default()
        };
        ok.validate().unwrap();
        assert_eq!(ok.l1.as_deref(), Some("ur"));
        assert_eq!(ok.level.as_deref(), Some("b1"));

        let mut bad = Settings {
            goal: Some("fame".into()),
            ..Default::default()
        };
        assert!(bad.validate().is_err());

        let mut empty_name = Settings {
            name: Some("   ".into()),
            ..Default::default()
        };
        assert!(empty_name.validate().is_err());
    }
}
