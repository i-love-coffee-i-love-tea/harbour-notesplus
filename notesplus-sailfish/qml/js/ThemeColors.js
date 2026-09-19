.pragma library

var kStatusGreen = "#4cd964"
var kCodeBlockBg = "#18181c"
var kCodeText = "#f2f2f7"
var kCodeCardBg = "#ffffff"
var kCodeLanguageLabel = "#f2f2f7"
var kAdmonitionWarning = "#d9534f"
var kAdmonitionTip = "#f0ad4e"
var kDiffAdd = "#4cd964"
var kDiffRemove = "#ff3b30"
var kVoiceActive = "#44ff88"
var kVoiceInactive = "#ff4444"
var kNoteCardPalette = [
    "#e74c3c", "#e67e22", "#f1c40f", "#8bc34a",
    "#2ecc71", "#00b894", "#00a8ff", "#3498db",
    "#3c40c6", "#9b59b6", "#e84393", "#e17055"
]

function randomNoteColor() {
    var idx = Math.floor(Math.random() * kNoteCardPalette.length);
    return kNoteCardPalette[idx];
}
