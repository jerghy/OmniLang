#!/bin/bash
set -e
node "$(dirname "$0")/test_sanitizers.js"
node "$(dirname "$0")/test_signals.js"
echo "All passed"
