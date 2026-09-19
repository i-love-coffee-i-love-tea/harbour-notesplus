.pragma library

var defaultCustomAiInstructions = [
    {
        "id": "beautify",
        "buttonText": qsTr("Beautify"),
        "icon": "icon-m-favorite",
        "instruction": "Please beautify the active note by adding visual structure, helpful admonition blocks (NOTE, TIP, WARNING), clean tables, and suitable emoji accents where appropriate. Call the edit_note tool with the complete beautified AsciiDoc content and filename."
    },
    {
        "id": "extract_todos",
        "buttonText": qsTr("Extract To-Dos"),
        "icon": "icon-m-select-all",
        "instruction": "Please analyze the active note and extract all actionable tasks and todo items into a clean AsciiDoc checklist using `* [ ]`."
    },
    {
        "id": "fix_grammar",
        "buttonText": qsTr("Fix Grammar"),
        "icon": "icon-m-edit",
        "instruction": "Please review and correct the spelling, grammar, punctuation, and formatting in the active note while strictly preserving and enforcing proper AsciiDoc syntax. Call the edit_note tool with the complete corrected AsciiDoc content and filename."
    },
    {
        "id": "expand_draft",
        "buttonText": qsTr("Expand & Draft"),
        "icon": "icon-m-document",
        "instruction": "Please expand and draft the ideas in the active note into a well-structured AsciiDoc document with appropriate sections, headings, and detailed explanations. Call the edit_note tool with the complete expanded AsciiDoc content and filename."
    },
    {
        "id": "analyze_external",
        "buttonText": qsTr("External Text"),
        "icon": "icon-m-website",
        "instruction": "Please analyze the following external text or content, summarize key points, and extract relevant action items into structured AsciiDoc."
    }
];

function parseCustomAiInstructions(raw) {
    if (raw && typeof raw === "string" && raw.trim().length > 0) {
        try {
            var parsed = JSON.parse(raw);
            if (Array.isArray(parsed) && parsed.length > 0) {
                return parsed;
            }
        } catch (e) {
            console.log("Error parsing custom AI instructions:", e);
        }
    }
    return defaultCustomAiInstructions;
}

function isDefaultAiInstruction(id) {
    if (!id) return false;
    for (var i = 0; i < defaultCustomAiInstructions.length; i++) {
        if (defaultCustomAiInstructions[i].id === id) {
            return true;
        }
    }
    return false;
}

function getDefaultAiInstruction(id) {
    if (!id) return null;
    for (var i = 0; i < defaultCustomAiInstructions.length; i++) {
        if (defaultCustomAiInstructions[i].id === id) {
            return defaultCustomAiInstructions[i];
        }
    }
    return null;
}

function saveCustomAiInstruction(currentList, item) {
    var list = [];
    var current = currentList || defaultCustomAiInstructions;
    for (var i = 0; i < current.length; i++) {
        list.push(current[i]);
    }
    var foundIndex = -1;
    var targetId = item.id || "";
    if (targetId.length > 0) {
        for (var j = 0; j < list.length; j++) {
            if (list[j].id === targetId) {
                foundIndex = j;
                break;
            }
        }
    } else {
        targetId = "custom_" + Date.now();
        item.id = targetId;
    }

    if (foundIndex >= 0) {
        list[foundIndex] = item;
    } else {
        list.push(item);
    }
    return JSON.stringify(list);
}

function deleteCustomAiInstruction(currentList, id) {
    var current = currentList || [];
    var filtered = [];
    for (var i = 0; i < current.length; i++) {
        if (current[i].id !== id) {
            filtered.push(current[i]);
        }
    }
    return JSON.stringify(filtered);
}

function resetCustomAiInstructions(currentList) {
    var current = currentList || [];
    var defaultIds = {};
    for (var i = 0; i < defaultCustomAiInstructions.length; i++) {
        defaultIds[defaultCustomAiInstructions[i].id] = true;
    }

    // Retain user-created instructions
    var userCustomList = [];
    for (var j = 0; j < current.length; j++) {
        if (current[j] && current[j].id && !defaultIds[current[j].id]) {
            userCustomList.push(current[j]);
        }
    }

    var resultList = [];
    for (var k = 0; k < defaultCustomAiInstructions.length; k++) {
        resultList.push(defaultCustomAiInstructions[k]);
    }
    for (var m = 0; m < userCustomList.length; m++) {
        resultList.push(userCustomList[m]);
    }

    return JSON.stringify(resultList);
}
