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

// This removes the ugly debug window that comes up on windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::cell::RefCell;
use std::rc::Rc;

mod manage;
mod ini;

#[tokio::main]
async fn main() {
    let win = MainWindow::new().unwrap();

    win.on_exit(move || {
        println!("Exiting");
        slint::quit_event_loop();
    });

    let installdir = manage::EldenRingDir::autodetect_install_path();
    if let Some(ref p) = installdir {
        win.set_install_path(p.display().into());
    }

    get_releases(&win, installdir.clone());

    win.on_new_password({
        let weak_win = win.as_weak();
        let installdir = installdir.clone();
        move |password| {
            let win = weak_win.unwrap();
            if let Some(installdir) = &installdir {
                if let Err(e) = installdir.set_password(&password) {
                    win.set_error(e.to_string().into());
                }
            }
        }
    });

    win.on_launch({
        let weak_win = win.as_weak();
        let installdir = installdir.clone();
        move || {
            let win = weak_win.unwrap();
            let installdir = installdir.as_ref().unwrap(); // unwrap can't fail because ui won't call us unless it's Some
            if let Err(e) = launch(installdir) {
                win.set_error(e.to_string().into());
            }
        }
    });

    win.on_refresh({
        let weak_win = win.as_weak();
        move || {
            let win = weak_win.unwrap();
            println!("Refreshing");
            get_releases(&win, installdir.clone());
        }
    });

    win.on_open_url(|url| {
        let _ = webbrowser::open(&url);
    });

    if let Some(v) = option_env!("VERSION") { win.set_my_version(v.into()); }

    if let Some(v) = manage::self_upgrade_version().unwrap_or(None) { win.set_my_upgrade_version(v.into()) }

    win.run();
}

fn get_releases(win: &MainWindow, installdir: Option<manage::EldenRingDir>) {
    match manage::get_releases() {
        Err(e) => {
            win.set_error(e.to_string().into());
            win.set_fatal_error(true);
        },
        Ok(r) => {
            let releases = Rc::new(RefCell::new(r));
            releases.borrow_mut().sort_by(|a,b| b.date.cmp(&a.date));
            //println!("Releases:\n{:?}", releases);
            win.set_available_versions(Rc::new(slint::VecModel::<slint::SharedString>::from(releases.borrow().iter()
                                                                                            .map(|r| format!("{}  --  {}  {}",
                                                                                                             r.tag, r.date,
                                                                                                             if r.downloaded() { "[ Downloaded ]" } else { "" }).into())
                                                                                            .collect::<Vec<slint::SharedString>>())).into());

            win.set_current_version("".into());
            if let Some(ref installdir) = installdir {
                if let Some(release) = releases.borrow().iter().find(|&release| release.installed(&installdir).unwrap_or(false)) {
                    win.set_current_version(release.tag.clone().into());
                }
                match installdir.get_password() {
                    Ok(ref password) => { win.set_password(password.into()) },
                    Err(e) => { println!("Error: {:?}", e) },
                }
            }

            win.on_version_at_index({
                let releases = releases.clone();
                move |version_index| {
                    let version = &releases.borrow()[version_index as usize];
                    version.tag.clone().into()
                }
            });

            win.on_changelog_at_index({
                let releases = releases.clone();
                move |version_index| {
                    let version = &releases.borrow()[version_index as usize];
                    version.changelog.clone().into()
                }
            });

            if let Some(installdir) = installdir {
                win.on_install({
                    let releases = releases.clone();
                    let weak_win = win.as_weak();
                    move |version_index| {
                        let win = weak_win.unwrap();
                        let version = &releases.borrow()[version_index as usize];
                        println!("Installing {}", version.tag);
                        if let Err(e) = version.install(&installdir) {
                            win.set_error(e.to_string().into());
                        }
                    }
                });
            }
        }
    };
}

fn launch(installdir: &manage::EldenRingDir) -> Result<(), Box<dyn std::error::Error>> {
    let exe = installdir.path().join("launch_elden_ring_seamlesscoop.exe");
    println!("Launching {:?}", &exe);
    if !exe.is_file() {
        Err(format!("Couldn't find {:?} to launch", exe))?;
    }
    let mut child = std::process::Command::new(exe.clone())
        .current_dir(&exe.parent().ok_or(format!("Couldn't find parent directory for {}", &exe.display()))?)
        .spawn().map_err(|e| format!("Launching {:?} failed: {}", &exe, e))?;
    std::thread::spawn(move || {
        let _ = child.wait(); // we really don't care if it failed
    });
    Ok(())
}

