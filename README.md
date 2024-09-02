## A (Rata)TUI interface to manage a single Docker Compose file.

WIP.

TODO:
- [x] Starting individual containers won't start their dependents' log stream.
- [x] Display the project name, directory and the compose file name _somewhere_
- [x] Add an info panel for containers with labels, volumes, networks, etc.
- [x] Rework the Keys section because it looks horrible (wrap around!)
- [x] enable Up/Down on alternate screen
- [x] anyhow::Result
- [x] clap::Parser
- [x] if logs are cleared, only fetch logs for that container from that timestamp
- [ ] The ALT modifier key on Mac is not working
- [ ] Auto-scroll logs to the latest entry
- [ ] cleanup
- [ ] Move from IndexMap to simple Vec if possible
- [ ] Memory usage?
