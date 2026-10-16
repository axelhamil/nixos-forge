import json

machine.wait_for_unit("home-manager-tester.service")
firefox = machine.succeed("su - tester -c 'readlink -f $(command -v firefox)'").strip()
policies = json.loads(machine.succeed(f"cat $(dirname {firefox})/../lib/firefox/distribution/policies.json"))
assert policies["policies"].get("DisableTelemetry") is True, "telemetry is not disabled"
