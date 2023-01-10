pub mod axum {
    use quote::quote;

    use crate::rpc_impl;

    #[test]
    pub fn simple_handler_works() {
        let attr_tokens = quote!(get("/api/simple_handler_works")).into();
        let handler_tokens = quote! {
            pub async fn handler() -> String {
                "hello world".into()
            }
        }
        .into();

        let expected = quote! {
            #[cfg(any(not(target_arch = "wasm32"), not(client)))]
            pub async fn handler() -> String {
                "hello world".into()
            }

            #[cfg(target_arch = "wasm32")]
            pub async fn call_handler() -> ::wrpc::Result<String> {
                ::reqwasm::http::Request::get("/api/simple_handler_works")
                    .send()
                    .await?
                    .text()
                    .await
            }

            #[cfg(not(target_arch = "wasm32"))]
            pub async fn call_handler() -> ::wrpc::Result<String> {
                let client = ::reqwest::Client::new();
                client.get("/api/simple_handler_works")
                    .send()
                    .await?
                    .text()
                    .await
            }
        };

        assert_eq!(
            rpc_impl(attr_tokens, handler_tokens).unwrap().to_string(),
            expected.to_string()
        );
    }

    #[test]
    pub fn string_conversion_works() {
        let attr_tokens = quote!(get("/api/string_coercion_works")).into();
        let handler_tokens = quote! {
            pub async fn handler() -> &'static str {
                "hello world"
            }
        }
        .into();

        let expected = quote! {
            #[cfg(any(not(target_arch = "wasm32"), not(client)))]
            pub async fn handler() -> &'static str {
                "hello world"
            }

            #[cfg(target_arch = "wasm32")]
            pub async fn call_handler() -> ::wrpc::Result<String> {
                ::reqwasm::http::Request::get("/api/string_coercion_works")
                    .send()
                    .await?
                    .text()
                    .await
            }

            #[cfg(not(target_arch = "wasm32"))]
            pub async fn call_handler() -> ::wrpc::Result<String> {
                let client = ::reqwest::Client::new();
                client.get("/api/string_coercion_works")
                    .send()
                    .await?
                    .text()
                    .await
            }
        };

        assert_eq!(
            rpc_impl(attr_tokens, handler_tokens).unwrap().to_string(),
            expected.to_string()
        );
    }

    #[test]
    pub fn json_response_works() {
        let attr_tokens = quote!(get("/api/json_response_works")).into();
        let handler_tokens = quote! {
            pub async fn handler() -> Json<MyType> {
                Json(MyType::new())
            }
        }
        .into();

        let expected = quote! {
            #[cfg(any(not(target_arch = "wasm32"), not(client)))]
            pub async fn handler() -> Json<MyType> {
                Json(MyType::new())
            }

            #[cfg(target_arch = "wasm32")]
            pub async fn call_handler() -> ::wrpc::Result<MyType> {
                ::reqwasm::http::Request::get("/api/json_response_works")
                    .send()
                    .await?
                    .json()
                    .await
            }

            #[cfg(not(target_arch = "wasm32"))]
            pub async fn call_handler() -> ::wrpc::Result<MyType> {
                let client = ::reqwest::Client::new();
                client.get("/api/json_response_works")
                    .send()
                    .await?
                    .json()
                    .await
            }
        };

        assert_eq!(
            rpc_impl(attr_tokens, handler_tokens).unwrap().to_string(),
            expected.to_string()
        );
    }

    #[test]
    pub fn type_override_works() {
        let attr_tokens = quote!(get("/api/type_override_works"), returns(MyType)).into();
        let handler_tokens = quote! {
            pub async fn handler() -> impl IntoResponse {
                (StatusCode::CREATED, Json(MyType::new()))
            }
        }
        .into();

        let expected = quote! {
            #[cfg(any(not(target_arch = "wasm32"), not(client)))]
            pub async fn handler() -> impl IntoResponse {
                (StatusCode::CREATED, Json(MyType::new()))
            }

            #[cfg(target_arch = "wasm32")]
            pub async fn call_handler() -> ::wrpc::Result<MyType> {
                ::reqwasm::http::Request::get("/api/type_override_works")
                    .send()
                    .await?
                    .json()
                    .await
            }

            #[cfg(not(target_arch = "wasm32"))]
            pub async fn call_handler() -> ::wrpc::Result<MyType> {
                let client = ::reqwest::Client::new();
                client.get("/api/type_override_works")
                    .send()
                    .await?
                    .json()
                    .await
            }
        };

        assert_eq!(
            rpc_impl(attr_tokens, handler_tokens).unwrap().to_string(),
            expected.to_string()
        );
    }

    #[test]
    pub fn string_input_works() {
        let attr_tokens = quote!(post("/api/simple_input_works")).into();
        let handler_tokens = quote! {
            pub async fn handler(payload: String) -> String {
                "hello world".into()
            }
        }
        .into();

        let expected = quote! {
            #[cfg(any(not(target_arch = "wasm32"), not(client)))]
            pub async fn handler(payload: String) -> String {
                "hello world".into()
            }

            #[cfg(target_arch = "wasm32")]
            pub async fn call_handler(payload: &str) -> ::wrpc::Result<String> {
                ::reqwasm::http::Request::post("/api/simple_input_works")
                    .body(::std::string::ToString::to_string(payload))
                    .send()
                    .await?
                    .text()
                    .await
            }

            #[cfg(not(target_arch = "wasm32"))]
            pub async fn call_handler(payload: &str) -> ::wrpc::Result<String> {
                let client = ::reqwest::Client::new();
                client.post("/api/simple_input_works")
                    .body(::std::string::ToString::to_string(payload))
                    .send()
                    .await?
                    .text()
                    .await
            }
        };

        assert_eq!(
            rpc_impl(attr_tokens, handler_tokens).unwrap().to_string(),
            expected.to_string()
        );
    }

    #[test]
    pub fn json_input_works() {
        let attr_tokens = quote!(post("/api/json_input_works")).into();
        let handler_tokens = quote! {
            pub async fn handler(payload: Json<MyType>) -> String {
                "hello world".into()
            }
        }
        .into();

        let expected = quote! {
            #[cfg(any(not(target_arch = "wasm32"), not(client)))]
            pub async fn handler(payload: Json<MyType>) -> String {
                "hello world".into()
            }

            #[cfg(target_arch = "wasm32")]
            pub async fn call_handler(payload: &MyType) -> ::wrpc::Result<String> {
                ::reqwasm::http::Request::post("/api/json_input_works")
                    .body(::serde_json::to_string(payload).unwrap())
                    .send()
                    .await?
                    .text()
                    .await
            }

            #[cfg(not(target_arch = "wasm32"))]
            pub async fn call_handler(payload: &MyType) -> ::wrpc::Result<String> {
                let client = ::reqwest::Client::new();
                client.post("/api/json_input_works")
                    .body(::serde_json::to_string(payload).unwrap())
                    .send()
                    .await?
                    .text()
                    .await
            }
        };

        assert_eq!(
            rpc_impl(attr_tokens, handler_tokens).unwrap().to_string(),
            expected.to_string()
        );
    }

    #[test]
    pub fn path_segment_works() {
        let attr_tokens = quote!(get("/api/path_segment_works/:id")).into();
        let handler_tokens = quote! {
            pub async fn handler(Path(id): Path<u32>) -> String {
                "hello world".into()
            }
        }
        .into();

        let expected = quote! {
            #[cfg(any(not(target_arch = "wasm32"), not(client)))]
            pub async fn handler(Path(id): Path<u32>) -> String {
                "hello world".into()
            }

            #[cfg(target_arch = "wasm32")]
            pub async fn call_handler(id: u32) -> ::wrpc::Result<String> {
                ::reqwasm::http::Request::get(&::std::format!("/api/path_segment_works/{}", id))
                    .send()
                    .await?
                    .text()
                    .await
            }

            #[cfg(not(target_arch = "wasm32"))]
            pub async fn call_handler(id: u32) -> ::wrpc::Result<String> {
                let client = ::reqwest::Client::new();
                client.get(&::std::format!("/api/path_segment_works/{}", id))
                    .send()
                    .await?
                    .text()
                    .await
            }
        };

        assert_eq!(
            rpc_impl(attr_tokens, handler_tokens).unwrap().to_string(),
            expected.to_string()
        );
    }

    #[test]
    pub fn multiple_path_segments_work() {
        let attr_tokens = quote!(get("/api/multiple_path_segments_work/team/:team/id/:id")).into();
        let handler_tokens = quote! {
            pub async fn handler(Path((team, id)): Path<(String, u32)>) -> String {
                "hello world".into()
            }
        }
        .into();

        let expected = quote! {
            #[cfg(any(not(target_arch = "wasm32"), not(client)))]
            pub async fn handler(Path((team, id)): Path<(String, u32)>) -> String {
                "hello world".into()
            }

            #[cfg(target_arch = "wasm32")]
            pub async fn call_handler(team: String, id: u32) -> ::wrpc::Result<String> {
                ::reqwasm::http::Request::get(&::std::format!("/api/multiple_path_segments_work/team/{}/id/{}", team, id))
                    .send()
                    .await?
                    .text()
                    .await
            }

            #[cfg(not(target_arch = "wasm32"))]
            pub async fn call_handler(team: String, id: u32) -> ::wrpc::Result<String> {
                let client = ::reqwest::Client::new();
                client.get(&::std::format!("/api/multiple_path_segments_work/team/{}/id/{}", team, id))
                    .send()
                    .await?
                    .text()
                    .await
            }
        };

        assert_eq!(
            rpc_impl(attr_tokens, handler_tokens).unwrap().to_string(),
            expected.to_string()
        );
    }

    #[test]
    pub fn query_works() {
        let attr_tokens = quote!(get("/api/query_works")).into();
        let handler_tokens = quote! {
            pub async fn handler(query: Query<Pagination>) -> String {
                "hello world".into()
            }
        }
        .into();

        let expected = quote! {
            #[cfg(any(not(target_arch = "wasm32"), not(client)))]
            pub async fn handler(query: Query<Pagination>) -> String {
                "hello world".into()
            }

            #[cfg(target_arch = "wasm32")]
            pub async fn call_handler(query: &Pagination) -> ::wrpc::Result<String> {
                ::reqwasm::http::Request::get(&::std::format!("/api/query_works?{}", ::serde_qs::to_string(query).unwrap()))
                    .send()
                    .await?
                    .text()
                    .await
            }

            #[cfg(not(target_arch = "wasm32"))]
            pub async fn call_handler(query: &Pagination) -> ::wrpc::Result<String> {
                let client = ::reqwest::Client::new();
                client.get(&::std::format!("/api/query_works?{}", ::serde_qs::to_string(query).unwrap()))
                    .send()
                    .await?
                    .text()
                    .await
            }
        };

        assert_eq!(
            rpc_impl(attr_tokens, handler_tokens).unwrap().to_string(),
            expected.to_string()
        );
    }

    #[test]
    pub fn query_and_path_segments_work() {
        let attr_tokens = quote!(get("/api/query_and_path_segments_work/:id")).into();
        let handler_tokens = quote! {
            pub async fn handler(id: Path<u32>, query: Query<Pagination>) -> String {
                "hello world".into()
            }
        }
        .into();

        let expected = quote! {
            #[cfg(any(not(target_arch = "wasm32"), not(client)))]
            pub async fn handler(id: Path<u32>, query: Query<Pagination>) -> String {
                "hello world".into()
            }

            #[cfg(target_arch = "wasm32")]
            pub async fn call_handler(id: u32, query: &Pagination) -> ::wrpc::Result<String> {
                ::reqwasm::http::Request::get(&::std::format!("/api/query_and_path_segments_work/{}?{}", id, ::serde_qs::to_string(query).unwrap()))
                    .send()
                    .await?
                    .text()
                    .await
            }

            #[cfg(not(target_arch = "wasm32"))]
            pub async fn call_handler(id: u32, query: &Pagination) -> ::wrpc::Result<String> {
                let client = ::reqwest::Client::new();
                client.get(&::std::format!("/api/query_and_path_segments_work/{}?{}", id, ::serde_qs::to_string(query).unwrap()))
                    .send()
                    .await?
                    .text()
                    .await
            }
        };

        assert_eq!(
            rpc_impl(attr_tokens, handler_tokens).unwrap().to_string(),
            expected.to_string()
        );
    }

    #[test]
    pub fn destructuring_works() {
        let attr_tokens = quote!(post("/api/json_input_works")).into();
        let handler_tokens = quote! {
            pub async fn handler(Json(payload): Json<MyType>) -> String {
                "hello world".into()
            }
        }
        .into();

        let expected = quote! {
            #[cfg(any(not(target_arch = "wasm32"), not(client)))]
            pub async fn handler(Json(payload): Json<MyType>) -> String {
                "hello world".into()
            }

            #[cfg(target_arch = "wasm32")]
            pub async fn call_handler(payload: &MyType) -> ::wrpc::Result<String> {
                ::reqwasm::http::Request::post("/api/json_input_works")
                    .body(::serde_json::to_string(payload).unwrap())
                    .send()
                    .await?
                    .text()
                    .await
            }

            #[cfg(not(target_arch = "wasm32"))]
            pub async fn call_handler(payload: &MyType) -> ::wrpc::Result<String> {
                let client = ::reqwest::Client::new();
                client.post("/api/json_input_works")
                    .body(::serde_json::to_string(payload).unwrap())
                    .send()
                    .await?
                    .text()
                    .await
            }
        };

        assert_eq!(
            rpc_impl(attr_tokens, handler_tokens).unwrap().to_string(),
            expected.to_string()
        );
    }
}
