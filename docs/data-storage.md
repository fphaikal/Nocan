# Nocan data storage

Core user data lives under `Nocan` in the Windows Local AppData known folder. This path does not depend on the executable name, version, or working directory. `NOCAN_DATA_DIR` can select a different core profile directory.

| File | Contents |
| --- | --- |
| `settings.json` | Preferences, water targets, and the stock watchlist |
| `todo.json` | Task IDs, titles, and completion states; the earlier list format remains supported |
| `water.json` | Current day/intake, hourly amounts, target, reminder state, and previous-day history |

Water changes are saved when intake, target, or reminder state changes. Updating a target does not clear recorded intake. A day transition archives the previous day in `history`; returning to a recorded date restores its totals.

## Writes and recovery

`src/storage.rs` writes to a uniquely named temporary file, synchronizes it, and replaces the primary file. The previous valid version becomes `.bak`. An unreadable primary can be loaded from a valid backup. The original damaged data is preserved when recovery permits a later write.

Loading does not overwrite unreadable files with empty defaults. Failed task or hydration writes leave the previous model state intact and report an error. A task list that failed to load blocks mutations until it is successfully loaded. Unknown future water formats are preserved rather than reset.

Close the application before copying or manually restoring data. Preserve the primary and `.bak` together, and keep a separate backup before editing them. Log files can contain diagnostic paths, so review them before sharing.

Earlier versions did not write water intake to disk. Totals from already-ended sessions of those versions cannot be recovered automatically. Existing task lists remain readable without a format conversion.

## Preview profiles and override scope

`--preview-*` and `--dump-*` modes select a temporary core profile unless `NOCAN_DATA_DIR` was explicitly supplied. Unit tests use separate temporary directories.

The override applies to settings, tasks, water history, and logging routed through core storage. The shelf and Discord configuration still use `%LOCALAPPDATA%\Nocan`; integrations use their own local data sources. Do not treat the override as isolation for every file the process can read.

## Startup after an update

If startup with Windows was already enabled, a normal application launch updates that entry to the current executable. Disabled startup remains disabled. Preview modes do not update it.
