#!/usr/bin/env bash
set -euo pipefail

arm="${1:-}"
task_id="${2:-}"
repo_root="${3:-.}"

if [[ -z "$arm" || -z "$task_id" ]]; then
  echo "usage: bash experiments/g0/materialize-prerequisite-base.sh ARM TASK_ID [REPO_ROOT]" >&2
  exit 2
fi

cd "$repo_root"
repo_root="$(pwd -P)"
manifest="$repo_root/experiments/g0/prerequisite-bases.json"

if [[ ! -f "$manifest" ]]; then
  echo "missing prerequisite manifest: $manifest" >&2
  exit 2
fi

if [[ -n "$(git status --porcelain --untracked-files=all)" ]]; then
  echo "G0 prerequisite materialization requires a clean worktree" >&2
  exit 2
fi

stage="$(jq -er --arg arm "$arm" --arg task "$task_id" '.taskBases[$arm][$task]' "$manifest")"
parent="$(git rev-parse HEAD)"

if [[ "$stage" == "EMPTY" ]]; then
  printf '%s\n' "$parent"
  exit 0
fi

template="$(jq -er --arg arm "$arm" --arg stage "$stage" '.templates[$arm][$stage]' "$manifest")"
template="$repo_root/$template"
if [[ ! -d "$template" ]]; then
  echo "no prerequisite template implemented for $arm $stage" >&2
  exit 2
fi

case "$arm" in
  B_FULL)
    arm_root="$repo_root/baselines/b-full"
    rm -rf "$arm_root/src/features"
    ;;
  C_UIKO)
    arm_root="$repo_root/fixtures/support-console"
    rm -rf "$arm_root/features"
    ;;
  R_RENDER_ONLY)
    arm_root="$repo_root/experiments/controls/r-render-only"
    rm -rf "$arm_root/src/features"
    ;;
  T_AUTHORING)
    arm_root="$repo_root/experiments/controls/t-authoring"
    rm -rf "$arm_root/generated"
    ;;
  *)
    echo "materialization for arm $arm is not implemented yet" >&2
    exit 2
    ;;
esac

cp -R "$template"/. "$arm_root"/
git add -A "$arm_root"

if git diff --cached --quiet; then
  echo "prerequisite template produced no measured-arm changes" >&2
  exit 2
fi

tree="$(git write-tree)"
author_name="$(jq -er '.syntheticCommit.authorName' "$manifest")"
author_email="$(jq -er '.syntheticCommit.authorEmail' "$manifest")"
timestamp="$(jq -er '.syntheticCommit.timestamp' "$manifest")"
message="g0 prerequisite $arm $stage for $task_id"

commit="$(
  printf '%s\n' "$message" |
    GIT_AUTHOR_NAME="$author_name" \
    GIT_AUTHOR_EMAIL="$author_email" \
    GIT_AUTHOR_DATE="$timestamp" \
    GIT_COMMITTER_NAME="$author_name" \
    GIT_COMMITTER_EMAIL="$author_email" \
    GIT_COMMITTER_DATE="$timestamp" \
    git commit-tree "$tree" -p "$parent"
)"

git reset --hard --quiet "$commit"
printf '%s\n' "$commit"
