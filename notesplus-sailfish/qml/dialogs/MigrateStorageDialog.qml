import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.notesplus 1.0
import "../components/common"

Dialog {
    id: migrateStorageDialog
    allowedOrientations: Orientation.All

    property string sourcePath: ""
    property string targetPath: ""
    property var scanStats: ({})

    canAccept: bridge.migration_finished && bridge.migration_success
    backNavigation: !bridge.is_migrating

    onOpened: {
        bridge.reset_migration_state()
        if (sourcePath.length > 0) {
            scanStats = bridge.scan_notes_dir(sourcePath)
        }
    }

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: contentColumn.height + Theme.paddingLarge

        Column {
            id: contentColumn
            width: parent.width
            spacing: Theme.paddingMedium

            DialogHeader {
                title: {
                    if (bridge.is_migrating) return qsTr("Migrating Notes...")
                    if (bridge.migration_finished) {
                        return bridge.migration_success ? qsTr("Migration Complete") : qsTr("Migration Failed")
                    }
                    return qsTr("Migrate Notes Storage")
                }
                acceptText: (bridge.migration_finished && bridge.migration_success) ? qsTr("Done") : ""
                cancelText: bridge.is_migrating ? "" : ((bridge.migration_finished && !bridge.migration_success) ? qsTr("Close") : qsTr("Cancel"))
            }

            // ==========================================================
            // Phase 1: Confirmation
            // ==========================================================
            Column {
                width: parent.width
                spacing: Theme.paddingMedium
                visible: !bridge.is_migrating && !bridge.migration_finished

                InfoCard {
                    Label {
                        width: parent.width
                        text: qsTr("Storage Location Change")
                        color: Theme.highlightColor
                        font.bold: true
                        font.pixelSize: Theme.fontSizeMedium
                    }

                    Label {
                        width: parent.width
                        text: qsTr("From (Current):")
                        color: Theme.secondaryColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                    }
                    Label {
                        width: parent.width
                        text: sourcePath
                        color: Theme.primaryColor
                        font.pixelSize: Theme.fontSizeSmall
                        wrapMode: Text.WrapAnywhere
                    }

                    Item { width: parent.width; height: Theme.paddingSmall }

                    Label {
                        width: parent.width
                        text: qsTr("To (New Location):")
                        color: Theme.secondaryColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                    }
                    Label {
                        width: parent.width
                        text: targetPath
                        color: Theme.highlightColor
                        font.pixelSize: Theme.fontSizeSmall
                        wrapMode: Text.WrapAnywhere
                    }
                }

                InfoCard {
                    Label {
                        width: parent.width
                        text: qsTr("Files to Migrate")
                        color: Theme.highlightColor
                        font.bold: true
                        font.pixelSize: Theme.fontSizeSmall
                    }

                    Label {
                        width: parent.width
                        text: qsTr("Found %1 note file(s) and %2 folder(s) (%3) in current storage.")
                            .arg(scanStats.notes_count !== undefined ? scanStats.notes_count : 0)
                            .arg(scanStats.folders_count !== undefined ? scanStats.folders_count : 0)
                            .arg(scanStats.formatted_size !== undefined ? scanStats.formatted_size : "0 B")
                        color: Theme.primaryColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                        wrapMode: Text.Wrap
                    }

                    Label {
                        width: parent.width
                        text: qsTr("All notes, journal entries, and folders will be copied to the new directory. Original files will remain untouched as a backup.")
                        color: Theme.secondaryColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                        wrapMode: Text.Wrap
                    }
                }

                Item { width: parent.width; height: Theme.paddingMedium }

                Button {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: qsTr("Start Migration")
                    onClicked: {
                        bridge.start_notes_migration(targetPath)
                    }
                }
            }

            // ==========================================================
            // Phase 2: In-Progress (Live Migration Process)
            // ==========================================================
            Column {
                width: parent.width
                spacing: Theme.paddingMedium
                visible: bridge.is_migrating

                ProgressBar {
                    width: parent.width - Theme.horizontalPageMargin * 2
                    anchors.horizontalCenter: parent.horizontalCenter
                    minimumValue: 0
                    maximumValue: 100
                    value: bridge.migration_progress
                    label: bridge.migration_status
                    valueText: Math.round(value) + "%"
                }

                BusyIndicator {
                    running: bridge.is_migrating
                    size: BusyIndicatorSize.Medium
                    anchors.horizontalCenter: parent.horizontalCenter
                }

                Label {
                    visible: bridge.migration_total_count > 0 && bridge.migration_current_file.length > 0
                    width: parent.width - Theme.horizontalPageMargin * 2
                    anchors.horizontalCenter: parent.horizontalCenter
                    horizontalAlignment: Text.AlignHCenter
                    text: qsTr("Copying [%1/%2]: %3")
                        .arg(bridge.migration_copied_count)
                        .arg(bridge.migration_total_count)
                        .arg(bridge.migration_current_file)
                    font.pixelSize: Theme.fontSizeExtraSmall
                    color: Theme.secondaryColor
                    truncationMode: TruncationMode.Fade
                }

                InfoCard {
                    Label {
                        width: parent.width
                        text: (bridge.migration_progress >= 5 ? "✓ " : "• ") + qsTr("1. Prepare target directory")
                        color: bridge.migration_progress >= 5 ? Theme.highlightColor : Theme.secondaryColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                    }
                    Label {
                        width: parent.width
                        text: (bridge.migration_progress >= 85 ? "✓ " : (bridge.migration_progress >= 5 ? "▶ " : "• ")) + qsTr("2. Copy notes and subfolders")
                        color: bridge.migration_progress >= 85 ? Theme.highlightColor : (bridge.migration_progress >= 5 ? Theme.primaryColor : Theme.secondaryColor)
                        font.pixelSize: Theme.fontSizeExtraSmall
                    }
                    Label {
                        width: parent.width
                        text: (bridge.migration_progress >= 95 ? "✓ " : (bridge.migration_progress >= 85 ? "▶ " : "• ")) + qsTr("3. Update SQLite search index")
                        color: bridge.migration_progress >= 95 ? Theme.highlightColor : (bridge.migration_progress >= 85 ? Theme.primaryColor : Theme.secondaryColor)
                        font.pixelSize: Theme.fontSizeExtraSmall
                    }
                    Label {
                        width: parent.width
                        text: (bridge.migration_progress >= 100 ? "✓ " : (bridge.migration_progress >= 95 ? "▶ " : "• ")) + qsTr("4. Reload notebook tree")
                        color: bridge.migration_progress >= 100 ? Theme.highlightColor : (bridge.migration_progress >= 95 ? Theme.primaryColor : Theme.secondaryColor)
                        font.pixelSize: Theme.fontSizeExtraSmall
                    }
                }
            }

            // ==========================================================
            // Phase 3: Completed
            // ==========================================================
            Column {
                width: parent.width
                spacing: Theme.paddingMedium
                visible: bridge.migration_finished

                InfoCard {
                    visible: bridge.migration_success

                    Label {
                        width: parent.width
                        text: qsTr("Migration Completed Successfully!")
                        color: Theme.highlightColor
                        font.bold: true
                        font.pixelSize: Theme.fontSizeMedium
                    }

                    Label {
                        width: parent.width
                        text: qsTr("Successfully migrated %1 file(s) to:\n%2").arg(bridge.migration_copied_count).arg(targetPath)
                        color: Theme.primaryColor
                        font.pixelSize: Theme.fontSizeSmall
                        wrapMode: Text.WrapAnywhere
                    }

                    Label {
                        width: parent.width
                        text: qsTr("Your active notes directory has been updated. Original files remain safely preserved in previous location.")
                        color: Theme.secondaryColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                        wrapMode: Text.Wrap
                    }
                }

                InfoCard {
                    visible: !bridge.migration_success

                    Label {
                        width: parent.width
                        text: qsTr("Migration Failed")
                        color: Theme.errorColor !== undefined ? Theme.errorColor : Theme.highlightColor
                        font.bold: true
                        font.pixelSize: Theme.fontSizeMedium
                    }

                    Label {
                        width: parent.width
                        text: bridge.migration_error.length > 0 ? bridge.migration_error : qsTr("An unknown error occurred during migration.")
                        color: Theme.primaryColor
                        font.pixelSize: Theme.fontSizeSmall
                        wrapMode: Text.Wrap
                    }

                    Label {
                        width: parent.width
                        text: qsTr("Your notes and active storage location remain unchanged.")
                        color: Theme.secondaryColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                        wrapMode: Text.Wrap
                    }
                }

                Item { width: parent.width; height: Theme.paddingMedium }

                Button {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: bridge.migration_success ? qsTr("Done") : qsTr("Close")
                    onClicked: {
                        if (bridge.migration_success) {
                            migrateStorageDialog.accept()
                        } else {
                            migrateStorageDialog.reject()
                        }
                    }
                }
            }
        }
    }
}
