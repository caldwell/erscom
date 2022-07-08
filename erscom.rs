// Copyright © 2022 David Caldwell <david@porkrind.org>

// This removes the ugly debug window that comes up on windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::rc::Rc;

fn main() {
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
    win.set_install_path("C:/blahblahblah/Steam/Something/Something/Elden Ring/Whatevs".into());
    win.set_current_version("1.1".into());
    win.set_available_versions(Rc::new(slint::VecModel::<slint::SharedString>::from(vec!["1.5".into(), "1.6".into(), "1.7".into(), "1.1".into()])).into());
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
        property<string> install-path;
        property<string> current-version;
        property<[string]> available-versions;

        title: "Elden Ring Seamless Co-op Manager";
        Rectangle {
            width: Math.max(parent.height,parent.width);
            height: Math.max(parent.height,parent.width);
            Image {
                source: @image-url("eldenring.jpg");
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
                    padding: 50px;
                    spacing: 10px;
                    Row {
                        LightText {
                            text: "Path:";
                        }
                        LightText {
                            text: root.install-path;
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
                            current-value: "1.7";
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
            }
        }
    }
}
