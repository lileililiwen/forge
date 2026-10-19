#!/usr/bin/env bash
# Workspace UI conformance check.
#
# The subject of this check is the *absence* of a second definition of a shared
# UI primitive, not the presence of the shared one: most repositories that ship
# a user interface do not consume this layer, and a repository that ships no user
# interface must not be forced to adopt anything. A repository is conformant by
# either consuming the committed feed (ui-conformance.json, state consumes-feed)
# or declaring a reasoned exemption (state exempt), and by not declaring
# .visually-hidden / .sr-only itself.
#
# The three refusals the .NET `forge` precedent settled are enforced here rather
# than by ui/scripts/verify-ui-feed.mjs alone, because a repository that ships no
# declaration is checked by this script and a repository that ships one is
# checked by both.
#
# The check is READ-ONLY over consumer repositories. It reports; it never writes
# to, installs into, or reformats another repository. It exits non-zero with a
# named diagnostic for:
#
#   ui-divergent-primitive  a repository outside the frozen legacy list declares
#                           a shared visually-hidden primitive itself
#   ui-conformance-undeclared a UI-bearing repository declares nothing
#   ui-breakpoint-undeclared  a width-based media query value that is neither the
#                           documented kit scale nor part of the frozen baseline
#   ui-feed-*                anything ui/scripts/verify-ui-feed.mjs reports for a
#                           repository that declares consumes-feed
#
# The frozen baseline (ui/legacy-divergent.json) is a data file that this script
# only reads (D16). Nothing here can add an entry to it; a list that grows fails
# the architecture guard with ui-legacy-list-grew instead.
#
# Usage:
#   scripts/check-ui-conformance.sh [--workspace <dir>] [--frozen <file>]
#                                   [--kit <name>] [--verbose]
#
# Exit codes:
#   0  every repository in the workspace is conformant, or is a known frozen
#      divergence, or is out of scope (no user interface, or Flutter - NG5)
#   1  at least one named diagnostic was reported
#   2  invocation error

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
WORKSPACE="$(cd "$ROOT_DIR/.." && pwd)"
FROZEN="$ROOT_DIR/ui/legacy-divergent.json"
KIT="$(basename "$ROOT_DIR")"
VERBOSE=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --workspace) WORKSPACE="$2"; shift 2 ;;
    --frozen) FROZEN="$2"; shift 2 ;;
    --kit) KIT="$2"; shift 2 ;;
    --verbose) VERBOSE=1; shift ;;
    -h|--help) sed -n '2,40p' "$0"; exit 0 ;;
    *) printf 'Unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
done

[[ -d "$WORKSPACE" ]] || { printf 'ui-check-invocation: workspace %s does not exist\n' "$WORKSPACE" >&2; exit 2; }
[[ -f "$FROZEN" ]] || { printf 'ui-check-invocation: frozen baseline %s does not exist\n' "$FROZEN" >&2; exit 2; }

python3 - "$WORKSPACE" "$FROZEN" "$KIT" "$ROOT_DIR" "$VERBOSE" <<'PY'
import json, os, re, subprocess, sys

workspace, frozen_path, kit, kit_root, verbose = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5] == '1'

EXCLUDED_DIRECTORIES = {'node_modules', '.git', 'obj', 'bin', 'target', 'dist', '.next',
                        '.dart_tool', 'vendor', '.venv', 'site-packages', 'openspec'}
# A vendored feed is the shared layer itself, not a second definition of it.
VENDORED_FEED = ('vendor/platform-ui',)
CSS_EXTENSIONS = {'.css', '.scss', '.less'}
UI_FILE_EXTENSIONS = {'.razor', '.cshtml', '.html', '.vue', '.svelte', '.jsx', '.tsx', '.astro'}
UI_PATH = re.compile(r'(^|/)(web|wwwroot|frontend|client|ui|app|site|src|public|assets)(/|$)')
# A suffix beginning with '-' is still this primitive: argoset's .sr-only-focusable is a
# second definition of the screen-reader-only utility, so the detector must see it too.
PRIMITIVE_RULE = re.compile(r'[^{};]*\.(?:visually-hidden|sr-only)(?!\w)[^{};]*\{')
COMMENT = re.compile(r'/\*.*?\*/', re.S)
MEDIA_WIDTH = re.compile(r'@media[^{]*?(?:min|max)-width\s*:\s*([0-9.]+)(px|rem|em)')

