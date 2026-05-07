#!/usr/bin/env python3
"""Epic Arc Tracker - CLI backed by text files

Usage: run `python epictracker.py --help` for commands.

Data layout (relative paths):
- data/epic_arcs.txt
- data/players/<player>/<character>.txt
- data/accounts/<account>.txt

This script provides scanning, aggregates, and simple mutations (mark-mid-choice, deliver, set-account).
It uses atomic writes (tempfile + os.replace) so edits are rsync-friendly.
"""

from __future__ import annotations

import argparse
import csv
import datetime
import os
import sys
import tempfile
from collections import defaultdict
from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple

ROOT = os.path.abspath(os.path.dirname(__file__))
DATA_DIR = os.path.join(ROOT, "data")
ARCS_FILE = os.path.join(DATA_DIR, "epic_arcs.txt")
PLAYERS_DIR = os.path.join(DATA_DIR, "players")
ACCOUNTS_DIR = os.path.join(DATA_DIR, "accounts")


def now_utc_iso() -> str:
    return datetime.datetime.now(datetime.timezone.utc).isoformat().replace("+00:00", "Z")


def parse_iso(s: str) -> Optional[datetime.datetime]:
    if not s:
        return None
    try:
        if s.endswith("Z"):
            s = s[:-1] + "+00:00"
        return datetime.datetime.fromisoformat(s)
    except Exception:
        return None


@dataclass
class Arc:
    key: str
    name: str
    allowed: List[str]
    cooldown_days: int
    notes: str


def load_arcs(path: str = ARCS_FILE) -> Dict[str, Arc]:
    arcs: Dict[str, Arc] = {}
    if not os.path.exists(path):
        return arcs
    with open(path, "r", encoding="utf-8") as f:
        for raw in f:
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            parts = [p.strip() for p in line.split("|")]
            if len(parts) < 3:
                continue
            key = parts[0]
            name = parts[1] if len(parts) > 1 else key
            allowed = [t.strip().lower() for t in parts[2].split(";") if t.strip()]
            cooldown_days = int(parts[3]) if len(parts) > 3 and parts[3].isdigit() else 90
            notes = parts[4] if len(parts) > 4 else ""
            arcs[key] = Arc(key=key, name=name, allowed=allowed, cooldown_days=cooldown_days, notes=notes)
    return arcs


@dataclass
class CharacterArcRow:
    arc_key: str
    status: str
    mid_choice: str
    final_choice: str
    last_delivered_iso: str
    notes: str


def load_character_file(path: str) -> List[CharacterArcRow]:
    rows: List[CharacterArcRow] = []
    if not os.path.exists(path):
        return rows
    with open(path, "r", encoding="utf-8") as f:
        for raw in f:
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            parts = [p.strip() for p in line.split("|")]
            # normalize length to 6
            parts += [""] * (6 - len(parts))
            row = CharacterArcRow(*parts[:6])
            rows.append(row)
    return rows


def write_atomic(path: str, lines: List[str]) -> None:
    dirname = os.path.dirname(path)
    os.makedirs(dirname, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=dirname)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            for l in lines:
                f.write(l.rstrip("\n") + "\n")
        os.replace(tmp, path)
    finally:
        if os.path.exists(tmp):
            try:
                os.remove(tmp)
            except Exception:
                pass


def save_character_file(path: str, rows: List[CharacterArcRow]):
    lines = ["# arc_key|status|mid_choice|final_choice|last_delivered_iso|notes"]
    for r in rows:
        lines.append("|".join([r.arc_key, r.status, r.mid_choice, r.final_choice, r.last_delivered_iso, r.notes]))
    write_atomic(path, lines)


def discover_characters(players_dir: str = PLAYERS_DIR) -> Dict[str, str]:
    # returns mapping player/char -> fullpath
    out: Dict[str, str] = {}
    if not os.path.isdir(players_dir):
        return out
    for player in os.listdir(players_dir):
        pdir = os.path.join(players_dir, player)
        if not os.path.isdir(pdir):
            continue
        for fname in os.listdir(pdir):
            if not fname.lower().endswith('.txt'):
                continue
            char = fname[:-4]
            key = f"{player}/{char}"
            out[key] = os.path.join(pdir, fname)
    return out


