import QtQuick 2.0
import Sailfish.Silica 1.0

Dialog {
    id: iconPickerDialog
    allowedOrientations: Orientation.All

    property string selectedIcon: "icon-m-note"
    property string searchQuery: ""

    // Complete list of all standard Sailfish OS theme icons (icon-m-*)
    readonly property var allIcons: [
        "icon-m-10s-back",
        "icon-m-10s-forward",
        "icon-m-about",
        "icon-m-accept",
        "icon-m-accessory-speaker",
        "icon-m-acknowledge",
        "icon-m-activity-messaging",
        "icon-m-add",
        "icon-m-add-to-grid",
        "icon-m-airplane-mode",
        "icon-m-alarm",
        "icon-m-ambience",
        "icon-m-android",
        "icon-m-annotation",
        "icon-m-answer",
        "icon-m-arrow-left-green",
        "icon-m-arrow-right-green",
        "icon-m-arrow-up-red",
        "icon-m-asterisk",
        "icon-m-attach",
        "icon-m-autocaps",
        "icon-m-back",
        "icon-m-back-tab",
        "icon-m-backspace",
        "icon-m-backspace-keypad",
        "icon-m-backup",
        "icon-m-battery",
        "icon-m-battery-saver",
        "icon-m-bluetooth",
        "icon-m-bluetooth-device",
        "icon-m-browser-camera",
        "icon-m-browser-camera-template",
        "icon-m-browser-cookies",
        "icon-m-browser-cookies-template",
        "icon-m-browser-javascript",
        "icon-m-browser-location",
        "icon-m-browser-location-template",
        "icon-m-browser-microphone",
        "icon-m-browser-microphone-template",
        "icon-m-browser-notifications",
        "icon-m-browser-notifications-template",
        "icon-m-browser-permissions",
        "icon-m-browser-popup",
        "icon-m-browser-popup-template",
        "icon-m-browser-sound",
        "icon-m-browser-sound-template",
        "icon-m-bubble-universal",
        "icon-m-calendar-cancelled",
        "icon-m-call",
        "icon-m-call-recording-off",
        "icon-m-call-recording-on",
        "icon-m-camera",
        "icon-m-cancel",
        "icon-m-capslock",
        "icon-m-car",
        "icon-m-certificates",
        "icon-m-change-type",
        "icon-m-charging",
        "icon-m-chat",
        "icon-m-clear",
        "icon-m-clipboard",
        "icon-m-clock",
        "icon-m-close",
        "icon-m-cloud-download",
        "icon-m-cloud-upload",
        "icon-m-company",
        "icon-m-computer",
        "icon-m-contact",
        "icon-m-crash-reporter",
        "icon-m-crop",
        "icon-m-data-download",
        "icon-m-data-sim-1",
        "icon-m-data-sim-2",
        "icon-m-data-traffic",
        "icon-m-data-upload",
        "icon-m-date",
        "icon-m-day",
        "icon-m-day-view",
        "icon-m-delete",
        "icon-m-developer-mode",
        "icon-m-device",
        "icon-m-device-download",
        "icon-m-device-landscape",
        "icon-m-device-lock",
        "icon-m-device-portrait",
        "icon-m-device-upload",
        "icon-m-diagnostic",
        "icon-m-dialpad",
        "icon-m-dismiss",
        "icon-m-display",
        "icon-m-do-not-disturb",
        "icon-m-document",
        "icon-m-dot",
        "icon-m-down",
        "icon-m-downloads",
        "icon-m-edit",
        "icon-m-enter",
        "icon-m-enter-accept",
        "icon-m-enter-close",
        "icon-m-enter-next",
        "icon-m-events",
        "icon-m-favorite",
        "icon-m-file-apk",
        "icon-m-file-archive-folder",
        "icon-m-file-audio",
        "icon-m-file-compressed",
        "icon-m-file-document",
        "icon-m-file-download-as-pdf",
        "icon-m-file-folder",
        "icon-m-file-folder-nextcloud",
        "icon-m-file-formatted",
        "icon-m-file-image",
        "icon-m-file-note",
        "icon-m-file-other",
        "icon-m-file-pdf",
        "icon-m-file-presentation",
        "icon-m-file-rpm",
        "icon-m-file-spreadsheet",
        "icon-m-file-vcard",
        "icon-m-file-video",
        "icon-m-flashlight",
        "icon-m-flip",
        "icon-m-folder",
        "icon-m-font-size",
        "icon-m-forward",
        "icon-m-game-controller",
        "icon-m-gesture",
        "icon-m-global-proxy",
        "icon-m-gps",
        "icon-m-headphone",
        "icon-m-headset",
        "icon-m-health",
        "icon-m-history",
        "icon-m-home",
        "icon-m-image",
        "icon-m-imaging",
        "icon-m-incognito",
        "icon-m-incognito-new",
        "icon-m-incoming-call",
        "icon-m-input-clear",
        "icon-m-input-remove",
        "icon-m-jolla",
        "icon-m-keyboard",
        "icon-m-keys",
        "icon-m-lan",
        "icon-m-left",
        "icon-m-levels",
        "icon-m-light-contrast",
        "icon-m-like",
        "icon-m-link",
        "icon-m-location",
        "icon-m-mail",
        "icon-m-mail-open",
        "icon-m-mark-unread",
        "icon-m-media",
        "icon-m-media-albums",
        "icon-m-media-artists",
        "icon-m-media-forward",
        "icon-m-media-playlists",
        "icon-m-media-radio",
        "icon-m-media-rewind",
        "icon-m-media-songs",
        "icon-m-menu",
        "icon-m-message",
        "icon-m-message-forward",
        "icon-m-message-reply",
        "icon-m-message-reply-all",
        "icon-m-mic",
        "icon-m-mic-mute",
        "icon-m-missed-call",
        "icon-m-mobile-network",
        "icon-m-month-view",
        "icon-m-moon",
        "icon-m-mouse",
        "icon-m-music",
        "icon-m-new",
        "icon-m-next",
        "icon-m-nfc",
        "icon-m-night",
        "icon-m-note",
        "icon-m-notifications",
        "icon-m-orientation-lock",
        "icon-m-other",
        "icon-m-outgoing-call",
        "icon-m-outline-chat",
        "icon-m-outline-like",
        "icon-m-page-down",
        "icon-m-page-up",
        "icon-m-pause",
        "icon-m-people",
        "icon-m-person",
        "icon-m-phone",
        "icon-m-pin",
        "icon-m-play",
        "icon-m-presence",
        "icon-m-previous",
        "icon-m-qr",
        "icon-m-question",
        "icon-m-reboot",
        "icon-m-redirect",
        "icon-m-refresh",
        "icon-m-region",
        "icon-m-reload",
        "icon-m-remote-security",
        "icon-m-remove",
        "icon-m-repeat",
        "icon-m-repeat-single",
        "icon-m-reset",
        "icon-m-right",
        "icon-m-rotate",
        "icon-m-rotate-left",
        "icon-m-rotate-right",
        "icon-m-sailfish",
        "icon-m-scale",
        "icon-m-screenlock",
        "icon-m-sd-card",
        "icon-m-search",
        "icon-m-search-on-page",
        "icon-m-select-all",
        "icon-m-send",
        "icon-m-service-devices",
        "icon-m-service-dropbox",
        "icon-m-service-exchange",
        "icon-m-service-facebook",
        "icon-m-service-fruux",
        "icon-m-service-generic-calendar",
        "icon-m-service-generic-cdav",
        "icon-m-service-generic-email",
        "icon-m-service-google",
        "icon-m-service-jolla",
        "icon-m-service-memotoo",
        "icon-m-service-onedrive",
        "icon-m-service-owncloud",
        "icon-m-service-sip",
        "icon-m-service-twitter",
        "icon-m-service-upload",
        "icon-m-service-vk",
        "icon-m-service-xmpp",
        "icon-m-service-yahoo",
        "icon-m-setting",
        "icon-m-share",
        "icon-m-share-bluetooth",
        "icon-m-share-gallery",
        "icon-m-share-mms",
        "icon-m-share-nfc",
        "icon-m-share-qr",
        "icon-m-share-sign",
        "icon-m-shortcut",
        "icon-m-shuffle",
        "icon-m-silent",
        "icon-m-sim-1",
        "icon-m-sim-2",
        "icon-m-simple-next",
        "icon-m-simple-pause",
        "icon-m-simple-play",
        "icon-m-simple-previous",
        "icon-m-sms",
        "icon-m-sounds",
        "icon-m-speaker",
        "icon-m-speaker-mute",
        "icon-m-speaker-on",
        "icon-m-stop",
        "icon-m-storage",
        "icon-m-swipe",
        "icon-m-sync",
        "icon-m-tab-close",
        "icon-m-tab-new",
        "icon-m-tab-return",
        "icon-m-tablet",
        "icon-m-tabs",
        "icon-m-tether",
        "icon-m-text-input",
        "icon-m-textselection-end",
        "icon-m-textselection-start",
        "icon-m-time",
        "icon-m-time-date",
        "icon-m-timer",
        "icon-m-top-menu",
        "icon-m-toy",
        "icon-m-traffic",
        "icon-m-train",
        "icon-m-transfer",
        "icon-m-up",
        "icon-m-usb",
        "icon-m-user",
        "icon-m-user-admin",
        "icon-m-users",
        "icon-m-vibration",
        "icon-m-video",
        "icon-m-voicemail",
        "icon-m-vpn",
        "icon-m-warning",
        "icon-m-watch",
        "icon-m-weather-d000",
        "icon-m-weather-d100",
        "icon-m-weather-d200",
        "icon-m-weather-d210",
        "icon-m-weather-d211",
        "icon-m-weather-d212",
        "icon-m-weather-d220",
        "icon-m-weather-d221",
        "icon-m-weather-d222",
        "icon-m-weather-d240",
        "icon-m-weather-d300",
        "icon-m-weather-d310",
        "icon-m-weather-d311",
        "icon-m-weather-d312",
        "icon-m-weather-d320",
        "icon-m-weather-d321",
        "icon-m-weather-d322",
        "icon-m-weather-d340",
        "icon-m-weather-d400",
        "icon-m-weather-d410",
        "icon-m-weather-d411",
        "icon-m-weather-d412",
        "icon-m-weather-d420",
        "icon-m-weather-d421",
        "icon-m-weather-d422",
        "icon-m-weather-d430",
        "icon-m-weather-d431",
        "icon-m-weather-d432",
        "icon-m-weather-d440",
        "icon-m-weather-d500",
        "icon-m-weather-d600",
        "icon-m-weather-n000",
        "icon-m-weather-n100",
        "icon-m-weather-n200",
        "icon-m-weather-n210",
        "icon-m-weather-n211",
        "icon-m-weather-n212",
        "icon-m-weather-n220",
        "icon-m-weather-n221",
        "icon-m-weather-n222",
        "icon-m-weather-n240",
        "icon-m-weather-n300",
        "icon-m-weather-n310",
        "icon-m-weather-n311",
        "icon-m-weather-n312",
        "icon-m-weather-n320",
        "icon-m-weather-n321",
        "icon-m-weather-n322",
        "icon-m-weather-n340",
        "icon-m-weather-n400",
        "icon-m-weather-n410",
        "icon-m-weather-n411",
        "icon-m-weather-n412",
        "icon-m-weather-n420",
        "icon-m-weather-n421",
        "icon-m-weather-n422",
        "icon-m-weather-n430",
        "icon-m-weather-n431",
        "icon-m-weather-n432",
        "icon-m-weather-n440",
        "icon-m-weather-n500",
        "icon-m-weather-n600",
        "icon-m-website",
        "icon-m-week-view",
        "icon-m-whereami",
        "icon-m-wizard",
        "icon-m-wlan",
        "icon-m-wlan-0",
        "icon-m-wlan-1",
        "icon-m-wlan-2",
        "icon-m-wlan-3",
        "icon-m-wlan-4",
        "icon-m-wlan-hotspot",
        "icon-m-wlan-no-signal"
    ]

    readonly property var iconAliases: ({
        "icon-m-favorite": ["star", "like", "bookmark", "rate", "fav", "highlight"],
        "icon-m-delete": ["trash", "bin", "remove", "erase", "discard"],
        "icon-m-setting": ["settings", "gear", "config", "preferences", "options", "setup"],
        "icon-m-developer-mode": ["code", "dev", "script", "terminal", "wrench", "settings", "tools"],
        "icon-m-edit": ["pencil", "write", "modify", "pen", "compose"],
        "icon-m-mail": ["email", "inbox", "message", "letter"],
        "icon-m-mail-open": ["email", "read", "opened"],
        "icon-m-accept": ["check", "tick", "ok", "confirm", "done", "yes"],
        "icon-m-acknowledge": ["check", "tick", "understood"],
        "icon-m-cancel": ["cross", "cancel", "close", "reject", "no", "x"],
        "icon-m-close": ["cross", "cancel", "quit", "x", "dismiss"],
        "icon-m-clear": ["clean", "wipe", "minus", "remove", "delete"],
        "icon-m-add": ["plus", "new", "create", "insert"],
        "icon-m-question": ["help", "info", "faq", "ask", "questionmark"],
        "icon-m-about": ["info", "information", "help", "details"],
        "icon-m-device-lock": ["lock", "security", "protect", "password"],
        "icon-m-screenlock": ["lock", "screen", "security"],
        "icon-m-refresh": ["reload", "update", "rotate"],
        "icon-m-reload": ["refresh", "redo", "restart"],
        "icon-m-reset": ["undo", "restore", "revert"],
        "icon-m-rotate-left": ["undo", "counterclockwise"],
        "icon-m-rotate-right": ["redo", "clockwise"],
        "icon-m-clipboard": ["copy", "paste", "board"],
        "icon-m-text-input": ["keyboard", "type", "font", "text", "edit", "code"],
        "icon-m-document": ["page", "file", "text", "doc"],
        "icon-m-file-document": ["doc", "text", "file", "page"],
        "icon-m-file-note": ["note", "memo", "post-it", "file"],
        "icon-m-file-formatted": ["markdown", "asciidoc", "code", "formatted", "document"],
        "icon-m-file-pdf": ["pdf", "document"],
        "icon-m-file-image": ["image", "picture", "photo"],
        "icon-m-file-audio": ["audio", "sound", "music"],
        "icon-m-file-video": ["video", "movie", "film"],
        "icon-m-file-spreadsheet": ["excel", "calc", "sheet", "table", "csv"],
        "icon-m-file-compressed": ["zip", "tar", "archive", "compress"],
        "icon-m-file-archive-folder": ["zip", "archive", "folder"],
        "icon-m-file-vcard": ["contact", "vcard", "person"],
        "icon-m-file-apk": ["android", "package", "app"],
        "icon-m-file-rpm": ["package", "sailfish", "rpm", "install"],
        "icon-m-warning": ["alert", "caution", "danger", "error", "exclamation"],
        "icon-m-notifications": ["bell", "alarm", "notice", "alert"],
        "icon-m-alarm": ["clock", "timer", "wake", "bell"],
        "icon-m-time": ["clock", "hour", "watch"],
        "icon-m-timer": ["stopwatch", "countdown", "time"],
        "icon-m-chat": ["talk", "conversation", "bubble", "discuss"],
        "icon-m-bubble-universal": ["chat", "speech", "comment", "message"],
        "icon-m-message": ["sms", "text", "chat", "inbox"],
        "icon-m-location": ["pin", "map", "gps", "place"],
        "icon-m-whereami": ["location", "gps", "map", "position"],
        "icon-m-pin": ["tag", "marker", "stick", "pin"],
        "icon-m-link": ["url", "hyperlink", "chain", "web"],
        "icon-m-website": ["browser", "web", "internet", "http", "www", "url", "globe"],
        "icon-m-user": ["person", "profile", "account", "human"],
        "icon-m-users": ["people", "team", "group", "family"],
        "icon-m-contact": ["person", "addressbook", "card"],
        "icon-m-search": ["find", "lookup", "magnifier", "glass", "query"],
        "icon-m-search-on-page": ["find", "search", "page"],
        "icon-m-select-all": ["checklist", "todos", "tasks", "select", "check"],
        "icon-m-levels": ["sliders", "adjust", "tuning", "filter", "settings"],
        "icon-m-light-contrast": ["brightness", "theme", "contrast", "sun", "display"],
        "icon-m-moon": ["night", "dark", "sleep", "lunar"],
        "icon-m-sync": ["synchronize", "arrows", "cloud", "update"]
    })

    readonly property var filteredIcons: {
        var q = searchQuery.trim().toLowerCase()
        if (q.length === 0) {
            return allIcons
        }
        var res = []
        for (var i = 0; i < allIcons.length; i++) {
            var iconName = allIcons[i]
            // search in full icon name and stripped name (without icon-m-)
            var stripped = iconName.replace(/^icon-m-/, "").replace(/-/g, " ")
            var match = (iconName.toLowerCase().indexOf(q) !== -1 || stripped.indexOf(q) !== -1)
            if (!match && iconAliases && iconAliases[iconName]) {
                var aliases = iconAliases[iconName]
                for (var a = 0; a < aliases.length; a++) {
                    if (aliases[a].indexOf(q) !== -1) {
                        match = true
                        break
                    }
                }
            }
            if (match) {
                res.push(iconName)
            }
        }
        return res
    }

    canAccept: selectedIcon.length > 0

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: contentColumn.height + Theme.paddingLarge

        Column {
            id: contentColumn
            width: parent.width
            spacing: Theme.paddingMedium

            DialogHeader {
                title: qsTr("Choose Icon")
                acceptText: qsTr("Select")
                cancelText: qsTr("Cancel")
            }

            // Selected Icon Preview Card
            Rectangle {
                width: parent.width - Theme.horizontalPageMargin * 2
                height: selectedRow.height + Theme.paddingMedium * 2
                anchors.horizontalCenter: parent.horizontalCenter
                radius: Theme.paddingSmall
                color: Theme.rgba(Theme.highlightBackgroundColor, 0.2)
                border.color: Theme.highlightColor
                border.width: 1

                Row {
                    id: selectedRow
                    anchors {
                        left: parent.left
                        right: parent.right
                        verticalCenter: parent.verticalCenter
                        margins: Theme.paddingMedium
                    }
                    spacing: Theme.paddingMedium

                    Rectangle {
                        width: Theme.itemSizeExtraSmall
                        height: Theme.itemSizeExtraSmall
                        radius: Theme.paddingSmall / 2
                        color: Theme.rgba(Theme.highlightColor, 0.2)
                        anchors.verticalCenter: parent.verticalCenter

                        Icon {
                            anchors.centerIn: parent
                            source: iconPickerDialog.selectedIcon.length > 0 ? "image://theme/" + iconPickerDialog.selectedIcon : ""
                            width: Theme.iconSizeMedium
                            height: Theme.iconSizeMedium
                            color: Theme.highlightColor
                        }
                    }

                    Column {
                        width: parent.width - Theme.itemSizeExtraSmall - Theme.paddingMedium
                        anchors.verticalCenter: parent.verticalCenter

                        Label {
                            text: qsTr("Selected Icon")
                            font.pixelSize: Theme.fontSizeExtraSmall
                            color: Theme.secondaryColor
                        }

                        Label {
                            text: iconPickerDialog.selectedIcon
                            font.pixelSize: Theme.fontSizeSmall
                            font.bold: true
                            color: Theme.highlightColor
                            truncationMode: TruncationMode.Fade
                            width: parent.width
                        }
                    }
                }
            }

            // Search Bar
            SearchField {
                id: searchField
                width: parent.width
                placeholderText: qsTr("Search %1 icons...").arg(iconPickerDialog.allIcons.length)
                text: iconPickerDialog.searchQuery
                onTextChanged: iconPickerDialog.searchQuery = text
            }

            // Results count
            Label {
                x: Theme.horizontalPageMargin
                text: qsTr("%1 icons available").arg(iconPickerDialog.filteredIcons.length)
                font.pixelSize: Theme.fontSizeExtraSmall
                color: Theme.secondaryColor
                visible: iconPickerDialog.filteredIcons.length > 0
            }

            // Empty state if search found nothing
            Label {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                text: qsTr("No icons match \"%1\"").arg(iconPickerDialog.searchQuery)
                font.pixelSize: Theme.fontSizeSmall
                color: Theme.secondaryColor
                horizontalAlignment: Text.AlignHCenter
                visible: iconPickerDialog.filteredIcons.length === 0
            }

            // Icons Grid View
            Grid {
                id: iconGrid
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                columns: Math.max(4, Math.floor(width / (Theme.itemSizeSmall + Theme.paddingSmall)))
                spacing: Theme.paddingSmall

                Repeater {
                    model: iconPickerDialog.filteredIcons

                    delegate: BackgroundItem {
                        id: iconItem
                        width: Math.floor((iconGrid.width - (iconGrid.columns - 1) * iconGrid.spacing) / iconGrid.columns)
                        height: width + Theme.paddingSmall
                        highlighted: isSelected

                        readonly property bool isSelected: iconPickerDialog.selectedIcon === modelData

                        Rectangle {
                            anchors.fill: parent
                            radius: Theme.paddingSmall / 2
                            color: iconItem.isSelected ? Theme.rgba(Theme.highlightBackgroundColor, 0.4) : (iconItem.down ? Theme.rgba(Theme.primaryColor, 0.1) : "transparent")
                            border.color: iconItem.isSelected ? Theme.highlightColor : Theme.rgba(Theme.primaryColor, 0.15)
                            border.width: iconItem.isSelected ? 2 : 1

                            Column {
                                anchors.centerIn: parent
                                spacing: 2

                                Icon {
                                    source: "image://theme/" + modelData
                                    width: Theme.iconSizeMedium
                                    height: Theme.iconSizeMedium
                                    color: iconItem.isSelected ? Theme.highlightColor : Theme.primaryColor
                                    anchors.horizontalCenter: parent.horizontalCenter
                                }

                                Label {
                                    text: modelData.replace(/^icon-m-/, "")
                                    font.pixelSize: Theme.fontSizeTiny
                                    color: iconItem.isSelected ? Theme.highlightColor : Theme.secondaryColor
                                    width: iconItem.width - 4
                                    horizontalAlignment: Text.AlignHCenter
                                    truncationMode: TruncationMode.Fade
                                }
                            }
                        }

                        onClicked: {
                            iconPickerDialog.selectedIcon = modelData
                        }
                    }
                }
            }
        }
    }
}
