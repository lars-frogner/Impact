#!/bin/bash
set -e

UTILS_DIR=$(dirname $(readlink -f $0))
APP_DIR=$(dirname "${UTILS_DIR}")

for APP_NAME in basic_app impact_game snapshot_tester voxel_generator; do
    cd "${APP_DIR}/${APP_NAME}"
    make generate-roc
done
