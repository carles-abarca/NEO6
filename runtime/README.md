# NEO6 Runtime Environment

This directory contains a complete, self-contained NEO6 runtime environment.

## Directory Structure

- `bin/` - Executable binaries (neo6-admin, neo6-proxy)
- `lib/` - Protocol shared libraries (.dylib on macOS, .so on Linux)
- `config/` - Configuration files
  - `admin/` - NEO6 Admin configuration
  - `proxy/` - NEO6 Proxy configuration
- `static/` - Web dashboard static files
- `logs/` - Runtime log files
- `neo6.sh` - Main control script

## Usage

### Start NEO6 Environment
```bash
./neo6.sh start
```

### Stop NEO6 Environment
```bash
./neo6.sh stop
```

### Check Status
```bash
./neo6.sh status
```

### Restart Environment
```bash
./neo6.sh restart
```

## Web Interface

Once started, the NEO6 Admin dashboard will be available at:
- Dashboard: http://localhost:8090
- API: http://localhost:8090/api

## Configuration

- Edit `config/admin/admin.yaml` to modify admin server settings and proxy instances
- Edit `config/proxy/default.toml` and `config/proxy/transactions.yaml` for proxy configuration

## Build Information

- Build type: debug
- Deployed: Fri Jun 13 18:28:26 CST 2025
- Workspace: /Users/carlesabarca/MyProjects/NEO6
