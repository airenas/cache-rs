use axum::{extract, Json};

use super::data::{ApiResult, LiveResponse};

pub async fn handler() -> ApiResult<extract::Json<LiveResponse>> {
    let res = LiveResponse {
        status: true,
        version: env!("CARGO_APP_VERSION").to_string(),
    };
    Ok(Json(res))
}
