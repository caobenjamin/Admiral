#!/bin/sh
# Run from script directory so "ui" is always found
cd "$(dirname "$0")/ui" && npm run dev
