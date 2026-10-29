#!/bin/sh
# Run the reference model without BEAST. Build the release binary first.
set -eu

if [ "$#" -ne 2 ]; then
    printf 'usage: sh fixtures/reference/20x50/rove.sh <new-output-directory> <seed>\n' >&2
    exit 2
fi

fixture=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repository=$(CDPATH= cd -- "$fixture/../../.." && pwd)
exec "$repository/target/release/rove" sample "$fixture/matrix.nex" \
    --model reference --output "$1" --seed "$2" \
    --steps 10000000 --sample-every 1000 \
    --monophyly T1,T2,T3,T4,T5,T6,T7,T8,T9,T10,T11,T12,T13,T14,T15,T16,T17,T18,T19
