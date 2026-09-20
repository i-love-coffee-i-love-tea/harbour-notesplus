import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components/common"

Page {
    id: aboutPage
    allowedOrientations: Orientation.All

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height + Theme.paddingLarge

        Column {
            id: column
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: qsTr("About Notes Plus")
            }

            Image {
                anchors.horizontalCenter: parent.horizontalCenter
                width: Theme.iconSizeLarge
                height: Theme.iconSizeLarge
                source: "/usr/share/icons/hicolor/86x86/apps/harbour-notesplus.png"
                onStatusChanged: {
                    if (status === Image.Error) {
                        source = "image://theme/icon-m-notes"
                    }
                }
            }

            Label {
                anchors.horizontalCenter: parent.horizontalCenter
                text: "Notes Plus v0.3.0"
                font.pixelSize: Theme.fontSizeLarge
                font.bold: true
                color: Theme.highlightColor
            }

            Label {
                anchors.horizontalCenter: parent.horizontalCenter
                text: qsTr("AsciiDoc reader & notebook for Sailfish OS")
                font.pixelSize: Theme.fontSizeSmall
                color: Theme.secondaryColor
            }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - Theme.horizontalPageMargin * 2
                text: qsTr("A fast, offline-first notes application designed for Sailfish OS, combining plain-text document storage with structured formatting, full-text search, offline AI assistance, voice dictation, and a local web companion.")
                color: Theme.primaryColor
                font.pixelSize: Theme.fontSizeSmall
                wrapMode: Text.Wrap
            }

            SectionHeader {
                text: qsTr("Inspirations")
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: qsTr("Jolla Notes")
                    color: Theme.highlightColor
                    font.pixelSize: Theme.fontSizeSmall
                    font.bold: true
                }
                Label {
                    width: parent.width
                    text: qsTr("Inspired by the clean simplicity, tactile note cards, and color accents of the original Sailfish OS Notes app by Jolla Ltd.")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: qsTr("SpeechNote")
                    color: Theme.highlightColor
                    font.pixelSize: Theme.fontSizeSmall
                    font.bold: true
                }
                Label {
                    width: parent.width
                    text: qsTr("Inspired by SpeechNote by Michal Kowalczyk (mkiol) for offline, privacy-first speech-to-text transcription and model management on Sailfish OS.")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: "Asciidoctor"
                    color: Theme.highlightColor
                    font.pixelSize: Theme.fontSizeSmall
                    font.bold: true
                }
                Label {
                    width: parent.width
                    text: qsTr("Used the sample document from Asciidoctor PDF and its reference renderings to verify AsciiDoc rendering correctness. By Dan Allen and the Asciidoctor community.")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: "Whisperfish"
                    color: Theme.highlightColor
                    font.pixelSize: Theme.fontSizeSmall
                    font.bold: true
                }
                Label {
                    width: parent.width
                    text: qsTr("Used as a reference for building a Rust-based application with the Sailfish SDK, including sfdk cross-compilation, static library linking, and RPM packaging. By Ruben De Smet and contributors.")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }
            }

            SectionHeader {
                text: qsTr("Third-Party Libraries & Attributions")
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: "whisper.cpp & ggml"
                    color: Theme.highlightColor
                    font.pixelSize: Theme.fontSizeSmall
                    font.bold: true
                }
                Label {
                    width: parent.width
                    text: qsTr("High-performance OpenAI Whisper speech recognition inference in C/C++ by Georgi Gerganov and contributors.\nLicense: MIT")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: "Vue.js 3 & Pinia"
                    color: Theme.highlightColor
                    font.pixelSize: Theme.fontSizeSmall
                    font.bold: true
                }
                Label {
                    width: parent.width
                    text: qsTr("Progressive reactive web framework and state management powering the browser companion by Evan You and contributors.\nLicense: MIT")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: "svgbob"
                    color: Theme.highlightColor
                    font.pixelSize: Theme.fontSizeSmall
                    font.bold: true
                }
                Label {
                    width: parent.width
                    text: qsTr("Converts ASCII diagrams directly into SVG vector graphics by ivanceras.\nLicense: Apache-2.0 / MIT")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: "syntect"
                    color: Theme.highlightColor
                    font.pixelSize: Theme.fontSizeSmall
                    font.bold: true
                }
                Label {
                    width: parent.width
                    text: qsTr("Syntax highlighting for code blocks using Sublime Text syntax definitions by Tristan Hume.\nLicense: MIT")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: "SQLite & rusqlite"
                    color: Theme.highlightColor
                    font.pixelSize: Theme.fontSizeSmall
                    font.bold: true
                }
                Label {
                    width: parent.width
                    text: qsTr("Full-text search engine (FTS5) and ergonomic SQLite bindings for Rust by John Gallagher and rusqlite contributors.\nLicense: Public Domain / MIT")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: "rustls & ring"
                    color: Theme.highlightColor
                    font.pixelSize: Theme.fontSizeSmall
                    font.bold: true
                }
                Label {
                    width: parent.width
                    text: qsTr("Modern, memory-safe TLS networking and cryptographic primitives by Brian Smith and the Rustls team.\nLicense: Apache-2.0 / ISC / MIT")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: "tiny_http & ureq"
                    color: Theme.highlightColor
                    font.pixelSize: Theme.fontSizeSmall
                    font.bold: true
                }
                Label {
                    width: parent.width
                    text: qsTr("Embedded HTTP/HTTPS server by Corentin Henry and lightweight HTTP client for Rust by Martin Algesten.\nLicense: Apache-2.0 / MIT")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }
            }

            SectionHeader {
                text: qsTr("License & Source Code")
            }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - Theme.horizontalPageMargin * 2
                text: qsTr("Notes Plus is open source software released under the MIT License.\n\nAuthor: Steffen Kremsler")
                color: Theme.primaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
                wrapMode: Text.Wrap
            }

            Button {
                anchors.horizontalCenter: parent.horizontalCenter
                text: "github.com/i-love-coffee-i-love-tea/harbour-notesplus"
                onClicked: {
                    Qt.openUrlExternally("https://github.com/i-love-coffee-i-love-tea/harbour-notesplus")
                }
            }
        }
    }
}