@dataclass
class Account:
    name: str
    type: str
    characters: List[str]
    notes: str


def load_accounts(accounts_dir: str = ACCOUNTS_DIR) -> Dict[str, Account]:
    out: Dict[str, Account] = {}
    if not os.path.isdir(accounts_dir):
        return out
    for fname in os.listdir(accounts_dir):
        path = os.path.join(accounts_dir, fname)
        if not os.path.isfile(path):
            continue
        with open(path, "r", encoding="utf-8") as f:
            for raw in f:
                line = raw.strip()
                if not line or line.startswith("#"):
                    continue
                parts = [p.strip() for p in line.split("|")]
                parts += [""] * (4 - len(parts))
                name, typ, chars, notes = parts[:4]
                char_list = [c.strip() for c in chars.split(";") if c.strip()]
                out[name] = Account(name=name, type=typ, characters=char_list, notes=notes)
    return out


def write_account(path: str, acc: Account) -> None:
    lines = ["# account_name|type|characters(semi-colon list of player/character)|notes"]
    chars = ";".join(acc.characters)
    lines.append("|".join([acc.name, acc.type, chars, acc.notes]))
    write_atomic(path, lines)


def cmd_scan(args):
    arcs = load_arcs()
    chars = discover_characters()
    accounts = load_accounts()
    print(f"Arcs: {len(arcs)}")
    print(f"Characters: {len(chars)}")
    print(f"Accounts: {len(accounts)}")


def compute_status_counts_for_character(rows: List[CharacterArcRow], arcs: Dict[str, Arc]) -> Dict[str, int]:
    counts = defaultdict(int)
    now = datetime.datetime.now(datetime.timezone.utc)
    for r in rows:
        st = r.status
        if st == 'ON_COOLDOWN' and r.last_delivered_iso:
            dt = parse_iso(r.last_delivered_iso)
            if dt:
                arc = arcs.get(r.arc_key)
                days = arc.cooldown_days if arc else 90
                if dt + datetime.timedelta(days=days) <= now:
                    # cooldown expired
                    st = 'READY_TO_RUN'
        counts[st] += 1
    return counts


def cmd_show_player(args):
    player = args.player
    pdir = os.path.join(PLAYERS_DIR, player)
    if not os.path.isdir(pdir):
        print(f"No such player dir: {pdir}")
        return
    arcs = load_arcs()
    for fname in sorted(os.listdir(pdir)):
        if not fname.endswith('.txt'):
            continue
        char = fname[:-4]
        path = os.path.join(pdir, fname)
        rows = load_character_file(path)
        counts = compute_status_counts_for_character(rows, arcs)
        print(f"{player}/{char}: RTR={counts.get('READY_TO_RUN',0)} RTC={counts.get('READY_TO_CHOOSE',0)} R2D={counts.get('READY_TO_COMPLETE',0)} CD={counts.get('ON_COOLDOWN',0)}")


def cmd_show_character(args):
    player = args.player
    char = args.character
    key = f"{player}/{char}"
    chars = discover_characters()
    if key not in chars:
        print(f"Character not found: {key}")
        return
    path = chars[key]
    rows = load_character_file(path)
    arcs = load_arcs()
    now = datetime.datetime.now(datetime.timezone.utc)
    for r in rows:
        cooldown = ""
        if r.status == 'ON_COOLDOWN' and r.last_delivered_iso:
            dt = parse_iso(r.last_delivered_iso)
            if dt:
                arc = arcs.get(r.arc_key)
                days = arc.cooldown_days if arc else 90
                until = dt + datetime.timedelta(days=days)
                rem = until - now
                if rem.total_seconds() > 0:
                    cooldown = str(rem)
                else:
                    cooldown = "expired"
        print(f"{r.arc_key}: status={r.status} mid={r.mid_choice} final={r.final_choice} last_delivered={r.last_delivered_iso} cooldown={cooldown} notes={r.notes}")


def find_character_file(player: str, character: str) -> Optional[str]:
    key = f"{player}/{character}"
    chars = discover_characters()
    return chars.get(key)


