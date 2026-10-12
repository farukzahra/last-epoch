# Superpowers docs

Design specs and implementation plans for the **Superpowers** workflow.

## Mandatory skills (after `/init`)

**Process:** `brainstorming` → `writing-plans` → implement → `verification-before-completion`  
**Debug:** `systematic-debugging`  
**Ship:** `/commit-push` with `semantic-version` + `caveman-commit`  
**Never forget:** `dont-forget` — automate recurring rules in CI/hooks, not prose  
**Recap:** `recap` + `/recap` — session delivery HTML in `docs/session-recaps/`

**Handoff:** `handoff` + `/handoff` — compact doc for the next agent (OS temp dir)

## Layout

```
docs/superpowers/
  specs/   # YYYY-MM-DD-<topic>-design.md
  plans/   # YYYY-MM-DD-<feature>.md
```

Invoke **`/init`** once per project to install skills and create this structure.