diagnostics = []
notes = []


def fail(code, message):
    diagnostics.append(f'{code}: {message}')


def skipped(path):
    try:
        stat = os.stat(path)
    except OSError:
        return True
    if stat.st_size > 200 * 1024:
        return True
    if stat.st_size > 20 * 1024:  # minified vendor bundle
        try:
            with open(path, 'rb') as handle:
                return sum(1 for _ in handle) < 5
        except OSError:
            return True
    return False


def scan(repo):
    root = os.path.join(workspace, repo)
    ui_signal = False
    primitive_rules = []
    media_values = {}
    flutter = False
    for base, directories, files in os.walk(root):
        directories[:] = [name for name in directories if name not in EXCLUDED_DIRECTORIES]
        for name in sorted(files):
            path = os.path.join(base, name)
            extension = os.path.splitext(name)[1]
            relative = os.path.relpath(path, root)
            if any(segment in relative for segment in VENDORED_FEED):
                continue
            if extension in UI_FILE_EXTENSIONS and not skipped(path):
                ui_signal = True
            if extension == '.dart':
                ui_signal = True
            if name == 'pubspec.yaml':
                flutter = True  # NG5: Dart repositories are reported, never failed
            if extension not in CSS_EXTENSIONS or skipped(path):
                continue
            if UI_PATH.search(relative):
                ui_signal = True
            try:
                text = open(path, encoding='utf-8', errors='ignore').read()
            except OSError:
                continue
            for match in PRIMITIVE_RULE.finditer(text):
                # Report the line the selector itself is on, not the line the rule's
                # prelude (often a comment) starts on, so the diagnostic matches the
                # frozen evidence in ui/legacy-divergent.json.
                selector_at = match.start() + re.search(r'\.(?:visually-hidden|sr-only)(?!\w)', match.group(0)).start()
                line = text[:selector_at].count('\n') + 1
                selector = ' '.join(COMMENT.sub(' ', match.group(0))[:-1].split())
                primitive_rules.append((relative, line, selector))
            for match in MEDIA_WIDTH.finditer(text):
                value = f'{match.group(1)}{match.group(2)}'
                media_values.setdefault(value, relative)
    return ui_signal, primitive_rules, media_values, flutter


frozen = json.load(open(frozen_path, encoding='utf-8'))
known_divergent = {entry['repo']: entry for entry in frozen['divergentPrimitive']}
undeclared_baseline = set(frozen['undeclaredUiRepos'])
flutter_baseline = set(frozen['flutterNotCoveredRepos'])
baselined_widths = set(frozen['mediaQueryBaselinedValues'])
kit_scale = list(frozen['kitScale']['breakpoints'])
kit_scale_px = [(value, f'{int(float(value[:-3]) * 16)}px') for value in kit_scale]
canonical = '.visually-hidden (9 properties, no !important) from @platform/design-tokens'

def report_breakpoint(repo, value, relative):
    if value in baselined_widths or value in kit_scale:
        return
    pixels = float(value[:-2]) if value.endswith('px') else float(value[:-3]) * 16
    nearest = min(kit_scale_px, key=lambda entry: abs(float(entry[1][:-2]) - pixels))
    fail('ui-breakpoint-undeclared',
         f'{repo} introduces the width value {value} at {relative}, which is neither a documented kit scale value ({", ".join(kit_scale)}) nor part of the frozen baseline. Use {nearest[0]} ({nearest[1]}).')


repositories = [name for name in sorted(os.listdir(workspace))
                if os.path.isdir(os.path.join(workspace, name)) and not name.startswith('.')]

counts = {'conformant': 0, 'known-divergent': 0, 'baselined-undeclared': 0, 'out-of-scope': 0}
known_hits = []

