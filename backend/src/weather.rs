use super::*;

#[derive(Deserialize)]
struct OpenMeteoResponse {
    current: OpenMeteoCurrent,
}

#[derive(Deserialize)]
struct OpenMeteoCurrent {
    time: String,
    temperature_2m: f64,
    apparent_temperature: f64,
    precipitation: f64,
    weather_code: u16,
    wind_speed_10m: f64,
}

pub(super) async fn get_weather(
    Path(city): Path<String>,
) -> Result<Json<WeatherObservation>, (StatusCode, String)> {
    fetch_weather(&city).await.map(Json).map_err(|error| {
        tracing::error!(%error, %city, "weather tool failed");
        (
            StatusCode::BAD_GATEWAY,
            format!("weather_fetch_failed: {error}"),
        )
    })
}

pub(super) async fn fetch_weather(city: &str) -> Result<WeatherObservation, String> {
    if !matches!(city.to_lowercase().as_str(), "munster" | "münster") {
        return Err(format!("unsupported location '{city}'; try Munster"));
    }

    let response = reqwest::Client::new()
        .get("https://api.open-meteo.com/v1/forecast")
        .query(&[
            ("latitude", "51.9607"),
            ("longitude", "7.6261"),
            (
                "current",
                "temperature_2m,apparent_temperature,precipitation,weather_code,wind_speed_10m",
            ),
            ("timezone", "Europe/Berlin"),
        ])
        .send()
        .await
        .map_err(|error| format!("Open-Meteo request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Open-Meteo returned an error: {error}"))?
        .json::<OpenMeteoResponse>()
        .await
        .map_err(|error| format!("Open-Meteo response was invalid: {error}"))?;

    Ok(WeatherObservation {
        location: "Münster, Germany".to_string(),
        temperature_c: response.current.temperature_2m,
        apparent_temperature_c: response.current.apparent_temperature,
        precipitation_mm: response.current.precipitation,
        wind_speed_kmh: response.current.wind_speed_10m,
        weather_code: response.current.weather_code,
        observed_at: response.current.time,
    })
}

pub(super) fn weather_message(observation: &WeatherObservation) -> ConversationMessage {
    let joke = if observation.precipitation_mm > 0.0 {
        "Münster brought its own water feature today."
    } else if observation.temperature_c < 10.0 {
        "A crisp forecast: even the bicycles may want tiny scarves."
    } else {
        "Good cycling weather, assuming the wind has not joined a sprint team."
    };

    ConversationMessage {
        usage: None,
        role: "assistant".to_string(),
        blocks: vec![
            RichTextBlock::Heading {
                level: 3,
                text: "Weather in Münster".to_string(),
            },
            RichTextBlock::Paragraph {
                text: format!(
                    "Right now it is {:.1}°C and feels like {:.1}°C.",
                    observation.temperature_c, observation.apparent_temperature_c
                ),
            },
            RichTextBlock::BulletList {
                items: vec![
                    format!("Precipitation: {:.1} mm", observation.precipitation_mm),
                    format!("Wind: {:.1} km/h", observation.wind_speed_kmh),
                    format!("Observed: {} local time", observation.observed_at),
                ],
            },
            RichTextBlock::Paragraph {
                text: joke.to_string(),
            },
        ],
        tool_calls: vec![ToolCall {
            tool_call_id: next_id("tool"),
            tool_name: "fetch_weather".to_string(),
            status: "completed".to_string(),
            summary: format!(
                "Fetched current conditions for {} from Open-Meteo",
                observation.location
            ),
        }],
    }
}

pub(super) fn message_plain_text(message: &ConversationMessage) -> String {
    message
        .blocks
        .iter()
        .map(|block| match block {
            RichTextBlock::Paragraph { text } | RichTextBlock::Heading { text, .. } => text.clone(),
            RichTextBlock::BulletList { items } => items.join(" · "),
            RichTextBlock::MultipleChoice { options, .. } => options.join(" / "),
            RichTextBlock::ChoiceAnswer { choice, .. } => choice.clone(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}
