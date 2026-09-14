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
