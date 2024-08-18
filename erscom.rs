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
use std::error::Error;
use std::path::PathBuf;
use std::rc::Rc;

mod manage;
mod ini;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let win = MainWindow::new()?;

    win.on_exit(move || {
        println!("Exiting");
        log_error(slint::quit_event_loop(), "quitting event loop");
    });

    let manager = Rc::new(RefCell::new(manage::EldenRingManager::new()));
    if let Some(ref p) = manager.borrow().dir {
        win.set_install_path(p.display().into());
    }

    get_releases(&win, &manager.clone());

    win.on_new_password({
        let manager = manager.clone();
        move |password| {
            println!("New password: {}", password);
            if manager.borrow().found_dir() {
                if let Err(e) = manager.borrow().set_password(&password) {
                    error(e);
                    return false;
                }
            }
            true
        }
    });

    win.on_launch({
        let manager = manager.clone();
        move || {
            let manager = manager.borrow();
            if let Err(e) = (|| -> Result<_,Box<dyn Error>> { launch(manager.launcher_path()?) })() { // My world for try/catch!!
                error(e);
            }
        }
    });

    win.on_refresh({
        let weak_win = win.as_weak();
        move || {
            let win = weak_win.unwrap();
            println!("Refreshing");
            get_releases(&win, &manager.clone());
        }
    });

    win.on_open_url(|url| {
        let _ = webbrowser::open(&url);
    });

    if let Some(v) = option_env!("VERSION") { win.set_my_version(v.into()); }

    if let Some(v) = manage::self_upgrade_version().unwrap_or(None) { win.set_my_upgrade_version(v.into()) }

    win.run()?;
    Ok(())
}

fn error(error: Box<dyn Error>) {
    let dialog = ErrorDialog::new().unwrap();
    dialog.set_error(format!("{}", error).into());
    dialog.on_ok_clicked({
        let dialog = dialog.as_weak();
        move || {
            let dialog = dialog.unwrap();
            log_error(dialog.hide(), "hiding dialog");
        }
    });
    log_error(dialog.show(), &format!("showing error dialog for {}", error));
}

fn fatal(error: Box<dyn Error>) {
    let dialog = FatalDialog::new().unwrap();
    dialog.set_error(format!("{}", error).into());
    dialog.on_abort_clicked(move || {
            log_error(slint::quit_event_loop(), "quitting event loop");
    });
    log_error(dialog.show(), &format!("showing fatal dialog for {}", error));
}

fn log_error<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) {
    match result {
        Err(e) => println!("Error while {context}: {e}"),
        _ => {},
    }
}

fn get_releases(win: &MainWindow, manager_ref: &Rc<RefCell<manage::EldenRingManager>>) {
    let mut manager = manager_ref.borrow_mut();
    if let Err(e) = manager.fetch_releases() {
        return fatal(e);
    }

    //println!("Releases:\n{:?}", releases);
    win.set_available_versions(Rc::new(slint::VecModel::<slint::SharedString>::from(manager.releases.iter()
                                                                                    .map(|r| format!("{}  --  {}  {}",
                                                                                                     r.tag, r.date,
                                                                                                     if r.downloaded() { "[ Downloaded ]" } else { "" }).into())
                                                                                    .collect::<Vec<slint::SharedString>>())).into());

    win.set_current_version("".into());
    if let Some(release) = manager.detect_current_release() {
        win.set_current_version(release.tag.clone().into());
    }
    match manager.get_password() {
        Ok(ref password) => { win.set_password(password.into()) },
        Err(e) => { println!("Couldn't get password: {:?}", e) },
    }

    win.on_version_at_index({
        let releases = manager.releases.clone();
        move |version_index| {
            if version_index < 0 { return "".into(); }
            let version = &releases[version_index as usize];
            version.tag.clone().into()
        }
    });

    win.on_changelog_at_index({
        let releases = manager.releases.clone();
        move |version_index| {
            if version_index < 0 { return "".into(); }
            let version = &releases[version_index as usize];
            version.changelog.clone().into()
        }
    });

    if let Some(installdir) = manager.dir.clone() {
        win.on_install({
            let manager_ref = manager_ref.clone();
            move |version_index| {
                let manager = manager_ref.borrow();
                let version = &manager.releases[version_index as usize];
                if let Some(ref current) = manager.current {
                    println!("Uninstalling {}", current.tag);
                    if let Err(e) = current.uninstall(&installdir) {
                        println!("Got error uninstalling {}: {}", current.tag, e);
                        // What do do about errors??
                    }
                }
                println!("Installing {}", version.tag);
                if let Err(e) = version.install(&installdir) {
                    error(e);
                    return false;
                }
                true
            }
        });
    }
}

