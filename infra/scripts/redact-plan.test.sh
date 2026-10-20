#!/usr/bin/env bash
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)

fail() { printf 'FAIL: %s\n--- got ---\n%s\n' "$1" "$2"; exit 1; }

got=$(printf 'email = "deploy@my-proj-123.iam.gserviceaccount.com"\nname = "projects/987654/locations/global"\n' | "$here/redact-plan.sh" my-proj-123 987654)
want=$(printf 'email = "deploy@***.iam.gserviceaccount.com"\nname = "projects/***/locations/global"\n')
[ "$got" = "$want" ] || fail "replaces every value" "$got"

got=$(printf 'a.b*c\n' | "$here/redact-plan.sh" 'a.b*c')
[ "$got" = "***" ] || fail "treats values literally, not as patterns" "$got"

got=$(printf 'unchanged\n' | "$here/redact-plan.sh" "")
[ "$got" = "unchanged" ] || fail "ignores empty values" "$got"

for order in "my-proj-123 987654 my-proj-123-tfstate" "my-proj-123-tfstate 987654 my-proj-123"; do
  # shellcheck disable=SC2086
  got=$(printf 'Refreshing state... [id=b/my-proj-123-tfstate/x] p=my-proj-123 n=987654\n' | "$here/redact-plan.sh" $order)
  [ "$got" = "Refreshing state... [id=b/***/x] p=*** n=***" ] || fail "redacts a bucket name containing the project ID ($order)" "$got"
done

got=$(printf 'bucket = "my-proj-123-whichride-backups"\n' | "$here/redact-plan.sh" my-proj-123 987654 my-proj-123-tfstate)
[ "$got" = 'bucket = "***-whichride-backups"' ] || fail "hides the project ID inside the backups bucket name" "$got"

echo "redact-plan: all tests passed"
