/* RenderHelper.cpp — AsciiDoc element preview rendering. */

#include "RenderHelper.h"

#include <QJsonObject>
#include <QJsonDocument>
#include <QJsonArray>

#include "../ffi/ffi_raii.h"

RenderHelper::RenderHelper(const BridgeContext &ctx)
    : m_ctx(ctx)
{
}

QString RenderHelper::render_element_previews()
{
    static const char *kSnippets[] = {
        "==== Section Title",
        "This is *bold* text",
        "This is _italic_ text",
        "Use `printf()` here",
        "This has ~deleted~ text",
        "E = mc^2^",
        "This is #highlighted# text",
        "Referencefootnote:[An important note.] here",
        "===== Deep Title",
        "====== Deepest Title",
        "[source]\n----\nfn main() {\n    println!(\"hello\");\n}\n----",
        "[quote]\n____\nFamous words.\n____",
        "[verse]\n____\nThe road goes ever on.\n____",
        "....\n  Literal text here\n....",
        ".Example\n====\nExample content\n====",
        "--\nOpen block content\n--",
        "---",
        "<<<",
        "[NOTE]\n====\nNote text.\n====",
        "[TIP]\n====\nTip text.\n====",
        "[WARNING]\n====\nWarning text.\n====",
        "[CAUTION]\n====\nBe very careful.\n====",
        "[IMPORTANT]\n====\nThis is critical.\n====",
        "Term:: Description text",
        "|===\n| Name | Age\n| Alice | 30\n|===",
        "image::photo.jpg[A photo]",
        "See image:icon.png[Icon,16] here",
        "[sidebar]\n****\nSidebar text.\n****",
        "Click icon:star[] to rate",
        "Press kbd:[Ctrl+S] to save",
        "Click btn:[Submit] to continue",
        "Use menu:File[Quit] to exit",
        "See xref:other.adoc[Other Page]",
        "The equation stem:[E = mc^2]",
        "Use pass:[<b>raw HTML</b>] here",
        "A ((concept)) in text",
        "////\nThis is a block comment\n////",
        "// This is a line comment",
        ":author: Jane Doe",
        nullptr
    };

    const std::string opts  = m_ctx.buildOptionsJson().toStdString();
    const std::string theme = m_ctx.themeColorsJson().toStdString();
    const int drop = m_ctx.dropComments() ? 1 : 0;

    QJsonObject map;
    for (int i = 0; kSnippets[i] != nullptr; ++i) {
        const char *snippet = kSnippets[i];

        char *blocksJson = notes_core_parse_blocks_json(snippet, drop);
        QString blocksStr = ffiStringToQString(blocksJson);
        QJsonDocument doc = QJsonDocument::fromJson(blocksStr.toUtf8());
        QJsonArray arr = doc.array();

        QString html;
        if (!arr.isEmpty()) {
            QString blockJson = QString::fromUtf8(
                QJsonDocument(arr[0].toObject())
                    .toJson(QJsonDocument::Compact));
            char *rendered = notes_core_render_qt_block_json(
                qstrToFFI(blockJson), 0,
                theme.empty() ? nullptr : theme.c_str(),
                opts.empty()  ? nullptr : opts.c_str());
            html = ffiStringToQString(rendered);
        }

        map[QString::fromUtf8(snippet)] = html;
    }

    return QString::fromUtf8(
        QJsonDocument(map).toJson(QJsonDocument::Compact));
}
