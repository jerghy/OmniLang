"""WebSocket heartbeat manager — 解决孤儿进程问题 (CRT-1).

浏览器每 5s 发送 WS ping。
连续 3 次未收到 ping（15s）→ 开始优雅退出倒计时（30s）。
倒计时内恢复 → 取消退出。
倒计时超时 → sys.exit(0)。
"""

import threading
import time
import sys


class HeartbeatManager:
    """监听浏览器 WebSocket heartbeat。连续 N 次 missed → 退出。"""

    def __init__(self, max_missed: int = 3, ping_interval: int = 5, shutdown_delay: int = 30):
        self._last_ping: float = time.time()
        self._lock = threading.Lock()
        self._max_missed = max_missed
        self._ping_interval = ping_interval
        self._shutdown_delay = shutdown_delay
        self._monitor_thread: threading.Thread | None = None
        self._running: bool = False
        self._missed_count: int = 0

    @property
    def last_ping(self) -> float:
        with self._lock:
            return self._last_ping

    @property
    def running(self) -> bool:
        return self._running

    def record_ping(self) -> None:
        """收到浏览器 WebSocket ping 时调用。"""
        with self._lock:
            self._last_ping = time.time()

    def start(self) -> None:
        """启动后台监控线程。"""
        self._running = True
        self._monitor_thread = threading.Thread(target=self._monitor_loop, daemon=True)
        self._monitor_thread.start()

    def stop(self) -> None:
        """停止监控。"""
        self._running = False

    def _monitor_loop(self) -> None:
        """后台循环：检查 heartbeat，超时则退出。"""
        missed_count = 0
        while self._running:
            time.sleep(self._ping_interval)
            with self._lock:
                elapsed = time.time() - self._last_ping

            if elapsed > self._ping_interval * 2:
                missed_count += 1
                if missed_count >= self._max_missed:
                    print(
                        f"[omnic serve] No heartbeat for {self._shutdown_delay}s. "
                        "Shutting down...",
                        file=sys.stderr,
                    )
                    time.sleep(self._shutdown_delay)
                    # 最后一次检查是否恢复
                    with self._lock:
                        final_elapsed = time.time() - self._last_ping
                    threshold = self._ping_interval * self._max_missed + self._shutdown_delay
                    if final_elapsed > threshold:
                        sys.exit(0)
                    else:
                        missed_count = 0
            else:
                missed_count = 0
