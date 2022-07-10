// Copyright © 2022 David Caldwell <david@porkrind.org>

// This removes the ugly debug window that comes up on windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::cell::RefCell;
use std::rc::Rc;

mod manage;
mod ini;

#[tokio::main]
async fn main() {
    let win = MainWindow::new();
    win.on_locate(move || {
        println!("Locating");
    });
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

    if let Some(v) = option_env!("VERSION") { win.set_my_version(v.into()); }

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

            win.on_install({
                let releases = releases.clone();
                let installdir = installdir.expect("Can't happen").clone();
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
        callback locate;
        callback exit;
        callback refresh;
        callback new-password(string);
        property<string> install-path;
        property<string> current-version;
        property<[string]> available-versions;
        property<string> error;
        property<bool> fatal-error: false;
        property<string> my-version: "0.0.0-local";
        property<bool> show-password: false;
        property password <=> pass.text;

        title: "Elden Ring Seamless Co-op Manager  v" + my-version;
        default-font-size: 16px;
        Rectangle {
            width: Math.max(parent.height,parent.width);
            height: Math.max(parent.height,parent.width);
            Image {
                source: root.error == "" ? @image-url("eldenring.jpg") : @image-url("youdied.png");
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
                            text: "Path:";
                        }
                        LightText {
                            text: root.install-path == "" ? "<Not Found>" : root.install-path;
                        }
                        Button {
                            visible: false;
                            text: "Locate";
                            clicked => {
                                root.locate()
                            }
                        }
                    }
                    Row {
                        LightText {
                            text: "Current Version:";
                        }
                        LightText {
                            text: root.current-version == "" ? "<Unknown>" : root.current-version;
                        }
                    }
                    Row {
                        LightText {
                            text: "New Version:";
                        }
                        cb := ComboBox {
                            model: root.available-versions;
                            selected => {
                                changelog-scroll.viewport-y = 0;
                            }
                        }
                        Button {
                            text: root.current-version == root.version-at-index(cb.current-index) ? "Reinstall" : "Install";
                            enabled: root.install-path != "";
                            clicked => {
                                root.install(cb.current-index);
                                root.new-password(pass.text);
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
                                source: root.show-password ? @image-url("eye-slash-fill.svg") : @image-url("eye-fill.svg");
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

                            enabled: root.install-path != "";
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
    }
}
