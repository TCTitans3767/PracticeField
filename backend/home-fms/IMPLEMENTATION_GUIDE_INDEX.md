# Home Practice Field FMS - Implementation Guide Index

Welcome to the comprehensive implementation guide for the Home Practice Field FMS (Field Management System). This guide covers the complete development of a FIRST Robotics Competition FMS system from a crash-prone prototype to a production-grade system with safety features, API endpoints, and telemetry tracking.

---

## 📋 Document Overview

This directory contains detailed implementation guides for all 5 phases of the FMS development plan:

### **Phase 1: Codebase Cleanup & Architecture** (2.5 hours)
📄 **[PHASE1_IMPLEMENTATION.md](PHASE1_IMPLEMENTATION.md)**

Focus: Eliminate panics, improve error handling, complete protocol implementation

**Key Deliverables:**
- Safe vector access helpers (no more panics on malformed packets)
- Complete TCP protocol implementation (Version, ErrorEventData handlers)
- Structured logging infrastructure with tracing
- Connection timeout mechanisms
- Graceful error propagation

**Status:** Ready to start immediately

---

### **Phase 2: Safety Infrastructure** (2-2.5 hours)
📄 **[PHASE2_IMPLEMENTATION.md](PHASE2_IMPLEMENTATION.md)**

Focus: Emergency stop mechanisms, stale command prevention, state machine enforcement

**Key Deliverables:**
- Per-robot e-stop system
- Global e-stop (disables all robots immediately)
- Heartbeat monitoring (disconnect detection)
- Robot state machine with valid transitions
- Graceful degradation on subsystem failures

**Status:** Depends on Phase 1

---

### **Phase 3: Core API Features** (2-2.5 hours)
📄 **[PHASE3_IMPLEMENTATION.md](PHASE3_IMPLEMENTATION.md)**

Focus: REST endpoints for frontend integration

**Key Deliverables:**
- Robot management endpoints (register, list, status, disconnect)
- Match control endpoints (start, stop, pause, resume)
- E-stop API endpoints
- Real-time telemetry monitoring
- Consistent JSON responses with error handling

**Endpoints Created:**
- `GET    /api/robots`
- `POST   /api/robots/{team}/register`
- `GET    /api/robots/{team}/status`
- `POST   /api/match/start`
- `POST   /api/match/stop`
- `POST   /api/robots/{team}/estop`
- `GET    /api/telemetry/live`

**Status:** Depends on Phase 2

---

### **Phase 4: Telemetry & Logging** (2 hours)
📄 **[PHASE4_IMPLEMENTATION.md](PHASE4_IMPLEMENTATION.md)**

Focus: Event tracking, historical data, observability

**Key Deliverables:**
- Event logging system (connections, matches, e-stops, errors)
- Per-robot telemetry collection (battery, modes, errors)
- SQLite database for persistent storage
- Telemetry history API with CSV export
- Enhanced structured logging with file output

**Endpoints Created:**
- `GET    /api/telemetry/history`
- `GET    /api/events/history`

**Status:** Depends on Phase 3

---

### **Phase 5: Field Radio Integration** (1.5 hours, Optional)
📄 **[PHASE5_IMPLEMENTATION.md](PHASE5_IMPLEMENTATION.md)**

Focus: Automatic team network configuration on field radio

**Key Deliverables:**
- Radio client for remote configuration
- Sync enabled teams to radio at match start
- Disable teams at match stop
- Radio status monitoring endpoint
- Graceful degradation if radio unavailable

**Status:** Optional enhancement, depends on Phase 3

---

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ (with Cargo)
- Understanding of async Rust (tokio)
- Basic networking knowledge (TCP/UDP/HTTP)

### Installation & First Build

```bash
# Navigate to project directory
cd C:\Users\jc\PracticeField\backend\home-fms

# Build the project
cargo build --release

# Run the server
cargo run

# The server will start on:
# - TCP: localhost:8080 (for driverstations)
# - HTTP: localhost:2000 (for API)
```

### Running Tests & Lint Checks

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run linter
cargo clippy --all-targets

# Build without warnings
cargo build --release
```

---

## 📊 Implementation Roadmap

### Recommended Execution Order

```
Phase 1: Codebase Cleanup (Do First!)
    ├─ P1.1: Error Handling [30min] ← START HERE
    ├─ P1.2: TCP Protocol [30min]
    ├─ P1.3: UDP Reliability [20min]
    ├─ P1.4: Code Structure [25min]
    └─ P1.5: Testing CI [20min]

Phase 2: Safety Infrastructure (After Phase 1)
    ├─ P2.1: E-Stop System [40min]
    ├─ P2.2: Stale Command Prevention [30min]
    ├─ P2.3: State Machine [25min]
    ├─ P2.4: Command Validation [15min]
    └─ P2.5: Graceful Degradation [20min]

