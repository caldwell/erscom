// Copyright © 2022 David Caldwell <david@porkrind.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::error::Error;

use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub struct Release {
    pub tag: String,
    pub url: String,
    pub date: String,
    pub changelog: String,
}

// These are the parts of the github release api that we care about.
// See https://docs.github.com/en/rest/releases/releases
#[derive(Clone, Debug, Serialize, Deserialize)]
struct GithubRelease {
    tag_name: String,
    published_at: String,
    body: String,
    assets: Vec<GithubAsset>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct GithubAsset {
    browser_download_url: String,
}

fn github_releases(project: &str) -> Result<Vec<GithubRelease>, Box<dyn Error>> {
    tokio::task::block_in_place(move || {
        let client = reqwest::blocking::Client::new();
        let resp = client.get(&format!("https://api.github.com/repos/{}/releases", project))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "erscom 1.0")
            .send()?;
        let status = resp.status();
        if !status.is_success() {
            Err(resp.text().unwrap_or(format!("Got status {}", status)))?;
            unreachable!();
        }
        Ok(resp.json()?)
    })
}

pub fn self_upgrade_version() -> Result<Option<String>, Box<dyn Error>> {
    if let Some(current_version) = option_env!("VERSION") {
        let my_releases = github_releases("caldwell/erscom")?;
        if my_releases.first().map(|r| &r.tag_name) != Some(&current_version.to_string()) {
            return Ok(Some(my_releases.first().unwrap().tag_name.clone()));
        }
    }
    Ok(None)
}

pub fn get_releases() -> Result<Vec<Release>, Box<dyn Error>> {
    Ok(github_releases("LukeYui/EldenRingSeamlessCoopRelease")?.iter().map(|release| {
        Release {
            tag: release.tag_name.clone(),
            url: release.assets[0].browser_download_url.clone(),
            date: release.published_at.clone(),
            changelog: release.body.clone(),
        }
    }).collect())
}

impl Release {
    pub fn install(&self, installdir: &EldenRingDir) -> Result<(), Box<dyn Error>> {
        self.install_uninstall(installdir, |file, dest_path| -> Result<(), Box<dyn Error>> {
            let name = file.enclosed_name().unwrap(); // Guaranteed by instal_uninstall()
            println!("Filename: {}{}  -> {:?}", name.to_string_lossy(), if name.is_dir() { "/" } else { "" }, dest_path);
            std::fs::create_dir_all(&dest_path.parent().ok_or(format!("No parent for {:?}??", dest_path))?)?;
            let mut dest = std::fs::File::create(&dest_path).map_err(|e| format!("Error creating {:?}: {}", dest_path, e))?;
            if let Err(e) = std::io::copy(file, &mut dest) {
                Err(format!("Error writing {:?}: {}", dest_path, e))?;
            }
            Ok(())
        })
    }

    pub fn uninstall(&self, installdir: &EldenRingDir) -> Result<(), Box<dyn Error>> {
        self.install_uninstall(installdir, |_file, dest_path| -> Result<(), Box<dyn Error>> {
            println!("{} Removing: {:?}", self.tag, dest_path);
            std::fs::remove_file(&dest_path)?;
            Ok(())
        })
    }

    fn install_uninstall<F>(&self, installdir: &EldenRingDir, handler: F) -> Result<(), Box<dyn Error>> where F: Fn(&mut zip::read::ZipFile, std::path::PathBuf) -> Result<(), Box<dyn Error>> {
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
                    (false, true,  false) => { handler(&mut file, dest_path)?; },
                    (_,_,_) => { println!("Ignoring {}", file.name()) },
                }
            }
        }
        Ok(())
    }

    pub fn installed(&self, installdir: &EldenRingDir) -> Option<bool> {
        match (self.file_installed(installdir, &std::path::Path::new("SeamlessCoop").join("elden_ring_seamless_coop.dll")),
               self.file_installed(installdir, &std::path::Path::new("SeamlessCoop").join("ersc.dll"))) {
            (None, None) => None,
            (Some(true), _) | (_, Some(true)) => Some(true),
            (_,_) => Some(false),
        }
    }

    pub fn file_installed(&self, installdir: &EldenRingDir, path: &std::path::PathBuf) -> Option<bool> {
        let disk_path = installdir.path().join(path);
        let zip_file_path = path.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>().join("/");
        use std::io::Read;
        if !self.downloaded() {
            return None;
        }
        let mut disk_file = std::fs::File::open(&disk_path).ok()?;
        let mut disk_dll = Vec::new();
        disk_file.read_to_end(&mut disk_dll).ok()?;

        let zip_path = self.download().ok()?;
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&zip_path).ok()?).map_err(|e| format!("Couldn't read {}: {}", zip_path.to_string_lossy(), e)).ok()?;
        let mut zip_file = zip.by_name(&zip_file_path).ok()?;
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
        tokio::task::block_in_place(move || {
            let client = reqwest::blocking::Client::new();
            let mut resp = client.get(&self.url)
                .header("User-Agent", "erscom 1.0")
                .send()?;

            let download_path = add_extension(&path, "partial");
            let mut file = std::fs::File::create(&download_path)?;
            resp.copy_to(&mut file)?;

            std::fs::rename(&download_path, &path)?;
            Ok(path)
        })
    }

}

