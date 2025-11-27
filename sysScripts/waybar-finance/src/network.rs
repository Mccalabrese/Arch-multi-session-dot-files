use anyhow::{Result, Context};
use yahoo_finance_api::YahooConnector;
use time::OffsetDateTime;
use serde::{Deserialize, Serialize};
use crate::config::{get_config_path, load_config};
use crate::app::StockDetails;
use chrono::{Utc, Datelike};
use reqwest::Client;

#[derive(Debug, Deserialize)]
pub struct FinnhubQuote {
    #[serde(rename = "c")]
    pub price: f64,
    #[serde(rename = "dp")]
    pub percent: f64,
}

#[derive(Debug, Deserialize)]
struct FinnhubMetricResponse {
    metric: FinnhubMetrics,
}

#[derive(Debug, Deserialize)]
struct FinnhubMetrics {
    #[serde(rename = "marketCapitalization")]
    market_cap: Option<f64>,
    #[serde(rename = "peBasicExclExtraTTM")]
    pe_ratio: Option<f64>,
    #[serde(rename = "52WeekHigh")]
    high_52w: Option<f64>,
    #[serde(rename = "52WeekLow")]
    low_52w: Option<f64>,
    #[serde(rename = "beta")]
    beta: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct FinnhubDividendResponse {
    data: Vec<FinnhubDividend>,
}

#[derive(Debug, Deserialize)]
pub struct FinnhubDividend {
    amount: f64,
}

#[derive(Debug, Serialize)]
pub struct WaybarOutput {
    pub text: String,
    pub tooltip: String,
    pub class: String,
}

#[derive(Debug, Deserialize)]
pub struct YahooSearchResponse {
    pub quotes: Vec<YahooSearchResult>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct YahooSearchResult {
    pub symbol: String,
    #[serde(rename = "shortname")]
    pub name: Option<String>,
    #[serde(rename = "quoteType")]
    pub quote_type: Option<String>,
    #[serde(rename = "exchDisp")]
    pub exchange: Option<String>,
}


pub async fn search_ticker(query: &str) -> Result<Vec<YahooSearchResult>> {
    // Construct a "browser-like" reqwest client
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                     AppleWebKit/537.36 (KHTML, like Gecko) \
                     Chrome/106 Safari/537.36")
        .cookie_store(true)
        .build()?;

    let url = format!(
        "https://query2.finance.yahoo.com/v1/finance/search?q={}&lang=en-US",
        query
    );

    // send the GET request
    let resp = client
        .get(&url)
        .header("Accept", "*/*")
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("Search failed: {}", resp.status()));
    }

    let data: YahooSearchResponse = resp.json().await?;
    Ok(data.quotes)
}


pub async fn fetch_details(client: &reqwest::Client, symbol: &str, key: &str) -> Result<StockDetails> {
    let url = format!(
        "https://finnhub.io/api/v1/stock/metric?symbol={}&metric=all&token={}",
        symbol, key
    );
    
    let resp = client.get(&url).send().await?;
    let data: FinnhubMetricResponse = resp.json().await?;

    let quote = fetch_quote(client, symbol, key).await?;
    let yield_finnhub = fetch_dividend_yield(client, symbol, key, quote.price).await?;
    
    Ok(StockDetails {
        price: quote.price, // We get price from the Quote endpoint, not here
        change_percent: quote.percent,
        // Finnhub gives Market Cap in Millions usually
        market_cap: data.metric.market_cap.unwrap_or(0.0) as u64,
        pe_ratio: data.metric.pe_ratio,
        dividend_yield: yield_finnhub,
        high_52w: data.metric.high_52w.unwrap_or(0.0),
        low_52w: data.metric.low_52w.unwrap_or(0.0),
        beta: data.metric.beta,
    })
}

pub async fn fetch_quote(client: &reqwest::Client, symbol: &str, key: &str) -> Result<FinnhubQuote> {
    let url = format!(
        "https://finnhub.io/api/v1/quote?symbol={}&token={}",
        symbol, key
    );
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("Failed to fetch quote: HTTP {}", resp.status()));
    }
    let quote: FinnhubQuote = resp.json().await?;
    Ok(quote)
}
pub async fn fetch_history(_client: &reqwest::Client, symbol: &str, _key: &str) -> Result<Vec<(f64, f64)>> {
    let provider = YahooConnector::new()?;
    let end = OffsetDateTime::now_utc();
    let start = end - time::Duration::days(365);
    let response = provider.get_quote_history(symbol, start, end).await
        .context("Yaho API Error")?;
    let quotes = response.quotes().context("No quotes in response")?;
    let points: Vec<(f64, f64)> = quotes.iter()
        .map(|q| (q.timestamp as f64, q.close))
        .collect();
    if points.is_empty() {
        return Err(anyhow::anyhow!("History data is empty"));
    }
    Ok(points)
}
pub async fn run_waybar_mode(client: &reqwest::Client) -> Result<()> {
    let config_path = get_config_path()?;
    let config = load_config(&config_path)?;
    let api_key = match &config.api_key {
        Some(k) => k,
        None => {
            eprintln!("Error: API key not found in config.json");
            return Ok(());
        }
    };
    let mut text_parts = Vec::new();
    let mut tooltip_parts = Vec::new();
    for symbol in &config.stocks {
        match fetch_quote(client, symbol, api_key).await {
            Ok(quote) => {
                let (color, icon) = if quote.percent >= 0.0 {
                    ("#a6e3a1", "")
                } else {
                    ("#f38ba8", "")
                };
                let part = format!(
                    "<span color='{}'>{} {:.2} {}</span>",
                    color, symbol, quote.price, icon
                );
                text_parts.push(part);
                tooltip_parts.push(format!("{}: ${:.2} ({:.2}%)", symbol, quote.price, quote.percent));
            }
            Err(_) => {
                text_parts.push(format!("<span color='#6c7086'>{} ???</span>", symbol));
            }
        }
    }
    let output = WaybarOutput {
        text: text_parts.join(" "),
        tooltip: tooltip_parts.join("\n"),
        class: "finance".to_string(),
    };
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
pub async fn fetch_dividend_yield(
    client: &Client,
    symbol: &str,
    key: &str,
    current_price: f64,
) -> Result<Option<f64>> {
    let end_year = Utc::now().year();
    let start = format!("{}-01-01", end_year - 1);
    let end = format!("{}-12-31", end_year);

    let url = format!(
        "https://finnhub.io/api/v1/stock/dividend?symbol={}&from={}&to={}&token={}",
        symbol, start, end, key
    );

    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Ok(None);
    }

    let data: FinnhubDividendResponse = resp.json().await?;
    if data.data.is_empty() {
        return Ok(None);
    }

    let total: f64 = data.data.iter().map(|d| d.amount).sum();
    let avg_div = total / data.data.len() as f64;
    let annualized = avg_div * 4.0;
    let yield_pct = (annualized / current_price) * 100.0;

    Ok(Some(yield_pct))
}
