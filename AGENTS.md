# AGENTS.md - last-epoch

Created by `/init` (project-init) on 2026-10-12.

## Agent workflow (mandatory skills)

| Phase | Skill / command | Rule |
|-------|-----------------|------|
| Design | `brainstorming` | No feature code until spec approved → `docs/superpowers/specs/` |
| Plan | `writing-plans` | `docs/superpowers/plans/YYYY-MM-DD-*.md` |
| Recurring guardrails | `dont-forget` | Prefer CI/hooks/codegen over "remember to…" |
| Automation | `automate-before-manual` | `../secrets/` before manual steps |
| Share links | `validate-before-share` | HTTP 200 + content before sending URL |
| Local test | `finish-with-dev-server` | Documented ports; conflict → kill own / next port |
| Architecture | `ask-before-architecture` | Ask before stack decisions |
| Diagrams | `diagrams-mermaid` | Mermaid + validate before show |
| UI / bugs | `ui-change-e2e` | E2E on screen change or bugfix |
| Build | stack skills + `tdd` | Per project stack |
| Verify | `verification-before-completion` | Evidence before "done" |
| Debug | `systematic-debugging` | Root cause before fix |
| Ship | `/commit-push` | `semantic-version` → `caveman-commit` → push → Actions → prod URL |
| Delivery recap | `/recap` + `recap` | HTML “pedido vs entregue” in `docs/session-recaps/` |
| Next-agent handoff | `/handoff` + `handoff` | Compact doc in OS temp dir for a fresh session |

**Language: always answer in Portuguese.** The user writes Portuguese. Never answer in Chinese, and do not switch language unless the user does.

**Gates:** no feature without approved spec; no "done" without verification; version bump only on `/commit-push`; obligations that must not be forgotten → automate (`dont-forget`).

**Always launch the fresh build when a task ends.** After any change, kill every running `le-turbo` process and start the binary you just built (`target\release\le-turbo.exe`), so the window on screen is never an older build than the code. The UI is embedded at compile time: a stale window keeps the old buttons and shows bugs that are already fixed. Say which build is now open, with its path and timestamp.

**Every `/commit-push` that ships something publishes the packages.** Run `scripts/publish.ps1`. It reads the version from `Cargo.toml` (so the tag cannot drift), builds the NSIS installer and the portable exe, creates or updates the GitHub release for that version, uploads both under stable names (`LE-Turbo-setup.exe`, `le-turbo.exe`) and verifies the download links. Then, and only then, update the version entry in `docs/release-history.json`. A version on GitHub with no downloadable packages is a broken delivery.

Invoke `/init` to (re)bootstrap skills and folders.

## Agent automation (mandatory)

Before manual steps (login, browser click, "run this yourself"): read **`automate-before-manual`** (`.agents/skills/`). If the user refuses manual work, also **`dont-be-lazy`**. Secrets vault: `../secrets/` (see `AGENTS.md` or `secrets.local.md`). Cursor rule: `.cursor/rules/automate-before-manual.mdc`.

Invoke `/init` to (re)install skills and this rule.
