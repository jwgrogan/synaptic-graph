#!/bin/bash
set -e

echo "synaptic-graph installer"
echo "========================"

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo not found. Install from https://rustup.rs"
    exit 1
fi

# Build
echo "Building synaptic-graph..."
cargo build --release

BINARY="$(pwd)/target/release/synaptic-graph"
echo "Binary: $BINARY"

# Detect platform and configure
configure_claude_code() {
    echo ""
    echo "Configuring Claude Code..."

    # Create .mcp.json in home directory
    MCP_FILE="$HOME/.mcp.json"
    if [ -f "$MCP_FILE" ]; then
        # Add to existing config using python
        python3 -c "
import json
with open('$MCP_FILE') as f:
    config = json.load(f)
config.setdefault('mcpServers', {})['synaptic-graph'] = {'command': '$BINARY', 'args': []}
with open('$MCP_FILE', 'w') as f:
    json.dump(config, f, indent=2)
print('Updated $MCP_FILE')
"
    else
        cat > "$MCP_FILE" << MCPEOF
{
  "mcpServers": {
    "synaptic-graph": {
      "command": "$BINARY",
      "args": []
    }
  }
}
MCPEOF
        echo "Created $MCP_FILE"
    fi
    echo "✓ Claude Code configured. Restart Claude Code to activate."
}

configure_claude_desktop() {
    echo ""
    echo "Configuring Claude Desktop..."

    CONFIG_FILE="$HOME/Library/Application Support/Claude/claude_desktop_config.json"
    if [ ! -f "$CONFIG_FILE" ]; then
        CONFIG_FILE="$HOME/.config/Claude/claude_desktop_config.json"
    fi

    if [ -f "$CONFIG_FILE" ]; then
        python3 -c "
import json
with open('$CONFIG_FILE') as f:
    config = json.load(f)
config.setdefault('mcpServers', {})['synaptic-graph'] = {'command': '$BINARY'}
with open('$CONFIG_FILE', 'w') as f:
    json.dump(config, f, indent=2)
print('Updated $CONFIG_FILE')
"
    else
        echo "Claude Desktop config not found. Add manually:"
        echo '  "synaptic-graph": { "command": "'$BINARY'" }'
    fi
    echo "✓ Claude Desktop configured. Restart the app to activate."
}

configure_codex() {
    echo ""
    echo "Configuring Codex..."

    CODEX_DIR="$HOME/.codex"
    mkdir -p "$CODEX_DIR"

    CONFIG_FILE="$CODEX_DIR/config.json"
    if [ -f "$CONFIG_FILE" ]; then
        python3 -c "
import json
with open('$CONFIG_FILE') as f:
    config = json.load(f)
config.setdefault('mcpServers', {})['synaptic-graph'] = {'command': '$BINARY', 'args': []}
with open('$CONFIG_FILE', 'w') as f:
    json.dump(config, f, indent=2)
print('Updated $CONFIG_FILE')
"
    else
        cat > "$CONFIG_FILE" << CODEXEOF
{
  "mcpServers": {
    "synaptic-graph": {
      "command": "$BINARY",
      "args": []
    }
  }
}
CODEXEOF
        echo "Created $CONFIG_FILE"
    fi
    echo "✓ Codex configured."
}

# Ask what to configure
echo ""
echo "Which clients do you want to configure?"
echo "  1) Claude Code"
echo "  2) Claude Desktop"
echo "  3) Codex"
echo "  4) All"
echo "  5) None (just build)"
echo ""
read -p "Choice [4]: " choice
choice=${choice:-4}

case $choice in
    1) configure_claude_code ;;
    2) configure_claude_desktop ;;
    3) configure_codex ;;
    4) configure_claude_code; configure_claude_desktop; configure_codex ;;
    5) echo "Build complete. Configure manually." ;;
    *) echo "Invalid choice" ;;
esac

echo ""
echo "Done! Binary at: $BINARY"
echo "Test it: $BINARY status"
echo ""
$BINARY status 2>/dev/null || echo "(No memories yet — start a conversation to begin building your graph)"
