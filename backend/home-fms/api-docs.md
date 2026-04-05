### POST `/safety/estop`
**Purpose:** Trigger E-Stop for a specific station or all stations.
**Request Body:**
```json
{
  "color-number": "optional, if not provided, E-Stop all stations"
}
```
**Response:**
```json
{
  "status": "success",
  "message": "E-Stop triggered for station X"
}
```

### POST `/safety/astop`
**Purpose:** Trigger A-Stop for a specific station or all stations.
**Request Body:**
```json
{
  "color-number": "required, specifies the station to trigger A-Stop for"
}
```
**Response:**
```json
{
  "status": "success",
  "message": "A-Stop triggered for station X"
}
```

### GET `/stations/pins`
**Purpose:** Retrieve the PIN codes for all stations.
**Response:**
```json
{
  "status": "success",
  "pins": {
    "station_1": "1234",
    "station_2": "5678",
    "station_3": "9012",
    "station_4": "3456"
  }
}
```

### POST `/stations/pins`
**Purpose:** Update the PIN codes for all stations.
**Request Body:**
```json
{
  "pins": {
    "station_1": "1234",
    "station_2": "5678",
    "station_3": "9012",
    "station_4": "3456"
  }
}
```
**Response:**
```json
{
  "status": "success",
  "message": "PIN codes updated successfully"
}
```

### GET `/stations/status`
**Purpose:** Retrieve the status of all stations.
**Response:**
```json
{
  "status": "success",
  "stations": {
    "station_1": {
      "network_status": "online",
      "robot_status": "active",
      "ds_connection_status": "connected",
      "estop_status": "inactive",
      "astop_status": "inactive",
      "team_number": 1
    },
    "station_2": {
      "network_status": "online",
      "robot_status": "active",
      "ds_connection_status": "connected",
      "estop_status": "inactive",
      "astop_status": "inactive",
      "team_number": 2
    },
    "station_3": {
      "network_status": "online",
      "robot_status": "active",
      "ds_connection_status": "connected",
      "estop_status": "inactive",
      "astop_status": "inactive",
      "team_number": 3
    },
    "station_4": {
      "network_status": "online",
      "robot_status": "active",
      "ds_connection_status": "connected",
      "estop_status": "inactive",
      "astop_status": "inactive",
      "team_number": 4
    }
  }
}
```

### POST `/match/add`
**Purpose:** Add a new match to the queue.
**Request Body:**
```json
{
  "team_number": {
    "blue1": 1,
    "blue2": 2,
    "blue3": 3,
    "red1": 4,
    "red2": 5,
    "red3": 6
  }
}
```
**Response:**
```json
{
  "status": "success",
  "message": "Match added successfully"
}
```

### POST `/match/prestart`
**Purpose:** Prestart a match.
**Request Body:**
```json
{
  "prestart": "required body"
}
```
**Response:**
```json
{
  "status": "success",
  "message": "Match prestarted successfully"
}
```


### POST `/match/start`
**Purpose:** Start a match.
**Request Body:**
```json
{
  "start": "required body"
}
```
**Response:**
```json
{
  "status": "success",
  "message": "Match started successfully"
}
```

### POST `/match/stop`
**Purpose:** Stop a match.
**Request Body:**
```json
{
  "stop": "required body"
}
```
**Response:**
```json
{
  "status": "success",
  "message": "Match stopped successfully"
}
```

### POST `/match/load_next`
**Purpose:** Load the next match.
**Request Body:**
```json
{
  "load_next": "required body"
}
```
**Response:**
```json
{
  "status": "success",
  "message": "Next match loaded successfully"
}
```

### POST `/match/bypass`
**Purpose:** Bypass a match.
**Request Body:**
```json
{
  "station_number": "required, specifies the station to bypass"
}
```
**Response:**
```json
{
  "status": "success",
  "message": "Station + station_number + bypassed successfully"
}
```

### GET `/match/time_left`
**Purpose:** Retrieve the remaining time for the current match.
**Response:**
```json
{
  "status": "success",
  "time_left": "remaining time in seconds"
}
```

### GET `/network/ap/status`
**Purpose:** Retrieve the status of the robot ap.
**Response:**
```json
{
  "status": "success",
  "ap_status": "ok (everything good), warn (configuring), error (not detected)"
}
```

### GET `/network/blue_hub/status`
**Purpose:** Retrieve the status of the blue hub.
**Response:**
```json
{
  "status": "success",
  "blue_hub_status": "ok (everything good), warn (disabled), error (not detected)"
}
```

### GET `/network/red_hub/status`
**Purpose:** Retrieve the status of the red hub.
**Response:**
```json
{
  "status": "success",
  "red_hub_status": "ok (everything good), warn (disabled), error (not detected)"
}
```

### 