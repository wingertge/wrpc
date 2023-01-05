//! Make your server side `axum` APIs consumable from WASM.
//! This crate introduces a simple attribute macro that transforms your API
//! handler's signature into a fully typed client side request function that
//! calls the API endpoint. Inspired by tRPC.
//!
//! # Quick Usage
//!
//! ```
//! # use axum::{Json, extract::Path};
//! # use wrpc_macro::rpc;
//! # struct User;
//!
//! #[rpc(get("/api/user/:id"))]
//! pub async fn get_user(Path(id): Path<u32>) -> Json<User> {
//!     // Do things here
//!     Json(User)
//! }
//! ```
//!
//! This will gate the handler to only exist on non-WASM targets and create a
//! WASM side function somewhat like this:
//!
//! ```
//! # #[derive(serde::Serialize, serde::Deserialize)]
//! # struct User;
//! pub async fn get_user(id: u32) -> Result<User, reqwasm::Error> {
//!     reqwasm::http::Request::get(&format!("/api/user/{id}"))
//!         .send()
//!         .await?
//!         .json()
//!         .await
//! }
//! ```
//!
//! # Configuration
//!
//! * `get(path)` - Specifiy this handler's path relative to the root of your
//! API. Extracted path segments are prefixed with `:`, i.e. `:id`.
//! * `returns(Type)` - Specify an overriding return type for your client side
//! function. This must be either `String` or a deserializable type. It's mostly
//! useful for handlers that return status codes or have an otherwise more
//! complex return type.
//!
//! # Requirements
//!
//! * Path inputs with multiple segments must be destructured. This is because
//! the macro separates these parameters into separate arguments to the client
//! side function and needs their names.
//! * Text body inputs must be `String`s
//! * All request-derived inputs must be `Json`, `Query`, `Path` or `String`.
//! Any other arguments are assumed to be state derived and skipped.
//! * The return type must be `Json` or `String`/`&str`. `&str` will be turned
//! into `String` on the client side.
//! * The full path to the API handler must be specified. wrpc currently can't
//! have access to your Router, so paths are unknown to the macro.
//!
//! # Kitchen Sink Example
//!
//! ```
//! # use axum::{Json, extract::{Query, Path}, http::StatusCode, response::IntoResponse};
//! use wrpc::rpc;
//! # #[derive(serde::Serialize, serde::Deserialize)]
//! # struct User;
//! struct UpdateQuery {
//!    force: bool
//! }
//!
//! #[rpc(post("/api/user/:id/update"), returns(User))]
//! pub async fn update_user(
//!     Path(id): Path<u32>,
//!     Query(settings): Query<UpdateQuery>,
//!     Json(new_user): Json<User>
//! ) -> impl IntoResponse {
//!     // Do actual things
//!     (StatusCode::OK, Json(User))
//! }
//! ```
//!

pub use wrpc_macro::rpc;

pub type Result<T> = std::result::Result<T, ::reqwasm::Error>;