fn launch(exe: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
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
    import { Button, ComboBox, LineEdit, ScrollView, StandardButton } from "std-widgets.slint";
    component LightText inherits Text {
        color: white;
    }

    component Frame inherits Rectangle {
        background: #000000aa;
        border-color: #000000;
        border-width: 1px;
        border-radius: 5px;
    }

    component PasswordEdit {
        callback new-password(string) -> bool;
        in-out property text <=> pass.text;
        property<bool> show-password: false;

        Rectangle {
            pass := LineEdit {
                width: 100%;
                input-type: root.show-password ? InputType.text : InputType.password;
                edited => {
                    root.new-password(pass.text)
                }
                accepted => {
                    root.new-password(pass.text)
                }
            }
            Rectangle {
                width: image.width;
                x: pass.width - image.width - 5px;

                image := Image {
                    colorize: white;
                    source: root.show-password ? @image-url("assets/eye-slash-fill.svg") : @image-url("assets/eye-fill.svg");
                    image-fit: cover;
                    //width: self.height;
                }
                TouchArea {
                    clicked => {
                        root.show-password = !root.show-password;
                    }
                }
            }
        }
    }

    export component MainWindow inherits Window {
        callback install(int) -> bool;
        pure callback version-at-index(int) -> string;
        pure callback changelog-at-index(int) -> string;
        callback launch;
        callback exit;
        callback refresh;
        callback new-password(string) -> bool;
        callback open-url(string);
        in property<string> install-path;
        in property<string> current-version;
        in property<[string]> available-versions;
        in property<string> my-version: "0.0.0-local";
        in property<string> my-upgrade-version: "";
        property<bool> show-password: false;
        in-out property password <=> pass.text;

        title: "Elden Ring Seamless Co-op Manager  v" + my-version;
        icon: @image-url("assets/eldenringlogo.jpg");
        default-font-size: 16px;
        max-width: 10000px;

        Rectangle {
            width: Math.max(parent.height,parent.width);
            height: Math.max(parent.height,parent.width);
            y: 0;
            x: 0;
            Image {
                source: @image-url("assets/eldenring.jpg");
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
                                if (!root.install(cb.current-index)) { return; }
                                if (!root.new-password(pass.text)) { return; }
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
                        pass := PasswordEdit {
                            new-password(new) => { root.new-password(new) }
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
        HorizontalLayout {
            y: parent.height - self.height;
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

    component ErrorGuts inherits Rectangle {
        in property<string> error;

        image := Image {
            source: @image-url("assets/youdied.png");
            image-fit: contain;
            width: parent.width;
            height: parent.height;
        }

        Frame {
            GridLayout {
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
            }
        }
    }

    export component ErrorDialog inherits Dialog {
        in property<string> error <=> message.error;
        callback ok-clicked;

        background: black;
        title: "Error!";
        message := ErrorGuts {
        }
        Button {
            text: "Sigh... Ok";
            dialog-button-role: action;
            clicked => { ok_clicked() }
        }
    }

    export component FatalDialog inherits Dialog {
        in property<string> error <=> message.error;

        background: black;
        title: "Fatal Error!";
        message := ErrorGuts {
        }
        StandardButton { kind: abort; }
    }

}
