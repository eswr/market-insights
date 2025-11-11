let’s turn Trading into a concrete “signals → rules → alerts” spec into math you can code, business-logic you can toggle per market, and a clean notification model.

# App at a glance (what to build)

* **Data:** OHLCV (1m/5m/1h/d), fundamentals (if equities), options chain (if options), news/econ calendar (optional).
* **Engine:** rolling feature calculator → rules engine (boolean triggers) → risk & throttle layer → notifier.
* **Backtest/validate:** walk-forward splits, slippage/fees, Sharpe/Sortino, max DD, profit factor, hit-rate.
* **Notify:** severity (info/warn/critical), cooldowns, channels (push/email/Slack), user TZ=Asia/Kolkata.

---

# Mathematical primitives (ready to code)

Let ( P_t ) be close price at t; ( H_t,L_t,O_t,V_t ) are high/low/open/volume.

* **SMA**: ( \text{SMA}*n(t)=\frac{1}{n}\sum*{i=0}^{n-1}P_{t-i} )
* **EMA**: ( \text{EMA}_n(t)=\alpha P_t+(1-\alpha)\text{EMA}_n(t-1),\ \alpha=\frac{2}{n+1} )
* **Rolling mean/σ**: ( \mu_n,\ \sigma_n ) over last n bars
* **Z-score**: ( z_n(t)=\frac{P_t-\mu_n}{\sigma_n} )
* **True Range**: ( TR_t=\max(H_t-L_t, |H_t-P_{t-1}|, |L_t-P_{t-1}|) )
* **ATR**: ( \text{ATR}_n=\text{EMA of } TR )
* **RSI (Wilder)**:

  * ( U_t=\max(P_t-P_{t-1},0) ), ( D_t=\max(P_{t-1}-P_t,0) )
  * ( RS=\frac{\text{EMA}(U)}{\text{EMA}(D)} ), ( RSI=100-\frac{100}{1+RS} )
* **MACD**: ( \text{EMA}*{12}-\text{EMA}*{26} ), **Signal** (=\text{EMA}_9(\text{MACD}) )
* **Bollinger Bands**: ( \mu_n \pm k\sigma_n ) (typ. (n=20,k=2))
* **Donchian** ( n ): upper (=\max(H_{t-n+1..t})), lower (=\min(L_{t-n+1..t}))
* **VWAP (session)**: ( \frac{\sum P_i V_i}{\sum V_i} )
* **OBV**: ( OBV_t=OBV_{t-1} + \text{sign}(P_t-P_{t-1})\cdot V_t )
* **Beta to benchmark B**: ( \beta=\frac{\text{Cov}(r_S,r_B)}{\text{Var}(r_B)} )
* **Realized vol (daily)**: ( \sigma_{ann}=\text{stdev}(r_{1d})\sqrt{252} )
* **Kelly (fractional)** for win-prob (p), payoff (b): ( f^*=\frac{bp-(1-p)}{b} ) (use small fraction)
* **Vol-target position**: ( w=\min!\Big(1,\frac{\sigma_{target}}{\hat{\sigma}}\Big) )

---

# Trading insights as business rules (switchable per instrument)

## 1) Trend & Momentum

1. **MA Trend Alignment**

   * **Rule:** price > SMA50 > SMA200 → **Uptrend**; inverse for downtrend.
   * **Alert:** “Trend flip” when SMA50 crosses SMA200 (golden/death cross).
2. **EMA Slope**

   * **Rule:** d(EMA50)/dt > 0 for N bars → sustained momentum.
3. **MACD Momentum Thrust**

   * **Rule:** MACD crosses above Signal and MACD>0; opposite for bearish.
4. **RSI Regime**

   * **Rule:** RSI>55 in uptrends (buy-the-dip zone 40–50), RSI<45 in downtrends (sell-the-rip 50–60).

## 2) Mean Reversion

5. **Bollinger Pinch Revert**

   * **Rule:** Close < lower band & z_20 < −2 while no downtrend (SMA50≈flat) → bounce alert.
6. **Z-Score Revert**

   * **Rule:** |z_lookback|>2 and returns negatively autocorrelated → fade toward μ.
7. **RSI Extremes**

   * **Rule:** RSI<30 then crosses back >30 → MR long; RSI>70 then <70 → MR short.

## 3) Breakout & Expansion

8. **Donchian Breakout**

   * **Rule:** Close breaks 20-day high with rising ATR → breakout long; mirror for short.
9. **NR7/Inside Bar Expansion**

   * **Rule:** Narrowest range in 7 bars (NR7) or inside bar cluster; alert on range break.
10. **Bollinger Squeeze**

* **Rule:** Bandwidth (σ/μ) at 1-yr 20th pctile + volume uptick → expansion watch.

11. **Opening Range Breakout (intraday)**

* **Rule:** Break of first 30-60m high/low with >1.5× volume.

## 4) Volatility & Risk