slint::slint! {
    import { Button, ComboBox, LineEdit, ScrollView } from "std-widgets.slint";
    LightText := Text {
        color: white;
    }

    Frame := Rectangle {
        background: #000000aa;
        border-color: #000000;
        border-width: 1px;
        border-radius: 5px;
    }

    MainWindow := Window {
        callback install(int);
        callback version-at-index(int) -> string;
        callback changelog-at-index(int) -> string;
        callback launch;
        callback exit;
        callback refresh;
        callback new-password(string);
        callback open-url(string);
        property<string> install-path;
        property<string> current-version;
        property<[string]> available-versions;
        property<string> error;
        property<bool> fatal-error: false;
        property<string> my-version: "0.0.0-local";
        property<string> my-upgrade-version: "";
        property<bool> show-password: false;
        property password <=> pass.text;

        title: "Elden Ring Seamless Co-op Manager  v" + my-version;
        icon: @image-url("assets/eldenringlogo.jpg");
        default-font-size: 16px;
        max-width: 10000px;

        Rectangle {
            width: Math.max(parent.height,parent.width);
            height: Math.max(parent.height,parent.width);
            Image {
                source: root.error == "" ? @image-url("assets/eldenring.jpg") : @image-url("assets/youdied.png");
                image-fit: cover;
                width: parent.height;
                height: parent.height;
            }
        }
        VerticalLayout {
            padding-top: 180px;
            padding-bottom: 30px;
            padding-left: 30px;
            padding-right: 30px;
            spacing: 30px;

            Frame {
                vertical-stretch: 0;
                GridLayout {
                    visible: root.error == "";
                    padding: 50px;
                    spacing: 10px;
                    Row {
                        LightText {
                            text: "Elden Ring:";
                        }
                        LightText {
                            wrap: word-wrap;
                            text: root.install-path == "" ? "<Not Found>" : root.install-path;
                        }
                    }
                    Row {
                        LightText {
                            text: "Current Mod Version:";
                        }
                        LightText {
                            text: root.current-version == "" ? "<Unknown>" : root.current-version;
                        }
                    }
                    Row {
                        LightText {
                            text: "New Mod Version:";
                        }
                        cb := ComboBox {
                            model: root.available-versions;
                            selected => {
                                changelog-scroll.viewport-y = 0;
                            }
                        }
                        Button {
                            text: root.current-version == root.version-at-index(cb.current-index) ? "Reinstall" : "Install";
                            enabled: root.install-path != "" && cb.current-index != -1;
                            clicked => {
                                root.install(cb.current-index);
                                if (root.error != "") { return; }
                                root.new-password(pass.text);
                                if (root.error != "") { return; }
                                root.refresh();
                                cb.current-value = cb.model[cb.current-index];
                            }
                            min-width: 1.5in;
                        }
                    }
                    Row {
                        LightText {
                            text: "Password:";
                        }
                        pass := LineEdit {
                            input-type: root.show-password ? InputType.text : InputType.password;
                            edited => {
                                root.new-password(pass.text)
                            }
                            accepted => {
                                root.new-password(pass.text)
                            }
                        }
                        Rectangle {
                            Image {
                                colorize: white;
                                source: root.show-password ? @image-url("assets/eye-slash-fill.svg") : @image-url("assets/eye-fill.svg");
                                image-fit: cover;
                                width: parent.height;
                            }
                            TouchArea {
                                clicked => {
                                    root.show-password = !root.show-password;
                                }
                            }
                        }
                    }
                    Row {
                        Button {
                            text: "Launch";
                            colspan: 3;
                            clicked => {
                                root.launch()
                            }

                            enabled: root.install-path != "" && cb.current-index != -1;
                        }
                    }
                }
                GridLayout {
                    visible: root.error != "";
                    padding: 50px;
                    spacing: 10px;
                    Row {
                        LightText {
                            text: "I'm terribly sorry but an error occurred!";
                            font-size: 36px;
                            font-weight: 900;
                        }
                    }
                    Row {
                        LightText {
                            text: root.error;
                            wrap: word-wrap;
                            max-width: 720px;
                        }
                    }
                    Row {
                        Button {
                            visible: root.fatal-error == true;
                            text: "Exit";
                            clicked => {
                                root.exit()
                            }
                        }
                    }
                    Row {
                        Button {
                            visible: root.fatal-error == false;
                            text: "Sigh... Ok";
                            clicked => {
                                root.error = ""
                            }
                        }
                    }
                }
            }
            Frame {
                visible: root.error == "";
                VerticalLayout {
                    spacing: 10px;
                    padding: 50px;
                    LightText {
                        font-size: 24px;
                        font-weight: 750;
                        text: root.version-at-index(cb.current-index) + " Release Notes";
                    }
                    changelog-scroll := ScrollView {
                        min-height:changelog.font-size*10;
                        viewport-height: changelog.height;

                        changelog := LightText {
                            font-size: 16px;
                            vertical-stretch: 1;
                            x: 5px;
                            width: parent.width - 25px;
                            wrap: word-wrap;
                            text: root.changelog-at-index(cb.current-index);
                        }
                    }
                }
            }
        }
        HorizontalLayout {
            y: parent.height - height;
            height: 12px;
            alignment: end;

            Rectangle {
                background: black;
                HorizontalLayout {
                    padding-left: 3px;
                    spacing: 3px;
                    alignment: start;
                    copyright := Text {
                        font-size: 10px;
                        color: white;
                        text: "© 2022 David Caldwell";
                    }
                    octocat := Image {
                        colorize: white;
                        source: @image-url("assets/github.svg");
                        height: 9px;
                        width: 9px;
                    }
                }
                TouchArea {
                    clicked => {
                        root.open-url("https://github.com/caldwell/erscom");
                    }
                }
            }
            Rectangle { // spacer
                background: black;
                width: 30px;
            }
        }
        if root.my-upgrade-version != "" : Rectangle {
            height: 20px;
            background: black;
            HorizontalLayout {
                alignment: center;
                HorizontalLayout {
                    alignment: start;
                    spacing: 5px;
                    Image {
                        colorize: white;
                        source: @image-url("assets/cloud-arrow-down-fill.svg");
                        width: 20px;
                        height: 20px;
                    }
                    Text {
                        text: "Download New Manager Version "+root.my-upgrade-version;
                        color: white;
                        font-size: 18px;
                        font-weight: 700;
                    }
                }
            }
            TouchArea {
                clicked => {
                    root.open-url("https://github.com/caldwell/erscom/releases/latest");
                }
            }
        }
    }
}
