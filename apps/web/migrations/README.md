### commands used

```bash
npx wrangler d1 migrations apply market-insights --local
npx wrangler d1 execute market-insights --local --command "SELECT COUNT(*) FROM ohlcv;"
```