def cmd_mark_mid_choice(args):
    player = args.player
    char = args.character
    arc_key = args.arc
    faction = args.faction.lower()
    path = find_character_file(player, char)
    if not path:
        print("character not found")
        return
    rows = load_character_file(path)
    arcs = load_arcs()
    arc = arcs.get(arc_key)
    if not arc:
        print("arc not known")
        return
    if faction not in arc.allowed:
        print(f"faction {faction} not allowed for arc {arc_key}")
        return
    changed = False
    for r in rows:
        if r.arc_key == arc_key:
            r.mid_choice = faction
            r.status = 'READY_TO_CHOOSE'
            changed = True
    if not changed:
        # append new row
        rows.append(CharacterArcRow(arc_key, 'READY_TO_CHOOSE', faction, '', '', ''))
    save_character_file(path, rows)
    print(f"marked mid choice {faction} for {player}/{char} {arc_key}")


def cmd_deliver(args):
    player = args.player
    char = args.character
    arc_key = args.arc
    faction = args.faction.lower() if args.faction else None
    path = find_character_file(player, char)
    if not path:
        print("character not found")
        return
    rows = load_character_file(path)
    arcs = load_arcs()
    arc = arcs.get(arc_key)
    if not arc:
        print("arc not known")
        return
    found = False
    for r in rows:
        if r.arc_key == arc_key:
            found = True
            use = faction or (r.mid_choice if r.mid_choice else None)
            if use is None:
                print("no faction specified and no mid_choice present; cannot deliver")
                return
            if use not in arc.allowed:
                print(f"chosen faction {use} not allowed for arc {arc_key}")
                return
            r.final_choice = use
            r.last_delivered_iso = now_utc_iso()
            r.status = 'ON_COOLDOWN'
    if not found:
        # create a new delivered row
        use = faction
        if not use:
            print("character had no row for this arc and no faction provided")
            return
        if use not in arc.allowed:
            print(f"chosen faction {use} not allowed for arc {arc_key}")
            return
        rows.append(CharacterArcRow(arc_key, 'ON_COOLDOWN', '', use, now_utc_iso(), 'delivered'))
    save_character_file(path, rows)
    print(f"delivered {arc_key} for {player}/{char}")


def cmd_set_account(args):
    name = args.name
    typ = args.type
    chars = [c.strip() for c in args.characters.split(';') if c.strip()]
    if len(chars) > 3:
        print("accounts may contain at most 3 characters")
        return
    # validate characters exist
    available = discover_characters()
    for c in chars:
        if c not in available:
            print(f"character not found: {c}")
            return
    acc = Account(name=name, type=typ, characters=chars, notes=args.notes or '')
    os.makedirs(ACCOUNTS_DIR, exist_ok=True)
    write_account(os.path.join(ACCOUNTS_DIR, f"{name}.txt"), acc)
    print(f"wrote account {name}")


def cmd_aggregates(args):
    arcs = load_arcs()
    chars = discover_characters()
    accounts = load_accounts()
    # map character -> account type
    char_to_account_type: Dict[str, str] = {}
    for acc in accounts.values():
        for c in acc.characters:
            char_to_account_type[c] = acc.type

    # per-arc counters
    per_arc = defaultdict(lambda: defaultdict(int))
    # per-account-type counters for READY_TO_COMPLETE
    per_arc_by_account_type = defaultdict(lambda: defaultdict(int))

    for key, path in chars.items():
        rows = load_character_file(path)
        for r in rows:
            if r.status == 'READY_TO_COMPLETE':
                per_arc[r.arc_key]['ready_to_deliver'] += 1
                atype = char_to_account_type.get(key, 'Unassigned')
                per_arc_by_account_type[r.arc_key][atype] += 1
            elif r.status == 'READY_TO_RUN':
                per_arc[r.arc_key]['ready_to_run'] += 1
            elif r.status == 'READY_TO_CHOOSE':
                per_arc[r.arc_key]['need_choice'] += 1
            elif r.status == 'ON_COOLDOWN':
                per_arc[r.arc_key]['on_cooldown'] += 1

    # print table
    print("arc, ready_to_deliver, ready_to_run, need_choice, on_cooldown")
    for arc_key in sorted(per_arc.keys()):
        d = per_arc[arc_key]
        print(f"{arc_key}, {d.get('ready_to_deliver',0)}, {d.get('ready_to_run',0)}, {d.get('need_choice',0)}, {d.get('on_cooldown',0)}")
        # account type split
        bytype = per_arc_by_account_type.get(arc_key, {})
        if bytype:
            print("  by account type:")
            for t, cnt in bytype.items():
                print(f"    {t}: {cnt}")


