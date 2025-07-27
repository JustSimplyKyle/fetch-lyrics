use argh::FromArgs;
use async_compat::Compat;
use chrono::{NaiveTime, TimeDelta};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

smol_macros::main! {
    async fn main() -> Result<(), AppError> {
        Compat::new(fake_main()).await
    }
}

#[derive(FromArgs)]
/// cli
struct Cli {
    /// token without preceding BEARER
    #[argh(option)]
    token: String,

    /// the last part of the open.spotify.com
    #[argh(option)]
    track_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Lyrics {
    sync_type: String,
    lines: Vec<Line>,
    provider: String,
    provider_lyrics_id: String,
    provider_display_name: String,
    sync_lyrics_uri: String,
    is_dense_typeface: bool,
    alternatives: Vec<String>,
    language: String,
    is_rtl_language: bool,
    cap_status: String,
    preview_lines: Vec<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Line {
    #[serde(deserialize_with = "parse_time", rename = "startTimeMs")]
    start_time: NaiveTime,
    words: String,
    syllables: Vec<String>,
    #[serde(deserialize_with = "parse_time", rename = "endTimeMs")]
    end_time: NaiveTime,
}

fn parse_time<'de, D>(deserializer: D) -> Result<NaiveTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    let duration = s.parse::<i64>().map_err(serde::de::Error::custom)?;
    Ok(NaiveTime::default() + TimeDelta::milliseconds(duration))
}

error_set::error_set! {
    AppError = {
        Serialzation(serde_json::Error),
    } || Internet;
    Internet = {
        /// failed to fetch
        Fetch(reqwest::Error),
        /// status code bad
        StatusCode(reqwest::Error),
        /// status code bad
        JsonParsing(reqwest::Error)
    };
}

async fn fake_main() -> Result<(), AppError> {
    let client = reqwest::Client::new();
    let cli: Cli = argh::from_env();

    let url = format!(
        "https://spclient.wg.spotify.com/color-lyrics/v2/track/{}?format=json&vocalRemoval=false",
        cli.track_id
    );

    let resp = client
        .get(&url)
        .header("user-agent", "curl/7.81.0")
        .header("accept", "*/*")
        .header("app-platform", "WebPlayer")
        .header("authorization", format!("Bearer {}", cli.token))
        .send()
        .await
        .map_err(Internet::Fetch)?
        .error_for_status()
        .map_err(Internet::StatusCode)?;

    let json = resp.json::<Value>().await.map_err(Internet::JsonParsing)?;

    let lyrics = Lyrics::deserialize(&json["lyrics"])?;
    let lrc = lyrics
        .lines
        .into_iter()
        .map(|line| format!("[{}]{}", line.start_time.format("%M:%S%.3f"), line.words))
        .collect::<Vec<_>>()
        .join("\n");

    println!("{lrc}");

    Ok(())
}
