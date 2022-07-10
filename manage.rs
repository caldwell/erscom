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

pub fn get_releases() -> Result<Vec<Release>, Box<dyn Error>> {
    let client = reqwest::blocking::Client::new();
    let resp = client.get("https://api.github.com/repos/LukeYui/EldenRingSeamlessCoopRelease/releases")
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "erscom 1.0")
        .send()?;
    let status = resp.status();
    if !status.is_success() {
        Err(resp.text().unwrap_or(format!("Got status {}", status)))?;
        unreachable!();
    }
    let response: Vec<GithubRelease> = resp.json()?;
    Ok(response.iter().map(|release| {
        Release {
            tag: release.tag_name.clone(),
            url: release.assets[0].browser_download_url.clone(),
            date: release.published_at.clone(),
        }
    }).collect())
}

impl Release {
    pub fn install(&self, installdir: &EldenRingDir) -> Result<(), Box<dyn Error>> {
        let path = self.download()?;
        println!("Local zip: {}", path.to_string_lossy());

        if !std::fs::metadata(&installdir.path()).map_err(|e| format!("Error reading {:?}: {}", installdir, e))?.is_dir() {
            Err(format!("{} is not a directory!", installdir))?;
        }

        let mut zip = zip::ZipArchive::new(std::fs::File::open(&path)?).map_err(|e| format!("Couldn't read {}: {}", path.to_string_lossy(), e))?;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            if let Some(name) = file.enclosed_name() {
                let dest_path = installdir.path().join(name);
                match (file.is_dir(), dest_path.is_file(), name.extension().map(|n| n.to_string_lossy().to_lowercase()) == Some("ini".to_string())) {
                    (false, false, _) |
                    (false, true,  false) => {
                        println!("Filename: {}{}  -> {:?}", name.to_string_lossy(), if name.is_dir() { "/" } else { "" }, dest_path);
                        std::fs::create_dir_all(&dest_path.parent().ok_or(format!("No parent for {:?}??", dest_path))?)?;
                        let mut dest = std::fs::File::create(&dest_path).map_err(|e| format!("Error creating {:?}: {}", dest_path, e))?;
                        if let Err(e) = std::io::copy(&mut file, &mut dest) {
                            Err(format!("Error writing {:?}: {}", dest_path, e))?;
                        }
                    },
                    (_,_,_) => { println!("Ignoring {}", file.name()) },
                }
            }
        }
        Ok(())
    }

    pub fn installed(&self, installdir: &EldenRingDir) -> Option<bool> {
        use std::io::Read;
        if !self.downloaded() {
            return None;
        }
        let disk_path = installdir.path().join("SeamlessCoop").join("elden_ring_seamless_coop.dll");
        let mut disk_file = std::fs::File::open(&disk_path).ok()?;
        let mut disk_dll = Vec::new();
        disk_file.read_to_end(&mut disk_dll).ok()?;

        let zip_path = self.download().ok()?;
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&zip_path).ok()?).map_err(|e| format!("Couldn't read {}: {}", zip_path.to_string_lossy(), e)).ok()?;
        let mut zip_file = zip.by_name("SeamlessCoop/elden_ring_seamless_coop.dll").ok()?;
        let mut zip_dll = Vec::new();
        zip_file.read_to_end(&mut zip_dll).ok()?;

        Some(disk_dll == zip_dll)
    }

    pub fn cache_path(&self) -> Result<std::path::PathBuf, Box<dyn Error>> {
        Ok(add_extension(&std::env::current_exe().map_err(|e| format!("Couldn't find my .exe: {}", e))?
           .parent().ok_or(format!("Couldn't find where my .exe lives"))?
           .join("release cache")
           .join(&self.tag), "zip"))
    }

    pub fn downloaded(&self) -> bool {
        if let Ok(path) = self.cache_path() {
            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.is_file() {
                    return true;
                }
            }
        }
        return false;
    }

    pub fn download(&self) -> Result<std::path::PathBuf, Box<dyn Error>> {
        let path = self.cache_path()?;
        if std::fs::metadata(&path).map(|m| m.is_file()).unwrap_or(false) {
            return Ok(path);
        }
        if !path.parent().ok_or("No parent for cache dir??")?.exists() {
            std::fs::create_dir(&path.parent().unwrap())?;
        }
        let client = reqwest::blocking::Client::new();
        let mut resp = client.get(&self.url)
            .header("User-Agent", "erscom 1.0")
            .send()?;

        let download_path = add_extension(&path, "partial");
        let mut file = std::fs::File::create(&download_path)?;
        resp.copy_to(&mut file)?;

        std::fs::rename(&download_path, &path)?;

        Ok(path)
    }

}

