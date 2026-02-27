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
            .acquire_timeout(std::time::Duration::from_secs(5))
            .connect(url)
            .await?;

        Ok(Self { pool })
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

/// Sets up a PostgreSQL LISTEN/NOTIFY listener for the given channel.
/// The callback receives the payload string for each notification.
pub async fn listen_notify(
    pool: &PgPool,
    channel: &str,
    mut callback: impl FnMut(String) + Send + 'static,
) -> anyhow::Result<()> {
    let mut listener = sqlx::postgres::PgListener::connect_with(pool).await?;
    listener.listen(channel).await?;

    tokio::spawn(async move {
        loop {
            match listener.recv().await {
                Ok(notification) => callback(notification.payload().to_string()),
                Err(e) => {
                    tracing::error!("LISTEN/NOTIFY error: {e}");
                    // Back off briefly then continue
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    });

    Ok(())
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
