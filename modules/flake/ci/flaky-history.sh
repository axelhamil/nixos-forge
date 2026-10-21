valid='
  type == "object"
  and (.tree == null or (.tree | type == "string"))
  and (.checks | type == "object")
  and all(.checks[]; type == "object" and (.runs | type == "array") and all(.runs[]; (.passed | type == "boolean") and (.seconds | type == "number")))
'

status=0
history=$(ci-badges read flaky.json) || status=$?

if [ "$status" -eq 1 ]; then
  echo "flaky-history: without the record, every VM check would be measured again; fix the fetch instead" >&2
  exit 1
fi

if [ "$status" -eq 0 ] && jq -e -s "length == 1 and (.[0] | $valid)" <<<"$history" >/dev/null 2>&1; then
  jq -c -s ".[0]" <<<"$history"
  exit 0
fi

echo "flaky-history: no valid flaky.json on the badges branch, every VM check is measured" >&2
echo '{"tree":null,"checks":{}}'
