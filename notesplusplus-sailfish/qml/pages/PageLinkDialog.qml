import QtQuick 2.6
import Sailfish.Silica 1.0

Dialog {
    id: pageLinkDialog
    allowedOrientations: Orientation.All

    property string mode: "link" // "link" or "select"
    property string selectedText: ""
    property string targetPageFilename: ""
    property string targetPageTitle: ""
    property string linkDisplayText: selectedText || ""
    property string formattedLink: ""
    property var pagesList: []

    canAccept: (targetPageFilename.length > 0) || (searchField.text.trim().length > 0) || (linkDisplayField.text.trim().length > 0)

    function updateFormattedLink() {
        var fn = targetPageFilename.trim()
        var title = targetPageTitle.trim()
        var display = linkDisplayField.text.trim()

        if (fn.length === 0 && searchField.text.trim().length > 0) {
            var rawQuery = searchField.text.trim()
            if (rawQuery.indexOf("http://") === 0 || rawQuery.indexOf("https://") === 0 || rawQuery.indexOf("mailto:") === 0 || rawQuery.indexOf("ftp://") === 0) {
                fn = rawQuery
                if (title.length === 0) title = rawQuery
            } else if (rawQuery.indexOf(".adoc") === rawQuery.length - 5) {
                fn = rawQuery
            } else {
                fn = rawQuery + ".adoc"
            }
            if (title.length === 0) {
                title = rawQuery.replace(/\.adoc$/, "")
            }
        }

        if (fn.length === 0) {
            formattedLink = ""
            return
        }

        // Check if external web link
        if (fn.indexOf("http://") === 0 || fn.indexOf("https://") === 0 || fn.indexOf("mailto:") === 0 || fn.indexOf("ftp://") === 0) {
            var label = display.length > 0 ? display : (title.length > 0 ? title : fn)
            formattedLink = fn + "[" + label + "]"
            return
        }

        // AsciiDoc xref link: xref:filename.adoc[Display text]
        if (display.length > 0) {
            formattedLink = "xref:" + fn + "[" + display + "]"
        } else if (title.length > 0) {
            formattedLink = "xref:" + fn + "[" + title + "]"
        } else {
            formattedLink = "xref:" + fn + "[" + fn.replace(/\.adoc$/, "") + "]"
        }
    }

    onTargetPageFilenameChanged: updateFormattedLink()
    onTargetPageTitleChanged: updateFormattedLink()

    Component.onCompleted: {
        loadPages("")
        updateFormattedLink()
    }

    function loadPages(query) {
        if (typeof bridge !== "undefined" && bridge && bridge.get_linkable_pages_json) {
            try {
                var jsonStr = bridge.get_linkable_pages_json(query)
                pagesList = JSON.parse(jsonStr) || []
            } catch (e) {
                pagesList = []
            }
        } else {
            pagesList = []
        }
    }

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: contentColumn.height + Theme.paddingLarge

        Column {
            id: contentColumn
            width: parent.width

            DialogHeader {
                title: pageLinkDialog.mode === "select" ? qsTr("Select Page") : qsTr("Link Page")
                acceptText: pageLinkDialog.mode === "select" ? qsTr("Select") : qsTr("Insert Link")
                cancelText: qsTr("Cancel")
            }

            SearchField {
                id: searchField
                width: parent.width
                placeholderText: pageLinkDialog.mode === "select" ? qsTr("Search page to select...") : qsTr("Search page to link or enter URL...")
                text: ""
                EnterKey.enabled: pageLinkDialog.canAccept
                EnterKey.iconSource: "image://theme/icon-m-enter-accept"
                EnterKey.onClicked: {
                    pageLinkDialog.updateFormattedLink()
                    pageLinkDialog.accept()
                }
                onTextChanged: {
                    loadPages(text)
                    pageLinkDialog.updateFormattedLink()
                }
                Component.onCompleted: {
                    forceActiveFocus()
                }
            }

            TextField {
                id: linkDisplayField
                width: parent.width
                visible: pageLinkDialog.mode === "link"
                label: qsTr("Link display text (optional)")
                placeholderText: pageLinkDialog.selectedText.length > 0 ? pageLinkDialog.selectedText : (pageLinkDialog.targetPageTitle.length > 0 ? pageLinkDialog.targetPageTitle : qsTr("Text to show for link"))
                text: pageLinkDialog.linkDisplayText
                EnterKey.enabled: pageLinkDialog.canAccept
                EnterKey.iconSource: "image://theme/icon-m-enter-accept"
                EnterKey.onClicked: {
                    pageLinkDialog.updateFormattedLink()
                    pageLinkDialog.accept()
                }
                onTextChanged: {
                    pageLinkDialog.linkDisplayText = text
                    pageLinkDialog.updateFormattedLink()
                }
            }

            // AsciiDoc Syntax Preview
            Item {
                width: parent.width
                height: previewBox.height + Theme.paddingMedium
                visible: pageLinkDialog.mode === "link" && pageLinkDialog.formattedLink.length > 0

                Rectangle {
                    id: previewBox
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.horizontalPageMargin
                    anchors.rightMargin: Theme.horizontalPageMargin
                    anchors.top: parent.top
                    height: previewLabel.implicitHeight + Theme.paddingMedium * 2
                    color: Theme.rgba(Theme.highlightBackgroundColor, 0.15)
                    border.color: Theme.rgba(Theme.highlightColor, 0.3)
                    border.width: 1
                    radius: Theme.paddingSmall

                    Row {
                        anchors.fill: parent
                        anchors.margins: Theme.paddingSmall
                        spacing: Theme.paddingSmall

                        Label {
                            text: qsTr("Preview:")
                            font.pixelSize: Theme.fontSizeExtraSmall
                            color: Theme.highlightColor
                            font.bold: true
                            anchors.verticalCenter: parent.verticalCenter
                        }

                        Label {
                            id: previewLabel
                            text: pageLinkDialog.formattedLink
                            font.pixelSize: Theme.fontSizeExtraSmall
                            font.family: "monospace"
                            color: Theme.primaryColor
                            truncationMode: TruncationMode.Fade
                            width: parent.width - Theme.itemSizeSmall
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }
                }
            }

            SectionHeader {
                text: searchField.text.trim().length > 0 ? qsTr("Matching Pages") : qsTr("All Pages")
            }

            // Create new page or use URL option when query doesn't match exact page
            BackgroundItem {
                id: createNewItem
                width: parent.width
                height: Theme.itemSizeSmall
                visible: searchField.text.trim().length > 0 && !exactMatchExists()
                property bool isWebUrl: {
                    var q = searchField.text.trim().toLowerCase()
                    return q.indexOf("http://") === 0 || q.indexOf("https://") === 0 || q.indexOf("mailto:") === 0 || q.indexOf("ftp://") === 0
                }
                onClicked: {
                    var raw = searchField.text.trim()
                    if (isWebUrl) {
                        pageLinkDialog.targetPageTitle = raw
                        pageLinkDialog.targetPageFilename = raw
                    } else {
                        pageLinkDialog.targetPageTitle = raw.replace(/\.adoc$/, "")
                        pageLinkDialog.targetPageFilename = raw.indexOf(".adoc") === -1 ? (raw + ".adoc") : raw
                    }
                    if (!pageLinkDialog.linkDisplayText) {
                        pageLinkDialog.linkDisplayText = pageLinkDialog.targetPageTitle
                    }
                    pageLinkDialog.updateFormattedLink()
                }
                onDoubleClicked: {
                    onClicked()
                    pageLinkDialog.accept()
                }

                function exactMatchExists() {
                    var q = searchField.text.trim().toLowerCase()
                    for (var i = 0; i < pagesList.length; i++) {
                        if ((pagesList[i].title || "").toLowerCase() === q || (pagesList[i].filename || "").toLowerCase() === q || (pagesList[i].filename || "").toLowerCase() === q + ".adoc") {
                            return true
                        }
                    }
                    return false
                }

                Row {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.horizontalPageMargin
                    anchors.rightMargin: Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.paddingMedium

                    Image {
                        source: createNewItem.isWebUrl ? ("image://theme/icon-m-link?" + Theme.highlightColor) : ("image://theme/icon-m-add?" + Theme.highlightColor)
                        width: Theme.iconSizeSmall
                        height: Theme.iconSizeSmall
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    Column {
                        width: parent.width - Theme.iconSizeSmall - Theme.paddingMedium
                        anchors.verticalCenter: parent.verticalCenter

                        Label {
                            text: createNewItem.isWebUrl ? (qsTr("Use URL: ") + searchField.text.trim()) : (qsTr("Create & Link: ") + searchField.text.trim())
                            color: Theme.highlightColor
                            font.pixelSize: Theme.fontSizeSmall
                            font.bold: true
                            truncationMode: TruncationMode.Fade
                            width: parent.width
                        }

                        Label {
                            text: createNewItem.isWebUrl ? qsTr("Links to external web address") : (qsTr("Links to a new page '") + (searchField.text.trim().indexOf(".adoc") === -1 ? (searchField.text.trim() + ".adoc") : searchField.text.trim()) + "'")
                            color: Theme.secondaryColor
                            font.pixelSize: Theme.fontSizeExtraSmall
                            truncationMode: TruncationMode.Fade
                            width: parent.width
                        }
                    }
                }
            }

            Label {
                anchors.horizontalCenter: parent.horizontalCenter
                visible: pagesList.length === 0 && (!createNewItem.visible)
                text: searchField.text.trim().length > 0 ? qsTr("No matching pages found") : qsTr("No pages available")
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeSmall
                topPadding: Theme.paddingLarge
                bottomPadding: Theme.paddingLarge
            }

            // Repeater of matching pages
            Repeater {
                model: pagesList

                delegate: BackgroundItem {
                    id: pageItemDelegate
                    width: parent.width
                    height: Math.max(Theme.itemSizeMedium, itemCol.implicitHeight + Theme.paddingSmall * 2)
                    highlighted: pageLinkDialog.targetPageFilename === modelData.filename

                    onClicked: {
                        pageLinkDialog.targetPageFilename = modelData.filename
                        pageLinkDialog.targetPageTitle = modelData.title || modelData.filename.replace(/\.adoc$/, "")
                        if (!pageLinkDialog.linkDisplayText) {
                            pageLinkDialog.linkDisplayText = pageLinkDialog.targetPageTitle
                        }
                        pageLinkDialog.updateFormattedLink()
                    }

                    onDoubleClicked: {
                        pageLinkDialog.targetPageFilename = modelData.filename
                        pageLinkDialog.targetPageTitle = modelData.title || modelData.filename.replace(/\.adoc$/, "")
                        if (!pageLinkDialog.linkDisplayText) {
                            pageLinkDialog.linkDisplayText = pageLinkDialog.targetPageTitle
                        }
                        pageLinkDialog.updateFormattedLink()
                        pageLinkDialog.accept()
                    }

                    Row {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.leftMargin: Theme.horizontalPageMargin
                        anchors.rightMargin: Theme.horizontalPageMargin
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.paddingMedium

                        Image {
                            source: modelData.is_journal ? ("image://theme/icon-m-events?" + (highlighted ? Theme.highlightColor : Theme.primaryColor)) : ("image://theme/icon-m-document?" + (highlighted ? Theme.highlightColor : Theme.primaryColor))
                            width: Theme.iconSizeSmall
                            height: Theme.iconSizeSmall
                            anchors.verticalCenter: parent.verticalCenter
                            opacity: highlighted ? 1.0 : 0.7
                        }

                        Column {
                            id: itemCol
                            width: parent.width - Theme.iconSizeSmall - Theme.paddingMedium - (selectedIndicator.visible ? Theme.iconSizeSmall : 0)
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 2

                            Label {
                                text: modelData.title || modelData.filename
                                color: highlighted ? Theme.highlightColor : Theme.primaryColor
                                font.pixelSize: Theme.fontSizeMedium
                                font.bold: highlighted
                                truncationMode: TruncationMode.Fade
                                width: parent.width
                            }

                            Label {
                                text: modelData.filename + (modelData.updated_at ? (" • " + formatDate(modelData.updated_at)) : "")
                                color: Theme.secondaryColor
                                font.pixelSize: Theme.fontSizeExtraSmall
                                truncationMode: TruncationMode.Fade
                                width: parent.width
                            }

                            Label {
                                visible: modelData.snippet && modelData.snippet.length > 0
                                text: modelData.snippet || ""
                                color: Theme.secondaryHighlightColor
                                font.pixelSize: Theme.fontSizeExtraSmall
                                textFormat: Text.StyledText
                                truncationMode: TruncationMode.Fade
                                width: parent.width
                            }
                        }

                        Image {
                            id: selectedIndicator
                            visible: pageLinkDialog.targetPageFilename === modelData.filename
                            source: "image://theme/icon-s-installed?" + Theme.highlightColor
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }
                }
            }

            Item {
                width: parent.width
                height: Theme.paddingLarge
            }
        }
    }

    function formatDate(rfc3339Str) {
        if (!rfc3339Str) return ""
        var d = new Date(rfc3339Str)
        if (isNaN(d.getTime())) return rfc3339Str
        return Qt.formatDate(d, "yyyy-MM-dd")
    }

    onAccepted: {
        updateFormattedLink()
    }
}
