cat > "$PLUGIN_DATA/input.json"
printf '%s\n' "$PLUGIN_ROOT" "$PWD" > "$PLUGIN_DATA/environment"
exit 1
