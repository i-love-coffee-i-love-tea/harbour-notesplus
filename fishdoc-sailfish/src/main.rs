use qmetaobject::*;
use std::ffi::CStr;
use std::path::PathBuf;

mod bridge;

fn main() {
    let mut app = sailors::sailfishapp::QmlApp::application("harbour-fishdoc".into());
    app.set_quit_on_last_window_closed(false);
    app.promote_gui_app_to_qml_context("RootApp".into());

    let uri = CStr::from_bytes_with_nul(b"harbour.fishdoc\0").unwrap();
    let name = CStr::from_bytes_with_nul(b"FishdocBridge\0").unwrap();
    qml_register_type::<bridge::FishdocBridge>(uri, 1, 0, name);

    let agent_name = CStr::from_bytes_with_nul(b"AgentBridge\0").unwrap();
    qml_register_type::<bridge::AgentBridge>(uri, 1, 0, agent_name);

    let qml_file = PathBuf::from("/usr/share/harbour-fishdoc/qml/harbour-fishdoc.qml");
    let qml_url = if qml_file.exists() {
        QUrl::from(QString::from("file:///usr/share/harbour-fishdoc/qml/harbour-fishdoc.qml"))
    } else {
        sailors::sailfishapp::QmlApp::path_to("qml/harbour-fishdoc.qml".into())
    };
    app.set_source(qml_url);
    app.show_full_screen();
    app.exec();
}
