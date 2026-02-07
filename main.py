import threading
import time

import config as cfg
from Components import (
    webserver,
    HubInterface, LightColor,
    AVController, MatchSound,
    Network, create_tcp_packet, create_udp_packet, GameDataTag, DsMode, DsStation, DsStationStatus
)

threading.Thread(target=lambda: webserver.start_server(), daemon=True).start()
# av = AVController()
# av.open_browser(f"http://localhost:{cfg.WEB_SERVER_PORT}/scoring")
network = Network()
# hub = HubInterface(cfg.HUB_BLE_ADDRESS)

# def hub_connect():
#     import asyncio
#     asyncio.run(hub.connect())
# threading.Thread(target=hub_connect, daemon=True).start()

match_running = False
match_start_time = 0
last_udp_send = 0
game_data_sent = False
period_sound_played = False
winning_alliance = None

def hub_active(elapsed):
    """Return True if hub should be active."""

    # Endgame always active
    if elapsed >= cfg.AUTO_TIME + cfg.TRANSITION_TIME + cfg.TELEOP_TOTAL:
        return True

    teleop_elapsed = elapsed - (cfg.AUTO_TIME + cfg.TRANSITION_TIME)
    if teleop_elapsed < 0:
        return False

    shift = int(teleop_elapsed // cfg.SHIFT_TIME)
    if shift >= cfg.SHIFT_COUNT:
        return False

    # Winning alliance inactive in shift 1
    # alternates each shift
    winner_inactive = (shift % 2 == 0)

    return not winner_inactive

try:
    while True:
        now = time.time()

        # check if match should start
        if not match_running and webserver.get_data("start_button_pressed"):
            match_running = True
            match_start_time = now
            last_udp_send = 0
            game_data_sent = False
            period_sound_played = False
            winning_alliance = None

            webserver.set_data({"phase": "auto", "match_time": cfg.MATCH_TOTAL, "start_button_pressed": False})

            print("Match started")

        # match logic
        if match_running:
            elapsed = now - match_start_time
            remaining = max(0, cfg.MATCH_TOTAL - elapsed)

            # update phase
            new_phase = None
            # phase = webserver.get_data("phase")
            # if elapsed < cfg.AUTO_TIME:
            #     new_phase = "auto"
            # elif elapsed < cfg.AUTO_TIME + cfg.TRANSITION_TIME:
            #     new_phase = "transition"
            # elif elapsed < cfg.AUTO_TIME + cfg.TRANSITION_TIME + cfg.TELEOP_TOTAL:
            #     new_phase = "teleop"
            # elif elapsed < cfg.MATCH_TOTAL:
            #     new_phase = "endgame"
            # else:
            #     new_phase = "ended"
            
            # # play sound on phase change (rising edge detection)
            # if new_phase != phase and phase is not None:
            #     if new_phase == "auto":
            #         av.play_sound(MatchSound.START_MATCH)
            #     elif new_phase == "transition":
            #         av.play_sound(MatchSound.END_AUTON)
            #     elif new_phase == "teleop":
            #         av.play_sound(MatchSound.START_TELEOP)
            #     elif new_phase == "endgame":
            #         av.play_sound(MatchSound.START_ENDGAME)
            
            # phase = new_phase

            webserver.set_data({"phase": "yo mama", "match_time": int(remaining)})

            # # send game data after delay
            # if not game_data_sent and elapsed >= cfg.GAME_DATA_DELAY:
            #     # TODO: determine actual game data to send based on match state and webserver data
            #     game_data_sent = True

            # hub activity
            # if hub_active(elapsed):
            #     await hub.set_lights(LightColor.BLUE)
            # else:
            #     await hub.set_lights(LightColor.OFF)

            # send UDP packet every period
            # if now - last_udp_send >= cfg.UDP_PERIOD:
            #     # send data
            #     last_udp_send = now

        
        # else: # match not running, ensure hub is off, reset hub, and poll webserver data for changes to network and if hub lights need to be tested
        #     if webserver.get_data("bt_color") == LightColor.FIELD_CONTROL:
        #         hub.set_lights(LightColor.OFF)
        #     elif webserver.get_data("bt_color") == LightColor.RED:
        #         hub.set_lights(LightColor.RED)
        #     elif webserver.get_data("bt_color") == LightColor.BLUE:
        #         hub.set_lights(LightColor.BLUE)
        #     elif webserver.get_data("bt_color") == LightColor.WHITE:
        #         hub.set_lights(LightColor.WHITE)

        #     if webserver.get_data("reset_hub"):
        #         import asyncio
        #         asyncio.run(hub.reset_hub())
        #         webserver.set_data({"reset_hub": False})

except KeyboardInterrupt:
    print("Exiting...")