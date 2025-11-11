use leptos::prelude::*;
use leptos::server_fn::ServerFnError;

#[cfg(feature = "ssr")]
use chrono::NaiveDate;
#[cfg(feature = "ssr")]
use leptos::context::use_context;
#[cfg(feature = "ssr")]
use once_cell::sync::OnceCell;
#[cfg(feature = "ssr")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use serde_json::{json, Value};
#[cfg(feature = "ssr")]
use std::sync::Arc;
#[cfg(feature = "ssr")]
use worker::{
    console_log, send::SendFuture, Date, Env, Error as WorkerError, Fetch, Result as WorkerResult,
    Url,
};

#[cfg(feature = "ssr")]
fn now_secs() -> u64 {
    Date::now().as_millis() / 1000
}

#[cfg(feature = "ssr")]
fn worker_env_from_context() -> Result<Arc<Env>, ServerFnError> {
    if let Some(env) = use_context::<Arc<Env>>() {
        return Ok(env);
    }
    if let Some(env) = GLOBAL_ENV.get() {
        return Ok(env.clone());
    }
    Err(ServerFnError::new("Env missing"))
}

#[cfg(feature = "ssr")]
pub static GLOBAL_ENV: OnceCell<Arc<Env>> = OnceCell::new();

#[cfg(feature = "ssr")]
#[derive(Clone)]
struct Bar {
    ts: i64,
    ticker: String,
    tf: String,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

#[cfg(feature = "ssr")]
#[derive(Copy, Clone)]
enum MarketSource {
    // Binance,
    AlphaVantage,
    Finnhub,
}

#[cfg(feature = "ssr")]
struct MarketFeed {
    ticker: &'static str,
    tf: &'static str,
    source: MarketSource,
}

#[cfg(feature = "ssr")]
fn market_feeds() -> Vec<MarketFeed> {
    let mut feeds = Vec::new();
    feeds.push(MarketFeed {
        ticker: "SPY",
        tf: "1d",
        source: MarketSource::AlphaVantage,
    });
    feeds.push(MarketFeed {
        ticker: "SPY",
        tf: "1d",
        source: MarketSource::Finnhub,
    });
    // Binance feeds temporarily disabled (re-enable by uncommenting):
    // feeds.push(MarketFeed {
    //     ticker: "BTCUSDT",
    //     tf: "1h",
    //     source: MarketSource::Binance,
    // });
    // feeds.push(MarketFeed {
    //     ticker: "BTCUSDT",
    //     tf: "4h",
    //     source: MarketSource::Binance,
    // });
    // feeds.push(MarketFeed {
    //     ticker: "BTCUSDT",
    //     tf: "1d",
    //     source: MarketSource::Binance,
    // });
    feeds
}

#[cfg(feature = "ssr")]
fn parse_f64(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(num)) => num.as_f64(),
        Some(Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

#[cfg(feature = "ssr")]
fn parse_i64(value: Option<&Value>) -> Option<i64> {
    match value {
        Some(Value::Number(num)) => num.as_i64(),
        Some(Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

#[cfg(feature = "ssr")]
async fn get_latest_ts(env: &Env, ticker: &str, tf: &str) -> WorkerResult<Option<i64>> {
    let db = env.d1("market_insights")?;
    let stmt = db.prepare("SELECT MAX(ts) FROM ohlcv WHERE ticker = ?1 AND tf = ?2");
    let res = stmt.bind(&[ticker.into(), tf.into()])?.all().await?;
    let rows = res.results::<(Option<i64>,)>()?;
    Ok(rows.into_iter().next().and_then(|(max_ts,)| max_ts))
}

// #[allow(dead_code)]
// #[cfg(feature = "ssr")]
// #[allow(dead_code)]
// async fn fetch_binance_klines(
//     ticker: &str,
//     tf: &str,
//     last_ts: Option<i64>,
// ) -> WorkerResult<Vec<Bar>> {
//     let mut url = format!(
//         "https://api.binance.com/api/v3/klines?symbol={ticker}&interval={tf}&limit=500",
//         ticker = ticker,
//         tf = tf
//     );
//     if let Some(ts) = last_ts {
//         let start = ts.saturating_add(1).saturating_mul(1000);
//         url.push_str(&format!("&startTime={start}"));
//     }

//     let mut resp = Fetch::Url(Url::parse(&url)?).send().await?;
//     if resp.status_code() != 200 {
//         return Err(WorkerError::RustError(format!(
//             "Binance request failed code={}",
//             resp.status_code()
//         )));
//     }

//     let body = resp.text().await?;
//     let rows: Vec<Vec<Value>> = serde_json::from_str(&body)?;
//     let mut bars = Vec::with_capacity(rows.len());

//     for row in rows {
//         let ts_millis = parse_i64(row.get(0));
//         if let Some(ms) = ts_millis {
//             let ts = ms / 1000;
//             if let Some(latest) = last_ts {
//                 if ts <= latest {
//                     continue;
//                 }
//             }

//             if let (Some(open), Some(high), Some(low), Some(close), Some(volume)) = (
//                 parse_f64(row.get(1)),
//                 parse_f64(row.get(2)),
//                 parse_f64(row.get(3)),
//                 parse_f64(row.get(4)),
//                 parse_f64(row.get(5)),
//             ) {
//                 bars.push(Bar {
//                     ts,
//                     ticker: ticker.to_string(),
//                     tf: tf.to_string(),
//                     open,
//                     high,
//                     low,
//                     close,
//                     volume,
//                 });
//             }
//         }
//     }

//     bars.sort_by_key(|bar| bar.ts);
//     Ok(bars)
// }

#[cfg(feature = "ssr")]
async fn fetch_alpha_vantage_daily(
    env: &Env,
    ticker: &str,
    tf: &str,
    last_ts: Option<i64>,
) -> WorkerResult<Vec<Bar>> {
    let secret = match env.secret("ALPHAVANTAGE_KEY") {
        Ok(secret) => secret.to_string(),
        Err(err) => {
            console_log!("AlphaVantage key missing: {err}");
            return Ok(Vec::new());
        }
    };

    let url = format!("https://www.alphavantage.co/query?function=TIME_SERIES_DAILY_ADJUSTED&symbol={ticker}&outputsize=compact&apikey={apikey}", ticker = ticker, apikey = secret);
    let mut resp = Fetch::Url(Url::parse(&url)?).send().await?;
    if resp.status_code() != 200 {
        return Err(WorkerError::RustError(format!(
            "AlphaVantage status = {}",
            resp.status_code()
        )));
    }

    let body = resp.text().await?;
    let payload: Value = serde_json::from_str(&body)?;

    if let Some(err) = payload.get("Error Message").and_then(|v| v.as_str()) {
        return Err(WorkerError::RustError(err.to_string()));
    }
    if let Some(note) = payload.get("Note").and_then(|v| v.as_str()) {
        console_log!("AlphaVantage note: {note}");
    }

    let series = match payload
        .get("Time Series (Daily)")
        .and_then(|v| v.as_object())
    {
        Some(series) => series,
        None => {
            return Err(WorkerError::RustError(
                "AlphaVantage response missing series".into(),
            ))
        }
    };

    let mut bars = Vec::with_capacity(series.len());
    for (date, data) in series {
        let ts = match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            Ok(parsed_date) => match parsed_date.and_hms_opt(0, 0, 0) {
                Some(dt) => dt.and_utc().timestamp(),
                None => continue,
            },
            Err(_) => continue,
        };

        if let Some(latest) = last_ts {
            if ts <= latest {
                continue;
            }
        }

        if let (Some(open), Some(high), Some(low), Some(close), Some(volume)) = (
            parse_f64(data.get("1. open")),
            parse_f64(data.get("2. high")),
            parse_f64(data.get("3. low")),
            parse_f64(data.get("4. close")),
            parse_f64(data.get("6. volume")),
        ) {
            bars.push(Bar {
                ts,
                ticker: ticker.to_string(),
                tf: tf.to_string(),
                open,
                high,
                low,
                close,
                volume,
            });
        }
    }

    bars.sort_by_key(|bar| bar.ts);
    Ok(bars)
}

#[cfg(feature = "ssr")]
fn finnhub_resolution(tf: &str) -> &'static str {
    match tf {
        "1h" => "60",
        "4h" => "240",
        "1d" => "D",
        _ => "D",
    }
}

#[cfg(feature = "ssr")]
fn finnhub_lookback_seconds(tf: &str) -> u64 {
    match tf {
        "1h" => 86400 * 3,
        "4h" => 86400 * 14,
        "1d" => 86400 * 30,
        _ => 86400 * 7,
    }
}

#[cfg(feature = "ssr")]
async fn fetch_finnhub_candles(
    env: &Env,
    ticker: &str,
    tf: &str,
    last_ts: Option<i64>,
) -> WorkerResult<Vec<Bar>> {
    let secret = match env.secret("FINNHUB_KEY") {
        Ok(secret) => secret.to_string(),
        Err(err) => {
            console_log!("Finnhub key missing: {err}");
            return Ok(Vec::new());
        }
    };

    let to = now_secs();
    let from = to.saturating_sub(finnhub_lookback_seconds(tf));
    let resolution = finnhub_resolution(tf);

    let url = format!(
        "https://finnhub.io/api/v1/stock/candle?symbol={ticker}&resolution={resolution}&from={from}&to={to}&token={token}",
        ticker = ticker,
        resolution = resolution,
        from = from,
        to = to,
        token = secret
    );

    let mut resp = Fetch::Url(Url::parse(&url)?).send().await?;
    if resp.status_code() != 200 {
        return Err(WorkerError::RustError(format!(
            "Finnhub status {} for {ticker}",
            resp.status_code()
        )));
    }

    let body = resp.text().await?;
    let payload: Value = serde_json::from_str(&body)?;

    if let Some(status) = payload.get("s").and_then(|v| v.as_str()) {
        if status != "ok" {
            console_log!("Finnhub response status {status}");
            return Ok(Vec::new());
        }
    }

    let ts_values = match payload.get("t").and_then(|v| v.as_array()) {
        Some(values) => values,
        None => return Ok(Vec::new()),
    };
    let open_values = match payload.get("o").and_then(|v| v.as_array()) {
        Some(values) => values,
        None => return Ok(Vec::new()),
    };
    let high_values = match payload.get("h").and_then(|v| v.as_array()) {
        Some(values) => values,
        None => return Ok(Vec::new()),
    };
    let low_values = match payload.get("l").and_then(|v| v.as_array()) {
        Some(values) => values,
        None => return Ok(Vec::new()),
    };
    let close_values = match payload.get("c").and_then(|v| v.as_array()) {
        Some(values) => values,
        None => return Ok(Vec::new()),
    };
    let volume_values = match payload.get("v").and_then(|v| v.as_array()) {
        Some(values) => values,
        None => return Ok(Vec::new()),
    };

    let len = ts_values
        .len()
        .min(open_values.len())
        .min(high_values.len())
        .min(low_values.len())
        .min(close_values.len())
        .min(volume_values.len());

    let mut bars = Vec::with_capacity(len);
    for i in 0..len {
        if let (Some(ts), Some(open), Some(high), Some(low), Some(close), Some(volume)) = (
            parse_i64(Some(&ts_values[i])),
            parse_f64(Some(&open_values[i])),
            parse_f64(Some(&high_values[i])),
            parse_f64(Some(&low_values[i])),
            parse_f64(Some(&close_values[i])),
            parse_f64(Some(&volume_values[i])),
        ) {
            if let Some(latest) = last_ts {
                if ts <= latest {
                    continue;
                }
            }

            bars.push(Bar {
                ts,
                ticker: ticker.to_string(),
                tf: tf.to_string(),
                open,
                high,
                low,
                close,
                volume,
            });
        }
    }

    bars.sort_by_key(|bar| bar.ts);
    Ok(bars)
}

#[cfg(feature = "ssr")]
async fn persist_bars(env: &Env, bars: &[Bar]) -> WorkerResult<()> {
    let db = env.d1("market_insights")?;
    for bar in bars {
        let stmt = db.prepare(
            "INSERT OR REPLACE INTO ohlcv (ts, ticker, tf, open, high, low, close, volume) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        );
        stmt.bind(&[
            bar.ts.into(),
            bar.ticker.clone().into(),
            bar.tf.clone().into(),
            bar.open.into(),
            bar.high.into(),
            bar.low.into(),
            bar.close.into(),
            bar.volume.into(),
        ])?
        .run()
        .await?;
    }
    Ok(())
}

#[cfg(feature = "ssr")]
async fn build_market_data(env: &Env) -> WorkerResult<()> {
    for feed in market_feeds() {
        let last = get_latest_ts(env, feed.ticker, feed.tf).await?;
        let bars = match feed.source {
            // MarketSource::Binance => fetch_binance_klines(feed.ticker, feed.tf, last).await?,
            MarketSource::AlphaVantage => {
                fetch_alpha_vantage_daily(env, feed.ticker, feed.tf, last).await?
            }
            MarketSource::Finnhub => fetch_finnhub_candles(env, feed.ticker, feed.tf, last).await?,
        };

        if bars.is_empty() {
            continue;
        }

        persist_bars(env, &bars).await?;
        console_log!("stored {} bars for {}/{}", bars.len(), feed.ticker, feed.tf);
    }
    Ok(())
}

#[cfg(feature = "ssr")]
pub(crate) async fn set_cooldown_internal(env: &Env, key: &str, minutes: u64) -> WorkerResult<()> {
    let kv = env.kv("STATE")?;
    let until = now_secs().saturating_add(minutes.saturating_mul(60));
    let cooldown_key = format!("cooldown:{key}");
    kv.put(cooldown_key.as_str(), until.to_string())?
        .execute()
        .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub(crate) async fn is_cooling_internal(env: &Env, key: &str) -> WorkerResult<bool> {
    let kv = env.kv("STATE")?;
    let cooldown_key = format!("cooldown:{key}");
    if let Some(txt) = kv.get(cooldown_key.as_str()).text().await? {
        let until = txt.parse::<u64>().unwrap_or(0);
        return Ok(now_secs() < until);
    }
    Ok(false)
}

#[cfg(feature = "ssr")]
pub(crate) async fn insert_alert_internal(
    env: &Env,
    ticker: &str,
    rule: &str,
    severity: &str,
    details_json: &str,
) -> WorkerResult<()> {
    let db = env.d1("market_insights")?;
    let ts = now_secs() as i64;
    let stmt = db.prepare(
        "INSERT INTO alerts (ts, ticker, rule, severity, details) VALUES (?1, ?2, ?3, ?4, ?5)",
    );
    let ts_js = (ts as f64).into();
    stmt.bind(&[
        ts_js,
        ticker.into(),
        rule.into(),
        severity.into(),
        details_json.into(),
    ])?
    .run()
    .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct AlertRow {
    ts: i64,
    ticker: String,
    rule: String,
    severity: String,
    details: String,
}

#[cfg(feature = "ssr")]
pub(crate) async fn list_alerts_internal(env: &Env, limit: i64) -> WorkerResult<Vec<AlertRow>> {
    let db = env.d1("market_insights")?;
    let stmt = db.prepare(
        "SELECT ts, ticker, rule, severity, details FROM alerts ORDER BY ts DESC LIMIT ?1",
    );
    let limit_js = (limit as f64).into();
    let res = stmt.bind(&[limit_js])?.all().await?;
    Ok(res.results::<AlertRow>()?)
}

#[server(SayHello)]
pub async fn say_hello(num: i32) -> Result<String, ServerFnError> {
    Ok(format!("Hello from the API!!! I got {num}"))
}

#[server(SetCooldown)]
pub async fn set_cooldown(key: String, minutes: u64) -> Result<String, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        return SendFuture::new(async move {
            let env = worker_env_from_context()?;
            set_cooldown_internal(&env, &key, minutes)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            Ok(format!("cooldown set to {minutes}m"))
        })
        .await;
    }

    #[cfg(not(feature = "ssr"))]
    {
        let _ = key;
        let _ = minutes;
        Err(ServerFnError::new(
            "set_cooldown is only available on the SSR bundle",
        ))
    }
}

#[server(IsCooling)]
pub async fn is_cooling(key: String) -> Result<bool, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        return SendFuture::new(async move {
            let env = worker_env_from_context()?;
            is_cooling_internal(&env, &key)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))
        })
        .await;
    }

