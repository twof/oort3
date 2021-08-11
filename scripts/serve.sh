#!/bin/bash -eu
eval "$(fnm env)"
set -x

cd $(realpath $(dirname $0))/../www
fnm use
npx webpack serve --mode=development "$@"
