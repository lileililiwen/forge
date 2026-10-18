# Tasks: ui-auth-adoption-evidence

## 0. Preconditions

- [x] 0.1 `scripts/ui_conformance.py` is read first; the change extends how the
      auth rule reads a file, it does not add a second check, and it does not
      re-implement anything the DTCG resolver owns.
- [x] 0.2 The fleet census is measured before any edit: how many auth-named
      files exist, how many carry a `platform-*` name, how many Razor pages
      render no markup.
- [x] 0.3 No sibling repository is edited by this change. The declaration that
      discharges the adopter's findings is that repository's own change.

## 1. The checker

- [x] 1.1 Add the three content signals as module constants, next to the existing
      `THEME_MECHANISM_RE`, with a comment each stating what it is and is not.
- [x] 1.2 Add `_shows_shared_layer(text)` returning whether a file carries a
      shared class name or a shared-layer stylesheet reference.
- [x] 1.3 Add the composition resolution: `@layout` first, then the nearest
      `_ViewStart.cshtml`. Every candidate path is resolved through
      `_resolve_inside`, so a layout cannot escape the project.
- [x] 1.4 Add `_renders_no_markup(text)`, restricted in use to Razor page files.
- [x] 1.5 Rewrite the auth loop: unreadable file → reported; shared-layer signal
      → unreported with a named note; declared adopter + no own markup +
      shared-layer composed surface → unreported with a named note; everything
      else → reported exactly as before.
- [x] 1.6 Read through `_read_text`, so the bounded-read rule is not bypassed.

## 2. Tests

- [x] 2.1 A bare `LoginPage.tsx` with no shared reference still fires.
- [x] 2.2 A login page using `platform-*` class names does not fire, and the note
      names the file and the signal.
- [x] 2.3 A login page importing a vendored shared-layer stylesheet does not fire.
- [x] 2.4 A declared adopter that writes a fresh bare login page still fires.
- [x] 2.5 A declared adopter's handler-only `.cshtml` page goes unreported when
      the composed layout shows the shared layer.
- [x] 2.6 The same page is still reported when the layout is unresolvable, when
      the layout shows no shared signal, and when no adoption is declared.
- [x] 2.7 A non-Razor file that renders nothing is still reported.
- [x] 2.8 An unreadable auth-named file is still reported.
- [x] 2.9 Every pre-existing auth-rule test still passes unchanged, including
      `test_adoption_plus_hand_rolled_login_still_fails` and
      `test_the_load_bearing_proof`.
- [x] 2.10 Scratch repositories are built in `tempfile.TemporaryDirectory()`. No
      test calls `rm`.

## 3. Evidence

- [x] 3.1 The fleet audit is run with no baseline before and after, and the two
      reports are diffed: exactly one repository's `UI_AUTH_HANDROLLED` count
      changes, and it is the provider.
- [x] 3.2 `generate_ui_conformance_baseline.py --check` is run before and after;
      the committed snapshot is regenerated with its own generator so the
      recorded count matches the measured state again.

## 4. Verification

- [x] 4.1 `python3 -m unittest discover -s tests` — no new failure.
- [x] 4.2 `openspec validate --all --strict --no-interactive`.
- [x] 4.3 `node scripts/check-openspec-change-names.mjs`.
- [x] 4.4 `python3 scripts/workspace_check.py --self-check`.
- [x] 4.5 `git diff --check`.

## 5. Deliberately not done

- [ ] No repository is migrated, and no repository's declaration is written here.
- [ ] No token semantics move into Python.
- [ ] No finding code is added, renamed or removed.
- [ ] The two failures that pre-date this change are recorded, not fixed.
