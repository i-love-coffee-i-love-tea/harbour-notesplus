.pragma library

function handleLink(link, xrefCallback, toggleCallback) {
    if (link.indexOf("xref:") === 0) {
        var target = link.substring(5);
        if (target.indexOf(".adoc") === target.length - 5) {
            target = target.substring(0, target.length - 5);
        }
        if (xrefCallback) xrefCallback(target);
    } else if (link.indexOf("#") === 0) {
        if (xrefCallback) xrefCallback(link);
    } else if (link.indexOf("toggle:") === 0) {
        var togglePath = link.substring(7);
        if (toggleCallback) toggleCallback(togglePath);
    } else if (link.indexOf("http") === 0) {
        Qt.openUrlExternally(link);
    }
}

function stripAndApplyPrefix(text, prefix) {
    var regex = /^(=+\s+|#+\s+|\*\s+\[[\sxX]\]\s+|\[[\sxX]\]\s+|\*\s+|\-\s+\[[\sxX]\]\s+|\-\s+|\+\s+|\.\s+|\d+[\.\)]\s+|•\s+)/;
    var lines = text.split(/\r?\n/);
    var stripped = lines.map(function(line) {
        var trimmed = line.trim();
        if (trimmed.length === 0) return "";
        return regex.test(trimmed) ? trimmed.replace(regex, '') : trimmed;
    });
    return stripped.map(function(line) {
        if (line.length === 0) return "";
        return prefix + line;
    }).join('\n');
}

function stripAndApplyPrefixToLine(line, prefix) {
    var regex = /^(=+\s+|#+\s+|\*\s+\[[\sxX]\]\s+|\[[\sxX]\]\s+|\*\s+|\-\s+\[[\sxX]\]\s+|\-\s+|\+\s+|\.\s+|\d+[\.\)]\s+|•\s+)/;
    var trimmed = line.trim();
    if (trimmed.length === 0) return "";
    var stripped = regex.test(trimmed) ? trimmed.replace(regex, '') : trimmed;
    return prefix + stripped;
}

function applyPrefixToSelectionOrCursor(text, selectionStart, selectionEnd, cursorPos, prefix) {
    if (!text || text.length === 0) return { text: text, cursorPos: cursorPos };

    var hasSelection = (selectionStart !== selectionEnd);
    var selStart = Math.min(selectionStart, selectionEnd);
    var selEnd = Math.max(selectionStart, selectionEnd);

    var lines = text.split('\n');

    var lineStarts = [];
    var pos = 0;
    for (var i = 0; i < lines.length; i++) {
        lineStarts.push(pos);
        pos += lines[i].length + 1;
    }

    var modify = [];
    for (var i = 0; i < lines.length; i++) { modify[i] = false; }

    if (hasSelection) {
        for (var i = 0; i < lines.length; i++) {
            var lineStart = lineStarts[i];
            var lineEnd = lineStart + lines[i].length;
            if (lineStart < selEnd && lineEnd > selStart) {
                modify[i] = true;
            }
        }
    } else {
        for (var i = 0; i < lines.length; i++) {
            if (lineStarts[i] + lines[i].length >= cursorPos) {
                modify[i] = true;
                break;
            }
        }
    }

    var diffs = [];
    for (var i = 0; i < lines.length; i++) { diffs[i] = 0; }
    for (var i = 0; i < lines.length; i++) {
        if (modify[i]) {
            var oldLine = lines[i];
            var newLine = stripAndApplyPrefixToLine(oldLine, prefix);
            lines[i] = newLine;
            diffs[i] = newLine.length - oldLine.length;
        }
    }

    var cursorLine = 0;
    for (var i = 0; i < lines.length; i++) {
        if (lineStarts[i] + lines[i].length >= cursorPos) {
            cursorLine = i;
            break;
        }
    }
    var cumulativeDiff = 0;
    for (var i = 0; i <= cursorLine; i++) {
        cumulativeDiff += diffs[i];
    }

    var newText = lines.join('\n');
    var newCursorPos = Math.max(0, Math.min(newText.length, cursorPos + cumulativeDiff));
    return { text: newText, cursorPos: newCursorPos };
}

function formatPasteWithPrefix(text, prefix) {
    if (!text || text.length === 0) return "";
    return stripAndApplyPrefix(text, prefix);
}

function escapeHtml(text) {
    if (!text || text.length === 0) return "";
    return text.replace(/&/g, '&amp;')
               .replace(/</g, '&lt;')
               .replace(/>/g, '&gt;')
               .replace(/"/g, '&quot;')
               .replace(/'/g, '&#39;');
}

function escapeRegex(str) {
    return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function countMatchesInPlainText(text, term) {
    if (!term || !text || text.length === 0) return 0;
    var query = term.trim();
    if (query.length === 0) return 0;
    var re = new RegExp(escapeRegex(query), "gi");
    var matches = text.match(re);
    return matches ? matches.length : 0;
}

function countMatchesInHtml(html, term) {
    if (!term || !html || html.length === 0) return 0;
    var query = term.trim();
    if (query.length === 0) return 0;
    var parts = html.split(/(<[^>]+>)/);
    var count = 0;
    var re = new RegExp(escapeRegex(query), "gi");
    for (var i = 0; i < parts.length; i++) {
        if (parts[i].charAt(0) !== '<') {
            var m = parts[i].match(re);
            if (m) count += m.length;
        }
    }
    return count;
}

function countMatchesInBlock(block, term) {
    if (!block || !term) return 0;
    var query = term.trim();
    if (query.length === 0) return 0;

    if (block.html && block.html.length > 0) {
        return countMatchesInHtml(block.html, query);
    }
    
    var count = 0;
    if (block.title && block.title.length > 0) {
        count += countMatchesInPlainText(block.title, query);
    }
    if (block.lines && block.lines.length > 0) {
        count += countMatchesInPlainText(block.lines.join("\n"), query);
    } else if (block.raw_text && block.raw_text.length > 0) {
        count += countMatchesInPlainText(block.raw_text, query);
    } else if (block.raw && block.raw.length > 0) {
        count += countMatchesInPlainText(block.raw, query);
    } else if (block.term && block.term.length > 0) {
        count += countMatchesInPlainText(block.term, query);
    }
    return count;
}

function findAllMatches(parsedBlocks, term) {
    if (!parsedBlocks || parsedBlocks.length === 0 || !term) return [];
    var query = term.trim();
    if (query.length === 0) return [];

    var matches = [];
    var globalIndex = 0;
    for (var i = 0; i < parsedBlocks.length; i++) {
        var count = countMatchesInBlock(parsedBlocks[i], query);
        for (var j = 0; j < count; j++) {
            matches.push({
                blockIndex: i,
                matchInBlock: j,
                globalIndex: globalIndex
            });
            globalIndex++;
        }
    }
    return matches;
}

function highlightSearchTerms(html, term, activeMatchIndexInBlock) {
    if (!term || !html || html.length === 0) return html;
    var query = term.trim();
    if (query.length === 0) return html;

    var activeIdx = (typeof activeMatchIndexInBlock === "number") ? activeMatchIndexInBlock : -1;
    var re = new RegExp("(" + escapeRegex(query) + ")", "gi");
    var parts = html.split(/(<[^>]+>)/);
    var rebuilt = "";
    var currentMatchInBlock = 0;

    for (var i = 0; i < parts.length; i++) {
        if (parts[i].charAt(0) === '<') {
            rebuilt += parts[i];
        } else {
            rebuilt += parts[i].replace(re, function(match, p1) {
                var isActive = (currentMatchInBlock === activeIdx);
                currentMatchInBlock++;
                var style = isActive
                    ? "background:#ff9800;color:#000;font-weight:bold;padding:0 2px;border-radius:2px;border:1px solid #e65100;"
                    : "background:#ffeb3b;color:#000;padding:0 1px;border-radius:2px;";
                return '<mark style="' + style + '">' + p1 + '</mark>';
            });
        }
    }
    return rebuilt;
}

function highlightPlainText(text, term, activeMatchIndexInBlock) {
    if (!text || text.length === 0) return "";
    var escaped = escapeHtml(text);
    if (!term || term.trim().length === 0) return escaped;
    return highlightSearchTerms(escaped, term, activeMatchIndexInBlock);
}

function resolveImagePath(target, notesDir, allowExternal) {
    if (!target) return ""
    if (target.indexOf("http://") === 0 || target.indexOf("https://") === 0) {
        return (allowExternal !== false) ? target : ""
    }
    if (target.indexOf("file://") === 0 || target.indexOf("/") === 0) {
        return target
    }
    if (notesDir && notesDir.length > 0) {
        return "file://" + notesDir + "/" + target
    }
    return "file:///usr/share/harbour-notesplusplus/examples/" + target
}

function indentListLine(line) {
    if (!line || line.trim().length === 0) return line;

    // Repeated asterisks (1 to 4 -> add one asterisk; 5 is clamped)
    var astMatch = line.match(/^(\s*)(\*{1,5})(\s*)(.*)$/);
    if (astMatch) {
        if (astMatch[2].length < 5) {
            return astMatch[1] + astMatch[2] + "*" + astMatch[3] + astMatch[4];
        }
        return line;
    }

    // Repeated dots (1 to 4 -> add one dot; 5 is clamped)
    var dotMatch = line.match(/^(\s*)(\.{1,5})(\s*)(.*)$/);
    if (dotMatch) {
        if (dotMatch[2].length < 5) {
            return dotMatch[1] + dotMatch[2] + "." + dotMatch[3] + dotMatch[4];
        }
        return line;
    }

    // Hyphen, numbered, or plain text: indent by 2 leading spaces
    return "  " + line;
}

function outdentListLine(line) {
    if (!line || line.trim().length === 0) return line;

    // Repeated asterisks (2 to 5 -> remove one asterisk; 1 is clamped at root)
    var astMatch = line.match(/^(\s*)(\*{1,5})(\s*)(.*)$/);
    if (astMatch) {
        if (astMatch[2].length > 1) {
            return astMatch[1] + astMatch[2].substring(1) + astMatch[3] + astMatch[4];
        }
        return line; // clamped at root
    }

    // Repeated dots (2 to 5 -> remove one dot; 1 is clamped at root)
    var dotMatch = line.match(/^(\s*)(\.{1,5})(\s*)(.*)$/);
    if (dotMatch) {
        if (dotMatch[2].length > 1) {
            return dotMatch[1] + dotMatch[2].substring(1) + dotMatch[3] + dotMatch[4];
        }
        return line; // clamped at root
    }

    // Hyphen, numbered, or plain text with leading whitespace:
    // Strip up to 2 leading spaces
    if (line.indexOf("  ") === 0) {
        return line.substring(2);
    } else if (line.indexOf(" ") === 0) {
        return line.substring(1);
    } else if (line.indexOf("\t") === 0) {
        return line.substring(1);
    }

    return line; // root clamped
}

function changeListLevel(text, selectionStart, selectionEnd, cursorPos, delta) {
    if (text === undefined || text === null) {
        return { text: "", cursorPosition: 0, selectionStart: 0, selectionEnd: 0 };
    }
    if (delta === 0) {
        return {
            text: text,
            cursorPosition: cursorPos !== undefined ? cursorPos : 0,
            selectionStart: selectionStart !== undefined ? selectionStart : 0,
            selectionEnd: selectionEnd !== undefined ? selectionEnd : 0
        };
    }

    var hasSelection = (selectionStart !== undefined && selectionEnd !== undefined && selectionStart !== selectionEnd);
    var isReversed = hasSelection && (selectionStart > selectionEnd);
    var s = hasSelection ? Math.min(selectionStart, selectionEnd) : (cursorPos !== undefined ? cursorPos : 0);
    var e = hasSelection ? Math.max(selectionStart, selectionEnd) : (cursorPos !== undefined ? cursorPos : 0);
    var cursor = (cursorPos !== undefined) ? cursorPos : (hasSelection ? (isReversed ? s : e) : 0);

    var lines = text.split('\n');
    var lineStarts = [];
    var p = 0;
    for (var i = 0; i < lines.length; i++) {
        lineStarts.push(p);
        p += lines[i].length + 1;
    }

    var L_start = 0;
    for (var i = 0; i < lines.length; i++) {
        if (lineStarts[i] + lines[i].length >= s || i === lines.length - 1) {
            L_start = i;
            break;
        }
    }

    var L_end = L_start;
    if (hasSelection) {
        for (var i = 0; i < lines.length; i++) {
            if (lineStarts[i] + lines[i].length >= e || i === lines.length - 1) {
                if (i > L_start && e === lineStarts[i]) {
                    L_end = i - 1;
                } else {
                    L_end = i;
                }
                break;
            }
        }
    }

    var newLines = [];
    for (var i = 0; i < lines.length; i++) {
        if (i >= L_start && i <= L_end) {
            var transformed = (delta > 0) ? indentListLine(lines[i]) : outdentListLine(lines[i]);
            newLines.push(transformed);
        } else {
            newLines.push(lines[i]);
        }
    }

    var newLineStarts = [];
    var np = 0;
    for (var i = 0; i < newLines.length; i++) {
        newLineStarts.push(np);
        np += newLines[i].length + 1;
    }

    function adjustPosition(origPos) {
        var k = 0;
        for (var i = 0; i < lines.length; i++) {
            if (lineStarts[i] + lines[i].length >= origPos || i === lines.length - 1) {
                k = i;
                break;
            }
        }
        var col = origPos - lineStarts[k];
        var newCol = col;
        if (k >= L_start && k <= L_end) {
            var diff = newLines[k].length - lines[k].length;
            if (col === 0) {
                newCol = 0;
            } else {
                newCol = Math.max(0, Math.min(newLines[k].length, col + diff));
            }
        }
        return newLineStarts[k] + newCol;
    }

    var newS = adjustPosition(s);
    var newE = adjustPosition(e);
    var newCursor = adjustPosition(cursor);

    var finalSelStart = hasSelection ? (isReversed ? newE : newS) : newCursor;
    var finalSelEnd = hasSelection ? (isReversed ? newS : newE) : newCursor;

    return {
        text: newLines.join('\n'),
        cursorPosition: newCursor,
        selectionStart: finalSelStart,
        selectionEnd: finalSelEnd
    };
}

function moveLines(text, selectionStart, selectionEnd, cursorPos, direction) {
    if (text === undefined || text === null) {
        return { text: "", cursorPosition: 0, selectionStart: 0, selectionEnd: 0 };
    }
    var lines = text.split('\n');
    if (lines.length <= 1 || (direction !== -1 && direction !== 1)) {
        return {
            text: text,
            cursorPosition: cursorPos !== undefined ? cursorPos : 0,
            selectionStart: selectionStart !== undefined ? selectionStart : 0,
            selectionEnd: selectionEnd !== undefined ? selectionEnd : 0
        };
    }

    var hasSelection = (selectionStart !== undefined && selectionEnd !== undefined && selectionStart !== selectionEnd);
    var isReversed = hasSelection && (selectionStart > selectionEnd);
    var s = hasSelection ? Math.min(selectionStart, selectionEnd) : (cursorPos !== undefined ? cursorPos : 0);
    var e = hasSelection ? Math.max(selectionStart, selectionEnd) : (cursorPos !== undefined ? cursorPos : 0);
    var cursor = (cursorPos !== undefined) ? cursorPos : (hasSelection ? (isReversed ? s : e) : 0);

    var lineStarts = [];
    var p = 0;
    for (var i = 0; i < lines.length; i++) {
        lineStarts.push(p);
        p += lines[i].length + 1;
    }

    var L_start = 0;
    for (var i = 0; i < lines.length; i++) {
        if (lineStarts[i] + lines[i].length >= s || i === lines.length - 1) {
            L_start = i;
            break;
        }
    }

    var L_end = L_start;
    if (hasSelection) {
        for (var i = 0; i < lines.length; i++) {
            if (lineStarts[i] + lines[i].length >= e || i === lines.length - 1) {
                if (i > L_start && e === lineStarts[i]) {
                    L_end = i - 1;
                } else {
                    L_end = i;
                }
                break;
            }
        }
    }

    if (direction === -1) {
        if (L_start === 0) {
            return {
                text: text,
                cursorPosition: cursor,
                selectionStart: selectionStart !== undefined ? selectionStart : cursor,
                selectionEnd: selectionEnd !== undefined ? selectionEnd : cursor
            };
        }

        var lineAbove = lines[L_start - 1];
        var shift = lineAbove.length + 1;

        var block = lines.slice(L_start, L_end + 1);
        var newLines = lines.slice(0, L_start - 1)
            .concat(block)
            .concat([lineAbove])
            .concat(lines.slice(L_end + 1));

        var newS = s - shift;
        var newE = e - shift;
        var newCursor = cursor - shift;

        return {
            text: newLines.join('\n'),
            cursorPosition: newCursor,
            selectionStart: hasSelection ? (isReversed ? newE : newS) : newCursor,
            selectionEnd: hasSelection ? (isReversed ? newS : newE) : newCursor
        };
    } else if (direction === 1) {
        if (L_end >= lines.length - 1) {
            return {
                text: text,
                cursorPosition: cursor,
                selectionStart: selectionStart !== undefined ? selectionStart : cursor,
                selectionEnd: selectionEnd !== undefined ? selectionEnd : cursor
            };
        }

        var lineBelow = lines[L_end + 1];
        var shift = lineBelow.length + 1;

        var block = lines.slice(L_start, L_end + 1);
        var newLines = lines.slice(0, L_start)
            .concat([lineBelow])
            .concat(block)
            .concat(lines.slice(L_end + 2));

        var newS = s + shift;
        var newE = e + shift;
        var newCursor = cursor + shift;

        return {
            text: newLines.join('\n'),
            cursorPosition: newCursor,
            selectionStart: hasSelection ? (isReversed ? newE : newS) : newCursor,
            selectionEnd: hasSelection ? (isReversed ? newS : newE) : newCursor
        };
    }
}

function handleSmartEnter(text, cursorPos) {
    if (text === undefined || text === null) {
        return { handled: false };
    }
    var pos = (cursorPos !== undefined) ? cursorPos : text.length;

    var lines = text.split('\n');
    var lineStarts = [];
    var p = 0;
    for (var i = 0; i < lines.length; i++) {
        lineStarts.push(p);
        p += lines[i].length + 1;
    }

    var currLineIdx = 0;
    for (var i = 0; i < lines.length; i++) {
        if (lineStarts[i] + lines[i].length >= pos || i === lines.length - 1) {
            currLineIdx = i;
            break;
        }
    }

    var currLine = lines[currLineIdx];
    var col = Math.max(0, Math.min(currLine.length, pos - lineStarts[currLineIdx]));
    var beforeCursor = currLine.substring(0, col);
    var afterCursor = currLine.substring(col);

    var matchType = null;
    var matchInfo = null;

    var mTaskAst = currLine.match(/^(\s*)(\*{1,5})(\s+\[[\sxX]\]\s*)(.*)$/);
    if (mTaskAst) {
        matchType = "task_ast";
        matchInfo = mTaskAst;
    } else {
        var mAst = currLine.match(/^(\s*)(\*{1,5})(\s+)(.*)$/);
        if (mAst) {
            matchType = "ast";
            matchInfo = mAst;
        } else {
            var mDot = currLine.match(/^(\s*)(\.{1,5})(\s+)(.*)$/);
            if (mDot) {
                matchType = "dot";
                matchInfo = mDot;
            } else {
                var mTaskHyphen = currLine.match(/^(\s*)(-\s+\[[\sxX]\]\s*)(.*)$/);
                if (mTaskHyphen) {
                    matchType = "task_hyphen";
                    matchInfo = mTaskHyphen;
                } else {
                    var mHyphen = currLine.match(/^(\s*)(-\s+)(.*)$/);
                    if (mHyphen) {
                        matchType = "hyphen";
                        matchInfo = mHyphen;
                    } else {
                        var mNum = currLine.match(/^(\s*)(\d+)([\.\)]\s+)(.*)$/);
                        if (mNum) {
                            matchType = "num";
                            matchInfo = mNum;
                        }
                    }
                }
            }
        }
    }

    if (!matchType) {
        return { handled: false };
    }

    var content = "";
    if (matchType === "task_ast" || matchType === "ast" || matchType === "dot") {
        content = matchInfo[4];
    } else if (matchType === "task_hyphen" || matchType === "hyphen") {
        content = matchInfo[3];
    } else if (matchType === "num") {
        content = matchInfo[4];
    }

    var isEmptyListItem = (content.trim().length === 0);

    if (isEmptyListItem) {
        var outdented = outdentListLine(currLine);
        if (outdented !== currLine) {
            lines[currLineIdx] = outdented;
            var newText = lines.join('\n');
            var newPos = lineStarts[currLineIdx] + outdented.length;
            return { handled: true, text: newText, cursorPosition: newPos };
        } else {
            lines[currLineIdx] = "";
            var newText = lines.join('\n');
            var newPos = lineStarts[currLineIdx];
            return { handled: true, text: newText, cursorPosition: newPos };
        }
    } else {
        var nextPrefix = "";
        if (matchType === "task_ast") {
            nextPrefix = matchInfo[1] + matchInfo[2] + " [ ] ";
        } else if (matchType === "ast") {
            nextPrefix = matchInfo[1] + matchInfo[2] + " ";
        } else if (matchType === "dot") {
            nextPrefix = matchInfo[1] + matchInfo[2] + " ";
        } else if (matchType === "task_hyphen") {
            nextPrefix = matchInfo[1] + "- [ ] ";
        } else if (matchType === "hyphen") {
            nextPrefix = matchInfo[1] + "- ";
        } else if (matchType === "num") {
            var nextNum = parseInt(matchInfo[2], 10) + 1;
            nextPrefix = matchInfo[1] + nextNum + matchInfo[3];
        }

        lines[currLineIdx] = beforeCursor;
        var nextLine = nextPrefix + afterCursor;
        lines.splice(currLineIdx + 1, 0, nextLine);

        var newText = lines.join('\n');
        var newPos = lineStarts[currLineIdx] + beforeCursor.length + 1 + nextPrefix.length;
        return { handled: true, text: newText, cursorPosition: newPos };
    }
}

function handleEditorKeyPress(event, targetTextArea) {
    if (!event || !targetTextArea) return false;

    var key = event.key;
    var modifiers = event.modifiers || 0;

    var Key_Tab = (typeof Qt !== "undefined" && Qt.Key_Tab !== undefined) ? Qt.Key_Tab : 0x01000001;
    var Key_Backtab = (typeof Qt !== "undefined" && Qt.Key_Backtab !== undefined) ? Qt.Key_Backtab : 0x01000002;
    var Key_Return = (typeof Qt !== "undefined" && Qt.Key_Return !== undefined) ? Qt.Key_Return : 0x01000004;
    var Key_Enter = (typeof Qt !== "undefined" && Qt.Key_Enter !== undefined) ? Qt.Key_Enter : 0x01000005;
    var Key_Up = (typeof Qt !== "undefined" && Qt.Key_Up !== undefined) ? Qt.Key_Up : 0x01000013;
    var Key_Down = (typeof Qt !== "undefined" && Qt.Key_Down !== undefined) ? Qt.Key_Down : 0x01000015;

    var ShiftModifier = (typeof Qt !== "undefined" && Qt.ShiftModifier !== undefined) ? Qt.ShiftModifier : 0x02000000;
    var ControlModifier = (typeof Qt !== "undefined" && Qt.ControlModifier !== undefined) ? Qt.ControlModifier : 0x04000000;
    var AltModifier = (typeof Qt !== "undefined" && Qt.AltModifier !== undefined) ? Qt.AltModifier : 0x08000000;
    var KeypadModifier = (typeof Qt !== "undefined" && Qt.KeypadModifier !== undefined) ? Qt.KeypadModifier : 0x20000000;

    // 1. Shift+Tab / Backtab -> Outdent
    if (key === Key_Backtab || (key === Key_Tab && (modifiers & ShiftModifier))) {
        var res = changeListLevel(targetTextArea.text, targetTextArea.selectionStart, targetTextArea.selectionEnd, targetTextArea.cursorPosition, -1);
        targetTextArea.text = res.text;
        targetTextArea.cursorPosition = res.cursorPosition;
        if (res.selectionStart !== res.selectionEnd && typeof targetTextArea.select === "function") {
            targetTextArea.select(res.selectionStart, res.selectionEnd);
        }
        event.accepted = true;
        return true;
    }

    // 2. Tab -> Indent
    if (key === Key_Tab) {
        var res = changeListLevel(targetTextArea.text, targetTextArea.selectionStart, targetTextArea.selectionEnd, targetTextArea.cursorPosition, 1);
        targetTextArea.text = res.text;
        targetTextArea.cursorPosition = res.cursorPosition;
        if (res.selectionStart !== res.selectionEnd && typeof targetTextArea.select === "function") {
            targetTextArea.select(res.selectionStart, res.selectionEnd);
        }
        event.accepted = true;
        return true;
    }

    // 3. Alt+Up -> Move Lines Up
    if ((modifiers & AltModifier) && (key === Key_Up)) {
        var res = moveLines(targetTextArea.text, targetTextArea.selectionStart, targetTextArea.selectionEnd, targetTextArea.cursorPosition, -1);
        targetTextArea.text = res.text;
        targetTextArea.cursorPosition = res.cursorPosition;
        if (res.selectionStart !== res.selectionEnd && typeof targetTextArea.select === "function") {
            targetTextArea.select(res.selectionStart, res.selectionEnd);
        }
        event.accepted = true;
        return true;
    }

    // 4. Alt+Down -> Move Lines Down
    if ((modifiers & AltModifier) && (key === Key_Down)) {
        var res = moveLines(targetTextArea.text, targetTextArea.selectionStart, targetTextArea.selectionEnd, targetTextArea.cursorPosition, 1);
        targetTextArea.text = res.text;
        targetTextArea.cursorPosition = res.cursorPosition;
        if (res.selectionStart !== res.selectionEnd && typeof targetTextArea.select === "function") {
            targetTextArea.select(res.selectionStart, res.selectionEnd);
        }
        event.accepted = true;
        return true;
    }

    // 5. Enter / Return -> Smart Enter
    if (key === Key_Return || key === Key_Enter) {
        var pureModifiers = modifiers & ~(KeypadModifier);
        if (pureModifiers === 0) {
            var res = handleSmartEnter(targetTextArea.text, targetTextArea.cursorPosition);
            if (res && res.handled) {
                targetTextArea.text = res.text;
                targetTextArea.cursorPosition = res.cursorPosition;
                event.accepted = true;
                return true;
            }
        }
    }

    return false;
}
