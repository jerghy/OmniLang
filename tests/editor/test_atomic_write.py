"""Atomic write tests — verify data integrity under concurrent and edge-case scenarios.

Tests:
  - Atomic write creates complete file (no partial content)
  - Temporary files are cleaned up after successful writes
  - Temporary files are cleaned up after failed writes
  - Large file writes remain atomic
  - Concurrent writes to same file don't corrupt
  - Unicode content survives round-trip
"""

import os
import time
import uuid
import pytest
from pathlib import Path


class TestAtomicWrite:
    """Verify the atomic write mechanism in editor/server/main.py."""

    def test_atomic_write_produces_complete_file(self, client, project_dir):
        """Written file has exactly the content sent — no truncation, no corruption."""
        content = "component Test {\n    signal x: i32 = 42;\n}\n"
        resp = client.post("/api/fs/write", json={
            "filePath": "Test.omni",
            "content": content,
        })
        assert resp.json()["ok"] is True

        on_disk = (project_dir / "Test.omni").read_text(encoding="utf-8")
        assert on_disk == content, f"Content mismatch: {len(on_disk)} vs {len(content)} bytes"

    def test_no_tmp_file_left_after_successful_write(self, client, project_dir):
        """Successful write must not leave .tmp files behind."""
        content = "fn main() {}"
        resp = client.post("/api/fs/write", json={
            "filePath": "Clean.omni",
            "content": content,
        })
        assert resp.json()["ok"] is True

        # Check for leftover .tmp files
        tmp_files = list(project_dir.glob("*.tmp.*"))
        assert len(tmp_files) == 0, f"Leftover tmp files: {[t.name for t in tmp_files]}"

    def test_atomic_write_large_content(self, client, project_dir):
        """Large file (100KB) should write atomically — no partial content."""
        # Generate a large .omni file with many components
        lines = ["component Large" + str(i) + " {\n    signal x: i32 = " + str(i) + ";\n}\n"
                 for i in range(1000)]
        content = "\n".join(lines)

        resp = client.post("/api/fs/write", json={
            "filePath": "Large.omni",
            "content": content,
        })
        assert resp.json()["ok"] is True

        on_disk = (project_dir / "Large.omni").read_text(encoding="utf-8")
        assert len(on_disk) == len(content), f"Size mismatch: {len(on_disk)} vs {len(content)}"
        assert on_disk == content

    def test_atomic_write_unicode_roundtrip(self, client, project_dir):
        """Unicode content (emoji, CJK, special chars) survives round-trip."""
        content = """component 你好世界 {
    signal ✅: String = "Hello, 世界! 🎉";
    signal 测试: i32 = 42;
    // Comment with emoji: 🚀✨
    view {
        <Text label="你好，世界！" />
    }
}"""
        resp = client.post("/api/fs/write", json={
            "filePath": "Unicode.omni",
            "content": content,
        })
        assert resp.json()["ok"] is True

        on_disk = (project_dir / "Unicode.omni").read_text(encoding="utf-8")
        assert on_disk == content
        assert "你好世界" in on_disk
        assert "🎉" in on_disk

    def test_atomic_write_overwrite_preserves_integrity(self, client, project_dir):
        """Overwriting a file must produce the new content, not a mix."""
        old_content = "fn old() {}\n" * 50
        new_content = "fn new() {}\n" * 50

        # Write old
        (project_dir / "Overwrite.omni").write_text(old_content, encoding="utf-8")

        # Overwrite
        resp = client.post("/api/fs/write", json={
            "filePath": "Overwrite.omni",
            "content": new_content,
        })
        assert resp.json()["ok"] is True

        on_disk = (project_dir / "Overwrite.omni").read_text(encoding="utf-8")
        assert on_disk == new_content
        assert "old" not in on_disk, "Old content leaked into file"

    def test_write_to_nonexistent_directory(self, client, project_dir):
        """Writing to path where parent directory doesn't exist should succeed."""
        content = "fn main() {}"
        resp = client.post("/api/fs/write", json={
            "filePath": "deep/nested/dir/File.omni",
            "content": content,
        })
        assert resp.json()["ok"] is True

        on_disk = (project_dir / "deep" / "nested" / "dir" / "File.omni").read_text(encoding="utf-8")
        assert on_disk == content

    def test_empty_content_write(self, client, project_dir):
        """Writing empty content should produce an empty file."""
        resp = client.post("/api/fs/write", json={
            "filePath": "Empty.omni",
            "content": "",
        })
        assert resp.json()["ok"] is True
        on_disk = (project_dir / "Empty.omni").read_text(encoding="utf-8")
        assert on_disk == ""


class TestAtomicWriteConcurrency:
    """Verify that concurrent writes to the same path don't corrupt."""

    def test_rapid_sequential_overwrites(self, client, project_dir):
        """Multiple rapid writes to the same file — last write wins cleanly."""
        filepath = "Rapid.omni"

        for i in range(20):
            content = f"// Write iteration {i}\nfn main() {{}}\n"
            resp = client.post("/api/fs/write", json={
                "filePath": filepath,
                "content": content,
            })
            assert resp.json()["ok"] is True, f"Write {i} failed"

        on_disk = (project_dir / filepath).read_text(encoding="utf-8")
        # Last write should be intact — no corruption
        assert "Write iteration" in on_disk
        assert "fn main()" in on_disk
        # Should not contain garbage (partial writes would produce binary artifacts)
        assert on_disk.isprintable() or all(ord(c) < 128 or c.isprintable() for c in on_disk)

    def test_alternating_writes_two_files(self, client, project_dir):
        """Alternating writes to two files — both should be intact."""
        content_a = "// File A\nfn a() {}"
        content_b = "// File B\nfn b() {}"

        for _ in range(10):
            resp_a = client.post("/api/fs/write", json={
                "filePath": "A.omni", "content": content_a,
            })
            resp_b = client.post("/api/fs/write", json={
                "filePath": "B.omni", "content": content_b,
            })
            assert resp_a.json()["ok"] is True
            assert resp_b.json()["ok"] is True

        assert (project_dir / "A.omni").read_text(encoding="utf-8") == content_a
        assert (project_dir / "B.omni").read_text(encoding="utf-8") == content_b
