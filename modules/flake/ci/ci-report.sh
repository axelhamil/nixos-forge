usage="usage: ci-report [--comment]"

limit=1048576
footer="The report is too long for the run summary: the end is cut, the build logs hold every detail."

case "${1:-}" in
"") ;;
--comment)
  limit=65536
  footer="The report is too long for a comment: read it in full in the [run summary](${RUN_URL:-})."
  ;;
*)
  echo "$usage" >&2
  exit 2
  ;;
esac

results=${RESULTS_DIR:-results}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

as_json() {
  local value
  value=$(jq -c "if type == \"$3\" then . else $2 end" <<<"${1:-null}" 2>/dev/null || true)
  echo "${value:-$2}"
}

shopt -s nullglob

for result in "$results"/*.json; do
  jq -c . "$result"
done | jq -s . >"$work/results.json"

for closure in "$results"/closure-*.md; do
  cat "$closure"
  echo
done >"$work/closures.md"

jq -n -r \
  --argjson planned "$(as_json "${PLANNED:-}" null array)" \
  --argjson skipped "$(as_json "${SKIPPED:-}" '[]' array)" \
  --argjson hosts "$(as_json "${HOST_CHECKS:-}" '{}' object)" \
  --argjson descriptions "$(as_json "${DESCRIPTIONS:-}" '{}' object)" \
  --argjson baseBoot "$(as_json "$(cat "$results/base/boot-time.json" 2>/dev/null || true)" '{}' object)" \
  --slurpfile results "$work/results.json" \
  --rawfile causes "$CAUSES" \
  --arg cache "${CACHE_OUTCOME:-}" \
  --arg lint "${LINT_RESULT:-}" \
  --arg plan "${PLAN_RESULT:-}" \
  --arg run "${RUN_URL:-}" '
  ($results[0] | map({ key: (.system + "." + .check), value: . }) | from_entries) as $byCheck
  | ($causes | split("\n") | map(select(. != "") | split("\t") | { pattern: .[0], cause: .[1], todo: .[2] })) as $causes

  | def bad: .status == "failed" or .status == "did not finish";

    def ready: "Nothing to fix: this commit is ready for the machines.";

    def toFix: "Fix what is listed below and push again: CI runs on every push.";

    def plural($n; $one; $many): if $n == 1 then $one else $many end;

    def names: if length < 2 then join("") else (.[:-1] | join(", ")) + " and " + .[-1] end;

    def cell:
      if .status == "skipped" then
        "skipped: " + (if .reason == "cached" then "already passed" else "runs after the merge" end)
      else .status end;

    def code:
      (([ scan("`+") | length ] | max // 0) + 1) as $n
      | ("`" * $n) as $delimiter
      | $delimiter + (if test("^`|`$") then " " + . + " " else . end) + $delimiter;

    def escapeCell: gsub("\\|"; "\\|");

    def row: "| \(.check | code | escapeCell)\(.where) | \($descriptions[.check] // "its own test passes" | escapeCell) | \(cell) |";

    def table: ["| Check | What it proves | Result |", "|---|---|---|"] + map(row);

    def diagnosis:
      if .status == "did not finish" then
        { cause: "the job stopped before it wrote a result: it was cancelled or hit its time limit", todo: "rerun the job; if it stops again, a test hangs" }
      elif .check == "treefmt" then
        { cause: "the code is not formatted or a linter found a problem", todo: "run `nix fmt`, then commit the result" }
      else
        .excerpt as $excerpt
        | (first($causes[] | select(.pattern as $p | any($excerpt[]; test($p))))
          // { cause: "no known cause", todo: "open the full log" })
      end;

    def failure:
      . as $f
      | diagnosis as $d
      | [
          "#### \(.check | code) (\(.system))",
          "",
          "This check proves that \($descriptions[.check] // "its own test passes").",
          "",
          "- Cause: \($d.cause).",
          "- What to do: \($d.todo | gsub("<system>"; $f.system) | gsub("<check>"; $f.check))."
        ]
        + (if (.excerpt | length) > 0 then
            ("`" * ([ 3, (.excerpt[] | scan("`+") | length + 1) ] | max)) as $fence
            | ["", $fence] + .excerpt + [$fence]
          else [] end)
        + (if $run != "" then ["", "[Open the full log](\($run))"] else [] end)
        + [""];

    def seconds: if type == "number" then "\((. * 10 | round) / 10 | tostring | if test("[.]") then . else . + ".0" end) s" else "n/a" end;

    def bootTimes:
      [ $hosts | to_entries | sort_by(.key)[]
        | .key as $host
        | select(.value | index($host + "-boots"))
        | { host: $host,
            current: (first($results[0][] | select(.check == $host + "-boots") | .bootTime | objects) // {}),
            base: ($baseBoot[$host] | objects // {}) } ]
      | if any(.[]; .current != {} or .base != {}) then
          [ "### Boot time",
            "",
            "`systemd-analyze` in each boot test, on a shared CI VM: read the trend over several runs, not one value.",
            "",
            "| Machine | Total | Total on the base | Userspace | Userspace on the base |",
            "|---|---|---|---|---|" ]
          + map("| \(.host | escapeCell) | \(.current.total | seconds) | \(.base.total | seconds) | \(.current.userspace | seconds) | \(.base.userspace | seconds) |")
          + [ "" ]
        else [] end;

    def screenshots:
      [ $results[0][] | select((.screenshots // []) != []) ] | sort_by(.check, .system)
      | if . == [] then []
        else
          [ "### Screenshots", "" ]
          + map("- \(.check | code) (\(.system)): \(.screenshots | map(code) | join(", ")) in the artifact \("screenshot-\(.system)-\(.check)" | code) of \(if $run != "" then "[this run](\($run)#artifacts)" else "this run" end).")
          + [ "" ]
        end;

    def lintStatus:
      if $lint == "success" then "passed"
      elif $lint == "failure" then "failed"
      elif $lint == "skipped" then "skipped"
      else "did not finish" end;

  if $planned == null and $plan == "failure" then
    [
      "**The flake did not evaluate, so nothing was built or tested.**",
      "",
      toFix,
      "",
      (if $run != "" then "The `plan` job log names the file and line: [open the run](\($run)), then the `plan` job." else "The `plan` job log names the file and line." end)
    ]
  elif $planned == null then
    [
      "**CI could not plan this run, so nothing was built.**",
      "",
      toFix,
      "",
      (if $run != "" then "[Open the full log](\($run)) to see why." else "Open the full log to see why." end)
    ]
  else
    (
      [ $planned[] | . as $p | { check, system, status: ($byCheck[$p.system + "." + $p.check].outcome // "did not finish"), excerpt: ($byCheck[$p.system + "." + $p.check].excerpt // []) } ]
      + [ $skipped[] | { check, system, status: "skipped", reason, excerpt: [] } ]
      + [ { check: "treefmt", system: "x86_64-linux", status: lintStatus, excerpt: [] } ]
    ) as $listed
    | ($listed | group_by(.check) | map(select(length > 1) | .[0].check)) as $onSeveralSystems
    | ($listed | map(. + { where: (if .check | IN($onSeveralSystems[]) then " (\(.system))" else "" end) })) as $all
    | ([ $hosts[][] ]) as $hostCheckNames
    | ($all | map(select(bad))) as $bad
    | ($bad | map(.check)) as $badNames
    | ([ $hosts | to_entries[] | select(any(.value[]; IN($badNames[]))) | .key ] | sort) as $badHosts
    | ($all | map(select(.status == "unstable")) | length) as $unstable
    | ($all | map(select(.status == "skipped" and .reason == "cached")) | length) as $cached
    | ($all | map(select(.check | IN($hostCheckNames[]) | not))) as $modules
    | ($bad | length) as $nbad
    | (
        if $nbad == 0 then
          "**Everything works: every machine builds, boots and passed its tests.**"
        else
          (if all($bad[]; .status == "failed") then "failed"
           elif all($bad[]; .status == "did not finish") then "did not finish"
           else "failed or did not finish" end) as $what
          | "**\($nbad) \(plural($nbad; "check"; "checks")) \($what): "
            + (if $badHosts == [] then "no machine is affected, but CI stays red until it is fixed."
               else "\($badHosts | names) would not get this change." end)
            + "**"
        end
      ) as $verdict
    | [ $verdict, "", (if $nbad == 0 then ready else toFix end), "" ]
      + (if $unstable > 0 then [ "\($unstable) \(plural($unstable; "test passed only on its second try and is"; "tests passed only on their second try and are")) marked unstable.", "" ] else [] end)
      + (if $cache == "failure" then [ "The binary cache was unreachable: everything was built from scratch.", "" ] else [] end)
      + (if any($skipped[]; .reason == "compiles a kernel") then [ "Checks that compile a kernel run after the merge.", "" ] else [] end)
      + (if $cached > 0 then [ "\($cached) \(plural($cached; "check was skipped: it already passed with the same code"; "checks were skipped: they already passed with the same code")).", "" ] else [] end)
      + ([
          $hosts | to_entries | sort_by(.key)[]
          | .key as $host
          | (.value | map(. as $name | $all[] | select(.check == $name))) as $rows
          | select($rows != [])
          | [ "### \($host)", "" ] + ($rows | table) + [ "" ]
        ] | add // [])
    | . + (
        if $modules == [] then []
        else
          ([ "passed", "unstable", "failed", "did not finish", "skipped" ]
           | map(. as $s | ($modules | map(select(.status == $s)) | length) as $n | select($n > 0) | "\($n) \($s)")
           | join(", ")) as $counts
          | [ (if any($modules[]; bad) then "<details open>" else "<details>" end),
              "<summary>Module checks: \($counts)</summary>",
              "" ]
            + ($modules | sort_by(.check) | table)
            + [ "", "</details>", "" ]
        end
      )
    | . + bootTimes + screenshots
    | . + (if $nbad > 0 then [ "### What went wrong", "" ] + ($bad | map(failure) | add) else [] end)
  end
  | .[]
' >"$work/report.md"

if [ -s "$work/closures.md" ]; then
  {
    echo "<details>"
    echo "<summary>Package changes per machine</summary>"
    echo
    cat "$work/closures.md"
    echo "</details>"
  } >>"$work/report.md"
fi

LC_ALL=C awk -v limit="$limit" -v footer="$footer" -f "$TRUNCATE" "$work/report.md"
