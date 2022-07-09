// Copyright © 2022 David Caldwell <david@porkrind.org>

// This removes the ugly debug window that comes up on windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::cell::RefCell;
use std::rc::Rc;

mod manage;

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

    let installdir = manage::autodetect_install_path();
    if let Some(ref p) = installdir {
        win.set_install_path(p.to_string_lossy().into_owned().into());
    }

    get_releases(&win, installdir.clone());

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

fn get_releases(win: &MainWindow, installdir: Option<std::path::PathBuf>) {
    match manage::get_releases() {
        Err(e) => { win.set_error(e.to_string().into()); },
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
            }

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

fn launch(installdir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let exe = installdir.join("Game").join("launch_elden_ring_seamlesscoop.exe");
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
    import { Button, ComboBox } from "std-widgets.slint";
    LightText := Text {
        color: white;
    }

    MainWindow := Window {
        callback install(int);
        callback launch;
        callback locate;
        callback exit;
        callback refresh;
        property<string> install-path;
        property<string> current-version;
        property<[string]> available-versions;
        property<string> error;
        property<string> my-version: "0.0.0-local";

        title: "Elden Ring Seamless Co-op Manager  v" + my-version;
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
            padding-top: 150px;
            padding-bottom: 250px;
            padding-left: 30px;
            padding-right: 30px;

            Rectangle {
                background: #000000aa;
                border-color: #000000;
                border-width: 1px;
                border-radius: 5px;
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
                        }
                        Button {
                            text: root.current-version == cb.current-value ? "Reinstall" : "Install";
                            enabled: root.install-path != "";
                            clicked => {
                                root.install(cb.current-index);
                                root.refresh();
                                cb.current-value = cb.model[cb.current-index];
                            }
                            min-width: 1.5in;
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
                            text: "Exit";
                            clicked => {
                                root.exit()
                            }
                        }
                    }
                }
            }
        }
    }
}
