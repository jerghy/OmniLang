# OmniLang Editor — Environment Requirements

## Prerequisites

| Component | Minimum Version | Notes |
|-----------|----------------|-------|
| Python | 3.10+ | With `pip` |
| Node.js | 18+ | For Vite dev server (preview) |
| npm | 9+ | For frontend dependencies |
| Git | 2.30+ | For version control of .omni files |

## Verified Platforms

| OS | Status | Notes |
|----|--------|-------|
| Windows 11 | ✅ Verified | Git Bash or WSL2 for shell commands |
| macOS | ⚠️ Untested | Should work, no known issues |
| Linux | ⚠️ Untested | Should work, inotify for file watching |

## Install

```bash
# Python dependencies
pip install -e ".[editor]"

# Frontend dependencies (for development)
cd editor/client && npm install

# Frontend dependencies (production build)
cd editor/client && npm run build
```

## Start

```bash
# Production mode (serves built React app)
omnic serve --dir ./my-project --port 5173

# Development mode (separate Vite + API server)
# Terminal 1: API server
python editor/server/main.py --dir ./my-project --port 8000

# Terminal 2: Vite dev server
cd editor/client && npm run dev  # → http://localhost:5174
# (Vite proxies /api → localhost:8000)
```

## Port Requirements

| Port | Service | Configurable |
|------|---------|-------------|
| 5173 | Production editor (API + static files) | `--port` flag |
| 5174 | Vite dev server (development) | `vite.config.ts` |
| 8000 | API only (when running separate) | `--port` flag |

## File Watching

- **Windows**: Uses `os.path.getmtime` polling (500ms interval) — no inotify required
- **macOS/Linux**: Same polling mechanism — no FSEvents/inotify dependency
- Debounce: 200ms after last file change before recompilation

## Known Platform Issues

### Windows
- Path separators: Use forward slashes in API requests, backslashes handled internally
- Shell: Use Git Bash or WSL2; cmd.exe may have encoding issues
- Long paths: Keep project paths under 260 chars (Windows MAX_PATH default)
- `os.fsync`: Requires writable file handle (`r+b` mode) — handled automatically

### macOS (untested)
- No known issues

### Linux (untested)
- No known issues

## API Health Check

```bash
# Verify server is running
curl http://localhost:5173/api/project
# Expected: {"dir":"/path/to/project","target":"react","mainFile":"src/main.omni"}

# Test compilation
curl -X POST http://localhost:5173/api/compile \
  -H "Content-Type: application/json" \
  -d '{"source":"component Test { view { <Text>Hello</Text> } }","target":"react"}'
# Expected: {"success":true,"files":{"Test.tsx":"..."}}
```

## Smoketest Results (2026-06-12)

11/11 checks passed against `build/smoketest` project:
- Project info ✅
- File listing ✅
- File read/write ✅
- Compile (.omni → .tsx) ✅
- Export (.omni → .omni.json) ✅
- Save/Load .omni.json ✅
- Import (.omni.json → .omni) ✅
- Round-trip (reconstructed compiles) ✅
- Hand-written detection ✅
- @native port extraction ✅
