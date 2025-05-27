mod version_object;

use base64::{engine::general_purpose, Engine};
use chrono::NaiveDateTime;
pub use futures_util::StreamExt;
use futures_util::TryStreamExt;
use regex::Regex;
use reqwest::Client;
use std::env;
use std::process::Command;
use std::time::Duration;
use tokio::fs::File as TokioFile;
use tokio_util::io::StreamReader;
use version_object::Version;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting...");

    const DEFAULT_INTERVAL: u64 = 5;
    let interval_secs: u64 = env::var("INTERVAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_INTERVAL);

    const DEFAULT_HOST: &str = "https://jfrog.io";
    let host = env::var("JFROG_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
    
    let jfrog_repro_path  = env::var("JFROG_REPRO_PATH").expect("JFROG_REPRO_PATH not set");
    
    let war_name = env::var("WAR_NAME").expect("WAR_NAME not set");
    
    let jfrog_url = format!("{}/artifactory/{}", host, jfrog_repro_path);
    
    let auth_token =  env::var("JFROG_AUTH").expect("JFROG_AUTH not set");

    let mut last_version: Version = Version {
        major: 0,
        minor: 0,
        patch: 0,
    };

    loop {
        println!("Checking for new version...");
        let version = get_latest_version(jfrog_url.clone(), auth_token.clone()).await?;

        if (last_version.to_string() != *version.to_string()) {
            println!("Latest version: {}", version.to_string());

            last_version = *version;

            let war_name = war_name.replace("{%VERSION}", &version.to_string());

            download_latest(*version, jfrog_url.clone(), auth_token.clone(), war_name)
                .await
                .expect("Download failed check JFROG");
            deploy_version(*version);
        } else {
            println!(
                "No new version found. Last version: {}",
                last_version.to_string()
            );
        }

        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
    }
}

async fn download_latest(
    lts: Version,
    url: String,
    auth_token: String,
    war_name: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let encoded = general_purpose::STANDARD.encode(auth_token);
    let auth_header = format!("Basic {}", encoded);

    let download_url = format!("{}{}/{}", url, lts.to_string(), war_name);

    let response = client
        .get(download_url)
        .header("Authorization", auth_header)
        .send()
        .await?;

    let status = response.status();

    if !status.is_success() {
        eprintln!("Download failed with status: {}", status);
        return Err("Download failed".into());
    }

    let mut stream = StreamReader::new(
        response
            .bytes_stream()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
    );

    let output_file_name = String::from("ARTIFACTORY.war");

    let mut file = TokioFile::create(output_file_name.clone()).await?;
    tokio::io::copy(&mut stream, &mut file).await?;

    println!("File saved with name: {}", output_file_name);

    Ok(())
}

fn deploy_version(version: Version) {
    Command::new("./deploy.sh")
        .env("ARTIFACTORY_VERSION", version.to_string())
        .spawn()
        .expect("Failed to start deploy.sh");
}

async fn get_latest_version(
    url: String,
    auth_token: String,
) -> Result<Box<Version>, Box<dyn std::error::Error>> {
    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    let encoded = general_purpose::STANDARD.encode(auth_token);
    let auth_header = format!("Basic {}", encoded);

    let response = client
        .get(url)
        .header("Authorization", auth_header)
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;

    let mut last_version: Version = Version {
        major: 0,
        minor: 0,
        patch: 0,
    };

    println!("Response status: {}", status);

    if status == 200 {
        let mut last_date: NaiveDateTime = NaiveDateTime::from_timestamp(0, 0);

        body.split("\n").for_each(|line| {
            let date = parse_date(line);

            let version = extract_version(line);

            last_version = if date > last_date {
                version
            } else {
                last_version
            };
            last_date = if date > last_date { date } else { last_date };
        });
    }

    Ok(Box::from(last_version))
}

fn extract_version(s: &str) -> Version {
    let re = Regex::new(r#"\b(\d+\.\d+\.\d+)/?"#).unwrap();

    if let Some(caps) = re.captures(s) {
        let version_str = &caps[1];
        Version::parse(version_str).unwrap_or(Version {
            major: 0,
            minor: 0,
            patch: 0,
        })
    } else {
        Version {
            major: 0,
            minor: 0,
            patch: 0,
        }
    }
}

fn parse_date(s: &str) -> NaiveDateTime {
    let re = Regex::new(r"\d{2}-[A-Za-z]{3}-\d{4} \d{2}:\d{2}").unwrap();

    let fallback = NaiveDateTime::parse_from_str("01-Jan-1900 00:00", "%d-%b-%Y %H:%M").unwrap();

    if let Some(mat) = re.find(s) {
        let date_str = mat;
        let format = "%d-%b-%Y %H:%M";
        NaiveDateTime::parse_from_str(date_str.as_str(), format).unwrap_or(fallback)
    } else {
        fallback
    }
}
