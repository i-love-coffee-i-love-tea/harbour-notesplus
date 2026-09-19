/* JournalStore.cpp — Journal-specific operations extracted from PageStore.
 *
 * All FFI calls use m_ctx.rawConn(), m_ctx.notesPath, m_ctx.dropComments(),
 * and m_ctx.reportError() exactly as in the original PageStore implementations.
 */

#include "JournalStore.h"

#include "../ffi/ffi_raii.h"

/* ================================================================== */
/*  Constructor                                                        */
/* ================================================================== */

JournalStore::JournalStore(const BridgeContext &ctx)
    : m_ctx(ctx)
{
}

/* ================================================================== */
/*  Callbacks                                                          */
/* ================================================================== */

void JournalStore::setRebuildCallback(std::function<void()> cb)
{
    m_rebuildCallback = std::move(cb);
}

void JournalStore::setReloadCallback(std::function<void()> cb)
{
    m_reloadCallback = std::move(cb);
}

/* ================================================================== */
/*  Journal operations                                                 */
/* ================================================================== */

void JournalStore::save_journal_block(int index, const QString &raw_text)
{
    if (index < 0) return;

    int rc = notes_core_page_save_block(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), "journal.adoc",
        index, 1, qstrToFFI(raw_text), m_ctx.dropComments() ? 1 : 0);

    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to save journal block"));
        return;
    }

    if (m_rebuildCallback)
        m_rebuildCallback();
}

void JournalStore::toggle_journal_checkbox(int block_index,
                                           const QString &item_path)
{
    if (block_index < 0) return;

    int rc = notes_core_page_toggle_checkbox(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), "journal.adoc",
        block_index, qstrToFFI(item_path));

    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to toggle journal checkbox"));
        return;
    }

    if (m_rebuildCallback)
        m_rebuildCallback();
}

void JournalStore::append_to_journal(const QString &text, bool is_task)
{
    const QString trimmed = text.trimmed();
    if (trimmed.isEmpty()) return;

    int rc = notes_core_journal_append(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath),
        qstrToFFI(trimmed), is_task ? 1 : 0);
    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to append to journal"));
        return;
    }

    if (m_rebuildCallback)
        m_rebuildCallback();
}
