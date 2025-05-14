#!/bin/bash

./scripts/update_cargo.toml.sh
git submodule update --init --recursive --remote && cargo tauri dev