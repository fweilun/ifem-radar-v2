use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 2 {
        eprintln!("Usage: create_account <account> <password> [full_name] [role]");
        std::process::exit(2);
    }

    let account = args.remove(0);
    let password = args.remove(0);
    let full_name = args.get(0).cloned();
    let role = args.get(1).cloned();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("hash password failed: {e}"))?
        .to_string();

    sqlx::query(
        r#"
        INSERT INTO account_info (id, account, password_hash, full_name, role, is_active)
        VALUES ($1, $2, $3, $4, $5, TRUE)
        ON CONFLICT (account) DO UPDATE
        SET password_hash = EXCLUDED.password_hash,
            full_name = COALESCE(EXCLUDED.full_name, account_info.full_name),
            role = COALESCE(EXCLUDED.role, account_info.role),
            is_active = TRUE,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&account)
    .bind(password_hash)
    .bind(full_name)
    .bind(role)
    .execute(&pool)
    .await?;

    println!("Account upserted: {account}");
    Ok(())
}
