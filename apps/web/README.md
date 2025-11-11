# Leptos

This template demonstrates the use of the [Leptos](https://leptos.dev/)
framework on Workers, including support for server side rendering and
server functions.

Frontend assets are built using `cargo leptos` by compiling the crate
with the `hydrate` feature. The backend module uses `workers-rs` and
is built by compiling the crate using `worker-build` with the `ssr`
feature. This is done automatically when using `wrangler` with
the custom build command specified in `wrangler.toml`.

Frontend assets are served using Workers Assets. Any request which
matches an asset path will be served directly and not invoke the
Worker. Requests which do not match an asset path will invoke the
Worker. This includes requests to `index.html` (which will be
server-side rendered) and any server function (API) routes.

# Setup

[Cargo Leptos](https://github.com/leptos-rs/cargo-leptos) is required
to build the project.

```
cargo install --locked cargo-leptos
```

# Run Locally

```
npx wrangler dev
```

# Deploy

```
npx wrangler deploy
```

# TODO

# 1) Lock the API surface (small edits)

**b) Validate inputs + cap limits:**

```rust
let limit = limit.clamp(1, 200);
```

Add basic validation for `key`, `ticker`, etc. Return friendly `ServerFnError`.

# 2) Pagination (so UI won’t stall later)

Update SQL + fn signature:

```sql
-- “keyset” pagination by ts
SELECT ts,ticker,rule,severity,details
FROM alerts
WHERE (?1 IS NULL OR ts < ?1)
ORDER BY ts DESC
LIMIT ?2;
```

```rust
#[server(ListAlerts, GetJson)]
pub async fn list_alerts(before_ts: Option<i64>, page_size: i64)
 -> Result<(Vec<AlertRow>, Option<i64>), ServerFnError> {
   let page_size = page_size.clamp(1, 100);
   // ... bind [before_ts.into(), (page_size as f64).into()]
   // next_cursor = rows.last().map(|r| r.ts)
}
```

Return `(rows, next_cursor)`; in the UI, pass `before_ts=next_cursor` to fetch older pages.

# 3) Wire the UI (quick wins)

* Render alerts with a “Load more” button using the cursor above.
* After `InsertAlert`, call `alerts.refetch()` (you already do).
* Show “Cooling…” banner by calling `is_cooling("BTCUSDT:donchian20")` on mount.

# 4) Scheduled job safety

Your cron calls `run_engine_for_active_symbols`. Guard it with a short KV lock to prevent overlap:

```rust
// at run start
if is_cooling_internal(&env, "engine:lock").await? { return Ok(()); }
set_cooldown_internal(&env, "engine:lock", 2).await?; // 2 minutes

// at run end (best-effort unlock: set 0)
set_cooldown_internal(&env, "engine:lock", 0).await?;
```

For **local test** (since cron isn’t triggered), add a dev-only route:

```rust
// in fetch() before router:
if cfg!(debug_assertions) && req.method()==Method::POST && req.uri().path()=="/__run_engine" {
  components::show_data_from_api::run_engine_for_active_symbols(env.clone()).await.ok();
  return Ok(Response::from_body("ok"));
}
```

Then: `curl -X POST http://localhost:8787/__run_engine`.

# 5) Observability

* Run `wrangler tail` in another terminal for live logs.
* Wrap DB/KV errors with context (e.g., `format!("list_alerts failed: {e}")`).
* (Optional) Add Sentry (Workers SDK) later; start with `console_log!`.

# 7) Hardening (fast passes)

* Rate-limit mutation fns with a simple KV sliding window per IP (e.g., 30/min).
* Add minimal auth (e.g., a shared HMAC header) if these will be public.
* Ensure `details_json` is valid JSON; reject oversized payloads (>8–16KB).

# 8) Tests you can run right now

* `InsertAlert` then `ListAlerts` shows the new row.
* `SetCooldown` → `IsCooling == true`, then again after 5m it’s `false`.
* Call `/__run_engine` once: an alert appears (and engine cooldown is set).
* Pagination: call `ListAlerts(page_size=5)` → use `next_cursor` → fetch next page.
