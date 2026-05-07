"""Minimal Flask web UI for Epic Arc Tracker

Provides simple read-only endpoints and actions for mark-mid-choice and deliver via POST.
This is intentionally tiny and uses the same data files as the CLI.
"""
from flask import Flask, jsonify, request, abort
import os
import sys
from pathlib import Path
# ensure repo root is on path so we can import epictracker
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
if ROOT not in sys.path:
    sys.path.insert(0, ROOT)
import epictracker as et
from flask import send_from_directory

app = Flask(__name__)


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
