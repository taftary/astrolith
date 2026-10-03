# Fixture for issue #92: a criteria file with an unsatisfiable run-conclusion criterion.
# The criteria-shape check must REFUSE this file, naming C2 and the matched phrase.
# (verify: this file exists only so the check has something to refuse)

## Layer 1 criteria (BAD - do not use as a model)

- [ ] C1: The ci job's checkout brings enough history that the guard can resolve a parent commit (verify: file .github/workflows/ci.yml contains fetch-depth: 0)
- [ ] C2: A self-test step exists in the ci job, is not gated by any if:, and on the pull-request run its conclusion is success and not skipped (verify: file .github/workflows/ci.yml contains - name: Intake path guard self-test)
