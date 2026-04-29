#!/bin/sh
CC="${1:-./build/cc2}"
echo "=== vidhana tests ==="
cat src/main.cyr | "$CC" > /tmp/vidhana_test && chmod +x /tmp/vidhana_test && /tmp/vidhana_test
echo "exit: $?"
rm -f /tmp/vidhana_test
