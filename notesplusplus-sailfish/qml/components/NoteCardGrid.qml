import QtQuick 2.6
import Sailfish.Silica 1.0

Grid {
    id: noteCardGrid

    property var model: []
    property string searchTerm: ""
    property bool isPortraitOrientation: true

    signal itemClicked(var itemData, int itemIndex)

    width: parent ? parent.width : Screen.width
    columns: isPortraitOrientation ? 2 : (width > 1200 ? 4 : 3)
    spacing: 0

    property real cellWidth: Math.floor(width / columns)
    property int count: model ? (typeof model.length !== "undefined" ? model.length : (model.count !== undefined ? model.count : 0)) : 0
    property int rows: count > 0 ? Math.ceil(count / columns) : 0
    height: rows > 0 ? (rows * cellWidth + (rows - 1) * spacing) : 0

    Repeater {
        model: noteCardGrid.model

        delegate: NoteCard {
            width: (index % noteCardGrid.columns === noteCardGrid.columns - 1) ? (noteCardGrid.width - noteCardGrid.cellWidth * (noteCardGrid.columns - 1)) : noteCardGrid.cellWidth
            height: noteCardGrid.cellWidth
            cardData: modelData
            noteIndex: index
            onClicked: {
                noteCardGrid.itemClicked(cardData, index)
            }
        }
    }
}