#[derive(Clone, Debug)]
pub struct EldenRingDir(std::path::PathBuf);

impl EldenRingDir {
    #[cfg(target_os = "windows")]
    pub fn autodetect_install_path() -> Option<EldenRingDir> {
        let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
        hklm.open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Steam App 1245620")
            .and_then(|subkey| subkey.get_value::<std::ffi::OsString,_>("InstallLocation"))
            .map(|oss| EldenRingDir(std::path::Path::new(&oss).to_path_buf()))
            .ok()
    }

    #[cfg(not(target_os = "windows"))]
    pub fn autodetect_install_path() -> Option<EldenRingDir> {
        Some(EldenRingDir(std::env::current_exe().unwrap()
                          .parent().unwrap()
                          .join("pretend-installdir")))
    }

    pub fn get_ini(&self) -> Option<std::path::PathBuf> {
        let old = self.path().join("SeamlessCoop").join("cooppassword.ini");
        let new = self.path().join("SeamlessCoop").join("seamlesscoopsettings.ini");

        match (new.is_file(),old.is_file()) {
            (true,  _)     => Some(new),
            (false, true)  => Some(old),
            (false, false) => None,
        }
    }

    pub fn get_password(&self) -> Result<String, Box<dyn Error>> {
        let ini_file = self.get_ini().ok_or(format!("Missing ini file in {}", self.0.join("SeamlessCoop").display()))?;
        let ini = crate::ini::Ini::read(&ini_file)?;
        Ok(ini.get("PASSWORD", "cooppassword").or(ini.get("SETTINGS", "cooppassword")).ok_or(format!("cooppassword setting not found in {}", ini_file.display()))?.to_string())
    }

    pub fn set_password(&self, password: &str) -> Result<(), Box<dyn Error>> {
        let old = self.path().join("SeamlessCoop").join("cooppassword.ini");
        let new = self.path().join("SeamlessCoop").join("seamlesscoopsettings.ini");

        if old.is_file() { self.set_password_for(password, &old, "SETTINGS")?; }
        if new.is_file() { self.set_password_for(password, &new, "PASSWORD")?; }
        if !old.is_file() && !new.is_file() { Err(format!("No ini file to save password in!"))? }
        Ok(())
    }

    pub fn set_password_for(&self, password: &str, ini_file: &std::path::Path, section: &str) -> Result<(), Box<dyn Error>> {
        let mut ini = crate::ini::Ini::read(&ini_file)?;
        ini.set(section, "cooppassword", password);
        ini.write(&ini_file)?;
        Ok(())
    }

    pub fn path(&self) -> std::path::PathBuf {
        self.0.join("Game")
    }

    pub fn display(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }
}

impl std::fmt::Display for EldenRingDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

// Stolen from https://users.rust-lang.org/t/append-an-additional-extension/23586/12
fn add_extension(path: &std::path::PathBuf, extension: impl AsRef<std::path::Path>) -> std::path::PathBuf {
    match path.extension() {
        Some(ext) => {
            let mut ext = ext.to_os_string();
            ext.push(".");
            ext.push(extension.as_ref());
            path.with_extension(ext)
        }
        None => path.with_extension(extension.as_ref()),
    }
}
