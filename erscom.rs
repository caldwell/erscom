// Copyright © 2022 David Caldwell <david@porkrind.org>

// This removes the ugly debug window that comes up on windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::rc::Rc;

mod manage;

#[tokio::main]
async fn main() {
    let win = MainWindow::new();
    win.on_install(move |version| {
        println!("Installing {}", version);
    });
    win.on_launch(move || {
        println!("Launching");
    });
    win.on_locate(move || {
        println!("Locating");
    });
    win.on_exit(move || {
        println!("Exiting");
        slint::quit_event_loop();
    });

    let installdir = manage::autodetect_install_path();
    win.set_install_path(installdir.unwrap_or("".to_string()).into());

    win.set_current_version("v1.2.5".into());
    match manage::get_releases() {
        Err(e) => { win.set_error(e.to_string().into()); },
        Ok(mut releases) => {
            releases.sort_by(|a,b| b.date.cmp(&a.date));
            //println!("Releases:\n{:?}", releases);
            win.set_available_versions(Rc::new(slint::VecModel::<slint::SharedString>::from(releases.iter().map(|r| r.tag.clone().into()).collect::<Vec<slint::SharedString>>())).into());
        }
    };
    win.run();
}

slint::slint! {
    import { Button, ComboBox } from "std-widgets.slint";
    LightText := Text {
        color: white;
    }

    MainWindow := Window {
        callback install(string);
        callback launch;
        callback locate;
        callback exit;
        property<string> install-path;
        property<string> current-version;
        property<[string]> available-versions;
        property<string> error;

        title: "Elden Ring Seamless Co-op Manager";
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
                            text: root.current-version;
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
                            clicked => {
                                root.install(cb.current-value)
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
