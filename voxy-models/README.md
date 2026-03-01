# voxy-models

Model catalog and local lifecycle state module for Voxy.

## Contains
- `ManagedModel` catalog metadata (id, vendor, size, title)
- `ModelLifecycle` install-state and artifact download/remove under XDG data home
- `InstallState` domain enum (`NotDownloaded`, `Downloaded`)
- `ModelLifecycleError` typed error contract
