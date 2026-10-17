import re

doctor_output =machine.succeed("doctor 2>&1 || true")
print(doctor_output)
assert re.search(r"\[\s*OK\]\s+Secure Boot\s", doctor_output), "doctor does not see Secure Boot enforced"
assert re.search(r"\[\s*OK\]\s+LUKS unlock\s", doctor_output), "doctor does not see the TPM2 token"
