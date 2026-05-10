#!/bin/sh
# examples/curl-send.sh
#
# Send a test mail via http-smtp-rele.
#
# Usage:
#   API_KEY=your-secret ./examples/curl-send.sh
#
# Environment:
#   API_KEY    API key secret (required)
#   BASE_URL   Relay base URL (default: http://127.0.0.1:8080)
#   TO         Recipient address (default: test@example.com)

set -e

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
TO="${TO:-test@example.com}"

if [ -z "$API_KEY" ]; then
    echo "Error: API_KEY environment variable is required" >&2
    echo "Usage: API_KEY=your-secret $0" >&2
    exit 1
fi

curl -s -w "\nHTTP %{http_code}\n" \
    -X POST "${BASE_URL}/v1/send" \
    -H "Authorization: Bearer ${API_KEY}" \
    -H "Content-Type: application/json" \
    -d "{
        \"to\": \"${TO}\",
        \"subject\": \"Test from http-smtp-rele\",
        \"body\": \"This is a test message sent at $(date -u +%Y-%m-%dT%H:%M:%SZ).\"
    }"
