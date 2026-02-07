const socket = io();

function startMatch() {
    socket.emit('match_control', {action: 'start'});
}

function stopMatch() {
    socket.emit('match_control', {action: 'stop'});
}

socket.on('update', (data) => {
    document.getElementById('match_time').textContent = data.match_time.toFixed(1);
    document.getElementById('phase').textContent = data.phase;
    document.getElementById('score_display').textContent = `${data.score}`;
    document.getElementById('auton_outcome').value = data.auton_outcome || "unknown";

    ['ds1', 'ds2', 'ds3'].forEach(ds => {
        document.getElementById(`${ds}_connected`).textContent = data[ds]?.connected ? "Yes" : "No";
        document.getElementById(`${ds}_estop`).textContent = data[ds]?.estop ? "Yes" : "No";
        if (data[ds]?.team) {
            document.getElementById(`${ds}_team`).value = data[ds].team;
        }
    });

    document.getElementById('radio_status').textContent = data.radio_status || "Disconnected";

    document.getElementById('bt_connected').textContent = data.bt_connected ? "Yes" : "No";
    if (data.bt_color) {
        document.getElementById('bt_color').value = data.bt_color;
    }
});

['ds1_team','ds2_team','ds3_team'].forEach(id => {
    document.getElementById(id).addEventListener('change', (e) => {
        socket.emit('update_field', {field: id, value: e.target.value});
    });
});

document.getElementById('auton_outcome').addEventListener('change', (e) => {
    socket.emit('update_field', {field: 'auton_outcome', value: e.target.value});
});

document.getElementById('bt_color').addEventListener('change', (e) => {
    socket.emit('update_field', {field: 'bt_color', value: e.target.value});
});

document.getElementById('bt_connect').addEventListener('change', (e) => {
    socket.emit('update_field', {field: 'bt_connect', value: e.target.value});
});