#!/bin/bash
# start-editor.sh — 一键启动 OmniLang Editor（后端 + 前端）
# 用法: bash start-editor.sh
# 输出: 最后一行是实际 Vite URL，其余是日志（被重定向到文件）

set -e

ROOT="/d/space/OmniLang"
SERVER_LOG="/tmp/editor_server.log"
VITE_LOG="/tmp/editor_vite.log"
VITE_DIR="$ROOT/editor/client"
PROJECT_DIR="$ROOT/build/test"

echo "=== [1/5] 检查依赖 ==="

# 检查 Python deps
python -c "import fastapi; import uvicorn" 2>/dev/null || {
  echo "FAIL: Python deps missing, run: pip install fastapi uvicorn"
  exit 1
}

# 检查 npm deps
[ -d "$VITE_DIR/node_modules" ] || {
  echo "FAIL: node_modules missing, run: cd $VITE_DIR && npm install"
  exit 1
}

# 检查项目目录
[ -d "$PROJECT_DIR" ] || {
  echo "FAIL: project dir $PROJECT_DIR not found"
  exit 1
}

echo "=== [2/5] 清理旧进程 ==="
# 尝试杀掉端口 8000 上的旧进程（Windows）
netstat -ano 2>/dev/null | grep ":8000" | grep "LISTENING" | awk '{print $5}' | xargs -r taskkill /F 2>/dev/null || true
sleep 1

echo "=== [3/5] 启动后端 (port 8000) ==="
cd "$ROOT"
# Convert POSIX path (e.g., /d/space/...) to Windows path (D:/space/...) for Python
WIN_DIR=$(cygpath -w "$PROJECT_DIR" 2>/dev/null || echo "$PROJECT_DIR")
PYTHONPATH=. python -m editor.server.main --dir "$WIN_DIR" --port 8000 > "$SERVER_LOG" 2>&1 &
BACKEND_PID=$!

# 等后端就绪（最长 10 秒）
for i in 1 2 3 4 5; do
  sleep 2
  curl -s -o /dev/null -w "" http://localhost:8000/api/project 2>/dev/null && break
done

# 验证后端存活
curl -s -o /dev/null -w "%{http_code}" http://localhost:8000/api/project 2>/dev/null | grep -q "200" || {
  echo "FAIL: Backend did not start. Check $SERVER_LOG"
  exit 1
}
echo "backend ok"

echo "=== [4/5] 启动前端 ==="
cd "$VITE_DIR"
npm run dev > "$VITE_LOG" 2>&1 &
FRONTEND_PID=$!

# 等 Vite 启动并提取实际端口（最长 15 秒）
VITE_URL=""
for i in 1 2 3 4 5 6 7; do
  sleep 2
  VITE_URL=$(sed 's/\x1b\[[0-9;]*[a-zA-Z]//g' "$VITE_LOG" 2>/dev/null | grep -oE 'http://localhost:[0-9]+' | tail -1)
  [ -n "$VITE_URL" ] && break
done

if [ -z "$VITE_URL" ]; then
  echo "FAIL: Vite did not start. Check $VITE_LOG"
  exit 1
fi

echo "frontend ok"

echo "=== [5/5] 验证前端 ==="
curl -s -o /dev/null -w "" "$VITE_URL" 2>/dev/null || {
  echo "FAIL: Frontend not reachable at $VITE_URL"
  exit 1
}

echo "ready"
echo "$VITE_URL"
