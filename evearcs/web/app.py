"""Minimal Flask web UI for Epic Arc Tracker

Provides simple read-only endpoints and actions for mark-mid-choice and deliver via POST.
This is intentionally tiny and uses the same data files as the CLI.
"""
from flask import Flask, jsonify, request, abort
import os
import sys
from pathlib import Path
import secrets
import re
# ensure repo root is on path so we can import epictracker
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
if ROOT not in sys.path:
    sys.path.insert(0, ROOT)
import epictracker as et
from flask import send_from_directory

app = Flask(__name__)

# simple name validator
NAME_RE = re.compile(r'^[A-Za-z0-9_-]{1,64}$')


def normalize_and_validate_name(raw: str):
    if raw is None:
        return None
    name = str(raw).strip().replace(' ', '_')
    if not name:
        return None
    if not NAME_RE.match(name):
        return None
    return name


def tokens_path():
    return os.path.join(ROOT, 'data', 'secret_tokens.txt')


def load_valid_tokens():
    path = tokens_path()
    tokens = set()
    if os.path.exists(path):
        with open(path, 'r', encoding='utf-8') as f:
            for raw in f:
                line = raw.strip()
                if not line or line.startswith('#'):
                    continue
                parts = [p.strip() for p in line.split('|')]
                if parts:
                    tokens.add(parts[0])
    return tokens


def append_token(token: str, purpose: str, created_by: str):
    path = tokens_path()
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'a', encoding='utf-8') as f:
        f.write(f"{token}|{purpose}|{created_by}\n")


def generate_player_token(player: str, created_by: str) -> str:
    token = secrets.token_urlsafe(18)
    append_token(token, f"player:{player}", created_by)
    return token


@app.get('/api/arcs')
def api_arcs():
    arcs = et.load_arcs()
    return jsonify({k: {'name': v.name, 'allowed': v.allowed, 'cooldown_days': v.cooldown_days} for k, v in arcs.items()})


@app.get('/')
def index():
    static_dir = os.path.join(os.path.dirname(__file__), 'static')
    return send_from_directory(static_dir, 'index.html')


@app.get('/static/<path:path>')
def static_files(path):
    static_dir = os.path.join(os.path.dirname(__file__), 'static')
    return send_from_directory(static_dir, path)


@app.get('/api/players')
def api_players():
    chars = et.discover_characters()
    players = {}
    for key in chars.keys():
        player, character = key.split('/', 1)
        players.setdefault(player, []).append(character)
    return jsonify(players)


@app.post('/api/player/add')
def api_add_player():
    data = request.json or {}
    raw_player = data.get('player')
    admin_token = request.args.get('token')
    valid = load_valid_tokens()
    if not admin_token or admin_token not in valid:
        return jsonify({'error': 'invalid or missing admin token'}), 401
    player = normalize_and_validate_name(raw_player)
    if not player:
        return jsonify({'error': 'invalid player name; use A-Za-z0-9_- (1-64)'}), 400
    pdir = os.path.join(et.PLAYERS_DIR, player)
    os.makedirs(pdir, exist_ok=True)
    ptoken = generate_player_token(player, admin_token)
    return jsonify({'ok': True, 'player': player, 'token': ptoken})


@app.post('/api/character/add')
def api_add_character():
    data = request.json or {}
    raw_player = data.get('player')
    raw_character = data.get('character')
    seed = data.get('seed') or 'none'
    force = bool(data.get('force'))
    admin_token = request.args.get('token')
    valid = load_valid_tokens()
    if not admin_token or admin_token not in valid:
        return jsonify({'error': 'invalid or missing admin token'}), 401
    player = normalize_and_validate_name(raw_player)
    character = normalize_and_validate_name(raw_character)
    if not player or not character:
        return jsonify({'error': 'invalid player or character name'}), 400
    pdir = os.path.join(et.PLAYERS_DIR, player)
    os.makedirs(pdir, exist_ok=True)
    path = os.path.join(pdir, f"{character}.txt")
    if os.path.exists(path) and not force:
        return jsonify({'error': 'character file exists (use force to overwrite)'}), 400
    arcs = et.load_arcs()
    rows = []
    if seed == 'please':
        for k in sorted(arcs.keys()):
            rows.append(et.CharacterArcRow(k, 'PLEASE_SELECT', '', '', '', ''))
    elif seed == 'ready':
        for k in sorted(arcs.keys()):
            rows.append(et.CharacterArcRow(k, 'READY_TO_RUN', '', '', '', ''))
    et.save_character_file(path, rows)
    return jsonify({'ok': True, 'player': player, 'character': character, 'seed': seed})


@app.get('/api/character/<player>/<character>')
def api_character(player, character):
    path = et.find_character_file(player, character)
    if not path:
        abort(404)
    rows = et.load_character_file(path)
    return jsonify([r.__dict__ for r in rows])


@app.post('/api/character/<player>/<character>/mark-mid-choice')
def api_mark_mid(player, character):
    data = request.json or {}
    arc = data.get('arc')
    faction = data.get('faction')
    if not arc or not faction:
        return jsonify({'error': 'arc and faction required'}), 400
    class Args: pass
    args = Args()
    args.player = player
    args.character = character
    args.arc = arc
    args.faction = faction
    et.cmd_mark_mid_choice(args)
    return jsonify({'ok': True})


@app.post('/api/character/<player>/<character>/deliver')
def api_deliver(player, character):
    data = request.json or {}
    arc = data.get('arc')
    faction = data.get('faction')
    if not arc:
        return jsonify({'error': 'arc required'}), 400
    class Args: pass
    args = Args()
    args.player = player
    args.character = character
    args.arc = arc
    args.faction = faction
    et.cmd_deliver(args)
    return jsonify({'ok': True})


if __name__ == '__main__':
    port = int(os.environ.get('PORT', 8000))
    app.run(host='0.0.0.0', port=port)
