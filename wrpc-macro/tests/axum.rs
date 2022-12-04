use axum::{extract::Path, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use wrpc_macro::rpc;

#[derive(Serialize, Deserialize)]
pub struct User {
    id: u32,
    name: String,
    team: String,
}

#[rpc(get("/api/handler/:team/:id"))]
pub async fn handler(Path((team, id)): Path<(String, u32)>) -> Json<User> {
    Json(User {
        id,
        team,
        name: "hello".to_string(),
    })
}

#[allow(unused)]
#[cfg(not(target_arch = "wasm32"))]
pub fn router() -> Router {
    Router::new().route("/api/handler/:team/:id", get(handler))
}
