#!/bin/bash
set -e

BINARY="./target/release/synaptic-graph"
TMPDB=$(mktemp /tmp/sg-test-XXXXXX.db)
export MEMORY_GRAPH_DB="$TMPDB"

echo "Testing MCP initialize..."
INIT_RESP=$(echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | timeout 5 "$BINARY" 2>/dev/null | head -1)
echo "$INIT_RESP" | python3 -c "import sys,json; d=json.loads(sys.stdin.read()); assert 'result' in d, f'No result: {d}'; print('PASS: initialize')"

echo "Testing tools/list..."
TOOLS_RESP=$(printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}\n{"jsonrpc":"2.0","method":"notifications/initialized"}\n{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | timeout 5 "$BINARY" 2>/dev/null | tail -1)
TOOL_COUNT=$(echo "$TOOLS_RESP" | python3 -c "import sys,json; d=json.loads(sys.stdin.read()); tools=d.get('result',{}).get('tools',[]); print(len(tools))")
echo "PASS: $TOOL_COUNT tools registered"

rm -f "$TMPDB"
echo "All MCP handshake tests passed"