    #[cfg(not(feature = "ssr"))]
    {
        let _ = key;
        Err(ServerFnError::new(
            "is_cooling is only available on the SSR bundle",
        ))
    }
}

#[server(InsertAlert)]
pub async fn insert_alert(
    ticker: String,
    rule: String,
    severity: String,
    details_json: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        return SendFuture::new(async move {
            let env = worker_env_from_context()?;
            insert_alert_internal(&env, &ticker, &rule, &severity, &details_json)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))
        })
        .await;
    }

    #[cfg(not(feature = "ssr"))]
    {
        let _ = ticker;
        let _ = rule;
        let _ = severity;
        let _ = details_json;
        Err(ServerFnError::new(
            "insert_alert is only available on the SSR bundle",
        ))
    }
}

#[server(ListAlerts)]
pub async fn list_alerts(
    limit: i64,
) -> Result<Vec<(i64, String, String, String, String)>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        return SendFuture::new(async move {
            let env = worker_env_from_context()?;
            list_alerts_internal(&env, limit)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))
                .map(|rows| {
                    rows.into_iter()
                        .map(|row| (row.ts, row.ticker, row.rule, row.severity, row.details))
                        .collect()
                })
        })
        .await;
    }

    #[cfg(not(feature = "ssr"))]
    {
        let _ = limit;
        Ok(Vec::new())
    }
}

