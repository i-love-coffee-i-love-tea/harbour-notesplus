.pragma library

// Shared editor manipulation functions used by both MainPage and PageView.
//
// Every function takes a `ctx` object with these properties:
//   target       — the active TextArea (or null if no editor is focused)
//   isAdding     — bool: are we adding a new block?
//   newBlockText — string ref for the new-block text
//   editingIndex — int: index of block being edited (-1 if none)
//   editingText  — string ref for the current editing text
//   editingRaw   — string ref for the raw editing text
//   focusCb      — function(target): called after text mutation to refocus
//
// The caller is responsible for reading back the mutated ctx properties
// and applying them to the actual page-level properties.

function _syncBlockText(ctx, newText) {
    if (ctx.isAdding) {
        ctx.newBlockText = newText
    } else if (ctx.editingIndex >= 0) {
        ctx.editingText = newText
        ctx.editingRaw = newText
    }
}

function _syncBlockTextWithRaw(ctx, newText, rawText) {
    if (ctx.isAdding) {
        ctx.newBlockText = newText
    } else if (ctx.editingIndex >= 0) {
        ctx.editingText = newText
        ctx.editingRaw = rawText
    }
}

function _focusAndSync(ctx, target, newText) {
    _syncBlockText(ctx, newText)
    if (ctx.focusCb) ctx.focusCb(target)
}

function insertSnippet(ctx, snippet) {
    if (!snippet || snippet.length === 0) return
    var target = ctx.target
    if (target) {
        var txt = target.text || ""
        var pos = target.cursorPosition
        if (pos < 0 || pos > txt.length) pos = txt.length

        var needsPrefix = snippet.indexOf('\n') >= 0 && pos > 0 && txt.charAt(pos - 1) !== '\n'
        var prefix = needsPrefix ? "\n\n" : ""
        var toInsert = prefix + snippet

        var before = txt.substring(0, pos)
        var after = txt.substring(pos)
        target.text = before + toInsert + after
        target.cursorPosition = pos + toInsert.length

        _syncBlockText(ctx, target.text)
        if (ctx.focusCb) ctx.focusCb(target)
    } else {
        if (ctx.isAdding) {
            var cur = ctx.newBlockText || ""
            ctx.newBlockText = (cur.length > 0 ? cur + "\n" : "") + snippet
        } else if (ctx.editingIndex >= 0) {
            var cur2 = ctx.editingText || ""
            var updated = (cur2.length > 0 ? cur2 + "\n" : "") + snippet
            ctx.editingRaw = updated
            ctx.editingText = updated
        }
    }
}

function pasteText(ctx, clipText) {
    if (!clipText || clipText.length === 0) return false
    var target = ctx.target
    if (target) {
        var start = Math.min(target.selectionStart, target.selectionEnd)
        var end = Math.max(target.selectionStart, target.selectionEnd)
        var txt = target.text || ""
        var pos = target.cursorPosition
        if (start !== end && start >= 0 && end <= txt.length) {
            var before = txt.substring(0, start)
            var after = txt.substring(end)
            target.text = before + clipText + after
            target.cursorPosition = start + clipText.length
        } else {
            if (pos < 0 || pos > txt.length) pos = txt.length
            var before = txt.substring(0, pos)
            var after = txt.substring(pos)
            target.text = before + clipText + after
            target.cursorPosition = pos + clipText.length
        }
        _focusAndSync(ctx, target, target.text)
    } else {
        if (ctx.isAdding) {
            ctx.newBlockText = (ctx.newBlockText && ctx.newBlockText.length > 0 ? ctx.newBlockText + "\n" : "") + clipText
        } else if (ctx.editingIndex >= 0) {
            var cur = ctx.editingText || ""
            var updated = (cur.length > 0 ? cur + "\n" : "") + clipText
            ctx.editingRaw = updated
            ctx.editingText = updated
        }
    }
    return true
}

