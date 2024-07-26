use super::{user::create_user_table, ModelManager};
use crate::Result;
use axum::extract::FromRef;
use leptos::LeptosOptions;

#[derive(FromRef, Debug, Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub mm: ModelManager,
}

impl AppState {
    pub async fn new(leptos_options: LeptosOptions) -> Result<Self> {
        let mm = ModelManager::new().await?;
        create_user_table(mm.clone()).await?;

        Ok(Self { leptos_options, mm })
    }
}
