def median:
  sort
  | if length % 2 == 1 then .[length / 2 | floor]
    else (.[length / 2 - 1] + .[length / 2]) / 2 | floor
    end;

def runs($count): if $count == 1 then "run" else "runs" end;

def key: .system + "." + .check;

def measure($record; $rerun):
  ($record.runs | length) as $total
  | ($record.runs | map(select(.passed)) | length) as $passed
  | { total: $total, passed: $passed, rate: ($passed * 100 / $total | floor), median: ($record.runs | map(.seconds) | median), rerun: $rerun };

($results[0] | map({ key: key, value: { check, system, runs } }) | from_entries) as $byCheck
| ($planned | map(key)) as $plannedKeys
| $history[0] as $old
| [
    $matrix[]
    | . as $job
    | ($byCheck[$job | key]) as $new
    | ($old.checks[$job.drvPath]) as $recorded
    | if $new != null and ($new.runs | length) > 0 then $job + { record: $new } + measure($new; true)
      elif (($plannedKeys | index($job | key)) | not) and $recorded != null then $job + { record: $recorded } + measure($recorded; false)
      else $job + { record: null }
      end
  ]
| sort_by(.check, .system) as $checks
| ($checks | map(select(.record != null and .passed < .total))) as $flaky
| ($checks | map(select(.record == null))) as $missing
| ($flaky == [] and $missing == []) as $ok
| {
    ok: $ok,
    history: {
      tree: (if $ok then $tree else null end),
      checks: ($checks | map(select(.record != null) | { key: .drvPath, value: .record }) | from_entries)
    },
    markdown: [
      "## Flaky VM checks",
      "",
      "Each VM check is realised once, then rebuilt with `nix build --rebuild` for every run. A run that fails is a test that does not always pass on the same code. A check whose derivation already passed all of its runs is not rerun.",
      "",
      "| Check | System | Pass rate | Median time |",
      "|---|---|---|---|",
      ($checks[]
        | if .record == null then "| `\(.check)` | \(.system) | no result | |"
          elif .rerun then "| `\(.check)` | \(.system) | \(.rate) % | \(.median) s |"
          else "| `\(.check)` | \(.system) | \(.rate) % (not rerun, unchanged) | \(.median) s |"
          end),
      "",
      if $ok then "Every VM check passed all of its runs."
      else
        ($flaky[] | "`\(.check)` passed \(.passed) of its \(.total) \(runs(.total)).")
        , ($missing[] | "`\(.check)` wrote no result: its job failed or hit its time limit, see its log.")
      end
    ]
  }
