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

    Repeater {
        model: noteCardGrid.model

        delegate: NoteCard {
            width: (index % noteCardGrid.columns === noteCardGrid.columns - 1) ? (noteCardGrid.width - noteCardGrid.cellWidth * (noteCardGrid.columns - 1)) : noteCardGrid.cellWidth
            height: noteCardGrid.cellWidth
            modelData: modelData
            noteIndex: index
            searchTerm: noteCardGrid.searchTerm
            onClicked: {
                noteCardGrid.itemClicked(modelData, index)
            }
        }
    }
}
