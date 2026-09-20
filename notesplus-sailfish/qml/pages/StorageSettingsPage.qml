import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.notesplus 1.0

Page {
    id: storageSettingsPage
    allowedOrientations: Orientation.All

    RemorsePopup {
        id: storageRemorsePopup
    }

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height + Theme.paddingLarge

        Column {
            id: column
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: qsTr("Storage")
            }

            SectionHeader {
                text: qsTr("Storage Location")
            }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - Theme.horizontalPageMargin * 2
                text: qsTr("Directory where notes, journals, and attachments are stored. Changing location safely copies all your notes.")
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
                wrapMode: Text.Wrap
            }

            TextField {
                id: notesPathField
                width: parent.width
                label: qsTr("Notes Storage Path")
                labelVisible: true
                text: bridge.notes_dir
                placeholderText: bridge.default_notes_dir
                EnterKey.iconSource: "image://theme/icon-m-enter-accept"
                EnterKey.onClicked: focus = false
            }

            Connections {
                target: bridge
                onNotes_dir_changed: {
                    notesPathField.text = bridge.notes_dir
                }
            }

            Row {
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Theme.paddingMedium

                Button {
                    text: qsTr("Migrate Storage...")
                    enabled: notesPathField.text.trim().length > 0 &&
                             notesPathField.text.trim() !== bridge.notes_dir &&
                             !bridge.is_migrating
                    onClicked: {
                        var target = notesPathField.text.trim()
                        var dialog = pageStack.push(Qt.resolvedUrl("../dialogs/MigrateStorageDialog.qml"), {
                            sourcePath: bridge.notes_dir,
                            targetPath: target
                        })
                        dialog.accepted.connect(function() {
                            if (typeof app !== "undefined" && app && app.setNotesPath) {
                                app.setNotesPath(target)
                            }
                        })
                    }
                }

                Button {
                    text: qsTr("Reset to Default")
                    visible: bridge.notes_dir !== bridge.default_notes_dir
                    enabled: !bridge.is_migrating
                    onClicked: {
                        var target = bridge.default_notes_dir
                        notesPathField.text = target
                        var dialog = pageStack.push(Qt.resolvedUrl("../dialogs/MigrateStorageDialog.qml"), {
                            sourcePath: bridge.notes_dir,
                            targetPath: target
                        })
                        dialog.accepted.connect(function() {
                            if (typeof app !== "undefined" && app && app.setNotesPath) {
                                app.setNotesPath(target)
                            }
                        })
                    }
                }
            }

            SectionHeader {
                text: qsTr("Database & Search Index")
            }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - Theme.horizontalPageMargin * 2
                text: qsTr("Rebuild the full-text search index if search results are incomplete or out of date.")
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
                wrapMode: Text.Wrap
            }

            Button {
                anchors.horizontalCenter: parent.horizontalCenter
                text: qsTr("Rebuild Search Index")
                onClicked: {
                    var res = bridge.rebuild_index()
                    if (res && res.indexOf("Error") === 0) {
                        storageRemorsePopup.execute(res, function() {})
                    } else {
                        storageRemorsePopup.execute(qsTr("Search index rebuilt"), function() {})
                    }
                }
            }
        }
    }
}
