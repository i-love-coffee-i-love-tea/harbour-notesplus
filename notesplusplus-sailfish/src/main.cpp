/* main.cpp — Sailfish OS bootstrap for harbour-notesplusplus (C++ bridge).
 *
 * Registers the C++ QObject bridge types with the QML engine.
 * Replaces the Rust main.rs that used qmetaobject crate.
 */

#include <QGuiApplication>
#include <QQmlContext>
#include <QQuickView>
#include <QUrl>
#include <sailfishapp.h>

#include "bridge/NotesBridge.h"
#include "bridge/AgentBridge.h"
#include "bridge/SpeechBridge.h"

int main(int argc, char* argv[])
{
    QGuiApplication* app = SailfishApp::application(argc, argv);
    app->setApplicationName("harbour-notesplusplus");
    app->setOrganizationName("org.gobuki");

    QQuickView* view = SailfishApp::createView();

    /* Register QML types under harbour.notesplusplus 1.0 */
    qmlRegisterType<NotesBridge>("harbour.notesplusplus", 1, 0, "NotesBridge");
    qmlRegisterType<AgentBridge>("harbour.notesplusplus", 1, 0, "AgentBridge");
    qmlRegisterType<SpeechBridge>("harbour.notesplusplus", 1, 0, "SpeechBridge");

    /* Set the QML source */
    QUrl qmlUrl;
    QFile localQml("/usr/share/harbour-notesplusplus/qml/harbour-notesplusplus.qml");
    if (localQml.exists()) {
        qmlUrl = QUrl::fromLocalFile("/usr/share/harbour-notesplusplus/qml/harbour-notesplusplus.qml");
    } else {
        qmlUrl = SailfishApp::pathTo("qml/harbour-notesplusplus.qml");
    }
    view->setSource(qmlUrl);
    view->showFullScreen();

    return app->exec();
}
