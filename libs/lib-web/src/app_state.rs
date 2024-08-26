use crate::Result;

use axum::extract::FromRef;
use leptos::LeptosOptions;
use lib_core::model::ModelManager;

#[derive(FromRef, Debug, Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub mm: ModelManager,
}

impl AppState {
    pub async fn new(leptos_options: LeptosOptions) -> Result<Self> {
        let mm = ModelManager::new().await?;

        Ok(Self { leptos_options, mm })
    }
}