def cmd_verify(args):
    """Scan and optionally apply safe fixes.

    Fixes applied with --apply:
    - ON_COOLDOWN -> READY_TO_RUN if cooldown expired
    - account membership duplicates resolved (keeps first occurrence)
    - accounts truncated to 3 characters if longer (keeps first 3)
    - invalid choices (mid_choice/final_choice not allowed) cleared and status set to PLEASE_SELECT
    """
    arcs = load_arcs()
    chars = discover_characters()
    accounts = load_accounts()
    now = datetime.datetime.now(datetime.timezone.utc)
    changes_made = False

    # check accounts for duplicates and size
    char_to_accounts = defaultdict(list)
    for acc_name, acc in accounts.items():
        for c in acc.characters:
            char_to_accounts[c].append(acc_name)

    # fix duplicates if apply
    for c, acc_list in char_to_accounts.items():
        if len(acc_list) > 1:
            print(f"Character {c} in multiple accounts: {acc_list}")
            if args.apply:
                # keep in first account, remove from others
                first = acc_list[0]
                for other in acc_list[1:]:
                    acc = accounts.get(other)
                    if acc and c in acc.characters:
                        acc.characters.remove(c)
                        path = os.path.join(ACCOUNTS_DIR, f"{other}.txt")
                        write_account(path, acc)
                        print(f"  removed {c} from account {other}")
                        changes_made = True

    # enforce account size
    for acc_name, acc in accounts.items():
        if len(acc.characters) > 3:
            print(f"Account {acc_name} has {len(acc.characters)} characters (max 3)")
            if args.apply:
                acc.characters = acc.characters[:3]
                write_account(os.path.join(ACCOUNTS_DIR, f"{acc_name}.txt"), acc)
                print(f"  truncated account {acc_name} to first 3 characters")
                changes_made = True

    # per-character files validation
    for key, path in chars.items():
        rows = load_character_file(path)
        modified = False
        for r in rows:
            arc = arcs.get(r.arc_key)
            if r.status == 'ON_COOLDOWN' and r.last_delivered_iso:
                dt = parse_iso(r.last_delivered_iso)
                if dt and arc:
                    until = dt + datetime.timedelta(days=arc.cooldown_days)
                    if until <= now:
                        print(f"{key} {r.arc_key}: cooldown expired (was ON_COOLDOWN)")
                        if args.apply:
                            r.status = 'READY_TO_RUN'
                            modified = True
                            changes_made = True
            # validate choices
            if arc:
                if r.mid_choice and r.mid_choice not in arc.allowed:
                    print(f"{key} {r.arc_key}: invalid mid_choice '{r.mid_choice}' for arc allowed={arc.allowed}")
                    if args.apply:
                        r.mid_choice = ''
                        r.status = 'PLEASE_SELECT'
                        modified = True
                        changes_made = True
                if r.final_choice and r.final_choice not in arc.allowed:
                    print(f"{key} {r.arc_key}: invalid final_choice '{r.final_choice}' for arc allowed={arc.allowed}")
                    if args.apply:
                        r.final_choice = ''
                        r.status = 'PLEASE_SELECT'
                        modified = True
                        changes_made = True
            else:
                print(f"{key} {r.arc_key}: arc key not found in epic_arcs.txt")
                # don't auto-remove
        if modified:
            save_character_file(path, rows)

    if args.apply and not changes_made:
        print("verify: no changes needed")
    elif not args.apply:
        print("verify: run with --apply to apply safe fixes")


def cmd_export_csv(args):
    outpath = args.output or '-'
    chars = discover_characters()
    fieldnames = ['player', 'character', 'arc_key', 'status', 'mid_choice', 'final_choice', 'last_delivered_iso', 'notes']
    rows_out = []
    for key, path in chars.items():
        player, character = key.split('/', 1)
        for r in load_character_file(path):
            rows_out.append({
                'player': player,
                'character': character,
                'arc_key': r.arc_key,
                'status': r.status,
                'mid_choice': r.mid_choice,
                'final_choice': r.final_choice,
                'last_delivered_iso': r.last_delivered_iso,
                'notes': r.notes,
            })
    if outpath == '-' or outpath == '/dev/stdout':
        writer = csv.DictWriter(sys.stdout, fieldnames=fieldnames)
        writer.writeheader()
        for r in rows_out:
            writer.writerow(r)
    else:
        with open(outpath, 'w', newline='', encoding='utf-8') as f:
            writer = csv.DictWriter(f, fieldnames=fieldnames)
            writer.writeheader()
            for r in rows_out:
                writer.writerow(r)
        print(f"wrote {len(rows_out)} rows to {outpath}")


