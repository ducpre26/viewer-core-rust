use summer::App;
use summer_job::JobPlugin;
use summer_web::{WebConfigurator, WebPlugin};
use viewer_app::api;

#[tokio::main]
async fn main() {
    let config = std::env::var("VIEWER_CONFIG").unwrap_or_else(|_| "config/app.toml".into());
    App::new()
        .use_config_file(&config)
        .add_plugin(WebPlugin)
        .add_plugin(JobPlugin)
        .add_router(api::router())
        .run()
        .await;
}
