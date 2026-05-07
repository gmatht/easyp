#!/usr/bin/env python3
"""CGI wrapper for Epic Arc Tracker

Routes (relative to the script URL):
- /arcs (GET) -> JSON list of arcs
- /players (GET) -> JSON map player -> [characters]
- /accounts (GET) -> JSON map account -> {type, characters, notes}
- /character/<player>/<character> (GET) -> JSON list of rows
- /character/<player>/<character>/mark-mid-choice (POST) -> JSON {arc, faction}
- /character/<player>/<character>/deliver (POST) -> JSON {arc, faction?}
- /aggregates (GET) -> per-arc aggregates JSON

This script uses the repo's epictracker module for core logic.
"""

from __future__ import annotations

import os
import sys
import json

# Ensure repo root on path so we can import epictracker
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
if ROOT not in sys.path:
    sys.path.insert(0, ROOT)

import epictracker as et


RESPONSES = {
    200: 'OK',
    201: 'Created',
    400: 'Bad Request',
    404: 'Not Found',
    500: 'Internal Server Error',
}


def send_json(obj, status=200):
    if status != 200:
        print(f"Status: {status} {RESPONSES.get(status,'')}")
    print("Content-Type: application/json")
    print()
    sys.stdout.write(json.dumps(obj))


def send_error(msg, status=400):
    send_json({'error': msg}, status=status)


def read_json_body():
    try:
        length = int(os.environ.get('CONTENT_LENGTH') or 0)
    except Exception:
        length = 0
    if length:
        data = sys.stdin.read(length)
        try:
            return json.loads(data)
        except Exception:
            return None
    return {}


def handle_get(parts):
    if len(parts) == 0 or parts == ['']:
        return send_json({'message': 'Epic Arc Tracker CGI'})
    if parts[0] == 'arcs':
        arcs = et.load_arcs()
        out = {k: {'name': v.name, 'allowed': v.allowed, 'cooldown_days': v.cooldown_days, 'notes': v.notes} for k, v in arcs.items()}
        return send_json(out)
    if parts[0] == 'players':
        chars = et.discover_characters()
        players = {}
        for key in chars.keys():
            player, character = key.split('/', 1)
            players.setdefault(player, []).append(character)
        return send_json(players)
    if parts[0] == 'accounts':
        accs = et.load_accounts()
        out = {name: {'type': a.type, 'characters': a.characters, 'notes': a.notes} for name, a in accs.items()}
        return send_json(out)
    if parts[0] == 'aggregates':
        # reuse cli aggregates logic
        arcs = et.load_arcs()
        chars = et.discover_characters()
        accounts = et.load_accounts()
        char_to_account_type = {}
        for acc in accounts.values():
            for c in acc.characters:
                char_to_account_type[c] = acc.type
        per_arc = {}
        per_arc_by_account_type = {}
        for key, path in chars.items():
            for r in et.load_character_file(path):
                if r.arc_key not in per_arc:
                    per_arc[r.arc_key] = {'ready_to_deliver': 0, 'ready_to_run': 0, 'need_choice': 0, 'on_cooldown': 0}
                if r.arc_key not in per_arc_by_account_type:
                    per_arc_by_account_type[r.arc_key] = {}
                if r.status == 'READY_TO_COMPLETE':
                    per_arc[r.arc_key]['ready_to_deliver'] += 1
                    atype = char_to_account_type.get(key, 'Unassigned')
                    per_arc_by_account_type[r.arc_key][atype] = per_arc_by_account_type[r.arc_key].get(atype, 0) + 1
                elif r.status == 'READY_TO_RUN':
                    per_arc[r.arc_key]['ready_to_run'] += 1
                elif r.status == 'READY_TO_CHOOSE':
                    per_arc[r.arc_key]['need_choice'] += 1
                elif r.status == 'ON_COOLDOWN':
                    per_arc[r.arc_key]['on_cooldown'] += 1
        return send_json({'per_arc': per_arc, 'by_account_type': per_arc_by_account_type})
    if parts[0] == 'character' and len(parts) >= 3:
        player = parts[1]
        character = parts[2]
        path = et.find_character_file(player, character)
        if not path:
            return send_error('character not found', 404)
        rows = et.load_character_file(path)
        return send_json([r.__dict__ for r in rows])
    return send_error('not found', 404)


def handle_post(parts):
    if parts[0] == 'character' and len(parts) >= 4:
        player = parts[1]
        character = parts[2]
        action = parts[3]
        body = read_json_body()
        if body is None:
            return send_error('invalid JSON body', 400)
        if action == 'mark-mid-choice':
            arc_key = body.get('arc')
            faction = body.get('faction')
            if not arc_key or not faction:
                return send_error('arc and faction required', 400)
            path = et.find_character_file(player, character)
            if not path:
                return send_error('character not found', 404)
            arcs = et.load_arcs()
            arc = arcs.get(arc_key)
            if not arc:
                return send_error('arc not found', 404)
            if faction not in arc.allowed:
                return send_error('faction not allowed', 400)
            rows = et.load_character_file(path)
            changed = False
            for r in rows:
                if r.arc_key == arc_key:
                    r.mid_choice = faction
                    r.status = 'READY_TO_CHOOSE'
                    changed = True
            if not changed:
                rows.append(et.CharacterArcRow(arc_key, 'READY_TO_CHOOSE', faction, '', '', ''))
            et.save_character_file(path, rows)
            return send_json({'ok': True})
        if action == 'deliver':
            arc_key = body.get('arc')
            faction = body.get('faction')
            if not arc_key:
                return send_error('arc required', 400)
            path = et.find_character_file(player, character)
            if not path:
                # create new character file
                pdir = os.path.join(et.PLAYERS_DIR, player)
                os.makedirs(pdir, exist_ok=True)
                path = os.path.join(pdir, f"{character}.txt")
            arcs = et.load_arcs()
            arc = arcs.get(arc_key)
            if not arc:
                return send_error('arc not found', 404)
            rows = et.load_character_file(path)
            found = False
            for r in rows:
                if r.arc_key == arc_key:
                    found = True
                    use = faction or (r.mid_choice if r.mid_choice else None)
                    if use is None:
                        return send_error('no faction specified and no mid_choice present', 400)
                    if use not in arc.allowed:
                        return send_error('chosen faction not allowed', 400)
                    r.final_choice = use
                    r.last_delivered_iso = et.now_utc_iso()
                    r.status = 'ON_COOLDOWN'
            if not found:
                use = faction
                if not use:
                    return send_error('no faction provided for new delivery', 400)
                if use not in arc.allowed:
                    return send_error('chosen faction not allowed', 400)
                rows.append(et.CharacterArcRow(arc_key, 'ON_COOLDOWN', '', use, et.now_utc_iso(), 'delivered'))
            et.save_character_file(path, rows)
            return send_json({'ok': True})
    return send_error('not found', 404)


def main():
    method = os.environ.get('REQUEST_METHOD', 'GET').upper()
    path_info = os.environ.get('PATH_INFO', '') or ''
    parts = [p for p in path_info.strip('/').split('/') if p]
    try:
        if method == 'GET':
            handle_get(parts)
        elif method == 'POST':
            handle_post(parts)
        else:
            send_error('method not allowed', 400)
    except Exception as e:
        send_error(f'internal error: {e}', 500)


if __name__ == '__main__':
    main()
