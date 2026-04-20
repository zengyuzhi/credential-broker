## 1. Compatibility baseline docs

- [x] 1.1 Align `README.md`, `docs/ARCHITECTURE.md`, and the phase-plan docs around the same Phase 0 compatibility-baseline terminology
- [x] 1.2 Document that brokered access is the target model and env injection is the compatibility path
- [x] 1.3 Verify the published docs make Phase 0 explicitly non-breaking and do not imply a forced migration

## 2. CLI help labeling

- [x] 2.1 Update `vault run --help` copy to disclose env injection as compatibility access
- [x] 2.2 Update `vault profile bind --help` mode descriptions to distinguish `inject`, `proxy`, and `either`
- [x] 2.3 Review root-command help text and adjust it if needed so the overall product description matches the broker-first architecture direction

## 3. Regression checks for the baseline

- [x] 3.1 Add or update help-text assertions that cover the new compatibility and brokered-access wording
- [x] 3.2 Run targeted verification for the current credential/profile/run/serve/ui/upgrade workflows to ensure Phase 0 remains non-breaking
- [x] 3.3 Update release-facing notes or changelog guidance if the new Phase 0 labeling affects user-facing messaging
