import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components/editor"
import "../js/BlockHtmlUtils.js" as BlockHtmlUtils

Dialog {
    id: pageSourceEditor
    allowedOrientations: Orientation.All

    property string pageName: ""
    property string initialText: ""

    canAccept: true

    Component.onCompleted: {
        if (pageName.length > 0) {
            initialText = bridge.get_page_source(pageName)
            textArea.text = initialText
        }
        textArea.forceActiveFocus()
    }

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height

        Column {
            id: column
            width: parent.width

            DialogHeader {
                title: qsTr("Edit ") + (pageName.indexOf('/') >= 0 ? pageName.split('/').pop().replace(/\.adoc$/i, '') : pageName.replace(/\.adoc$/i, ''))
                acceptText: qsTr("Save")
                cancelText: qsTr("Cancel")
            }

            EditorToolbar {
                id: toolbar
                width: parent.width
                targetTextArea: textArea
            }

            TextArea {
                id: textArea
                width: parent.width
                height: Math.max(implicitHeight, pageSourceEditor.height - Theme.itemSizeLarge * 2)
                text: initialText
                font.family: "monospace"
                font.pixelSize: Math.round(Theme.fontSizeSmall * (typeof app !== "undefined" && app && app.codeFontScale ? app.codeFontScale : 1.0))
                color: Theme.primaryColor
                placeholderText: qsTr("Enter AsciiDoc document source...")
                Keys.onPressed: function(event) {
                    BlockHtmlUtils.handleEditorKeyPress(event, textArea)
                }
            }
        }
    }

    onAccepted: {
        bridge.save_page_source(pageName, textArea.text)
    }
}
