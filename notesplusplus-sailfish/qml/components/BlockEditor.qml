import QtQuick 2.6
import Sailfish.Silica 1.0

Dialog {
    id: blockEditor
    allowedOrientations: Orientation.All

    property string rawText: ""
    property int blockIndex: -1
    property int blockCount: 1
    property int initialCursorPosition: -1

    canAccept: true

    Component.onCompleted: {
        if (initialCursorPosition >= 0) {
            textArea.cursorPosition = initialCursorPosition
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
                acceptText: "Save"
                cancelText: "Cancel"
            }

            EditorToolbar {
                id: toolbar
                width: parent.width
                targetTextArea: textArea
            }

            TextArea {
                id: textArea
                width: parent.width
                height: Math.max(implicitHeight, blockEditor.height - Theme.itemSizeLarge * 2)
                text: rawText
                font.family: "monospace"
                font.pixelSize: Math.round(Theme.fontSizeSmall * (typeof app !== "undefined" && app && app.codeFontScale ? app.codeFontScale : 1.0))
                color: Theme.primaryColor
                placeholderText: "Enter AsciiDoc content..."
                onTextChanged: {
                    rawText = textArea.text
                }
            }
        }
    }

    onAccepted: {
        rawText = textArea.text
        if (blockCount > 1) {
            bridge.save_block_range(blockIndex, blockCount, rawText)
        } else {
            bridge.save_block(blockIndex, rawText)
        }
    }
}
