// Copyright © 2022 David Caldwell <david@porkrind.org>

use std::error::Error;

use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub struct Release {
    pub tag: String,
    pub url: String,
    pub date: String,
}

// These are the parts of the github release api that we care about.
// See https://docs.github.com/en/rest/releases/releases
#[derive(Clone, Debug, Serialize, Deserialize)]
struct GithubRelease {
    tag_name: String,
    published_at: String,
    assets: Vec<GithubAsset>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct GithubAsset {
    browser_download_url: String,
}

pub async fn get_releases() -> Result<Vec<Release>, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let resp = client.get("https://api.github.com/repos/LukeYui/EldenRingSeamlessCoopRelease/releases")
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "erscom 1.0")
        .send().await?;
    let status = resp.status();
    if !status.is_success() {
        Err(resp.text().await.unwrap_or(format!("Got status {}", status)))?;
        unreachable!();
    }
    let response: Vec<GithubRelease> = resp.json().await?;
    Ok(response.iter().map(|release| {
        Release {
            tag: release.tag_name.clone(),
            url: release.assets[0].browser_download_url.clone(),
            date: release.published_at.clone(),
        }
    }).collect())
}

#[cfg(target_os = "windows")]
pub fn autodetect_install_path() -> Option<String> {
    let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    hklm.open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Steam App 1245620")
        .and_then(|subkey| subkey.get_value::<String,_>("InstallLocation"))
        .ok()
}

#[cfg(not(target_os = "windows"))]
pub fn autodetect_install_path() -> Option<String> {
    None
}
