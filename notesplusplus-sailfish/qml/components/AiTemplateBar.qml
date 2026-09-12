import QtQuick 2.0
import Sailfish.Silica 1.0

SilicaFlickable {
    id: templateBar
    width: parent.width
    height: templateRow.height + Theme.paddingSmall
    contentWidth: templateRow.width + Theme.horizontalPageMargin * 2
    clip: true

    property bool enabled: true

    signal templateSelected(string templateName)

    Row {
        id: templateRow
        x: Theme.horizontalPageMargin
        spacing: Theme.paddingSmall

        Button {
            text: qsTr("✨ Beautify")
            preferredWidth: Theme.buttonWidthExtraSmall
            enabled: templateBar.enabled
            opacity: enabled ? 1.0 : 0.4
            onClicked: templateBar.templateSelected("beautify")
        }

        Button {
            text: qsTr("📋 Extract To-Dos")
            preferredWidth: Theme.buttonWidthExtraSmall
            enabled: templateBar.enabled
            opacity: enabled ? 1.0 : 0.4
            onClicked: templateBar.templateSelected("extract_todos")
        }

        Button {
            text: qsTr("✍️ Fix Grammar")
            preferredWidth: Theme.buttonWidthExtraSmall
            enabled: templateBar.enabled
            opacity: enabled ? 1.0 : 0.4
            onClicked: templateBar.templateSelected("fix_grammar")
        }

        Button {
            text: qsTr("📝 Expand & Draft")
            preferredWidth: Theme.buttonWidthExtraSmall
            enabled: templateBar.enabled
            opacity: enabled ? 1.0 : 0.4
            onClicked: templateBar.templateSelected("expand_draft")
        }

        Button {
            text: qsTr("🌐 External Text")
            preferredWidth: Theme.buttonWidthExtraSmall
            enabled: templateBar.enabled
            opacity: enabled ? 1.0 : 0.4
            onClicked: templateBar.templateSelected("analyze_external")
        }
    }
}
