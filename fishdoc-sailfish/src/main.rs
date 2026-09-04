use qmetaobject::*;
use std::ffi::CStr;

mod bridge;

fn main() {
    let mut app = sailors::sailfishapp::QmlApp::application("harbour-fishdoc".into());
    app.set_quit_on_last_window_closed(false);
    app.promote_gui_app_to_qml_context("RootApp".into());

    let uri = CStr::from_bytes_with_nul(b"harbour.fishdoc\0").unwrap();
    let name = CStr::from_bytes_with_nul(b"FishdocBridge\0").unwrap();
    qml_register_type::<bridge::FishdocBridge>(uri, 1, 0, name);

    app.set_source(sailors::sailfishapp::QmlApp::path_to(
        "qml/harbour-fishdoc.qml".into(),
    ));
    app.show_full_screen();
    app.exec();
}
