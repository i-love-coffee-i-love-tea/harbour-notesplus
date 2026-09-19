import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"
import "../../common"
import "../../../js/BlockHtmlUtils.js" as BlockHtmlUtils
import "../../../js/ThemeColors.js" as TC

Column {
    id: codeDelegate
    property var blockData: ({})
    property string searchTerm: ""
    property int activeMatchIndexInBlock: -1

    property bool isSvgbob: Boolean(blockData && (
        (blockData.language && blockData.language.toLowerCase() === "svgbob") ||
        (blockData.svg_data && blockData.svg_data.length > 0)
    ))

    anchors.left: parent ? parent.left : undefined
    anchors.right: parent ? parent.right : undefined
    anchors.leftMargin: Theme.horizontalPageMargin
    anchors.rightMargin: Theme.horizontalPageMargin
    spacing: Theme.paddingSmall / 2
    clip: true

    InlineText {
        visible: Boolean(blockData && ((blockData.title_spans && blockData.title_spans.length > 0) || (blockData.title && blockData.title.length > 0)))
        spans: (blockData && blockData.title_spans && blockData.title_spans.length > 0) ? blockData.title_spans : ((blockData && blockData.title) ? [{ type: "text", value: blockData.title }] : undefined)
        font.family: app.resolvedFontFamily()
        font.pixelSize: app.scaledFontSize(Theme.fontSizeExtraSmall)
        font.bold: true
        color: Theme.highlightColor
        wrapMode: Text.WrapAtWordBoundaryOrAnywhere
        width: parent.width
    }

    Rectangle {
        id: svgCard
        visible: isSvgbob && svgImg.status !== Image.Error
        color: TC.kCodeCardBg
        border.color: Theme.rgba(Theme.primaryColor, 0.2)
        border.width: 1
        radius: Theme.paddingSmall / 2

        property real densityScale: (typeof Theme !== "undefined" && Theme.pixelRatio) ? Theme.pixelRatio : Math.max(1.0, Theme.fontSizeMedium / 20)
        property real zoom: 1.0
        property real minZoom: 0.8
        property real maxZoom: 4.0

        property real diagramWidth: (svgImg.implicitWidth > 0 ? svgImg.implicitWidth : width) * densityScale * zoom
        property real diagramHeight: (svgImg.implicitHeight > 0 ? svgImg.implicitHeight : Theme.itemSizeMedium) * densityScale * zoom

        height: visible ? (svgFlickable.height + Theme.paddingMedium * 2) : 0
        anchors.left: parent.left
        anchors.right: parent.right
        clip: true

        Flickable {
            id: svgFlickable
            anchors {
                left: parent.left
                right: parent.right
                top: parent.top
                margins: Theme.paddingMedium
            }
            height: Math.max(Theme.itemSizeSmall, Math.min(Screen.height * 0.65, svgCard.diagramHeight))
            contentWidth: Math.max(width, Math.round(svgCard.diagramWidth))
            contentHeight: Math.max(height, Math.round(svgCard.diagramHeight))
            flickableDirection: Flickable.HorizontalAndVerticalFlick
            boundsBehavior: Flickable.StopAtBounds
            pressDelay: 0
            clip: true

            PinchArea {
                id: pinchArea
                anchors.fill: parent

                property real initialZoom: 1.0
                property real initialContentX: 0
                property real initialContentY: 0

                onPinchStarted: {
                    initialZoom = svgCard.zoom
                    initialContentX = svgFlickable.contentX
                    initialContentY = svgFlickable.contentY
                }

                onPinchUpdated: function(pinch) {
                    var targetZoom = Math.max(svgCard.minZoom, Math.min(svgCard.maxZoom, initialZoom * pinch.scale))
                    if (Math.abs(targetZoom - svgCard.zoom) > 0.002) {
                        var oldZoom = svgCard.zoom
                        svgCard.zoom = targetZoom
                        svgFlickable.contentX = Math.max(0, Math.min(svgFlickable.contentWidth - svgFlickable.width, (initialContentX + pinch.center.x) * (targetZoom / initialZoom) - pinch.center.x))
                        svgFlickable.contentY = Math.max(0, Math.min(svgFlickable.contentHeight - svgFlickable.height, (initialContentY + pinch.center.y) * (targetZoom / initialZoom) - pinch.center.y))
                    }
                }

                onPinchFinished: {
                    svgFlickable.returnToBounds()
                }

                MouseArea {
                    id: cardTouchInterceptor
                    anchors.fill: parent
                    preventStealing: false
                    onClicked: {
                        // Absorb taps so touching/tapping the diagram doesn't trigger source editing
                    }
                }

                Item {
                    id: imgContainer
                    width: Math.round(svgCard.diagramWidth)
                    height: Math.round(svgCard.diagramHeight)
                    x: (width < svgFlickable.width) ? Math.round((svgFlickable.width - width) / 2) : 0
                    y: (height < svgFlickable.height) ? Math.round((svgFlickable.height - height) / 2) : 0

                    Image {
                        id: svgImg
                        anchors.fill: parent
                        source: (blockData && blockData.svg_data) ? blockData.svg_data : ""
                        sourceSize.width: (implicitWidth > 0) ? Math.round(implicitWidth * svgCard.densityScale * Math.max(1.5, svgCard.zoom)) : undefined
                        fillMode: Image.PreserveAspectFit
                        asynchronous: false
                        smooth: true
                    }
                }
            }
        }
    }

    Rectangle {
        visible: !isSvgbob || svgImg.status === Image.Error
        color: TC.kCodeBlockBg
        border.color: Theme.rgba(Theme.primaryColor, 0.2)
        border.width: 1
        radius: 0
        height: visible ? (codeCol.height + Theme.paddingMedium * 2) : 0
        anchors.left: parent.left
        anchors.right: parent.right
        clip: true

        Column {
            id: codeCol
            anchors {
                left: parent.left
                right: parent.right
                top: parent.top
                margins: Theme.paddingMedium
            }
            spacing: Theme.paddingSmall / 2

            Label {
                visible: (blockData.language || "").length > 0
                text: (blockData.language || "").toUpperCase()
                font.family: "monospace"
                font.pixelSize: Math.round(Theme.fontSizeExtraSmall * (typeof app !== "undefined" && app && app.codeFontScale ? app.codeFontScale : 1.0))
                color: Theme.rgba(TC.kCodeLanguageLabel, 0.6)
            }

            Label {
                id: codeLabel
                anchors.left: parent.left
                anchors.right: parent.right
                property bool hasHighlightedHtml: Boolean(blockData && blockData.highlighted_html && blockData.highlighted_html.length > 0)
                textFormat: (hasHighlightedHtml || (searchTerm && searchTerm.length > 0)) ? Text.RichText : Text.PlainText
                text: {
                    if (hasHighlightedHtml) {
                        var html = blockData.highlighted_html
                        if (searchTerm && searchTerm.length > 0) {
                            return BlockHtmlUtils.highlightSearchTerms(html, searchTerm, activeMatchIndexInBlock)
                        }
                        return html
                    }
                    var raw = (blockData.lines || []).join("\n")
                    if (searchTerm && searchTerm.length > 0) {
                        return BlockHtmlUtils.highlightPlainText(raw, searchTerm, activeMatchIndexInBlock)
                    }
                    return raw
                }
                font.family: "monospace"
                font.pixelSize: Math.round(Theme.fontSizeSmall * (typeof app !== "undefined" && app && app.codeFontScale ? app.codeFontScale : 1.0))
                color: TC.kCodeText
                wrapMode: Text.WrapAtWordBoundaryOrAnywhere
            }
        }
    }
}
