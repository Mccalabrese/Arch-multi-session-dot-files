use std::process::Command;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use regex::Regex;
use anyhow::{Context, Result};
use chrono::{DateTime, FixedOffset, TimeZone};
use toml;
use shellexpand;

//Config Structs
#[derive(Deserialize, Debug)]
struct WaybarWeatherConfig {
    owm_api_key: String,
}
#[derive(Deserialize, Debug)]
struct GlobalConfig {
    waybar_weather: WaybarWeatherConfig,
}
fn load_config() -> Result<GlobalConfig> {
    let config_path = shellexpand::tilde("~/.config/rust-dotfiles/config.toml").to_string();

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file from path: {}", config_path))?;

    let config: GlobalConfig = toml::from_str(&config_str)
        .context("Failed to parse config.toml. Check for syntax errors.")?;
    
    Ok(config)
}

// --- 1. Structs de Ubicación (Sin cambios) ---
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Location {
    latitude: f64,
    longitude: f64,
    accuracy: f64,
}

// --- 2. Structs de OWM (¡EXPANDIDAS!) ---

#[derive(Deserialize, Debug, Clone)] // <-- Added Clone
struct Weather {
    id: u32,
    description: String,
}

#[derive(Deserialize, Debug, Clone)] // <-- Added Clone
struct Main {
    temp: f64,
    feels_like: f64,
    humidity: f64,
    pressure: f64,
    temp_min: f64,
    temp_max: f64,
}

#[derive(Deserialize, Debug)]
struct Wind {
    speed: f64,
    deg: Option<f64>,
}

#[derive(Deserialize, Debug)]
struct Sys {
    sunrise: i64,
    sunset: i64,
}

#[derive(Deserialize, Debug)]
struct CurrentWeather {
    weather: Vec<Weather>,
    main: Main,
    sys: Sys,
    wind: Wind,
    visibility: Option<f64>,
    dt: i64,
    timezone: i64, // <-- NEW: Timezone offset in seconds from UTC
}

// --- 3. Structs de Nominatim (Sin cambios) ---
#[derive(Deserialize, Debug)]
struct NominatimAddress {
    city: Option<String>,
    town: Option<String>,
    village: Option<String>,
    state: Option<String>,
    country: Option<String>,
}

#[derive(Deserialize, Debug)]
struct NominatimResponse {
    address: NominatimAddress,
}

// --- 4. NUEVO: Structs for Forecast API ---
#[derive(Deserialize, Debug)]
struct ForecastItem {
    dt: i64, // Timestamp
    main: Main,
    weather: Vec<Weather>,
    pop: f64, // Probability of precipitation
}

#[derive(Deserialize, Debug)]
struct Forecast {
    list: Vec<ForecastItem>,
}


