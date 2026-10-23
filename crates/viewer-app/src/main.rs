use summer::App;
use summer_job::JobPlugin;
use summer_web::{WebConfigurator, WebPlugin};
use viewer_app::{api, persistence::Persistence};

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let database = match Persistence::connect_from_env().await {
        Ok(database) => database,
        Err(message) => {
            eprintln!("Database startup error: {message}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let config = std::env::var("VIEWER_CONFIG").unwrap_or_else(|_| "config/app.toml".into());
    App::new()
        .use_config_file(&config)
        .add_plugin(WebPlugin)
        .add_plugin(JobPlugin)
        .add_router(api::router(database.clone()))
        .run()
        .await;
    if database.close().await.is_err() {
        eprintln!("Database shutdown error: failed to close PostgreSQL pool");
        return std::process::ExitCode::FAILURE;
    }
    eprintln!("PostgreSQL pool closed");
    std::process::ExitCode::SUCCESS
}
