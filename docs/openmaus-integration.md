# OpenMaus integration

The OpenMaus section provides Activity, Schedule, and Usage tabs in a 560 × 232 panel. It returns to the next section's dimensions when navigating away. Lists show two rows per page.

Usage shows today's tokens and turns plus recorded total cost on the left. A Bot/Model/Provider segmented control selects the accumulated usage list on the right. Rows separate names, token totals, turns, and reported costs. Missing cost data displays a dash.

## Data sources

The default directory is `%USERPROFILE%\.openmausbot`. Set `NOCAN_OPENMAUS_DIR` to read another location. Nocan only reads these sources; it does not edit schedules, send messages, or execute automations.

| Source | Data |
| --- | --- |
| `bots.json` | Bot names, thread titles, and relationships |
| `routines.json` | Enabled/disabled schedules, next-run times, and run transitions |
| `events/*.ndjson` | Agent turn start/completion, terminal failures, response requests, and activity limits |
| `openmausbot-server.lease` | Server PID and start time; Nocan verifies the process instead of trusting a stale file |
| `usage/*.jsonl`, `config.json` | Recorded tokens, turns, reported costs, and provider names |

Activity polling runs on a worker every second. Usage updates every five seconds; refresh requests an earlier update. The activity reader resumes at the previous log offset and bounds the initial tail to one MiB per file. Incomplete trailing lines wait for completion. Temporarily invalid metadata keeps the last valid snapshot and exposes a sync error.

## In-notch alerts

Alerts cover automation start/completion/failure, agent completion/failure, and attention requests. They appear in the notch for five seconds. Clicking an alert opens Activity; its close control or Escape dismisses it.

Alerts wait while another panel or hydration reminder is active. The queue is bounded, and stale events are discarded. The Settings switch and section bell control share the same preference.

Initial history does not trigger alerts. Event IDs prevent repeats, and automation-associated threads are not also reported as independent agent replies. `item.completed` represents an individual item, so it is not treated as a final agent response. Running state is checked against the current server generation. Usage dates and schedule times follow the Windows time zone.

## Diagnostics

```powershell
.\target\release\nocan.exe --inspect-openmaus
.\target\release\nocan.exe --preview-branding target\branding-preview
```

The inspect mode reads a local summary without changing OpenMaus. The branding preview uses isolated core application data and includes a labeled sample notification. Normal application startup does not insert sample events. Previewing can still read real local OpenMaus data, so review snapshots before sharing them.