function pasteSpecial(ctx, formatted, BlockHtmlUtils) {
    if (!formatted || formatted.length === 0) return false
    var target = ctx.target
    if (target) {
        var start = Math.min(target.selectionStart, target.selectionEnd)
        var end = Math.max(target.selectionStart, target.selectionEnd)
        var txt = target.text || ""
        var pos = target.cursorPosition
        if (start !== end && start >= 0 && end <= txt.length) {
            var before = txt.substring(0, start)
            var after = txt.substring(end)
            target.text = before + formatted + after
            target.cursorPosition = start + formatted.length
        } else {
            if (pos < 0 || pos > txt.length) pos = txt.length
            var prefixNewline = ""
            if (pos > 0 && txt.charAt(pos - 1) !== '\n') {
                prefixNewline = "\n"
            }
            var toInsert = prefixNewline + formatted
            var before = txt.substring(0, pos)
            var after = txt.substring(pos)
            target.text = before + toInsert + after
            target.cursorPosition = pos + toInsert.length
        }
        _focusAndSync(ctx, target, target.text)
    } else {
        if (ctx.isAdding) {
            ctx.newBlockText = (ctx.newBlockText && ctx.newBlockText.length > 0 ? ctx.newBlockText + "\n" : "") + formatted
        } else if (ctx.editingIndex >= 0) {
            var cur = ctx.editingText || ""
            var updated = (cur.length > 0 ? cur + "\n" : "") + formatted
            ctx.editingRaw = updated
            ctx.editingText = updated
        }
    }
    return true
}

function applyPrefix(ctx, prefix, BlockHtmlUtils) {
    var target = ctx.target
    if (target) {
        var curPos = target.cursorPosition
        var txt = target.text || ""
        var result = BlockHtmlUtils.applyPrefixToSelectionOrCursor(
            txt, target.selectionStart, target.selectionEnd, curPos, prefix)
        target.text = result.text
        target.cursorPosition = result.cursorPos
        _syncBlockText(ctx, result.text)
        if (ctx.focusCb) ctx.focusCb(target)
    } else {
        var curText = ctx.isAdding ? (ctx.newBlockText || "") : (ctx.editingText || "")
        var result = BlockHtmlUtils.applyPrefixToSelectionOrCursor(
            curText, curText.length, curText.length, curText.length, prefix)
        if (ctx.isAdding) {
            ctx.newBlockText = result.text
        } else {
            ctx.editingRaw = result.text
            ctx.editingText = result.text
        }
    }
}

function changeListLevel(ctx, delta, BlockHtmlUtils) {
    var target = ctx.target
    if (!target) return
    var res = BlockHtmlUtils.changeListLevel(
        target.text, target.selectionStart, target.selectionEnd,
        target.cursorPosition, delta)
    target.text = res.text
    target.cursorPosition = res.cursorPosition
    if (res.selectionStart !== res.selectionEnd && typeof target.select === "function") {
        target.select(res.selectionStart, res.selectionEnd)
    }
    _syncBlockText(ctx, target.text)
    if (ctx.focusCb) ctx.focusCb(target)
}

function moveLines(ctx, direction, BlockHtmlUtils) {
    var target = ctx.target
    if (!target) return
    var res = BlockHtmlUtils.moveLines(
        target.text, target.selectionStart, target.selectionEnd,
        target.cursorPosition, direction)
    target.text = res.text
    target.cursorPosition = res.cursorPosition
    if (res.selectionStart !== res.selectionEnd && typeof target.select === "function") {
        target.select(res.selectionStart, res.selectionEnd)
    }
    _syncBlockText(ctx, target.text)
    if (ctx.focusCb) ctx.focusCb(target)
}

function insertLink(ctx, formattedLink) {
    if (!formattedLink) return
    var target = ctx.target
    if (target) {
        var start = Math.min(target.selectionStart, target.selectionEnd)
        var end = Math.max(target.selectionStart, target.selectionEnd)
        var txt = target.text || ""
        var pos = target.cursorPosition
        if (start !== end && start >= 0 && end <= txt.length) {
            var before = txt.substring(0, start)
            var after = txt.substring(end)
            target.text = before + formattedLink + after
            target.cursorPosition = start + formattedLink.length
        } else {
            if (pos < 0 || pos > txt.length) pos = txt.length
            var before = txt.substring(0, pos)
            var after = txt.substring(pos)
            target.text = before + formattedLink + after
            target.cursorPosition = pos + formattedLink.length
        }
        _syncBlockText(ctx, target.text)
        if (ctx.focusCb) ctx.focusCb(target)
    } else {
        if (ctx.isAdding) {
            ctx.newBlockText = (ctx.newBlockText && ctx.newBlockText.length > 0 ? ctx.newBlockText + " " : "") + formattedLink
        } else if (ctx.editingIndex >= 0) {
            var cur = ctx.editingText || ""
            var updated = (cur.length > 0 ? cur + " " : "") + formattedLink
            ctx.editingRaw = updated
            ctx.editingText = updated
        }
    }
}
