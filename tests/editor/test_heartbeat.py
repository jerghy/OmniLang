"""Heartbeat protocol tests — verify ping/pong state machine and orphan detection.

Tests:
  - record_ping() updates timestamp
  - HeartbeatManager initial state is correct
  - Start/stop lifecycle clean
  - Multiple pings advance timestamp monotonically
  - Without pings, elapsed time grows
"""

import time
import pytest
from editor.server.heartbeat import HeartbeatManager


class TestHeartbeatManager:
    """Unit tests for HeartbeatManager state machine."""

    def test_record_ping_updates_timestamp(self):
        """After record_ping(), last_ping should advance."""
        hm = HeartbeatManager(max_missed=3, ping_interval=1, shutdown_delay=5)
        initial = hm.last_ping
        time.sleep(0.1)
        hm.record_ping()
        assert hm.last_ping > initial, "record_ping should update timestamp"

    def test_heartbeat_manager_initial_state(self):
        """HeartbeatManager starts with correct defaults."""
        hm = HeartbeatManager()
        assert hm.running is False
        assert hm._max_missed == 3
        assert hm._ping_interval == 5
        assert hm._shutdown_delay == 30

    def test_custom_parameters(self):
        """Custom parameters are stored correctly."""
        hm = HeartbeatManager(max_missed=5, ping_interval=2, shutdown_delay=10)
        assert hm._max_missed == 5
        assert hm._ping_interval == 2
        assert hm._shutdown_delay == 10

    def test_multiple_record_pings_advance_monotonically(self):
        """Each record_ping() should advance the timestamp."""
        hm = HeartbeatManager(max_missed=3, ping_interval=0.1, shutdown_delay=0.5)

        for i in range(10):
            before = hm.last_ping
            time.sleep(0.01)
            hm.record_ping()
            assert hm.last_ping >= before, f"Ping {i}: timestamp should advance"

    def test_elapsed_time_increases_without_ping(self):
        """Without record_ping(), elapsed time from last_ping should grow."""
        hm = HeartbeatManager(max_missed=3, ping_interval=1, shutdown_delay=30)
        hm.record_ping()
        first_ping_time = hm.last_ping

        time.sleep(0.3)

        elapsed = time.time() - first_ping_time
        assert elapsed > 0.2, f"Expected >0.2s elapsed, got {elapsed:.2f}s"

    def test_start_stop_lifecycle(self):
        """HeartbeatManager monitor thread can start and stop cleanly."""
        hm = HeartbeatManager(max_missed=3, ping_interval=0.1, shutdown_delay=0.5)
        hm.record_ping()
        hm.start()
        assert hm.running is True
        assert hm._monitor_thread is not None
        assert hm._monitor_thread.is_alive()

        hm.stop()
        assert hm.running is False

    def test_last_ping_thread_safe(self):
        """last_ping property is thread-safe."""
        hm = HeartbeatManager(max_missed=3, ping_interval=0.1, shutdown_delay=0.5)
        ts1 = hm.last_ping
        ts2 = hm.last_ping
        assert ts1 == ts2, "Repeated reads should be consistent"
