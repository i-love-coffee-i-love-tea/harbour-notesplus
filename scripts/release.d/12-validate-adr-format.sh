#!/usr/bin/env bash
# release.d hook: validate that ADRs follow the AsciiDoc ADR template.
#
# ADRs must follow the AsciiDoc specification defined in 000-index.adoc:
#   - Filename: NNN-descriptive-slug.adoc
#   - Document title: = ADR-NNN: Title (matching the filename number)
#   - Attributes: :status: and :date:
#   - Required sections:
#       == Context
#       == Decision
#       == Consequences
#   - Required subsections under Consequences:
#       === Positive / Utility Delivered
#       === Trade-offs & Mitigations
#
# Exit 0 if valid, exit 1 if errors found.
set -euo pipefail

adr_dir="docs/dev/adr"
errors=0

report() {
    echo "$1" >&2
    errors=$((errors + 1))
}

validate_adr() {
    local file="$1"
    local ferrors=0
    local filename
    filename=$(basename "$file")
    local content
    content=$(<"$file")

    freport() {
        echo "$file: $1" >&2
        ferrors=$((ferrors + 1))
    }

    local nr
    nr=$(echo "$filename" | grep -oE '^[0-9]+' || true)

    # Document title: must be '= ADR-NNN: Title' matching the number
    local title_line
    title_line=$(head -n 1 "$file")
    if ! echo "$title_line" | grep -qE '^= ADR-[0-9]{3}: '; then
        freport "first line must be '= ADR-NNN: Title' (got '$title_line')"
    elif [ -n "$nr" ] && ! echo "$title_line" | grep -qE "^= ADR-$nr: "; then
        freport "title prefix does not match filename number $nr (got '$title_line')"
    fi

    # Document attributes: :status: and :date:
    if ! echo "$content" | head -20 | grep -qE '^:status:'; then
        freport "missing ':status:' document attribute"
    fi

    if ! echo "$content" | head -20 | grep -qE '^:date:'; then
        freport "missing ':date:' document attribute"
    fi

    # Required section: == Context
    if ! echo "$content" | grep -q '^== Context'; then
        freport "missing '== Context' section"
    fi

    # Required section: == Decision
    if ! echo "$content" | grep -q '^== Decision'; then
        freport "missing '== Decision' section"
    fi

    # Required section: == Consequences
    if ! echo "$content" | grep -q '^== Consequences'; then
        freport "missing '== Consequences' section"
    fi

    # Required consequences subsections
    if ! echo "$content" | grep -q '^=== Positive / Utility Delivered'; then
        freport "missing '=== Positive / Utility Delivered' subsection"
    fi

    if ! echo "$content" | grep -q '^=== Trade-offs & Mitigations'; then
        freport "missing '=== Trade-offs & Mitigations' subsection"
    fi

    errors=$((errors + ferrors))
}

# Collect all ADR files
adr_files=()
if [ ! -d "$adr_dir" ]; then
    echo "No ADR directory at $adr_dir, skipping"
    exit 0
fi

for file in "$adr_dir"/[0-9][0-9][0-9]-*.adoc; do
    [ -f "$file" ] || continue
    [[ "$(basename "$file")" == "000-index.adoc" ]] && continue
    adr_files+=("$file")
done

if [ ${#adr_files[@]} -eq 0 ]; then
    echo "No ADR files found, skipping"
    exit 0
fi

# Check for duplicate ADR numbers
declare -A seen_numbers
for file in "${adr_files[@]}"; do
    nr=$(basename "$file" | grep -oE '^[0-9]+' || true)
    [ -z "$nr" ] && continue
    if [ -n "${seen_numbers[$nr]+_}" ]; then
        report "duplicate ADR number $nr: $file and ${seen_numbers[$nr]}"
    else
        seen_numbers[$nr]="$file"
    fi
done

# Validate each ADR
for file in "${adr_files[@]}"; do
    validate_adr "$file"
done

if [ "$errors" -gt 0 ]; then
    echo "$errors error(s) found" >&2
    exit 1
fi

echo "All ADRs pass AsciiDoc validation (${#adr_files[@]} checked)."
