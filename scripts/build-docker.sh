#!/usr/bin/env bash
set -euo pipefail

usage() { echo "Usage: bash scripts/build-docker.sh [--image NAME:TAG] [--export FILE.tar]"; }
image=viewer-core-rust:test
export_path=
while (($#)); do
    case "$1" in
        --image|--export)
            if (($# < 2)) || [[ -z "$2" ]]; then usage >&2; exit 2; fi
            if [[ "$1" == --image ]]; then image=$2; else export_path=$2; fi
            shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) usage >&2; exit 2 ;;
    esac
done
for tool in git docker; do
    command -v "$tool" >/dev/null || { echo "$tool is required." >&2; exit 1; }
done
if [[ -n "$export_path" ]]; then
    if [[ -e "$export_path" ]]; then echo "Export already exists: $export_path" >&2; exit 1; fi
    export_path="$(cd -- "$(dirname -- "$export_path")" && pwd)/$(basename -- "$export_path")"
fi
repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"
[[ "$(docker info --format '{{.OSType}}')" == linux ]] || {
    echo 'Docker must be running with a Linux container engine.' >&2; exit 1;
}
docker buildx version
revision=$(git rev-parse --verify HEAD)
changes=$(git status --porcelain)
dirty=false
if [[ -n "$changes" ]]; then dirty=true; fi
docker buildx build --platform linux/amd64 --load --tag "$image" \
    --build-arg "SOURCE_REVISION=$revision" --build-arg "SOURCE_DIRTY=$dirty" .
if [[ -n "$export_path" ]]; then docker image save --output "$export_path" "$image"; fi
echo "Built $image (linux/amd64); revision=$revision; dirty=$dirty"