Phase 3: API Features (After Phase 2)
    ├─ P3.1: API Framework [20min]
    ├─ P3.2: Robot Management [30min]
    ├─ P3.3: Match Control [25min]
    ├─ P3.4: E-Stop API [15min] ← Already done in Phase 2
    └─ P3.5: Real-Time Monitoring [20min]

Phase 4: Telemetry (After Phase 3)
    ├─ P4.1: Event Logging [25min]
    ├─ P4.2: Telemetry Collection [20min]
    ├─ P4.3: SQLite Persistence [20min]
    ├─ P4.4: History API [20min]
    └─ P4.5: Logging Infrastructure [15min]

Phase 5: Field Radio (Optional, After Phase 3)
    ├─ P5.1: Radio Configuration [40min]
    └─ P5.2: State Sync [20min]
```

**Total Estimated Time:** 9-10 hours

---

## 📁 Project Structure

```
home-fms/
├── src/
│   ├── main.rs                          # Entry point, HTTP server setup
│   ├── db.rs                            # SQLite database (Phase 4)
│   ├── logging.rs                       # Log rotation (Phase 4)
│   └── driverstation_comms/
│       ├── mod.rs                       # Module exports
│       ├── fms.rs                       # FMS core state
│       ├── tcp.rs                       # TCP protocol parsing
│       ├── udp.rs                       # UDP packet creation
│       ├── driverstation_connection.rs  # Per-driverstation logic
│       ├── events.rs                    # Event definitions (Phase 4)
│       ├── monitor.rs                   # Connection monitoring (Phase 2)
│       └── radio.rs                     # Field radio integration (Phase 5)
├── Cargo.toml                           # Rust dependencies
├── Cargo.lock                           # Locked dependency versions
├── PHASE1_IMPLEMENTATION.md             # Detailed Phase 1 guide
├── PHASE2_IMPLEMENTATION.md             # Detailed Phase 2 guide
├── PHASE3_IMPLEMENTATION.md             # Detailed Phase 3 guide
├── PHASE4_IMPLEMENTATION.md             # Detailed Phase 4 guide
├── PHASE5_IMPLEMENTATION.md             # Detailed Phase 5 guide
├── copilot-instructions.md              # AI assistant instructions
└── fms_telemetry.db                     # SQLite database (created at runtime)
```

---

## 🎯 Key Features by Phase

| Feature | Phase | Status |
|---------|-------|--------|
| Safe TCP parsing | 1 | ✅ Foundation |
| Error propagation | 1 | ✅ Foundation |
| Structured logging | 1 | ✅ Foundation |
| Connection timeouts | 1 | ✅ Foundation |
| Per-robot e-stop | 2 | ⚠️ Depends on Phase 1 |
| Global e-stop | 2 | ⚠️ Depends on Phase 1 |
| Stale command prevention | 2 | ⚠️ Depends on Phase 1 |
| State machine | 2 | ⚠️ Depends on Phase 1 |
| Robot REST API | 3 | ⚠️ Depends on Phase 2 |
| Match control API | 3 | ⚠️ Depends on Phase 2 |
| Live telemetry API | 3 | ⚠️ Depends on Phase 2 |
| Event logging | 4 | ⚠️ Depends on Phase 3 |
| Telemetry persistence | 4 | ⚠️ Depends on Phase 3 |
| History API | 4 | ⚠️ Depends on Phase 3 |
| Radio integration | 5 | 🔲 Optional |

---

## 🔍 API Reference Quick Links

### Health & Status
```
GET  /status                               # Server health check
GET  /api/estop/status                     # E-stop status
GET  /api/match/status                     # Match status
GET  /api/radio/status                     # Radio status (Phase 5)
```

### Robot Management
```
GET    /api/robots                         # List all robots
POST   /api/robots/{team}/register         # Register team
POST   /api/robots/{team}/deregister       # Unregister team
GET    /api/robots/{team}/status           # Get robot status
DELETE /api/robots/{team}                  # Force disconnect
```

### Match Control
```
POST /api/match/start                      # Start match (enable robots)
POST /api/match/stop                       # Stop match (disable robots)
POST /api/match/pause                      # Pause match
POST /api/match/resume                     # Resume paused match
```

### E-Stop Control
```
POST /api/robots/{team}/estop              # E-stop individual robot
POST /api/robots/{team}/estop/clear        # Clear robot e-stop
POST /api/estop                            # Global e-stop (all robots)
POST /api/estop/clear                      # Clear global e-stop
```

### Telemetry & Monitoring
```
GET /api/telemetry/live                    # Live robot status
GET /api/telemetry/history                 # Historical telemetry
GET /api/events/history                    # Event logs
```

---

## 🛠️ Development Workflow

### For Each Phase

1. **Read the phase guide** - Understand the objectives and architecture
2. **Review code examples** - Study before/after code snippets
3. **Implement changes** - Follow the step-by-step instructions
4. **Build and check** - Run `cargo build --release && cargo clippy`
5. **Manual testing** - Test endpoints and functionality
6. **Commit changes** - Include meaningful commit messages
7. **Move to next phase** - Only after current phase is complete

### Testing During Development

```bash
# After each major change
cargo build                    # Compile check
cargo clippy --all-targets     # Lint check
cargo fmt --check              # Format check

