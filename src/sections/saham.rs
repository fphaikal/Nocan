use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Quote {
    pub name: String,
    pub currency: String,
    pub price: f64,
    pub change_pct: Option<f64>,
    pub timestamp: u64,
    pub history: Vec<f64>,
}

pub struct StockItem {
    pub symbol: String,
    pub quote: Option<Quote>,
    pub error: bool,
}

pub struct SahamSection {
    pub stocks: Vec<StockItem>,
    pub selected_index: usize,
    pub selection_position: f32,
    receiver: Option<Receiver<(String, Result<Quote, String>)>>,
    next_refresh: Instant,
}

pub fn parse_symbols(input: &str) -> Result<Vec<String>, &'static str> {
    let mut symbols = Vec::new();
    for raw in input.split(|c: char| c == ',' || c == ';' || c.is_whitespace()).filter(|s| !s.is_empty()) {
        let symbol = raw.to_ascii_uppercase();
        if symbol.len() > 20 || !symbol.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'^' | b'=')) {
            return Err("Invalid ticker. Use letters, numbers, periods, - or ^.");
        }
        if !symbols.contains(&symbol) { symbols.push(symbol); }
        if symbols.len() > 6 { return Err("You can add up to 6 tickers."); }
    }
    Ok(symbols)
}

/// Parses watchlist input and applies the `.JK` suffix for Indonesian tickers
/// that were typed without an exchange suffix (e.g. `BBCA` -> `BBCA.JK`).
/// Global tickers (containing `.`, `-`, `^` or `=`) are left untouched.
pub fn normalize_symbols(input: &str, idx_market: bool) -> Result<Vec<String>, &'static str> {
    let symbols = parse_symbols(input)?;
    Ok(symbols
        .into_iter()
        .map(|symbol| {
            let plain_ticker = symbol.bytes().all(|c| c.is_ascii_alphanumeric());
            if idx_market && plain_ticker {
                format!("{symbol}.JK")
            } else {
                symbol
            }
        })
        .collect())
}

fn parse_quote(body: &str) -> Result<Quote, String> {
    let root: Value = serde_json::from_str(body).map_err(|_| "Invalid response")?;
    let result = &root["chart"]["result"][0];
    let meta = &result["meta"];
    let price = meta["regularMarketPrice"].as_f64().filter(|n| n.is_finite()).ok_or("Price unavailable")?;
    let history: Vec<f64> = result["indicators"]["quote"][0]["close"].as_array()
        .map(|a| a.iter().filter_map(Value::as_f64).filter(|v| v.is_finite()).collect()).unwrap_or_default();
    let change_pct = meta["regularMarketChangePercent"].as_f64().filter(|n| n.is_finite())
        .or_else(|| history.iter().rev().nth(1).filter(|n| **n > 0.0).map(|prev| (price / prev - 1.0) * 100.0));
    Ok(Quote {
        name: meta["shortName"].as_str().or_else(|| meta["longName"].as_str()).unwrap_or("").to_string(),
        currency: meta["currency"].as_str().unwrap_or("").to_string(), price, change_pct,
        timestamp: meta["regularMarketTime"].as_u64().unwrap_or(0), history,
    })
}

impl SahamSection {
    pub fn new() -> Self { Self { stocks: Vec::new(), selected_index: 0, selection_position: 0.0, receiver: None, next_refresh: Instant::now() } }

    pub fn set_symbols(&mut self, symbols: &[String]) {
        let normalized = parse_symbols(&symbols.join(",")).unwrap_or_default();
        if self.stocks.iter().map(|s| &s.symbol).eq(normalized.iter()) { return; }
        let selected = self.stocks.get(self.selected_index).map(|s| s.symbol.clone());
        let mut old = std::mem::take(&mut self.stocks);
        self.stocks = normalized.into_iter().map(|symbol| {
            if let Some(index) = old.iter().position(|s| s.symbol == symbol) { old.remove(index) }
            else { StockItem { symbol, quote: None, error: false } }
        }).collect();
        self.selected_index = selected.and_then(|name| self.stocks.iter().position(|s| s.symbol == name)).unwrap_or(0);
        self.selection_position = self.selected_index as f32;
        self.receiver = None;
        self.next_refresh = Instant::now();
    }

