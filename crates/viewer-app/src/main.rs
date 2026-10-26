use summer::App;
use summer_job::JobPlugin;
use summer_web::{WebConfigurator, WebPlugin};
use viewer_app::{
    api,
    auth::{Auth, AuthConfig},
    dicom::{Jvm, JvmConfig},
    persistence::Persistence,
};

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let auth_config = match AuthConfig::from_env() {
        Ok(config) => config,
        Err(message) => {
            eprintln!("Authentication configuration error: {message}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let database = match Persistence::connect_from_env().await {
        Ok(database) => database,
        Err(message) => {
            eprintln!("Database startup error: {message}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let jvm = match JvmConfig::from_env() {
        Ok(Some(config)) => match Jvm::start(config).await {
            Ok(jvm) => Some(jvm),
            Err(error) => {
                eprintln!("JVM startup error: {error}");
                let _ = database.close().await;
                // A timed-out JNI worker must never keep this process running.
                std::process::exit(1);
            }
        },
        Ok(None) => None,
        Err(error) => {
            eprintln!("JVM configuration error: {error}");
            let _ = database.close().await;
            return std::process::ExitCode::FAILURE;
        }
    };
    let config = std::env::var("VIEWER_CONFIG").unwrap_or_else(|_| "config/app.toml".into());
    let mut app = App::new();
    let auth = Auth::new(database.clone(), auth_config);
    app.use_config_file(&config)
        .add_plugin(WebPlugin)
        .add_plugin(JobPlugin)
        .add_router(api::router(database.clone(), jvm.clone(), auth));
    tokio::select! {
        _ = app.run() => {},
        _ = async {
            match &jvm {
                Some(jvm) => jvm.wait_failed().await,
                None => std::future::pending::<()>().await,
            }
        } => {
            eprintln!("JVM fatal error: worker unavailable; restarting the process is required");
            std::process::exit(1);
        }
    }
    if let Some(jvm) = jvm
        && let Err(error) = jvm.shutdown().await
    {
        eprintln!("JVM shutdown error: {error}");
        std::process::exit(1);
    }
    if database.close().await.is_err() {
        eprintln!("Database shutdown error: failed to close PostgreSQL pool");
        return std::process::ExitCode::FAILURE;
    }
    eprintln!("PostgreSQL pool closed");
    std::process::ExitCode::SUCCESS
}
