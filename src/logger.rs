use tokio_postgres::{NoTls, Error};
use dotenvy::dotenv;
use std::env;

pub async fn add_log(message: &str) -> Result<(), Box<dyn std::error::Error>> {
    dotenv()?;
    let conn_string = env::var("DATABASE_URL")?;

    // Connect to database without SSL for simplicity
    let (client, connection) = tokio_postgres::connect(&conn_string, NoTls).await?;
    println!("Database connection established for logging");

    // Spawn connection handler
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    // Insert a new log entry into the table
    let inserted_rows = client.execute(
        "INSERT INTO logs (message, created_at) VALUES ($1, CURRENT_TIMESTAMP)",
        &[&message],
    ).await?;

    if inserted_rows > 0 {
        println!("Logged message: '{}'", message);
    } else {
        println!("Failed to log message: '{}'", message);
    }

    Ok(())
}