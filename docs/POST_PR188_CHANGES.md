# Post-PR #188 Branch Changes

Last updated: July 23, 2026

## Purpose

This file tracks user-visible features developed on `codex/post-pr188`, which is based on
the source branch for PR #188. Keep it current as work is added so it can serve as the
scope, testing summary, and starting point for a future pull request.

Do not include PR #188's existing consolidated work here. Record only changes introduced
after this branch diverged from `codex/test-full-stack`.

## Changes

### Task Details dialog

- Added a read-only modal dialog for the selected task.
- Changed `Enter` in the task list from toggling completion to opening task details.
- Kept `Space` as the task-completion shortcut, including marked-task bulk completion.
- Displayed the full task title, description, project, status, priority, due date or time,
  deadline, duration, and recurring state.
- Added scrolling with `j`/`k`, arrow keys, Page Up/Down, Home, and End.
- Updated the shortcut bar, built-in help, and keyboard-shortcut documentation.
- Added behavior and rendering tests.

Known follow-up:

- Task labels are not shown because task-label associations are not currently loaded into
  the dialog's UI data. Add them when that relationship becomes available without an
  extra per-render database query.

Validation:

- `cargo fmt --all -- --check`
- `cargo test` (103 tests passed)

### Modeless sidebar navigation

- Kept the task list permanently active instead of switching focus between panes.
- Added unshifted `]`/`[` shortcuts to move down/up through navigation items while
  retaining `J`/`K` as alternatives.
- Routed `H`/`L` project-folder folding without requiring sidebar focus.
- Kept the navigation border passive and the task-list border active.
- Updated the shortcut bar, built-in help, README, PRD, and keyboard-shortcut
  documentation.
- Added behavior tests for bracket and uppercase sidebar navigation.

## Future pull request outline

Suggested title:

> Add task details dialog and post-PR #188 UI improvements

Draft summary:

> Adds a read-only, scrollable details dialog so task titles and metadata are available
> without editing the task. `Enter` opens details and `Space` remains the completion
> shortcut.

Before opening a pull request:

1. Rebase onto the final PR #188 merge result or the latest intended base.
2. Review this file against the actual branch diff and remove stale notes.
3. Run the required formatting and test commands again.
4. Replace the validation counts above with the final results.
5. Decide whether the accumulated changes form one coherent PR or should be split.
