shopt -s inherit_errexit

duration=${BENCH_DURATION:-20}
runs=${BENCH_RUNS:-5}
io_dir=${BENCH_DIR:-$PWD}
min_runs=5
pipe_loops=1000000

workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT
trap 'exit 130' INT

usage() {
  echo "usage: bench run <label>" >&2
  echo "       bench compare <before.json> <after.json>" >&2
  exit 2
}

fail() {
  echo "bench: $1" >&2
  exit 1
}

active_scheduler() {
  if [ -r /sys/kernel/sched_ext/root/ops ]; then
    cat /sys/kernel/sched_ext/root/ops
  else
    echo eevdf
  fi
}

require_disk_dir() {
  [ -d "$io_dir" ] || fail "$io_dir does not exist"

  local fstype
  fstype=$(findmnt -no FSTYPE -T "$io_dir")

  case $fstype in
  tmpfs | ramfs)
    fail "$io_dir is on $fstype, random_read_iops would measure RAM; run from a disk directory or set BENCH_DIR"
    ;;
  esac
}

require_value() {
  local metric=$1 value=$2
  [ -n "$value" ] && [ "$value" != null ] || fail "$metric returned no value"
  echo "$value"
}

summarize() {
  local metric=$1 unit=$2 better=$3
  shift 3

  printf '%s\n' "$@" | jq -s \
    --arg metric "$metric" --arg unit "$unit" --arg better "$better" '
      def median: sort | if length % 2 == 1 then .[length / 2 | floor] else (.[length / 2 - 1] + .[length / 2]) / 2 end;
      { ($metric): { unit: $unit, better: $better, values: ., min: min, max: max, median: median } }'
}

measure_schbench() {
  local report="$workdir/schbench.json"
  local wakeup=() request=()

  for i in $(seq "$runs"); do
    echo "schbench: run $i/$runs" >&2
    rm -f "$report"
    schbench -m 2 -r "$duration" -j "$report" >/dev/null 2>&1
    wakeup+=("$(require_value wakeup_latency_p99 "$(jq '.int["wakeup_latency_pct99.0"]' "$report")")")
    request+=("$(require_value request_latency_p99 "$(jq '.int["request_latency_pct99.0"]' "$report")")")
  done

  summarize wakeup_latency_p99 us lower "${wakeup[@]}"
  summarize request_latency_p99 us lower "${request[@]}"
}

probe_pipe_latency_under_load() (
  stress-ng --cpu "$(nproc)" --timeout 0 --quiet >/dev/null &
  load_pid=$!
  trap 'kill "$load_pid" 2>/dev/null || true; wait "$load_pid" 2>/dev/null || true' EXIT
  sleep 1

  seconds=$(perf bench -f simple sched pipe -l "$pipe_loops")
  jq -n "$seconds * 1000000 / $pipe_loops"
)

probe_hackbench() {
  hackbench -g 10 -l 5000 | awk '/^Time:/ { print $2 }'
}

probe_cpu_throughput() {
  local report="$workdir/cpu.yaml"
  rm -f "$report"
  stress-ng --cpu 0 --timeout "${duration}s" --metrics-brief --yaml "$report" --quiet
  awk '/^      bogo-ops-per-second-real-time:/ { print $2; exit }' "$report"
}

probe_random_read_iops() {
  fio --name=bench-randread --directory="$io_dir" --rw=randread --bs=4k --size=1G \
    --ioengine=io_uring --direct=1 --runtime="$duration" --time_based \
    --output-format=json --unlink=1 | jq '.jobs[0].read.iops'
}

measure() {
  local metric=$1 unit=$2 better=$3
  local values=()

  for i in $(seq "$runs"); do
    echo "$metric: run $i/$runs" >&2
    values+=("$(require_value "$metric" "$("probe_$metric")")")
  done

  summarize "$metric" "$unit" "$better" "${values[@]}"
}

run() {
  local label=$1
  local output="bench-$label.json"

  require_disk_dir

  {
    measure_schbench
    measure pipe_latency_under_load us lower
    measure hackbench s lower
    measure cpu_throughput bogo_ops_per_s higher
    measure random_read_iops iops higher
  } | jq -s \
    --arg label "$label" \
    --arg kernel "$(uname -r)" \
    --arg scheduler "$(active_scheduler)" \
    --argjson runs "$runs" \
    --argjson duration "$duration" \
    '{ label: $label, kernel: $kernel, scheduler: $scheduler, runs: $runs, duration_s: $duration, results: add }' \
    >"$workdir/result.json"

  mv "$workdir/result.json" "$output"
  echo "$output"
}

compare() {
  jq -r -n --slurpfile before "$1" --slurpfile after "$2" --argjson min_runs "$min_runs" '
    def verdict($old; $new; $change):
      if ($old.values | length) < $min_runs or ($new.values | length) < $min_runs then "too few runs"
      elif $old.min <= $new.max and $new.min <= $old.max then "noise"
      elif ($change < 0) == ($old.better == "lower") then "better"
      else "worse" end;

    $before[0] as $b | $after[0] as $a |
    ["metric", $b.label, $a.label, "change", "verdict"],
    ($b.results | keys[] as $metric |
      $b.results[$metric] as $old | $a.results[$metric] as $new |
      select($new != null and $old.median != 0) |
      (($new.median - $old.median) / $old.median * 100) as $change |
      [
        $metric,
        ($old.median | tostring),
        ($new.median | tostring),
        (($change * 10 | round / 10 | tostring) + "%"),
        verdict($old; $new; $change)
      ]) | @tsv' | column -t -s "$(printf '\t')"
}

case ${1:-} in
run)
  [ $# -eq 2 ] || usage
  run "$2"
  ;;
compare)
  [ $# -eq 3 ] || usage
  compare "$2" "$3"
  ;;
*)
  usage
  ;;
esac