for repo in repositories:
    if repo == kit:
        notes.append(f'{repo}: the kit itself; it owns the canonical primitive and is not a consumer.')
        continue

    ui_signal, primitive_rules, media_values, flutter = scan(repo)

    for relative, line, selector in primitive_rules:
        if repo in known_divergent:
            known_hits.append(repo)
            notes.append(f'{repo}: known frozen divergence {relative}:{line} {selector}')
        else:
            fail('ui-divergent-primitive',
                 f'{repo} declares {selector!r} at {relative}:{line}. Use the shared {canonical} instead.')

    declaration_path = os.path.join(workspace, repo, 'ui-conformance.json')

    if flutter:
        # NG5: a Dart repository is not required to adopt this capability, and no Flutter
        # conformance rule is claimed. It is still scanned above, so a divergence it
        # declares in its own web assets stays visible rather than hidden by this branch.
        counts['out-of-scope'] += 1
        if repo not in flutter_baseline:
            fail('ui-conformance-undeclared', f'{repo}: a new Flutter repository is outside the frozen not-covered baseline; this capability ships no Dart artifact (NG5), so it is recorded rather than failed silently.')
        else:
            notes.append(f'{repo}: Flutter, not covered by this capability (NG5); still scanned for shared primitives.')
        # A Dart repository is not required to adopt the layer, so the declaration rules
        # below do not apply to it; its media queries are still facts about its own CSS.
        for value, relative in sorted(media_values.items()):
            report_breakpoint(repo, value, relative)
        continue

    if not os.path.exists(declaration_path):
        if not ui_signal:
            counts['out-of-scope'] += 1
            notes.append(f'{repo}: no user-interface signal and no declaration; conformant without adopting the layer.')
        elif repo in undeclared_baseline:
            counts['baselined-undeclared'] += 1
            notes.append(f'{repo}: ships a user interface with no ui-conformance.json; frozen baseline, not failing today.')
        else:
            fail('ui-conformance-undeclared',
                 f'{repo} ships a user interface (detected in its own stylesheets and templates) and declares no ui-conformance.json.')
    else:
        declaration = json.load(open(declaration_path, encoding='utf-8'))
        state = declaration.get('state')
        if state == 'exempt':
            counts['conformant'] += 1
            notes.append(f"{repo}: exempt - {declaration.get('reason', '(no reason recorded)')} (owner {declaration.get('owner', '?')}, review by {declaration.get('reviewBy', '?')}).")
        elif state == 'consumes-feed':
            result = subprocess.run(['node', os.path.join(kit_root, 'ui/scripts/verify-ui-feed.mjs'), os.path.join(workspace, repo)],
                                    capture_output=True, text=True)
            if result.returncode == 1:
                diagnostics.extend(line for line in result.stderr.splitlines() if line.startswith('ui-'))
            elif result.returncode not in (0, 2):
                fail('ui-check-toolchain-missing', f'{repo}: ui/scripts/verify-ui-feed.mjs could not run (exit {result.returncode}); the feed is declared but unverified.')
            else:
                counts['conformant'] += 1
                notes.append(f'{repo}: consumes-feed at kit version {declaration.get("kitVersion", "?")}, digests verified.')
        else:
            fail('ui-conformance-invalid', f'{repo}: ui-conformance.json declares state "{state}"; expected "consumes-feed" or "exempt".')

    for value, relative in sorted(media_values.items()):
        report_breakpoint(repo, value, relative)

for line in notes:
    if verbose or line.startswith(('known frozen divergence', 'exempt', 'not covered')):
        print(f'  note  {line}')

print(f'ui-conformance: {len(repositories)} repositories scanned in {workspace}; '
      f'{counts["conformant"]} conformant, {len(set(known_hits))} known frozen divergence(s) present against a list of {len(known_divergent)}, '
      f'{counts["baselined-undeclared"]} frozen undeclared-UI baseline, {counts["out-of-scope"]} out of scope.')

if diagnostics:
    for line in diagnostics:
        print(line, file=sys.stderr)
    print(f'ui-conformance: FAILED with {len(diagnostics)} diagnostic(s).', file=sys.stderr)
    sys.exit(1)

print('ui-conformance: OK (frozen baseline unchanged; no new divergent primitive, no undeclared UI repository, no new non-scale breakpoint).')
PY
