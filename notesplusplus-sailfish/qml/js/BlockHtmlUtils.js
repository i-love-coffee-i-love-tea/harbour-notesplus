.pragma library

function handleLink(link, xrefCallback, toggleCallback) {
    if (link.indexOf("xref:") === 0) {
        var target = link.substring(5);
        if (target.indexOf(".adoc") === target.length - 5) {
            target = target.substring(0, target.length - 5);
        }
        if (xrefCallback) xrefCallback(target);
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

function formatPasteWithPrefix(text, prefix) {
    if (!text || text.length === 0) return "";
    return stripAndApplyPrefix(text, prefix);
}

function highlightSearchTerms(html, term) {
    if (!term || term.length === 0 || !html || html.length === 0) return html
    var terms = term.toLowerCase().trim().split(/\s+/).filter(function(t) { return t.length > 0 })
    if (terms.length === 0) return html

    var result = html
    for (var ti = 0; ti < terms.length; ti++) {
        var t = terms[ti]
        if (t.length === 0) continue
        // Split on HTML tags to avoid highlighting inside tag attributes
        var parts = result.split(/(<[^>]+>)/)
        var rebuilt = ""
        for (var i = 0; i < parts.length; i++) {
            if (parts[i].charAt(0) === '<') {
                rebuilt += parts[i]
            } else {
                // Case-insensitive replace of search term with highlighted version
                var re = new RegExp("(" + t.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + ")", "gi")
                rebuilt += parts[i].replace(re, '<mark style="background:#ffeb3b;color:#000;padding:0 1px;border-radius:2px;">$1</mark>')
            }
        }
        result = rebuilt
    }
    return result
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
