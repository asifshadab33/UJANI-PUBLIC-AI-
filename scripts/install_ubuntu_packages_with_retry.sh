#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -eq 0 ]]; then
    echo "usage: install_ubuntu_packages_with_retry.sh PACKAGE..." >&2
    exit 2
fi

max_attempts=4
for attempt in $(seq 1 "$max_attempts"); do
    if sudo apt-get update \
        -o Acquire::Retries=2 \
        -o Acquire::http::Timeout=30 \
        -o Acquire::https::Timeout=30 \
        && sudo apt-get install --yes --no-install-recommends \
            -o Acquire::Retries=2 \
            -o Acquire::http::Timeout=30 \
            -o Acquire::https::Timeout=30 \
            "$@"; then
        exit 0
    fi

    if [[ "$attempt" -lt "$max_attempts" ]]; then
        delay_seconds=$((attempt * 10))
        echo "Ubuntu package installation failed on attempt $attempt/$max_attempts; retrying in $delay_seconds seconds." >&2
        sleep "$delay_seconds"
    fi
done

echo "Ubuntu package installation failed after $max_attempts attempts." >&2
exit 1
