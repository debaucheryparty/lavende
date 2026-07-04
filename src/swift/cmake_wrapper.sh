#!/bin/bash
if [[ "$*" == *"--build"* ]]; then
  cmake "$@"
else
  cmake "$@" -DCMAKE_POLICY_VERSION_MINIMUM=3.5
fi
