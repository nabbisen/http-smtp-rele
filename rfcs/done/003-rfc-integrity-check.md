# RFC 003 — RFC Index and Integrity Check

**Status.** Implemented  
**Tracks.** Governance  
**Touches.** `rfcs/README.md`, `scripts/check-rfcs.sh`

## Summary

Create `rfcs/README.md` as the always-current RFC index, and `scripts/check-rfcs.sh` as a
shell script that enforces structural invariants on the `rfcs/` directory.

## Motivation

Without an index, contributors must scan all three folders to understand the current state of
the project. Without automated checks, the index drifts out of sync and RFC numbers collide.
Both problems compound over time.

## Scope

- Define the structure of `rfcs/README.md`.
- Write `scripts/check-rfcs.sh` covering:
  - Filename convention (`NNN-slug.md`).
  - No duplicate RFC numbers across folders.
  - Every RFC file is listed in `rfcs/README.md`.
  - Every entry in `rfcs/README.md` points to an existing file.
  - Status field in each RFC matches the folder it lives in.
- Add `make check-rfcs` or equivalent local command.

## Non-goals

- Linting RFC prose or section completeness (could be a future addition).
- CI pipeline setup (RFC 004 covers quality gates).
- Enforcing that the slug matches the title exactly.

## Design

### `rfcs/README.md` structure

```markdown
# http-smtp-rele RFCs

## Proposed

| ID  | Title | Milestone | Priority |
|-----|-------|-----------|----------|
| 001 | [RFC governance](./proposed/001-rfc-governance.md) | M0 | High |

## Implemented

| ID  | Title | Shipped in |
|-----|-------|------------|

## Archive

| ID  | Title | Reason |
|-----|-------|--------|
```

State groupings match the folder names. Each row links to the RFC file using a relative path.

### `scripts/check-rfcs.sh`

Checks (in order):

1. **Filename pattern** — every `.md` file under `rfcs/{proposed,done,archive}/` matches
   `[0-9][0-9][0-9]-*.md`.

2. **Duplicate numbers** — extract the leading `NNN` from every filename; fail if any NNN
   appears more than once across all three folders.

3. **README completeness** — every file found by `find rfcs/{proposed,done,archive} -name '*.md'`
   is referenced somewhere in `rfcs/README.md`. Check by extracting the filename portion from
   the README's relative links.

4. **README accuracy** — every relative link in `rfcs/README.md` resolves to an existing file
   (i.e., the file it points to actually exists on disk).

5. **Status consistency** — for each RFC file, grep for `^\*\*Status\.\*\*` and compare to
   the folder name:
   - `proposed/` → Status must be `Proposed`
   - `done/` → Status must start with `Implemented`
   - `archive/` → Status must be `Withdrawn` or `Superseded`

Script exits 0 if all checks pass, 1 with a descriptive error message otherwise.

### Script skeleton

```sh
#!/bin/sh
set -e

RFCS_DIR="$(cd "$(dirname "$0")/.." && pwd)/rfcs"
README="$RFCS_DIR/README.md"
ERRORS=0

fail() { echo "ERROR: $1" >&2; ERRORS=$((ERRORS + 1)); }

# 1. Filename pattern
find "$RFCS_DIR/proposed" "$RFCS_DIR/done" "$RFCS_DIR/archive" \
    -name '*.md' | while read -r f; do
    base=$(basename "$f")
    case "$base" in
        [0-9][0-9][0-9]-*.md) ;;
        *) fail "Bad filename: $f" ;;
    esac
done

# 2. Duplicate numbers
nums=$(find "$RFCS_DIR/proposed" "$RFCS_DIR/done" "$RFCS_DIR/archive" \
    -name '*.md' | xargs -I{} basename {} | sed 's/-.*//')
dupes=$(echo "$nums" | sort | uniq -d)
[ -n "$dupes" ] && fail "Duplicate RFC numbers: $dupes"

# 3 & 4. README completeness and accuracy
# (extract links from README and cross-check with filesystem)
...

# 5. Status consistency
...

[ "$ERRORS" -eq 0 ] || exit 1
echo "RFC check passed."
```

(Full implementation is in `scripts/check-rfcs.sh`.)

## Implementation Plan

1. Create `scripts/check-rfcs.sh` with all five checks.
2. Make it executable (`chmod +x`).
3. Create `rfcs/README.md` with the initial state-grouped table.
4. Run `scripts/check-rfcs.sh` against the initial RFC set and confirm it passes.
5. Document the command in the project's contribution guide.

## Test Plan

### Unit Tests

- Script exits 0 on a tree with one valid RFC in each folder.
- Script exits 1 when a filename does not match `NNN-slug.md`.
- Script exits 1 when two RFCs share the same number.
- Script exits 1 when an RFC file exists but is not in `README.md`.
- Script exits 1 when `README.md` links to a file that does not exist.
- Script exits 1 when a file in `done/` has `Status: Proposed`.

### Operational Tests

- Running `scripts/check-rfcs.sh` on the initial `rfcs/` tree exits 0.

## Security Considerations

No security implications — this RFC covers documentation tooling only.

## Operational Considerations

- `scripts/check-rfcs.sh` should be run locally before every commit that touches `rfcs/`.
- It may be added to a pre-commit hook.
- Future CI can call it as a quality gate (see RFC 004).

## Documentation Changes

- Create `rfcs/README.md`.
- Create `scripts/check-rfcs.sh`.
- Mention the check command in `docs/README.md` or `CONTRIBUTING.md`.

## Acceptance Criteria

| ID | Criterion |
|----|-----------|
| AC-003-01 | `rfcs/README.md` exists and lists all initial RFCs by state. |
| AC-003-02 | `scripts/check-rfcs.sh` is executable and exits 0 on the initial tree. |
| AC-003-03 | `scripts/check-rfcs.sh` exits 1 when a duplicate number is introduced. |
| AC-003-04 | `scripts/check-rfcs.sh` exits 1 when README is missing an RFC. |
| AC-003-05 | `scripts/check-rfcs.sh` exits 1 when Status mismatches folder. |

## Open Questions

None.