#[derive(Clone, Debug)]
pub struct EldenRingDir(std::path::PathBuf);

impl EldenRingDir {
    #[cfg(target_os = "windows")]
    pub fn autodetect_install_path() -> Option<EldenRingDir> {
        let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
        // Find the install dir in the registry
        hklm.open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Steam App 1245620")
            .and_then(|subkey| subkey.get_value::<std::ffi::OsString,_>("InstallLocation"))
            .map(|oss| EldenRingDir(std::path::Path::new(&oss).join("Game").to_path_buf()))
            .ok().or(std::env::current_exe().ok()  // Not in registry? Check the dir our exe is in
                     .and_then(|me| me.parent().map(|p| p.to_path_buf()))
                     .and_then(|mydir| mydir.join("eldenring.exe").is_file().then(|| EldenRingDir(mydir))))
    }

    #[cfg(not(target_os = "windows"))]
    pub fn autodetect_install_path() -> Option<EldenRingDir> {
        Some(EldenRingDir(std::env::current_exe().unwrap()
                          .parent().unwrap()
                          .join("pretend-installdir")))
    }

    pub fn get_ini(&self) -> Option<std::path::PathBuf> {
        let old1 = self.path().join("SeamlessCoop").join("cooppassword.ini");
        let old2 = self.path().join("SeamlessCoop").join("seamlesscoopsettings.ini");
        let new  = self.path().join("SeamlessCoop").join("ersc_settings.ini");

        match (new.is_file(),old2.is_file(),old1.is_file()) {
            (true,  _,     _)     => Some(new),
            (false, true,  _)     => Some(old2),
            (false, false, true)  => Some(old1),
            (false, false, false) => { println!("{}: {}", old1.display(), false); println!("{}: {}", old2.display(), false); None },
        }
    }

    pub fn get_password(&self) -> Result<String, Box<dyn Error>> {
        let ini_file = self.get_ini().ok_or(format!("Missing ini file in {}", self.0.join("SeamlessCoop").display()))?;
        let ini = crate::ini::Ini::read(&ini_file)?;
        Ok(ini.get("PASSWORD", "cooppassword").or(ini.get("SETTINGS", "cooppassword")).ok_or(format!("cooppassword setting not found in {}", ini_file.display()))?.to_string())
    }

    pub fn set_password(&self, password: &str) -> Result<(), Box<dyn Error>> {
        let old1 = self.path().join("SeamlessCoop").join("cooppassword.ini");
        let old2 = self.path().join("SeamlessCoop").join("seamlesscoopsettings.ini");
        let new  = self.path().join("SeamlessCoop").join("ersc_settings.ini");

        if old1.is_file() { self.set_password_for(password, &old1, "SETTINGS")?; }
        if old2.is_file() { self.set_password_for(password, &old2, "PASSWORD")?; }
        if new.is_file()  { self.set_password_for(password, &new,  "PASSWORD")?; }
        if !old1.is_file() && !old2.is_file() && !new.is_file() { Err(format!("No ini file to save password in!"))? }
        Ok(())
    }

    pub fn set_password_for(&self, password: &str, ini_file: &std::path::Path, section: &str) -> Result<(), Box<dyn Error>> {
        let mut ini = crate::ini::Ini::read(&ini_file)?;
        ini.set(section, "cooppassword", password);
        ini.write(&ini_file)?;
        Ok(())
    }

    pub fn path(&self) -> &std::path::Path {
        &self.0
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