def cmd_import_csv(args):
    path = args.input
    if not os.path.exists(path):
        print(f"input file not found: {path}")
        return
    arcs = load_arcs()
    updated_characters = defaultdict(list)
    with open(path, newline='', encoding='utf-8') as f:
        reader = csv.DictReader(f)
        for row in reader:
            player = row.get('player')
            character = row.get('character')
            arc_key = row.get('arc_key')
            status = row.get('status') or 'PLEASE_SELECT'
            mid = (row.get('mid_choice') or '').strip()
            final = (row.get('final_choice') or '').strip()
            last = (row.get('last_delivered_iso') or '').strip()
            notes = row.get('notes') or ''
            if not player or not character or not arc_key:
                print(f"skipping invalid row (missing player/character/arc_key): {row}")
                continue
            arc = arcs.get(arc_key)
            if arc is None:
                print(f"warning: arc {arc_key} not known; skipping row")
                continue
            if mid and mid not in arc.allowed:
                print(f"warning: mid_choice {mid} not allowed for {arc_key}; clearing mid_choice")
                mid = ''
            if final and final not in arc.allowed:
                print(f"warning: final_choice {final} not allowed for {arc_key}; clearing final_choice")
                final = ''
            key = f"{player}/{character}"
            updated_characters[key].append(CharacterArcRow(arc_key, status, mid, final, last, notes))

    # apply updates
    for key, rows in updated_characters.items():
        player, character = key.split('/', 1)
        pdir = os.path.join(PLAYERS_DIR, player)
        os.makedirs(pdir, exist_ok=True)
        path = os.path.join(pdir, f"{character}.txt")
        existing = load_character_file(path)
        # merge by arc_key
        existing_map = {r.arc_key: r for r in existing}
        for r in rows:
            existing_map[r.arc_key] = r
        merged = list(existing_map.values())
        if args.apply:
            save_character_file(path, merged)
            print(f"wrote {len(merged)} rows to {path}")
        else:
            print(f"would write {len(merged)} rows to {path} (use --apply to make changes)")



def build_parser():
    p = argparse.ArgumentParser(description="Epic Arc Tracker CLI")
    sub = p.add_subparsers(dest='cmd')

    sub.add_parser('scan')

    sp = sub.add_parser('show-player')
    sp.add_argument('player')

    sc = sub.add_parser('show-character')
    sc.add_argument('player')
    sc.add_argument('character')

    mm = sub.add_parser('mark-mid-choice')
    mm.add_argument('player')
    mm.add_argument('character')
    mm.add_argument('arc')
    mm.add_argument('faction')

    dl = sub.add_parser('deliver')
    dl.add_argument('player')
    dl.add_argument('character')
    dl.add_argument('arc')
    dl.add_argument('--faction', default=None)

    sa = sub.add_parser('set-account')
    sa.add_argument('name')
    sa.add_argument('type')
    sa.add_argument('characters', help='semi-colon separated player/character entries')
    sa.add_argument('--notes', default='')

    ag = sub.add_parser('aggregates')

    return p


def main(argv=None):
    p = build_parser()
    args = p.parse_args(argv)
    if not args.cmd:
        p.print_help()
        return 1
    if args.cmd == 'scan':
        cmd_scan(args)
    elif args.cmd == 'show-player':
        cmd_show_player(args)
    elif args.cmd == 'show-character':
        cmd_show_character(args)
    elif args.cmd == 'mark-mid-choice':
        cmd_mark_mid_choice(args)
    elif args.cmd == 'deliver':
        cmd_deliver(args)
    elif args.cmd == 'set-account':
        cmd_set_account(args)
    elif args.cmd == 'aggregates':
        cmd_aggregates(args)
    else:
        print("unknown command", args.cmd)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main())
