import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.notesplus 1.0
import "../components/common"

Page {
    id: backupPage
    allowedOrientations: Orientation.All

    RemorsePopup {
        id: backupRemorsePopup
    }

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height + Theme.paddingLarge

        Column {
            id: column
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: qsTr("Backup & Export")
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: qsTr("Note Backups")
                    color: Theme.highlightColor
                    font.bold: true
                    font.pixelSize: Theme.fontSizeMedium
                }

                Label {
                    width: parent.width
                    text: qsTr("Export your notes as formatted PDF documents or self-contained HTML5 files. Exported files are saved to the Notes++ Exports folder in your Documents directory.")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }
            }

            SectionHeader {
                text: qsTr("Export All Notes")
            }

            Button {
                anchors.horizontalCenter: parent.horizontalCenter
                text: qsTr("Export All Notes as PDF")
                onClicked: {
                    var doExportAll = function() {
                        var out = bridge.export_all_pdf()
                        if (out) {
                            try {
                                var arr = JSON.parse(out)
                                backupRemorsePopup.execute(qsTr("Exported %1 notes to PDF").arg(arr.length), function() {})
                            } catch (e) {
                                backupRemorsePopup.execute(qsTr("Exported notes to PDF"), function() {})
                            }
                        }
                    }

                    if (bridge.any_pdf_export_exists()) {
                        var dialog = pageStack.push(Qt.resolvedUrl("../dialogs/ConfirmDialog.qml"), {
                            title: qsTr("Overwrite Existing PDFs?"),
                            message: qsTr("Some notes already have exported PDF files in Notes++ Exports. Do you want to overwrite them?"),
                            acceptText: qsTr("Overwrite")
                        })
                        dialog.accepted.connect(function() {
                            doExportAll()
                        })
                    } else {
                        doExportAll()
                    }
                }
            }

            Button {
                anchors.horizontalCenter: parent.horizontalCenter
                text: qsTr("Export All Notes as HTML5")
                onClicked: {
                    var out = bridge.export_all_html()
                    if (out) {
                        backupRemorsePopup.execute(qsTr("Exported to %1").arg(out), function() {})
                    }
                }
            }
        }
    }
}
