use sqlx::{postgres::PgPoolOptions, PgPool};

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .min_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(15))
            .test_before_acquire(true)
            .connect(url)
            .await?;

        Ok(Self { pool })
    }

    /// Spawn a background task that pings the database every 4 minutes to prevent
    /// Neon serverless from auto-suspending (5-min idle timeout). Without this,
    /// the first query after idle pays a 500ms–2s cold-start penalty.
    pub fn start_keepalive(&self) {
        let pool = self.pool.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(240));
            interval.tick().await; // first tick fires immediately — skip it
            loop {
                interval.tick().await;
                if let Err(e) = sqlx::query("SELECT 1").execute(&pool).await {
                    tracing::warn!("Database keepalive ping failed: {e}");
                }
            }
        });
    }

    pub async fn run_migrations(&self) -> anyhow::Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        tracing::info!("Database migrations applied successfully");
        Ok(())
    }

    pub async fn ping(&self) -> anyhow::Result<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    // Integration tests use sqlx::test macro which provides a test transaction
    // that auto-rolls back — no manual cleanup needed.
    // Example:
    // #[sqlx::test]
    // async fn test_ping(pool: sqlx::PgPool) {
    //     let db = Database { pool };
    //     assert!(db.ping().await.is_ok());
    // }
}
