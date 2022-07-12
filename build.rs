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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" { // We can cross compile, so don't use cfg!(target_os = "windows")
        let mut res = winres::WindowsResource::new();
        if which::which("x86_64-w64-mingw32-windres").is_ok() { // Are we cross-compiling?
            res.set_windres_path("x86_64-w64-mingw32-windres");
            res.set_ar_path("x86_64-w64-mingw32-ar");
        }
        res.set_icon("assets/eldenringlogo.ico")
            .set("ProductName", "Elden Ring Seamless CoOp Manager")
            .set("InternalName", "Elden-Ring-Seamless-Co-Op-Manager")
            .set("LegalCopyright", "© 2022 David Caldwell <david_erscom@porkrind.org>")
            .set("Comments", "I think this David Caldwell guy is a cool dude.");
        if let Ok(version) = std::env::var("VERSION") {
            res.set("FileVersion", &version);
            res.set("ProductVersion", &version);
            let v: Vec<u64> = version.split(".")
                .map(|s| s.parse::<u16>().unwrap_or(0) as u64)
                .chain([0].iter().map(|r| *r).cycle())
                .take(4).collect();
            res.set_version_info(winres::VersionInfo::PRODUCTVERSION, v[0] << 48 | v[1] << 32 | v[2] << 16 | v[3]);
            res.set_version_info(winres::VersionInfo::FILEVERSION,    v[0] << 48 | v[1] << 32 | v[2] << 16 | v[3]);
        }
        res.compile()?;
     }
    Ok(())
}
