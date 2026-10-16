# Performance

Each profile only ships settings backed by a source or a measurement. CI checks that they are active; whether they make your machine faster is measured on your machine, with `nix run .#bench`.

## Profiles

| | `desktop` | `laptop` | `server` |
|---|---|---|---|
| Kernel | latest | latest | default (LTS) |
| CPU scheduler | sched-ext `scx_lavd` | `scx_lavd --autopower` (follows the power profile) | kernel default (EEVDF) |
| Power | power-profiles-daemon | power-profiles-daemon | tuned |
| Out of memory | systemd-oomd on root and user slices, `nix-daemon` spared | same | kernel default |
| `/tmp` | tmpfs | tmpfs | on disk |

Every host also gets zram swap (zstd), a weekly `nh clean` keeping 14 days and the last 5 generations ([dev.md](dev.md)), store optimisation on a timer, and, with `disk`, a monthly btrfs scrub.

systemd-oomd kills a cgroup of a slice whose memory pressure stays above 80% (the NixOS default), picking the one under the most pressure. It does not kill on swap usage: Fedora did and Ubuntu dropped it as too eager. `nix-daemon` is marked `ManagedOOMPreference=avoid`, so a big build is not the first victim. `/tmp` on tmpfs does not hold Nix builds: Nix builds in `/nix/var/nix/builds`.

Not adopted until a measurement backs them: the CachyOS kernel, thermald on Intel laptops, and the zram sysctls (`vm.swappiness=180`, `vm.page-cluster=0`). Measure them with the steps below before adding them to your fork.

## Measure

`nix run .#bench -- run <label>` runs 6 benchmarks, 5 times each (about 10 minutes), and writes `bench-<label>.json` in the current directory:

| Metric | What it shows | Better |
|---|---|---|
| `wakeup_latency_p99` | 99th percentile wake-up latency ([schbench](https://git.kernel.org/pub/scm/linux/kernel/git/mason/schbench.git), packaged in this flake as `.#schbench`): under saturation (`-m 2`, workers on every core): the reference for scheduler responsiveness | lower |
| `request_latency_p99` | 99th percentile request latency of the same schbench run | lower |
| `pipe_latency_under_load` | round trip between two tasks (`perf bench sched pipe`) while every core is busy | lower |
| `hackbench` | scheduler throughput with many communicating tasks | lower |
| `cpu_throughput` | raw compute (`stress-ng --cpu`) | higher |
| `random_read_iops` | 4 KiB random reads on the disk of the current directory (`fio`, direct IO) | higher |

`nix run .#bench -- compare bench-a.json bench-b.json` prints the median of each metric, the change and a verdict. `better` or `worse` only when the min/max ranges of both runs do not overlap, which with 5 runs per side matches a two-sided exact Mann-Whitney test at p ≈ 0.008; `noise` otherwise, and `too few runs` below 5 runs. Close other applications and keep the machine on AC power while it runs.

`BENCH_RUNS` (default 5), `BENCH_DURATION` in seconds (default 20) and `BENCH_DIR` (default the current directory) change the defaults. The bench refuses a tmpfs directory: `random_read_iops` would measure RAM, not the disk.

### Compare schedulers without rebooting

sched-ext schedulers load and unload at runtime; stopping the service falls back to the kernel scheduler:

```sh
sudo systemctl stop scx
nix run .#bench -- run eevdf
sudo systemctl start scx
nix run .#bench -- run lavd
nix run .#bench -- compare bench-eevdf.json bench-lavd.json
```

To try another scheduler, set `services.scx.scheduler` (for example `scx_bpfland`) in your host's `module`, switch, and run the bench again.

### Compare kernels or settings that need a reboot

Run the bench on the current generation, switch to the new configuration, reboot, and run it again with another label. Each JSON records the kernel and the active scheduler, so the comparison stays readable later.
