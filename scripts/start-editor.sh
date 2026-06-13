#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════
# OmniLang Visual Editor — 一键启动脚本
# ═══════════════════════════════════════════════════════════════
# 同时启动 Python 后端 (port 5173) 和 Vite 前端 (port 5174)
# Ctrl+C 一键停止全部
# ═══════════════════════════════════════════════════════════════
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

BACKEND_PORT="${OMNI_BACKEND_PORT:-5173}"
FRONTEND_PORT="${OMNI_FRONTEND_PORT:-5174}"
PROJECT_PATH="${OMNI_PROJECT_DIR:-$PROJECT_DIR/build/test}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

cleanup() {
    echo -e "\n${YELLOW}🛑 正在停止所有服务...${NC}"
    [ -n "${BACKEND_PID:-}" ] && kill "$BACKEND_PID" 2>/dev/null
    [ -n "${FRONTEND_PID:-}" ] && kill "$FRONTEND_PID" 2>/dev/null
    wait 2>/dev/null
    echo -e "${GREEN}✅ 已停止${NC}"
    exit 0
}
trap cleanup SIGINT SIGTERM

# ── 清理旧端口占用 ──
free_port() {
    local port="$1"
    local pid
    pid=$(netstat -ano 2>/dev/null | grep ":$port " | grep LISTENING | awk '{print $NF}' | head -1)
    if [ -n "$pid" ] && [ "$pid" != "0" ]; then
        echo -e "${YELLOW}⚠ 端口 $port 被 PID $pid 占用，正在释放...${NC}"
        powershell -Command "Stop-Process -Id $pid -Force" 2>/dev/null || taskkill //F //PID "$pid" 2>/dev/null || true
        sleep 1
    fi
}

free_port "$BACKEND_PORT"
free_port "$FRONTEND_PORT"

# ── 启动 Python 后端 ──
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}🚀 OmniLang 可视化编辑器${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "  项目目录: ${YELLOW}$PROJECT_PATH${NC}"
echo -e "  后端端口: ${YELLOW}$BACKEND_PORT${NC}"
echo -e "  前端端口: ${YELLOW}$FRONTEND_PORT${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

echo -e "\n${GREEN}▶ 启动后端 (port $BACKEND_PORT)...${NC}"
cd "$PROJECT_DIR"
python -m editor.server.main --dir "$PROJECT_PATH" --port "$BACKEND_PORT" &
BACKEND_PID=$!
sleep 2

if ! kill -0 "$BACKEND_PID" 2>/dev/null; then
    echo -e "${RED}❌ 后端启动失败${NC}"
    exit 1
fi

# ── 启动 Vite 前端 ──
echo -e "${GREEN}▶ 启动前端 (port $FRONTEND_PORT)...${NC}"
cd "$PROJECT_DIR/editor/client"
npx vite --port "$FRONTEND_PORT" --host &
FRONTEND_PID=$!
sleep 3

# ── 验证 ──
echo ""
if kill -0 "$BACKEND_PID" 2>/dev/null && kill -0 "$FRONTEND_PID" 2>/dev/null; then
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}✅ 编辑器已就绪！${NC}"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "  编辑器 (开发): ${CYAN}http://localhost:$FRONTEND_PORT${NC}"
    echo -e "  编辑器 (生产): ${CYAN}http://localhost:$BACKEND_PORT${NC}"
    echo -e "  按 ${RED}Ctrl+C${NC} 停止所有服务"
    echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    wait
else
    echo -e "${RED}❌ 服务启动异常${NC}"
    cleanup
fi
