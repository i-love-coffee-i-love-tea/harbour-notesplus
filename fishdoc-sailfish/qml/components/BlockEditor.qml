import QtQuick 2.6
import Sailfish.Silica 1.0

Dialog {
    id: blockEditor
    allowedOrientations: Orientation.All

    property string rawText: ""
    property int blockIndex: -1

    canAccept: true

    Column {
        width: parent.width
        spacing: Theme.paddingMedium

        DialogHeader {
            acceptText: "Save"
            cancelText: "Cancel"
        }

        TextArea {
            id: textArea
            width: parent.width
            height: Math.max(implicitHeight, blockEditor.height - 200)
            text: rawText
            font.family: "monospace"
            font.pixelSize: Theme.fontSizeSmall
            color: Theme.primaryColor
            placeholderText: "Enter AsciiDoc content..."
            onTextChanged: {
                rawText = textArea.text
            }
        }
    }

    onAccepted: {
        rawText = textArea.text
        // Save while dialog is still alive — accessing properties after
        // the dialog pops from pageStack is use-after-free.
        bridge.save_block(blockIndex, rawText)
    }
}
