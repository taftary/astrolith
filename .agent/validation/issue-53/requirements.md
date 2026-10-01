# Requirements — issue #53 (validator self-proof probes)

Owner source: issue #53 body (agent fix request). These two probes are the
validator's own smoke criteria for this Bevy workspace: trivial and unambiguous
(exact log strings), so no owner confirmation round is needed for them.
The full owner checklist for the agent-fix request itself will be appended
separately and will carry its own pending-confirmation marker until approved.

- [ ] C1: headless verify run prints VERIFY-OK (verify: contains VERIFY-OK in verify.log)
- [ ] C2: workspace test run reports ok (verify: contains test result: ok in cargo-test.log)
