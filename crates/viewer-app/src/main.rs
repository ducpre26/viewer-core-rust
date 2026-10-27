use summer::App;
use summer_job::JobPlugin;
use summer_web::{WebConfigurator, WebPlugin};
use viewer_app::{api, auth::Auth, config::AppConfig, dicom::Jvm, persistence::Persistence};

#[tokio::main]
async fn main() -> std::process::ExitCode {
    // Preserve Summer's default app.toml + profile + interpolation + logging.
    let mut app = App::new();
    let config = match AppConfig::from_registry(&app) {
        Ok(config) => config,
        Err(message) => {
            eprintln!("Application configuration error: {message}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let jvm_config = match config.jni.validate() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("JVM configuration error: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let database = match Persistence::connect(&config.database).await {
        Ok(database) => database,
        Err(message) => {
            eprintln!("Database startup error: {message}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let jvm = match jvm_config {
        Some(config) => match Jvm::start(config).await {
            Ok(jvm) => Some(jvm),
            Err(error) => {
                eprintln!("JVM startup error: {error}");
                let _ = database.close().await;
                // A timed-out JNI worker must never keep this process running.
                std::process::exit(1);
            }
        },
        None => None,
    };
    let auth = Auth::new(database.clone(), config.auth);
    app.add_plugin(WebPlugin)
        .add_plugin(JobPlugin)
        .add_router(api::router(
            database.clone(),
            jvm.clone(),
            auth,
            config.session,
        ));
    let cleanup = tokio::spawn(api::run_cleanup(database.clone()));
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
    cleanup.abort();
    let _ = cleanup.await;
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