12. **ATR Spike**

* **Rule:** ATR_n / median(ATR_n, lookback) > 1.5 → regime shift.

13. **Gap Risk**

* **Rule:** |Open–PrevClose| > k·ATR → special handling (wider stops, no new entries).

14. **Vol Target Drift**

* **Rule:** Realized vol deviates from target by >X% → auto resize or alert.

## 5) Volume / Flow

15. **Price–Volume Confirmation**

* **Rule:** Breakout only if volume_z > +1.5.

16. **OBV/MFI Divergence**

* **Rule:** Price makes new high but OBV doesn’t → bearish divergence alert; inverse bullish.

## 6) Price Action Micro-patterns

17. **Support/Resistance Retest (S/R)**

* **Rule:** Break + retest within k ticks of level with rejection wick → continuation.

18. **Failed Breakout (FB)**

* **Rule:** Close back inside range within 3 bars after breakout → fade to opposite boundary.

19. **Swing Structure**

* **Rule:** Higher-high + higher-low sequence length ≥ m → trend confidence ↑; reverse for down.

## 7) Cross-Asset / Factor Context

20. **Beta/Correlation Watch**

* **Rule:** If β to index rises above 1.3 and corr>0.8 → expect amplified moves; tighten stops.

21. **Spread/Pair Mean Reversion**

* **Rule:** z-score of (A − βB) > +2 → short spread; < −2 → long spread.

22. **Risk-On/Off Filter**

* **Rule:** Signal allowed only if benchmark above SMA200 and credit/funding proxies not stressed.

## 8) Seasonality & Calendar (toggle if relevant)

23. **Day-of-Week/Turn-of-Month**

* **Rule:** If historical edge > t-stat threshold → loosen/tighten MR/breakout criteria on those sessions.

24. **Earnings/Econ Events**

* **Rule:** No new positions X bars before/after earnings; widen stops on CPI/FOMC equivalents.

## 9) Options-Aware (if options data available)

25. **IV Rank / Crush**

* **Rule:** IVR>70 → prefer premium selling strategies; <20 → prefer debit structures.

26. **Skew Shifts**

* **Rule:** Put-call skew steepens rapidly → downside tail risk alert.

27. **Gamma Exposure (GEX) Zone**

* **Rule:** Price near high-gamma strikes → expect pinned ranges until expiry.

## 10) Risk Management & Exits (enforced uniformly)

28. **Initial Stop by ATR**

* **Rule:** Long stop = entry − k·ATR; short stop = entry + k·ATR.

29. **Time-Based Exit**

* **Rule:** If trade not in profit after N bars → reduce or exit.

30. **Trailing Stop**

* **Rule:** Chandelier (highest close − k·ATR) long; mirror short.

31. **Position Sizing**

* **Rule:** Units = min( maxRiskPerTrade / (k·ATR·$perPoint), vol-target sizing, cap).

32. **Portfolio Guards**

* **Rule:** Max concurrent risk Σ(risk_at_stop) ≤ R_port; halt if drawdown > D_max.

## 11) Signal Quality & Debiasing

33. **Multi-Signal Consensus**

* **Rule:** Fire “strong” alert only if ≥2 of {trend, momentum, volume} agree.

34. **Regime Classifier**

* **Rule:** Use features (trend slope, vol, breadth) to classify {trending, choppy}; enable/disable MR/breakout.

35. **Look-ahead / Survivorship Controls**

* **Rule:** Prevent future data leakage; roll computations; use point-in-time constituents.

---

# Alert logic (practical rules)

* **De-dup/cooldowns:** Suppress repeats for same rule+symbol within X minutes unless state flips.
* **Severity mapping:** e.g., “Heads-up” (one condition), “Actionable” (consensus), “Critical” (+vol/volume confirm).
* **Windows:** Only during instrument trading hours; user timezone for delivery.
* **Context payload:** Always include price, rule, timeframe, confidence, stop/size suggestion (if enabled).

---

# Backtesting & safety checks (must-haves)

* **Walk-forward:** rolling windows (train N, test M) for any learned thresholds.
* **Fees/slippage:** per venue; model limit/market order fill probabilities.
* **Multiple-testing control:** keep a fixed rule set; version them; use out-of-sample.
* **Live parity:** same indicator library for backtest and live (no numeric drift).
* **Kill-switches:** halt on data gaps, extreme slippage, or broken clocks.

---

# Data & infra notes

* **Feeds:** REST for history + WebSocket for live (per exchange/broker). Cache normalized OHLCV per timeframe.
* **Compute:** rolling features with incremental updates per new bar; keep last N values only.
* **Storage:** columnar (Parquet) or time-series DB; symbol×timeframe partitions.
* **Scheduler:** cron for higher TF, bar-close detection for intraday; deliver in Asia/Kolkata.
* **Config:** per-user rule toggles, thresholds, max notifications/day.

---
