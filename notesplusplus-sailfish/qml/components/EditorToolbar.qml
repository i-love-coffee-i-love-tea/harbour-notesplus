import QtQuick 2.6
import Sailfish.Silica 1.0

Rectangle {
    id: editorToolbar
    width: parent.width
    height: Theme.itemSizeSmall + Theme.paddingSmall
    color: Theme.rgba(Theme.highlightBackgroundColor, 0.1)

    property var targetTextArea: null

    SilicaListView {
        id: listView
        anchors.fill: parent
        anchors.leftMargin: Theme.horizontalPageMargin / 2
        anchors.rightMargin: Theme.horizontalPageMargin / 2
        anchors.topMargin: Theme.paddingSmall / 2
        anchors.bottomMargin: Theme.paddingSmall / 2
        orientation: ListView.Horizontal
        flickableDirection: Flickable.HorizontalFlick
        clip: true
        spacing: Theme.paddingSmall

        model: ListModel {
            ListElement {
                itemId: "h2"
                icon: ""
                label: "H2"
                actionType: "block"
                snippet: "== Heading\n"
                cursorOffset: 11
                prefix: ""
                suffix: ""
                defaultText: ""
            }
            ListElement {
                itemId: "h3"
                icon: ""
                label: "H3"
                actionType: "block"
                snippet: "=== Subheading\n"
                cursorOffset: 15
                prefix: ""
                suffix: ""
                defaultText: ""
            }
            ListElement {
                itemId: "bold"
                icon: ""
                label: "B"
                actionType: "wrap"
                snippet: ""
                cursorOffset: 0
                prefix: "*"
                suffix: "*"
                defaultText: "bold text"
            }
            ListElement {
                itemId: "italic"
                icon: ""
                label: "I"
                actionType: "wrap"
                snippet: ""
                cursorOffset: 0
                prefix: "_"
                suffix: "_"
                defaultText: "italic text"
            }
            ListElement {
                itemId: "mono"
                icon: ""
                label: "` `"
                actionType: "wrap"
                snippet: ""
                cursorOffset: 0
                prefix: "`"
                suffix: "`"
                defaultText: "code"
            }
            ListElement {
                itemId: "checkbox"
                icon: ""
                label: "☐"
                actionType: "line"
                snippet: "- [ ] "
                cursorOffset: 6
                prefix: ""
                suffix: ""
                defaultText: ""
            }
            ListElement {
                itemId: "bullet"
                icon: ""
                label: "•"
                actionType: "line"
                snippet: "* "
                cursorOffset: 2
                prefix: ""
                suffix: ""
                defaultText: ""
            }
            ListElement {
                itemId: "numbered"
                icon: ""
                label: "1."
                actionType: "line"
                snippet: ". "
                cursorOffset: 2
                prefix: ""
                suffix: ""
                defaultText: ""
            }
            ListElement {
                itemId: "link"
                icon: "image://theme/icon-m-link"
                label: ""
                actionType: "link"
                snippet: "https://example.com[Link title]"
                cursorOffset: 29
                prefix: ""
                suffix: ""
                defaultText: ""
            }
            ListElement {
                itemId: "image"
                icon: "image://theme/icon-m-file-image"
                label: ""
                actionType: "block"
                snippet: "image::image.png[Alt text]\n"
                cursorOffset: 27
                prefix: ""
                suffix: ""
                defaultText: ""
            }
            ListElement {
                itemId: "table"
                icon: "image://theme/icon-m-add-to-grid"
                label: ""
                actionType: "block"
                snippet: "[cols=\"1,1\", options=\"header\"]\n|===\n| Header 1 | Header 2\n\n| Cell 1 | Cell 2\n|===\n"
                cursorOffset: 85
                prefix: ""
                suffix: ""
                defaultText: ""
            }
            ListElement {
                itemId: "source"
                icon: ""
                label: "[src]"
                actionType: "block"
                snippet: "[source,rust]\n----\n// code here\n----\n"
                cursorOffset: 41
                prefix: ""
                suffix: ""
                defaultText: ""
            }
            ListElement {
                itemId: "note"
                icon: "image://theme/icon-m-about"
                label: ""
                actionType: "block"
                snippet: "[NOTE]\n====\nNote content\n====\n"
                cursorOffset: 30
                prefix: ""
                suffix: ""
                defaultText: ""
            }
            ListElement {
                itemId: "tip"
                icon: "image://theme/icon-s-high-importance"
                label: ""
                actionType: "block"
                snippet: "[TIP]\n====\nTip content\n====\n"
                cursorOffset: 28
                prefix: ""
                suffix: ""
                defaultText: ""
            }
            ListElement {
                itemId: "warning"
                icon: "image://theme/icon-s-warning"
                label: ""
                actionType: "block"
                snippet: "[WARNING]\n====\nWarning content\n====\n"
                cursorOffset: 36
                prefix: ""
                suffix: ""
                defaultText: ""
            }
            ListElement {
                itemId: "sidebar"
                icon: "image://theme/icon-m-file-note"
                label: ""
                actionType: "block"
                snippet: "[sidebar]\n****\nSidebar text\n****\n"
                cursorOffset: 34
                prefix: ""
                suffix: ""
                defaultText: ""
            }
            ListElement {
                itemId: "date"
                icon: "image://theme/icon-m-day-view"
                label: ""
                actionType: "date"
                snippet: ""
                cursorOffset: 0
                prefix: ""
                suffix: ""
                defaultText: ""
            }
        }

        delegate: BackgroundItem {
            id: buttonItem
            width: Math.max(Theme.itemSizeSmall, (itemLabel.visible ? itemLabel.implicitWidth + Theme.paddingMedium * 2 : Theme.itemSizeSmall))
            height: Theme.itemSizeSmall

            Rectangle {
                anchors.centerIn: parent
                width: parent.width - 4
                height: parent.height - 8
                radius: Theme.paddingSmall
                color: buttonItem.highlighted ? Theme.rgba(Theme.highlightBackgroundColor, 0.3) : Theme.rgba(Theme.primaryColor, 0.06)
                border.color: buttonItem.highlighted ? Theme.highlightColor : Theme.rgba(Theme.primaryColor, 0.2)
                border.width: 1
            }

            Image {
                id: itemIcon
                anchors.centerIn: parent
                visible: model.icon && model.icon.length > 0
                source: model.icon ? (model.icon + (model.icon.indexOf("?") === -1 ? ("?" + (buttonItem.highlighted ? Theme.highlightColor : Theme.primaryColor)) : "")) : ""
                height: Theme.iconSizeSmall
                width: Theme.iconSizeSmall
            }

            Label {
                id: itemLabel
                anchors.centerIn: parent
                visible: !itemIcon.visible
                text: model.label || ""
                color: buttonItem.highlighted ? Theme.highlightColor : Theme.primaryColor
                font.pixelSize: (model.label === "•" || model.label === "☐") ? Theme.fontSizeLarge : Theme.fontSizeMedium
                font.bold: model.itemId === "bold" || model.itemId === "h2" || model.itemId === "h3" || model.itemId === "numbered"
                font.italic: model.itemId === "italic"
                font.family: (model.itemId === "mono" || model.itemId === "source") ? "monospace" : Theme.fontFamily
            }

            onClicked: {
                handleItemClick(model)
            }
        }
    }

    function handleItemClick(item) {
        if (!targetTextArea) return

        if (item.actionType === "wrap") {
            wrapSelectionOrInsert(item.prefix, item.suffix, item.defaultText)
        } else if (item.actionType === "link") {
            insertLink()
        } else if (item.actionType === "line") {
            insertLinePrefix(item.snippet)
        } else if (item.actionType === "block") {
            insertBlockSnippet(item.snippet, item.cursorOffset)
        } else if (item.actionType === "date") {
            var dateStr = Qt.formatDate(new Date(), "yyyy-MM-dd")
            insertInlineSnippet(dateStr)
        }
    }

    function wrapSelectionOrInsert(prefix, suffix, defaultText) {
        var start = Math.min(targetTextArea.selectionStart, targetTextArea.selectionEnd)
        var end = Math.max(targetTextArea.selectionStart, targetTextArea.selectionEnd)
        var txt = targetTextArea.text || ""

        if (start !== end && start >= 0 && end <= txt.length) {
            var selected = txt.substring(start, end)
            var replacement = prefix + selected + suffix
            var before = txt.substring(0, start)
            var after = txt.substring(end)
            targetTextArea.text = before + replacement + after
            targetTextArea.cursorPosition = start + replacement.length
        } else {
            var pos = targetTextArea.cursorPosition
            if (pos < 0 || pos > txt.length) pos = txt.length
            var sample = defaultText || ""
            var replacement = prefix + sample + suffix
            var before = txt.substring(0, pos)
            var after = txt.substring(pos)
            targetTextArea.text = before + replacement + after
            if (sample.length > 0) {
                targetTextArea.cursorPosition = pos + prefix.length + sample.length
            } else {
                targetTextArea.cursorPosition = pos + prefix.length
            }
        }
        targetTextArea.forceActiveFocus()
    }

    function insertLink() {
        var start = Math.min(targetTextArea.selectionStart, targetTextArea.selectionEnd)
        var end = Math.max(targetTextArea.selectionStart, targetTextArea.selectionEnd)
        var txt = targetTextArea.text || ""
        var selected = (start !== end && start >= 0 && end <= txt.length) ? txt.substring(start, end) : ""

        var dialog = pageStack.push(Qt.resolvedUrl("../pages/PageLinkDialog.qml"), {
            selectedText: selected
        })
        dialog.accepted.connect(function() {
            var link = dialog.formattedLink
            if (!link) return
            if (start !== end && start >= 0 && end <= txt.length) {
                var before = txt.substring(0, start)
                var after = txt.substring(end)
                targetTextArea.text = before + link + after
                targetTextArea.cursorPosition = start + link.length
            } else {
                var pos = targetTextArea.cursorPosition
                if (pos < 0 || pos > txt.length) pos = txt.length
                var before = txt.substring(0, pos)
                var after = txt.substring(pos)
                targetTextArea.text = before + link + after
                targetTextArea.cursorPosition = pos + link.length
            }
            targetTextArea.forceActiveFocus()
        })
    }

    function insertLinePrefix(snippet) {
        var pos = targetTextArea.cursorPosition
        var txt = targetTextArea.text || ""
        if (pos < 0 || pos > txt.length) pos = txt.length

        var prefix = ""
        if (pos > 0 && txt.charAt(pos - 1) !== '\n') {
            prefix = "\n"
        }

        var inserted = prefix + snippet
        var before = txt.substring(0, pos)
        var after = txt.substring(pos)
        targetTextArea.text = before + inserted + after
        targetTextArea.cursorPosition = pos + inserted.length
        targetTextArea.forceActiveFocus()
    }

    function insertBlockSnippet(snippet, cursorOffset) {
        var pos = targetTextArea.cursorPosition
        var txt = targetTextArea.text || ""
        if (pos < 0 || pos > txt.length) pos = txt.length

        var prefix = ""
        if (pos > 0 && txt.charAt(pos - 1) !== '\n') {
            prefix = "\n"
            if (pos > 1 && txt.charAt(pos - 2) !== '\n') {
                prefix = "\n\n"
            }
        }

        var inserted = prefix + snippet
        var before = txt.substring(0, pos)
        var after = txt.substring(pos)
        targetTextArea.text = before + inserted + after
        var offset = (cursorOffset !== undefined && cursorOffset > 0) ? (prefix.length + cursorOffset) : inserted.length
        targetTextArea.cursorPosition = pos + offset
        targetTextArea.forceActiveFocus()
    }

    function insertInlineSnippet(snippet) {
        var pos = targetTextArea.cursorPosition
        var txt = targetTextArea.text || ""
        if (pos < 0 || pos > txt.length) pos = txt.length

        var before = txt.substring(0, pos)
        var after = txt.substring(pos)
        targetTextArea.text = before + snippet + after
        targetTextArea.cursorPosition = pos + snippet.length
        targetTextArea.forceActiveFocus()
    }
}
