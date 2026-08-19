# Configuration Guide

This document explains how to configure Terminalist.

## Configuration Files

Terminalist supports configuration via TOML files. Configuration is loaded in this order:

1. `./terminalist.toml` in the current working directory
2. `terminalist/config.toml` beneath the operating system's standard configuration directory
3. Built-in defaults

Typical user paths are `~/Library/Application Support/terminalist/config.toml` on macOS and `${XDG_CONFIG_HOME:-~/.config}/terminalist/config.toml` on Linux.

## Generate Default Configuration

```bash
terminalist --generate-config
```

This creates the user configuration file at the platform-appropriate path and prints that path.

## Configuration Options

### Example Configuration

```toml
[ai]
model = "gpt-5.6-sol"
api_key_env = "OPENAI_API_KEY"

[ui]
default_project = "today"         # Options: "inbox", "today", "tomorrow", "upcoming", project ID, or project name
mouse_enabled = true              # Enable mouse support
sidebar_width = 26                # Sidebar width in columns (15-50)
sidebar_visible = true            # Initial state before a last-used layout has been saved
shortcut_bar_visible = true       # Show common keyboard shortcuts along the bottom

[sync]
auto_sync_interval_minutes = 5    # Auto-sync interval (0 = disabled)

[display]
date_format = "%Y-%m-%d"          # Date format for task due dates
time_format = "%H:%M"             # Time format for datetime fields
show_descriptions = true          # Show task descriptions in list view
show_durations = true             # Show task durations
show_labels = true                # Show task labels
show_project_colors = false       # Show project colors

[logging]
enabled = false                   # Enable logging to file
```

### AI Task Management

- **model**: OpenAI model used to generate proposals. The default is `gpt-5.6-sol`.
- **api_key_env**: Name of the environment variable containing the OpenAI API key. The checked-in default uses `OPENAI_VALERIA_API_KEY` for a personal development environment; public users should set this explicitly, commonly to `OPENAI_API_KEY` as shown above.

Terminalist reads the OpenAI key at runtime and never writes its value to the configuration file, database, or logs. Export the configured variable in the shell that launches Terminalist.

The separate `TODOIST_API_TOKEN` is also read from the environment, but the current backend implementation persists that Todoist token in the local SQLite database as unencrypted JSON credentials. See [Privacy and Local Data](../PRIVACY.md).

### UI Configuration

- **default_project**: Set the initial view when starting the app
  - Options: `"inbox"`, `"today"`, `"tomorrow"`, `"upcoming"`, a specific project ID, or project name
- **mouse_enabled**: Enable or disable mouse support
- **sidebar_width**: Width of the sidebar in columns (must be between 15-50)
- **sidebar_visible**: Initial sidebar state. After the first layout change, Terminalist restores the last expanded/collapsed state and expanded width from `ui-state.toml` beside the user configuration file.
- **shortcut_bar_visible**: Show or hide the common keyboard-shortcut bar at the bottom

### Sync Configuration

- **auto_sync_interval_minutes**: How often to automatically sync with Todoist
  - Set to `0` to disable automatic syncing (manual sync only with `r` key)

### Display Configuration

- **date_format**: Format for displaying dates (uses [chrono format strings](https://docs.rs/chrono/latest/chrono/format/strftime/index.html))
- **time_format**: Format for displaying times
- **show_descriptions**: Whether to show task descriptions in the list view
- **show_durations**: Whether to show task duration information
- **show_labels**: Whether to show task labels as colored badges
- **show_project_colors**: Whether to show project colors

### Logging Configuration

- **enabled**: Enable debug logging to file for troubleshooting
