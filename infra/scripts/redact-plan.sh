#!/usr/bin/env bash
# Replaces each argument with *** in stdin, literally. Used before posting a plan publicly.
# Longest values go first so a value that contains another one is still fully redacted.
set -euo pipefail

text=$(cat)
while IFS= read -r value; do
  [ -n "$value" ] || continue
  text=${text//"$value"/***}
done < <(for v in "$@"; do printf '%s\t%s\n' "${#v}" "$v"; done | sort -t "$(printf '\t')" -k1,1nr | cut -f2-)
printf '%s\n' "$text"
