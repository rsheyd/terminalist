# Keyboard Shortcuts

This document lists all available keyboard shortcuts and TUI controls.

## Navigation

- **`j/k`** Navigate between tasks (down/up)
- **`Left/Right`** Cycle through smart views in the top navigation bar
- **`]/[`** or **`J/K`** Navigate between projects and labels in the sidebar (down/up)
- **`H/L`** Collapse/expand the selected project folder
- **Mouse** Click smart views, projects, or labels to navigate

The task list is always the active pane. Navigation shortcuts change the
selected view without moving keyboard focus away from the task list.

## Task Management

- **`x`** Mark or unmark a task for bulk actions
- **`Esc`** Clear all marked tasks
- **`Enter`** Open the selected task's details dialog
- **`e`** Edit the selected task title; also works from the task details dialog
- **`m`** Open `Manage task with AI…` from the task details dialog; proposals are reviewed and confirmed before Todoist changes
- **`Space`** Toggle completion for marked tasks, or the current task when none are marked
- **`a`** Create new task
- **`d`** Delete selected task (with confirmation)
- **`p`** Cycle task priority
- **`u`** Remove the due date from marked tasks, or the current task when none are marked
- **`t`** Set marked tasks due today
- **`T`** Set marked tasks due tomorrow
- **`w`** Set marked tasks due next week (Monday)
- **`W`** Set marked tasks due next week end (Saturday)

When no tasks are marked, due-date and completion shortcuts operate on the
currently highlighted task.

## Project Management

- **`A`** Create new project
- **`D`** Delete selected project (with confirmation)

## System

- **`b`** Expand or collapse the Projects & Labels sidebar
- **`/`** Open task search dialog (search across all tasks)
- **`r`** Force sync with Todoist
- **`D`** Empty Trash while the Trash view is selected (with confirmation)
- **`i`** Cycle through icon themes
- **`?`** Toggle help panel
- **`q`** Quit the application
- **`Esc`** Cancel action or close dialogs
- **`Ctrl+C`** Quit application

## Task Search

- **`/`** Open search dialog
- **Type** Search across all tasks by content
- **`↓`** Move focus from the query to search results
- **`j/k`** or **`↑/↓`** Navigate focused search results
- **`Space`** Complete or reopen the focused result
- **`t`** Set the focused result's due date to today
- **`Enter`** No action
- **`Esc`** Close search dialog
- **`Backspace/Delete`** Edit search query
- **`Left/Right`** Move cursor in search box

## Help Panel Scrolling

- **`↑/↓`** Scroll help content up/down
- **`Home/End`** Jump to top/bottom of help

## Interface Layout

### Layout Structure
- **Top Bar**: Today, Agenda, Tomorrow, Upcoming, and Trash smart views
- **Main Area**: Projects and labels (sidebar) | Tasks list (main area) - side by side

### Components
- **Smart Views (Top)**: Always-visible smart-view navigation; use Left/Right or click a view
- **Projects & Labels (Left)**: Hierarchical projects and labels; collapses to a
  three-column rail showing `b` and remembers its state and expanded width
  - Configurable width via `sidebar_width` in config
  - Long project names are automatically truncated with ellipsis (…)
  - Parent-child relationships clearly shown
- **Tasks List (Right)**: Shows tasks for the currently selected project
  - Takes remaining width after projects list
  - Displays task content, priority, labels, and status
- **Help Panel**: Modal overlay accessible with `?` key
- **Task Details**: Modal dialog showing the full task title, description, project, priority, due information, and status; press `e` to edit the title or `m` to open `AI Task Management`
- **AI proposal review**: After generation, use `←`/`→` to switch between the scrollable Recommendation page and the Proposed Actions page. On Proposed Actions, use `j`/`k` or `↑`/`↓` to select, `Space` to enable or disable, `r` to request a revised proposal, and `Enter` to continue.
- **Shortcut Bar**: Common controls shown along the bottom (configurable with `shortcut_bar_visible`)

## Agenda

- **`s`** Set the selected task's Todoist due time (for example, `2pm` or `14:30`)
- Timed tasks use their saved Todoist time. Untimed Today tasks receive dimmed, local-only one-hour suggestions beginning at the next whole hour.

### Task Display Features
Tasks are displayed with:
- **Trash**: Appears only while locally restorable deleted tasks exist; deleted tasks expire after 30 days
- **Restore**: Press `d` on a task in Trash to recreate it in Todoist
- **Status Icons**: ☐ (pending), ☒ (completed), ✗ (deleted)
- **Priority Badges**: Colored flags represent low, medium, high, and urgent priorities
- **Label Badges**: Colored badges showing task labels
- **Task Content**: Truncated to fit the display width
- **Completion Visual**: Completed tasks appear dimmed
- **Interactive**: Press Enter for details or Space to toggle completion
