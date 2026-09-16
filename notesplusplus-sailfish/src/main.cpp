/* main.cpp — Sailfish OS bootstrap for harbour-notesplus (C++ bridge).
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
    app->setApplicationName("harbour-notesplus");
    app->setOrganizationName("org.gobuki");

    QQuickView* view = SailfishApp::createView();

    /* Register QML types under harbour.notesplus 1.0 */
    qmlRegisterType<NotesBridge>("harbour.notesplus", 1, 0, "NotesBridge");
    qmlRegisterType<AgentBridge>("harbour.notesplus", 1, 0, "AgentBridge");
    qmlRegisterType<SpeechBridge>("harbour.notesplus", 1, 0, "SpeechBridge");

    /* Set the QML source */
    QUrl qmlUrl;
    QFile localQml("/usr/share/harbour-notesplus/qml/harbour-notesplus.qml");
    if (localQml.exists()) {
        qmlUrl = QUrl::fromLocalFile("/usr/share/harbour-notesplus/qml/harbour-notesplus.qml");
    } else {
        qmlUrl = SailfishApp::pathTo("qml/harbour-notesplus.qml");
    }
    view->setSource(qmlUrl);
    view->showFullScreen();

    return app->exec();
}
