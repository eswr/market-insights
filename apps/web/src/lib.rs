#[cfg(feature = "ssr")]
use worker::*;

use crate::app::*;
#[cfg(feature = "ssr")]
use std::sync::Arc;

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
#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    use tower_service::Service;

    let _ = crate::components::show_data_from_api::GLOBAL_ENV.set(Arc::new(env.clone()));

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