#[cfg(feature = "ssr")]
pub async fn run_engine_for_active_symbols(env: Env) -> WorkerResult<()> {
    build_market_data(&env).await?;

    if !is_cooling_internal(&env, "BTCUSDT:donchian20").await? {
        let details = json!({
            "tf": "1h",
            "price": 100.0,
            "confidence": 0.62
        })
        .to_string();

        insert_alert_internal(
            &env,
            "BTCUSDT",
            "donchian20_breakout",
            "actionable",
            &details,
        )
        .await?;
        set_cooldown_internal(&env, "BTCUSDT:donchian20", 30).await?;
    }
    Ok(())
}

#[component]
pub fn ShowDataFromApi() -> impl IntoView {
    let value = RwSignal::new("".to_string());
    let counter = RwSignal::new(0);

    #[cfg(feature = "hydrate")]
    let alerts_section = {
        let alerts = Resource::new(
            || (),
            |_| async move { list_alerts(20).await.unwrap_or_default() },
        );

        let insert_alert_click = {
            let alerts = alerts.clone();
            move |_| {
                leptos::task::spawn_local(async move {
                    let _ = insert_alert(
                        "TEST".into(),
                        "rule_x".into(),
                        "info".into(),
                        "{\"demo\":true}".into(),
                    )
                    .await;
                    alerts.refetch();
                });
            }
        };

        view! {
            <div class="space-y-2">
                <button on:click=insert_alert_click>"Insert test alert"</button>
                <Suspense fallback=|| view!{ <p>"Loading alerts…"</p> }>
                    {move || alerts.get().map(|rows| view!{
                        <ul>
                            {rows.iter().map(|(ts,ticker,rule,sev,det)| view!{
                                <li>{format!("{ts} | {ticker} | {rule} | {sev} | {det}")}</li>
                            }).collect_view()}
                        </ul>
                    })}
                </Suspense>
            </div>
        }
    };

    #[cfg(not(feature = "hydrate"))]
    let alerts_section = view! {
        <p class="text-sm text-slate-500">"Alerts will load after hydration."</p>
    };

    let on_click = move |_| {
        leptos::task::spawn_local(async move {
            let api_said = say_hello(counter.get()).await.unwrap();
            value.set(api_said);
            counter.update(|v| *v += 1);
        });
    };

    view! {
        <div class="p-4 space-y-4">
            <div class="space-y-2">
                <button on:click=on_click>"What does the API say?"</button>
                <p>{move || value.get()}</p>
            </div>
            {alerts_section}
        </div>
    }
}
