#!/bin/sh
# Fails when the repository holds something shaped like a Resend API key (feature 013, SC-004).
# The key lives only in the report Worker's secrets; worker/test uses fake values.
set -eu
cd "$(dirname "$0")/../../.."
pattern='(^|[^A-Za-z0-9_])re_[A-Za-z0-9_]{20,}'

# The pattern itself must catch a key and ignore ordinary identifiers.
printf 'key = "re_AbCdEfGh12345678_ijklmnop"\n' | grep -Eq "$pattern"
if printf 'prepare_default_provider_install\n' | grep -Eq "$pattern"; then
  echo "pattern matches an ordinary identifier" >&2
  exit 1
fi

if git grep -nIE "$pattern" -- . ':!worker/test' ':!worker/package-lock.json' ':!package-lock.json'; then
  echo "Possible Resend API key committed; keep it in Worker secrets only." >&2
  exit 1
fi
echo "No Resend key in the repository."
