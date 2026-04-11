#!/usr/bin/env bash
set -euo pipefail

SMOKE_LANE=custom-image "$(dirname "$0")/common.sh" ubuntu uefi
