from flask import Flask, jsonify, request
from flask_socketio import SocketIO
from pathlib import Path
import threading
import time
import config as cfg

FRONTEND_DIR = Path(__file__).parent.parent / "FrontEnd"

_web_data = {
    "match_time": 0.0,
    "phase": "init",
    "score": 0,
    "start_button_pressed": False,
    "stop_button_pressed": False,
    "auton_outcome": "random",
    "bt_color": "field_control",
    "bt_connect": "",
    "ds1" : {
        "team": "",
        "connected": False,
        "estop": False
    },
    "ds2" : {
        "team": "",
        "connected": False,
        "estop": False
    },
    "ds3" : {
        "team": "",
        "connected": False,
        "estop": False
    },
}


app = Flask(__name__, static_folder=str(FRONTEND_DIR), static_url_path='')
socketio = SocketIO(
    app,
    cors_allowed_origins="*",
    async_mode="threading"
)


@app.route('/')
def home():
    return app.send_static_file('home.html')

@app.route('/dashboard')
def dashboard():
    return app.send_static_file('dashboard.html')

@app.route('/scoring')
def scoring():
    return app.send_static_file('scoring.html')

@app.route('/api/status')
def api_status():
    return jsonify(_web_data)


def set_data(data: dict):
    global _web_data
    for key, value in data.items():
        if key in _web_data:
            _web_data[key] = value
    socketio.emit('update', _web_data)

def get_data(key: str = None):
    if not key : return _web_data.copy()
    return _web_data.get(key)

@socketio.on('match_control')
def handle_match_control(data):
    action = data.get('action')
    if action == 'start':
        _web_data['start_button_pressed'] = True
        print("Match START triggered from dashboard")
    elif action == 'stop':
        _web_data['stop_button_pressed'] = True
        print("Match STOP triggered from dashboard")

@socketio.on('update_field')
def handle_update_field(data):
    field = data.get('field')
    value = data.get('value')

    if field in ['auton_outcome','bt_color','bt_connect']:
        _web_data[field] = value
    elif field in ['ds1_team','ds2_team','ds3_team']:
        ds = field.split('_')[0]
        if ds in _web_data:
            _web_data[ds]['team'] = value

    socketio.emit('update', _web_data)
    print(_web_data)



def start_background_updates(interval: float = 0.1):
    def _loop():
        while True:
            socketio.emit('update', _web_data)
            socketio.sleep(interval)
    thread = threading.Thread(target=_loop, daemon=True)
    thread.start()


def start_server(host=cfg.WEB_SERVER_HOST, port=cfg.WEB_SERVER_PORT):
    start_background_updates()
    socketio.run(app, host=host, port=port)