    pub fn update(&mut self, dt: f32) {
        self.selection_position += (self.selected_index as f32 - self.selection_position) * (1.0 - (-12.0 * dt).exp());
        if let Some(rx) = &self.receiver {
            while let Ok((symbol, result)) = rx.try_recv() {
                if let Some(stock) = self.stocks.iter_mut().find(|s| s.symbol == symbol) {
                    match result { Ok(quote) => { stock.quote = Some(quote); stock.error = false; }, Err(_) => stock.error = true }
                }
            }
        }
        if !self.stocks.is_empty() && Instant::now() >= self.next_refresh {
            self.next_refresh = Instant::now() + Duration::from_secs(120);
            let symbols: Vec<_> = self.stocks.iter().map(|s| s.symbol.clone()).collect();
            let (tx, rx) = mpsc::channel();
            self.receiver = Some(rx);
            std::thread::spawn(move || {
                let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(8)).build();
                for symbol in symbols {
                    let url = format!("https://query1.finance.yahoo.com/v8/finance/chart/{}?range=5d&interval=1d", symbol.replace('^', "%5E").replace('=', "%3D"));
                    let result = agent.get(&url).set("User-Agent", "Mozilla/5.0").call()
                        .map_err(|e| e.to_string()).and_then(|r| r.into_string().map_err(|e| e.to_string())).and_then(|s| parse_quote(&s));
                    if let Err(error) = &result { crate::config::debug_log(&format!("Stock {symbol}: {error}")); }
                    if tx.send((symbol, result)).is_err() { break; }
                }
            });
        }
    }
}

pub fn quote_age(timestamp: u64) -> String {
    if timestamp == 0 { return "Quote time unavailable".into(); }
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let minutes = now.saturating_sub(timestamp) / 60;
    if minutes < 60 { format!("Updated {} min ago", minutes) }
    else if minutes < 1440 { format!("Updated {} hr ago", minutes / 60) }
    else { format!("Updated {} days ago", minutes / 1440) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn validates_tickers() {
        assert_eq!(parse_symbols("bbca.jk, AAPL bbca.JK").unwrap(), vec!["BBCA.JK", "AAPL"]);
        assert!(parse_symbols("A/B").is_err());
        assert!(parse_symbols("A B C D E F G").is_err());
        assert!(parse_symbols("").unwrap().is_empty());
    }
    #[test] fn normalizes_idx_tickers_without_suffix() {
        assert_eq!(normalize_symbols("bbca, tlkm.jk, aapl", true).unwrap(), vec!["BBCA.JK", "TLKM.JK", "AAPL.JK"]);
        assert_eq!(normalize_symbols("bbca, ^GSPC, BRK-B", true).unwrap(), vec!["BBCA.JK", "^GSPC", "BRK-B"]);
        assert_eq!(normalize_symbols("bbca, aapl", false).unwrap(), vec!["BBCA", "AAPL"]);
    }
    #[test] fn parses_provider_values_without_fake_prices() {
        let body = r#"{"chart":{"result":[{"meta":{"regularMarketPrice":105,"regularMarketTime":100,"currency":"IDR"},"indicators":{"quote":[{"close":[90,100,105]}]}}]}}"#;
        let q = parse_quote(body).unwrap();
        assert_eq!(q.price, 105.0); assert!((q.change_pct.unwrap()-5.0).abs()<0.001);
        assert!(parse_quote("{\"chart\":{\"result\":null}} ").is_err());
    }
    #[test] #[ignore = "requires a network connection"] fn fetches_real_quote() {
        let body = ureq::get("https://query1.finance.yahoo.com/v8/finance/chart/BBCA.JK?range=5d&interval=1d")
            .set("User-Agent", "Mozilla/5.0").call().unwrap().into_string().unwrap();
        let quote = parse_quote(&body).unwrap();
        assert_eq!(quote.currency, "IDR");
        assert!(quote.price > 0.0 && quote.timestamp > 0 && !quote.history.is_empty());
    }
}
