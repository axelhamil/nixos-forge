# Performance

What each profile tunes, and how to measure it on your machine with `nix run .#bench`.

## Profiles

| | `desktop` | `laptop` | `server` | `pi` |
|---|---|---|---|---|
| Kernel | latest | latest | default (LTS) | vendor kernel from the board module |
| CPU scheduler | sched-ext `scx_lavd` | `scx_lavd --autopower` (follows the power profile) | kernel default (EEVDF) | kernel default |
| Power | power-profiles-daemon | power-profiles-daemon | tuned | tuned |
| Out of memory | systemd-oomd on root and user slices, `nix-daemon` spared | same | kernel default | kernel default |
| `/tmp` | tmpfs | tmpfs | on disk | on disk |

`pi` extends `server` and adds a 15 s hardware watchdog and a 100 MB journal cap ([raspberry-pi.md](raspberry-pi.md)).

Every host also gets zram swap, a weekly `nh clean` ([dev.md](dev.md)), automatic store optimisation, and, with `disk`, a monthly btrfs scrub.

systemd-oomd kills the cgroup under the most memory pressure once a slice stays above 80%; it does not act on swap usage. `nix-daemon` has `ManagedOOMPreference=avoid`, so a big build is not the first victim. Nix builds in `/nix/var/nix/builds`, not in `/tmp`.

## Measure

`nix run .#bench -- run <label>` runs 6 benchmarks, 5 times each (about 10 minutes), and writes `bench-<label>.json` in the current directory. Close other applications and stay on AC power.

| Metric | What it shows | Better |
|---|---|---|
| `wakeup_latency_p99` | 99th percentile wake-up latency under saturation ([schbench](https://git.kernel.org/pub/scm/linux/kernel/git/mason/schbench.git), `.#schbench`): scheduler responsiveness | lower |
| `request_latency_p99` | 99th percentile request latency of the same schbench run | lower |
| `pipe_latency_under_load` | round trip between two tasks (`perf bench sched pipe`) while every core is busy | lower |
| `hackbench` | scheduler throughput with many communicating tasks | lower |
| `cpu_throughput` | raw compute (`stress-ng --cpu`) | higher |
| `random_read_iops` | 4 KiB random reads on the disk of the current directory (`fio`, direct IO) | higher |

`nix run .#bench -- compare bench-a.json bench-b.json` prints the median of each metric, the change and a verdict: `better` or `worse` when the min/max ranges of both runs do not overlap, `noise` otherwise, `too few runs` below 5.

`BENCH_RUNS` (default 5), `BENCH_DURATION` in seconds (default 20) and `BENCH_DIR` (default the current directory) change the defaults. A tmpfs directory is refused, since `random_read_iops` would measure RAM.

### Compare schedulers without rebooting

Stopping `scx` falls back to the kernel scheduler:

```sh
sudo systemctl stop scx
nix run .#bench -- run eevdf
sudo systemctl start scx
nix run .#bench -- run lavd
nix run .#bench -- compare bench-eevdf.json bench-lavd.json
```

To try another scheduler, set `services.scx.scheduler` (for example `scx_bpfland`) in your host's `module`, switch, and run the bench again.

### Compare kernels or settings that need a reboot

Run the bench, switch to the new configuration, reboot, and run it again with another label. Each JSON records the kernel and the active scheduler.
