#!/bin/sh
# Set the git identity, the hooks path and the branch for each session.
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
git config user.name Azathothas
git config user.email ajamx101@gmail.com
# The commit-msg hook refuses agent attribution lines.
git config core.hooksPath .githooks
# A cloud session owns its global identity; a local session keeps the
# owner's own global settings.
if [ "${CLAUDE_CODE_REMOTE:-}" = true ]; then
  git config --global user.name Azathothas
  git config --global user.email ajamx101@gmail.com
fi
# Work on main. Switch only from a clean tree, and never reset a local
# main: it can hold commits that origin/main does not have.
cur=$(git symbolic-ref --short -q HEAD)
if [ "$cur" != main ] && [ -z "$(git status --porcelain 2>/dev/null)" ]; then
  git fetch -q origin main 2>/dev/null
  if git rev-parse -q --verify refs/heads/main >/dev/null; then
    git checkout -q main 2>/dev/null || true
  elif git rev-parse -q --verify refs/remotes/origin/main >/dev/null; then
    git checkout -q -b main origin/main 2>/dev/null || true
  fi
fi
exit 0