# Manual testing
cargo run                      # Start server

# In another terminal
curl http://localhost:2000/status  # Test HTTP
# Send TCP packets via driverstation or telnet
```

---

## 🔐 Safety Checklist

Before deploying to production, verify:

- [ ] Phase 1 complete - no panics on malformed input
- [ ] Phase 2 complete - e-stops functional and tested
- [ ] All connections have timeouts (no zombie connections)
- [ ] Global e-stop immediately disables all robots
- [ ] Stale commands never sent to disconnected robots
- [ ] All errors are logged with context
- [ ] Database works and persists data
- [ ] Telemetry collection is working
- [ ] API responses are in correct format
- [ ] No hardcoded credentials or secrets

---

## 🐛 Common Issues & Solutions

### Compilation Errors

**Issue:** `error: failed to resolve: use of undeclared crate`
- **Solution:** Run `cargo update` and check `Cargo.toml` dependencies

**Issue:** `error: unused variable`
- **Solution:** Add `#[allow(unused)]` or use the variable
- Remove after Phase 1 lint check

### Runtime Issues

**Issue:** `TCP listener failed to bind`
- **Solution:** Port 8080 already in use. Change port or kill existing process
- Check with: `lsof -i :8080` (Linux/Mac) or `netstat -ano | findstr :8080` (Windows)

**Issue:** `FMS lock poisoned` errors
- **Solution:** Panic occurred while holding the lock. Fix the panic source.
- Use Phase 1 safe vector access helpers

**Issue:** Database file locked
- **Solution:** Another process has the database. Close or kill.
- Or specify different database path via environment variable

---

## 📖 Learning Resources

### Rust Async Programming
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [async/await basics](https://rust-lang.github.io/async-book/)

### Actix-web Framework
- [Actix documentation](https://actix.rs/)
- [JSON handling](https://actix.rs/docs/extractors/#json)

### FIRST Robotics Protocols
- Ask your local team leads for protocol specifications
- Examine commented test code in `main.rs`

---

## 📞 Support & Questions

If you encounter issues not covered in the guides:

1. **Check error messages carefully** - They usually indicate the problem
2. **Look at the before/after code examples** - Compare with your changes
3. **Review the relevant phase guide** - Re-read the section
4. **Check the copilot-instructions.md** - Contains additional context
5. **Look at git history** - See how changes were made

---

## 📝 Implementation Tips

### General
- **Small commits** - Commit after each small working unit (not all at once)
- **Test incrementally** - Don't implement entire phase before testing
- **Read the code** - Understand what you're changing
- **Keep changes focused** - Don't refactor unrelated code

### For Each Phase
- **Phase 1:** Focus on robustness - test with malformed packets
- **Phase 2:** Test safety features thoroughly - e-stops, timeouts, state transitions
- **Phase 3:** Test API responses with curl - verify JSON format
- **Phase 4:** Verify database persistence - restart and check data is there
- **Phase 5:** Only do if you have radio hardware

---

## 🎓 Success Criteria

After completing all 5 phases:

✅ Server never panics on malformed input  
✅ E-stops work and disable robots immediately  
✅ No stale commands sent to disconnected robots  
✅ All errors logged with context  
✅ REST API endpoints all functional  
✅ Telemetry collected and persisted  
✅ Frontend can fully control FMS via API  
✅ Teams can access historical data for analysis  
✅ System runs continuously without manual intervention  
✅ Code is maintainable and well-documented  

---

## 📄 Document Convention

Throughout these guides, you'll see code formatted as follows:

**BEFORE:** Current (buggy) code
```rust
// The problem code
```

**AFTER:** Fixed code
```rust
// The solution
```

**Step 1, 2, 3...** Implementation steps

**Testing:** How to verify the fix works

---

## 🎯 Next Steps

1. **Read Phase 1** → [PHASE1_IMPLEMENTATION.md](PHASE1_IMPLEMENTATION.md)
2. **Start with P1.1** → Safe vector access (highest impact fix)
3. **Build and verify** → `cargo build --release && cargo clippy`
4. **Test manually** → Send malformed packets, verify no crash
5. **Move through phases** → In order, testing each before next

---

## Version Information

- **Project:** home-fms (Home Practice Field FMS)
- **Version:** 0.1.0
- **Rust Edition:** 2021
- **Status:** Development (Phase 1 starting point)

---

Happy coding! 🚀

