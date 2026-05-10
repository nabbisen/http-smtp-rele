#!/bin/sh
# scripts/check-rfcs.sh
# Verify structural integrity of the rfcs/ directory.
#
# Checks:
#   1. Every RFC file under rfcs/{proposed,done,archive}/ matches NNN-*.md
#   2. No RFC number appears more than once across all three folders
#   3. Every RFC file is referenced in rfcs/README.md
#   4. Every relative link in rfcs/README.md resolves to an existing file
#   5. Status field in each RFC matches the folder it lives in
#
# Exit 0 if all checks pass; 1 otherwise.

set -e

# ── Locate root ──────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RFCS="$ROOT/rfcs"
README="$RFCS/README.md"

ERRORS=0

fail() {
    echo "ERROR: $*" >&2
    ERRORS=$((ERRORS + 1))
}

# ── Guard: required paths ────────────────────────────────────────────────────
for dir in "$RFCS/proposed" "$RFCS/done" "$RFCS/archive"; do
    if [ ! -d "$dir" ]; then
        fail "Directory missing: $dir"
    fi
done

if [ ! -f "$README" ]; then
    fail "rfcs/README.md not found"
fi

# ── 1. Filename pattern ──────────────────────────────────────────────────────
find "$RFCS/proposed" "$RFCS/done" "$RFCS/archive" \
    -maxdepth 1 -name '*.md' 2>/dev/null | while IFS= read -r f; do
    base="$(basename "$f")"
    case "$base" in
        [0-9][0-9][0-9]-*.md) ;;
        *) fail "Bad filename (must be NNN-slug.md): $f" ;;
    esac
done

# ── 2. Duplicate RFC numbers ─────────────────────────────────────────────────
ALL_NUMS=$(find "$RFCS/proposed" "$RFCS/done" "$RFCS/archive" \
    -maxdepth 1 -name '*.md' 2>/dev/null \
    | xargs -I{} basename {} \
    | sed 's/-.*//' \
    | sort)
DUPES=$(echo "$ALL_NUMS" | uniq -d)
if [ -n "$DUPES" ]; then
    for n in $DUPES; do
        fail "Duplicate RFC number: $n"
    done
fi

# ── 3. README completeness: every file listed ────────────────────────────────
find "$RFCS/proposed" "$RFCS/done" "$RFCS/archive" \
    -maxdepth 1 -name '*.md' 2>/dev/null | while IFS= read -r f; do
    base="$(basename "$f")"
    # Strip leading path; check if basename appears in README
    if ! grep -qF "$base" "$README"; then
        fail "RFC not listed in rfcs/README.md: $base"
    fi
done

# ── 4. README accuracy: every link resolves ──────────────────────────────────
# Extract relative links of the form ./proposed/NNN-*.md ./done/NNN-*.md etc.
grep -oE '\./[a-z]+/[0-9]{3}-[^)]+\.md' "$README" 2>/dev/null | while IFS= read -r link; do
    target="$RFCS/${link#./}"
    if [ ! -f "$target" ]; then
        fail "Broken link in rfcs/README.md: $link (file not found: $target)"
    fi
done

# ── 5. Status field matches folder ───────────────────────────────────────────
check_status() {
    local folder="$1"
    local expected_pattern="$2"
    find "$RFCS/$folder" -maxdepth 1 -name '*.md' 2>/dev/null | while IFS= read -r f; do
        status_line=$(grep -m1 '^\*\*Status\.\*\*' "$f" 2>/dev/null || true)
        if [ -z "$status_line" ]; then
            fail "No Status field found in: $f"
            continue
        fi
        if ! echo "$status_line" | grep -qiE "$expected_pattern"; then
            fail "Status mismatch in $f (folder=$folder, got: $status_line)"
        fi
    done
}

check_status "proposed" "Proposed"
check_status "done"     "Implemented"
check_status "archive"  "Withdrawn|Superseded"

# ── Result ───────────────────────────────────────────────────────────────────
if [ "$ERRORS" -eq 0 ]; then
    echo "RFC check passed. $(find "$RFCS/proposed" "$RFCS/done" "$RFCS/archive" \
        -maxdepth 1 -name '*.md' 2>/dev/null | wc -l | tr -d ' ') RFCs verified."
    exit 0
else
    echo "$ERRORS error(s) found." >&2
    exit 1
fi
