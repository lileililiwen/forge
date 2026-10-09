# Flywheel demo (hookit, small candidate)

Canonical idea→scaffold→gate→publish→maintain loop for one small project.
Candidate: `hookit` (single-purpose webhook receiver, one profile, one
provider). Every step names the exact CLI and the browser URL carrying
`?project=` plus `?step=`.

## 0. Register the candidate

```sh
forge register ./hookit
forge inspect hookit
```

Web: `/workbench?project=hookit&step=idea`

## 1. Idea — graduation brief in

Validate the brief without writing, then import it on explicit confirm.
The workbench rail step 0 (`#wb-idea-entry`) previews these strings and
links to the studio spec entry.

```sh
forge graduation preview ./hookit-brief.json
forge graduation import ./hookit-brief.json --path ./hookit --profile rust-web --confirm
forge studio spec hookit
```

Web: `/workbench?project=hookit&step=idea` → studio entry `/workbench?project=hookit&step=spec`

## 2. Scaffold — new + doctor baseline

```sh
forge new ./hookit-fresh --profile rust-web --id hookit
forge doctor ./hookit
```

Web: `/workbench?project=hookit&step=scaffold`

## 3. Gate — rehearse then run bounded

```sh
forge gate --dry-run
forge gate --timeout-secs 500
forge cap list
forge cap inspect gate
```

Web: `/workbench?project=hookit&step=test`

## 4. Publish — approved preview only

```sh
forge publish --project hookit --dry-run
forge delivery status hookit
```

Delivery view: `/delivery` → `Publish approved preview` → success renders
`#delivery-next-idea` (Next-idea prompt + maintain shortcut).

Web: `/workbench?project=hookit&step=deploy`

## 5. Maintain — refresh and propose the next idea

```sh
forge portal view projects
forge plugins list --project ./hookit
```

Workbench maintain: open `/workbench?project=hookit&step=operate`, press
`Refresh` (`#wb-maintain-refresh`) or the delivery `#delivery-next-idea`
shortcut, then start the next loop at `/workbench?project=hookit&step=idea`.