// --- 5. Funciones de Ubicación (Sin cambios) ---
fn run_where_am_i() -> Result<Location> {
    // ... (identical to previous version) ...
    let output = Command::new("/usr/lib/geoclue-2.0/demos/where-am-i")
        .output()
        .context("Failed to run 'where-am-i' command")?;
    if !output.status.success() {
        anyhow::bail!("'where-am-i' command failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lat_re = Regex::new(r"Latitude:\s*(-?\d+\.\d+)")?;
    let lon_re = Regex::new(r"Longitude:\s*(-?\d+\.\d+)")?;
    let acc_re = Regex::new(r"Accuracy:\s*(\d+\.?\d*)\s*meters")?;
    let lat_str = lat_re.captures(&stdout).context("Failed to parse Latitude")?[1].to_string();
    let lon_str = lon_re.captures(&stdout).context("Failed to parse Longitude")?[1].to_string();
    let acc_str = acc_re.captures(&stdout).context("Failed to parse Accuracy")?[1].to_string();
    let location = Location {
        latitude: lat_str.parse()?,
        longitude: lon_str.parse()?,
        accuracy: acc_str.parse()?,
    };
    Ok(location)
}

fn get_cache_path() -> Result<PathBuf> {
    // ... (identical) ...
    let mut path = dirs::cache_dir().context("Failed to find cache directory")?;
    path.push("weather_location.json");
    Ok(path)
}

fn write_to_cache(location: &Location) -> Result<()> {
    // ... (identical) ...
    let path = get_cache_path()?;
    let json_data = serde_json::to_string(location)?;
    fs::write(path, json_data)?;
    Ok(())
}

fn read_from_cache() -> Result<Location> {
    // ... (identical) ...
    let path = get_cache_path()?;
    let json_data = fs::read_to_string(path)?;
    let location: Location = serde_json::from_str(&json_data)?;
    Ok(location)
}

// --- 6. Funciones de Red (MODIFICADAS) ---

fn get_weather_icon(condition_id: u32, is_day: bool) -> &'static str {
    // ... (identical) ...
    match condition_id {
        200..=299 => "󰖓", // Thunderstorm
        300..=399 => "󰖖", // Drizzle
        500..=599 => "󰖖", // Rain
        600..=699 => "󰖘", // Snow
        700..=799 => "󰖑", // Atmosphere
        800 => if is_day { "󰖙" } else { "󰖔" }, // Clear
        801..=804 => if is_day { "󰖐" } else { "󰖑" }, // Clouds
        _ => "󰖐", // Default
    }
}

async fn fetch_weather(client: &reqwest::Client, loc: &Location, api_key: &str) -> Result<CurrentWeather> {
    let url = format!(
        "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units=imperial",
        loc.latitude, loc.longitude, api_key
    );
    let response = client.get(&url)
        .send()
        .await
        .context("Failed to send OWM request")?
        .json::<CurrentWeather>()
        .await
        .context("Failed to parse OWM JSON response")?;
    Ok(response)
}

async fn get_city_state(client: &reqwest::Client, loc: &Location) -> Result<(String, String)> {
    // ... (identical) ...
    let url = format!(
        "https://nominatim.openstreetmap.org/reverse?format=json&lat={}&lon={}&zoom=10",
        loc.latitude, loc.longitude
    );
    let response = client.get(&url)
        .send()
        .await
        .context("Failed to send Nominatim request")?
        .json::<NominatimResponse>()
        .await
        .context("Failed to parse Nominatim JSON response")?;
    let addr = response.address;
    let city = addr.city.or(addr.town).or(addr.village)
        .unwrap_or_else(|| "Unknown City".to_string());
    let state = addr.state
        .unwrap_or_else(|| "Unknown State".to_string());
    Ok((city, state))
}

// --- 7. NUEVA: Función de Pronóstico ---
async fn fetch_forecast(client: &reqwest::Client, loc: &Location, api_key: &str) -> Result<Forecast> {
    let url = format!(
        "https://api.openweathermap.org/data/2.5/forecast?lat={}&lon={}&appid={}&units=imperial",
        loc.latitude, loc.longitude, api_key
    );

    let response = client.get(&url)
        .send()
        .await
        .context("Failed to send OWM forecast request")?
        .json::<Forecast>()
        .await
        .context("Failed to parse OWM forecast JSON response")?;
    
    Ok(response)
}


// --- 8. Función Principal (¡MODIFICADA!) ---
#[tokio::main]
async fn main() -> Result<()> {
    //load config
    let global_config = load_config()?;
    let api_key = global_config.waybar_weather.owm_api_key;
    const NOMINATIM_USER_AGENT: &str = "WaybarWeatherScript/2.0-owm (User: michael-arch; contact: nw.calabrese@proton.me)";
    
    let http_client = reqwest::Client::builder()
        .user_agent(NOMINATIM_USER_AGENT)
        .build()?;

    // --- Obtener Ubicación (Sin cambios) ---
    let location_result = run_where_am_i();
    let location = match location_result {
        // ... (identical cache logic) ...
        Ok(fresh_location) => {
            if fresh_location.accuracy < 1500.0 {
                if let Err(e) = write_to_cache(&fresh_location) {
                    eprintln!("Warning: Failed to write to cache: {}", e);
                }
                fresh_location
            } else {
                match read_from_cache() {
                    Ok(cached_location) => cached_location,
                    Err(_) => fresh_location,
                }
            }
        },
        Err(e) => {
            eprintln!("'where-am-i' failed: {}. Trying cache...", e);
            read_from_cache().context("Failed to get fresh location AND failed to read cache")?
        }
    };

    // --- MODIFICADO: Ejecutar TRES llamadas en paralelo ---
    let (weather_result, geo_result, forecast_result) = tokio::join!(
        fetch_weather(&http_client, &location, &api_key),
        get_city_state(&http_client, &location),
        fetch_forecast(&http_client, &location, &api_key) // <-- La tercera llamada
    );

    // --- Manejar resultados ---
    let weather_data = match weather_result {
        Ok(data) => data,
        Err(e) => {
            let output_json = serde_json::json!({
                "text": "󰖕 API?",
                "tooltip": format!("Failed to fetch weather: {}", e),
                "class": "error"
            });
            println!("{}", output_json);
            anyhow::bail!("Weather fetch failed: {}", e);
        }
    };

    let (city, state) = match geo_result {
        Ok((city, state)) => (city, state),
        Err(e) => {
            eprintln!("Warning: Failed to get city/state: {}", e);
            ("Unknown City".to_string(), "Unknown State".to_string())
        }
    };
    
    // El pronóstico es opcional. Si falla, solo imprimimos un error a stderr.
    let forecast_data = match forecast_result {
        Ok(data) => Some(data),
        Err(e) => {
            eprintln!("Warning: Failed to get forecast data: {}", e);
            None
        }
    };

    // --- Construir el Tooltip Completo (¡MODIFICADO!) ---
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs() as i64;
    let is_day = now >= weather_data.sys.sunrise && now <= weather_data.sys.sunset;
    let icon = get_weather_icon(weather_data.weather[0].id, is_day);

    let mut tooltip_parts = Vec::new();
    tooltip_parts.push(format!(
        "<b>{}, {}</b> (Acc: ~{:.0}m)",
        city, state, location.accuracy
    ));
    tooltip_parts.push(format!(
        "<span size=\"large\">{:.0}°F</span> {} <b>{}</b>",
        weather_data.main.temp, icon, weather_data.weather[0].description
    ));
    tooltip_parts.push(format!(
        "<small>Feels like {:.0}°F</small>",
        weather_data.main.feels_like
    ));
    tooltip_parts.push(format!(
        "Low {:.0}°F / High {:.0}°F",
        weather_data.main.temp_min, weather_data.main.temp_max
    ));
    tooltip_parts.push("".to_string());
    if let Some(deg) = weather_data.wind.deg {
        tooltip_parts.push(format!(
            "󰖝 Wind: {:.1} mph ({:.0}°)",
            weather_data.wind.speed, deg
        ));
    } else {
        tooltip_parts.push(format!(
            "󰖝 Wind: {:.1} mph",
            weather_data.wind.speed
        ));
    }
    tooltip_parts.push(format!("󰖌 Humidity: {:.0}%", weather_data.main.humidity));
    tooltip_parts.push(format!("󰥡 Pressure: {:.0} hPa", weather_data.main.pressure));
    if let Some(vis) = weather_data.visibility {
        tooltip_parts.push(format!("󰖑 Visibility: {:.1} mi", vis / 1609.34));
    }

    // --- NUEVO: Bucle de Pronóstico ---
    if let Some(forecast) = forecast_data {
        tooltip_parts.push("\n--- Forecast (3hr) ---".to_string());
        
        // Crear el objeto timezone desde el offset de segundos
        let tz_offset = FixedOffset::east_opt(weather_data.timezone as i32)
            .unwrap_or_else(|| FixedOffset::east_opt(0).unwrap());

        // Tomar solo los primeros 4 intervalos, igual que en Python
        for item in forecast.list.iter().take(4) {
            // Convertir el timestamp UTC a un objeto DateTime
            let dt = DateTime::from_timestamp(item.dt, 0).unwrap();
            // Aplicar nuestro offset para obtener la hora local
            let local_time = dt.with_timezone(&tz_offset);
            
            // Formatear la hora, p.ej. "09PM" -> "9PM"
            let time_str = local_time.format("%I%p").to_string();
            let time_str_clean = time_str.strip_prefix('0').unwrap_or(&time_str);

            // Determinar si es de día/noche para el ícono
            let is_fc_day = item.dt >= weather_data.sys.sunrise && item.dt <= weather_data.sys.sunset;
            let fc_icon = get_weather_icon(item.weather[0].id, is_fc_day);
            let pop_percent = item.pop * 100.0;

            tooltip_parts.push(format!(
                "{}: {:.0}°F {} (󰖗 {:.0}%)",
                time_str_clean, item.main.temp, fc_icon, pop_percent
            ));
        }
    }
    // --- Fin del Bucle de Pronóstico ---

    let tooltip = tooltip_parts.join("\n");
    
    let pango_re = Regex::new(r"</?b>|</b>|</?span.*?>|</?small>")?;
    let cleaned_tooltip = pango_re.replace_all(&tooltip, "").to_string();
    if let Some(mut cache_path) = dirs::cache_dir() {
        cache_path.push(".weather_cache");

        // 3. Escribir en el archivo
        // Usamos un `if let Err` para que, si esto falla, no colapse todo el script.
        // (p.ej., si los permisos son incorrectos). Simplemente imprimirá un error en stderr.
        if let Err(e) = fs::write(&cache_path, cleaned_tooltip) {
            eprintln!("Warning: Failed to write hyprlock cache file: {}", e);
        }
    }
    // --- Salida Final ---
    let output_json = serde_json::json!({
        "text": format!("{:.0}°F {}", weather_data.main.temp, icon),
        "tooltip": tooltip,
        "class": "weather"
    });

    println!("{}", output_json);
    
    Ok(())
}
