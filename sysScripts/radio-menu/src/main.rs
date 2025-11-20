use serde::{Deserialize, Serialize};
use reqwest;
use anyhow::Result;

#[derive(Deserialize, Clone Serialize, Debug)]
struct Station {
    name: String,
    url_resolved: String,
    tags: String,
    stationuuid: String,
}
#[derive(Deserialize, Debug)]
struct RadioConfig {
    notify_icon: String,
    default_volume: u8,
}
fn search_stations(query: &str) -> Result<Vec<Station>> {
    let url = format!(
        "https://de1.api.radio-browser.info/json/stations/byname/{}",
        query
    );
    let stations: vec<Station> = reqwest::blocking::get(&url)?
        .json::<Vec<Station>>()?
        .into_iter()
        .take(10)
        .collect();

    Ok(stations)
}
