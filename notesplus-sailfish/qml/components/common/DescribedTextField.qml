import QtQuick 2.6
import Sailfish.Silica 1.0

TextField {
    id: describedTextField

    property string fieldLabel: ""
    property string fieldDescription: ""

    placeholderText: fieldLabel
    label: fieldLabel
    description: fieldDescription

    EnterKey.iconSource: "image://theme/icon-m-enter-close"
    EnterKey.onClicked: focus = false
}
