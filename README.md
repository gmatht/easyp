# Epic Arc Tracker

Small CLI tool to track EVE Online Epic Arcs per character using plain text files.

Features
- Track per-character arc status (PLEASE_SELECT, LACK_STANDINGS, READY_TO_RUN, READY_TO_CHOOSE, READY_TO_COMPLETE, ON_COOLDOWN)
- Per-character files (data/players/<player>/<character>.txt) for easy rsync-friendly backups
- Accounts grouping (data/accounts/<account>.txt) with Alpha/Omega type and up to 3 characters
- Aggregates split by account type (Alpha/Omega/Unassigned)
- Atomic writes for safe rsync
- CLI commands: scan, show-player, show-character, mark-mid-choice, deliver, set-account, aggregates, verify, export-csv, import-csv

Quick Start
1. Ensure Python 3.8+ is installed.
2. Inspect seed data in `data/epic_arcs.txt` and `data/players/`.
3. Run the CLI:

```
python3 epictracker.py scan
python3 epictracker.py show-player sample_player
python3 epictracker.py aggregates
```

Mark mid-arc choice and deliver
```
python3 epictracker.py mark-mid-choice alice AliceMain blood_stained_stars amarr
python3 epictracker.py deliver alice AliceMain blood_stained_stars
```

Verify and auto-fix
```
# dry-run
python3 epictracker.py verify
# apply safe fixes (cooldown expiry, invalid choices, account cleanup)
python3 epictracker.py verify --apply
```

CSV import/export
```
python3 epictracker.py export-csv --output /tmp/all_chars.csv
python3 epictracker.py import-csv /tmp/changes.csv --apply
```

File formats
- data/epic_arcs.txt: pipe-separated rows: key|display_name|allowed_factions(semi-colon)|cooldown_days|notes
- data/players/<player>/<character>.txt: pipe-separated rows: arc_key|status|mid_choice|final_choice|last_delivered_iso|notes
- data/accounts/<account>.txt: single pipe-separated line: account_name|type|characters(semi-colon list of player/character)|notes

Rsync recommendation (push backups)
```
rsync -av --update --partial --compress data/ backup:/path/to/data/
```

Running tests
```
python -m unittest discover -v
```
