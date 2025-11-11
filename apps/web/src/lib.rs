#[cfg(feature = "ssr")]
use worker::*;

use crate::app::*;
#[cfg(feature = "ssr")]
use crate::components::show_data_from_api::{
    insert_alert_internal, is_cooling_internal, list_alerts_internal, set_cooldown_internal,
};
#[cfg(feature = "ssr")]
use std::sync::Arc;

#[cfg(feature = "ssr")]
use axum::body::Body;
#[cfg(feature = "ssr")]
use axum::http::{header::CONTENT_TYPE, Method, Response, StatusCode};
#[cfg(feature = "ssr")]
use futures_util::stream::TryStreamExt;
#[cfg(feature = "ssr")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use serde_json::json;

pub mod app;
mod components;

#[cfg(feature = "ssr")]
pub fn register_server_functions() {
    use leptos::server_fn::axum::register_explicit;

    // Add all of your server functions here
    register_explicit::<components::show_data_from_api::SayHello>();
    register_explicit::<components::show_data_from_api::SetCooldown>();
    register_explicit::<components::show_data_from_api::IsCooling>();
    register_explicit::<components::show_data_from_api::InsertAlert>();
    register_explicit::<components::show_data_from_api::ListAlerts>();
}

#[cfg(feature = "ssr")]
async fn router(env: Env) -> axum::Router {
    use axum::{Extension, Router};
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(App);
    register_server_functions();

    // build our application with a route
    Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .with_state(leptos_options)
        .layer(Extension(Arc::new(env))) // <- Allow leptos server functions to access Worker stuff
}

#[cfg(feature = "ssr")]
#[derive(Deserialize)]
struct SetCooldownJson {
    key: String,
    minutes: u64,
}

#[cfg(feature = "ssr")]
#[derive(Deserialize)]
struct KeyPayload {
    key: String,
}

#[cfg(feature = "ssr")]
#[derive(Deserialize)]
struct InsertAlertJson {
    ticker: String,
    rule: String,
    severity: String,
    details_json: String,
}

#[cfg(feature = "ssr")]
#[derive(Deserialize)]
struct ListAlertsJson {
    limit: i64,
}

#[cfg(feature = "ssr")]
fn json_response<T: Serialize>(status: StatusCode, payload: &T) -> Response<Body> {
    match serde_json::to_string(payload) {
        Ok(body) => Response::builder()
            .status(status)
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("serialization error"))
            .unwrap(),
    }
}

#[cfg(feature = "ssr")]
fn bad_request(message: impl Into<String>) -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("content-type", "text/plain")
        .body(Body::from(message.into()))
        .unwrap()
}

#[cfg(feature = "ssr")]
fn internal_error(message: impl Into<String>) -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("content-type", "text/plain")
        .body(Body::from(message.into()))
        .unwrap()
}

#[cfg(feature = "ssr")]
async fn handle_set_cooldown_json(bytes: &[u8], env: &Env) -> Response<Body> {
    let payload = match serde_json::from_slice::<SetCooldownJson>(bytes) {
        Ok(payload) => payload,
        Err(err) => return bad_request(err.to_string()),
    };
    match set_cooldown_internal(env, &payload.key, payload.minutes).await {
        Ok(_) => json_response(
            StatusCode::OK,
            &json!({"status":"cooldown set","minutes":payload.minutes}),
        ),
        Err(err) => internal_error(err.to_string()),
    }
}

#[cfg(feature = "ssr")]
async fn handle_is_cooling_json(bytes: &[u8], env: &Env) -> Response<Body> {
    let payload = match serde_json::from_slice::<KeyPayload>(bytes) {
        Ok(payload) => payload,
        Err(err) => return bad_request(err.to_string()),
    };
    match is_cooling_internal(env, &payload.key).await {
        Ok(result) => json_response(StatusCode::OK, &json!({ "cooling": result })),
        Err(err) => internal_error(err.to_string()),
    }
}

#[cfg(feature = "ssr")]
async fn handle_insert_alert_json(bytes: &[u8], env: &Env) -> Response<Body> {
    let payload = match serde_json::from_slice::<InsertAlertJson>(bytes) {
        Ok(payload) => payload,
        Err(err) => return bad_request(err.to_string()),
    };
    match insert_alert_internal(
        env,
        &payload.ticker,
        &payload.rule,
        &payload.severity,
        &payload.details_json,
    )
    .await
    {
        Ok(_) => json_response(StatusCode::OK, &json!({"status":"alert created"})),
        Err(err) => internal_error(err.to_string()),
    }
}

#[cfg(feature = "ssr")]
async fn handle_list_alerts_json(bytes: &[u8], env: &Env) -> Response<Body> {
    let payload = match serde_json::from_slice::<ListAlertsJson>(bytes) {
        Ok(payload) => payload,
        Err(err) => return bad_request(err.to_string()),
    };
    match list_alerts_internal(env, payload.limit).await {
        Ok(rows) => json_response(StatusCode::OK, &rows),
        Err(err) => internal_error(err.to_string()),
    }
}

#[cfg(feature = "ssr")]
async fn handle_json_server(
    req: &mut HttpRequest,
    env: &Env,
) -> std::result::Result<Option<Response<Body>>, worker::Error> {
    if req.method() != &Method::POST {
        return Ok(None);
    }
    let path = req.uri().path().to_owned();
    let base = path.trim_end_matches(|c: char| c.is_ascii_digit());
    if !base.starts_with("/api/") {
        return Ok(None);
    }
    let content_type = req
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|val| val.to_str().ok())
        .unwrap_or("");
    if !content_type.contains("application/json") {
        return Ok(None);
    }
    let bytes_vec = req
        .body_mut()
        .try_fold(Vec::new(), |mut acc, chunk| async move {
            acc.extend_from_slice(&chunk);
            Ok(acc)
        })
        .await
        .map_err(|err| worker::Error::RustError(err.to_string()))?;
    let bytes = bytes_vec.as_slice();
    if bytes.is_empty() {
        return Ok(None);
    }

    let response = match base {
        "/api/set_cooldown" => handle_set_cooldown_json(&bytes, env).await,
        "/api/is_cooling" => handle_is_cooling_json(&bytes, env).await,
        "/api/insert_alert" => handle_insert_alert_json(&bytes, env).await,
        "/api/list_alerts" => handle_list_alerts_json(&bytes, env).await,
        _ => return Ok(None),
    };
    Ok(Some(response))
}

#[cfg(feature = "ssr")]
#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    use tower_service::Service;

    let _ = crate::components::show_data_from_api::GLOBAL_ENV.set(Arc::new(env.clone()));

    let mut req = req;
    if let Some(resp) = handle_json_server(&mut req, &env).await? {
        return Ok(resp);
    }

    Ok(router(env).await.call(req).await?)
}

#[cfg(feature = "ssr")]
#[event(scheduled)]
pub async fn scheduled(_evt: worker::ScheduledEvent, env: Env, _ctx: worker::ScheduleContext) {
    let _ = crate::components::show_data_from_api::GLOBAL_ENV.set(Arc::new(env.clone()));
    if let Err(err) = components::show_data_from_api::run_engine_for_active_symbols(env).await {
        console_log!("scheduled engine error: {}", err);
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    leptos::mount::hydrate_body(App);
}
