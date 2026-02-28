# voxy-app

GTK4 application shell for Voxy.

## Responsibilities
- Build widgets and window layout
- Dispatch UI actions as `AppEvent`
- Execute side-effect commands produced by `voxy-core`
- Render view state

## Non-Responsibilities
- No business/domain rules in GTK callbacks
- No direct STT logic
- No direct audio pipeline logic
