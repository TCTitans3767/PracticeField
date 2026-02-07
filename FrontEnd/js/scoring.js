const socket2 = io();

socket2.on('update', (data) => {
    document.getElementById('score').textContent = data.score;
    document.getElementById('match-time').textContent = data.match_time;
    document.getElementById('mode').textContent = data.phase;
});
