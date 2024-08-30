## A (Rata)TUI interface to manage a single Docker Compose file.

WIP.

TODO:
- [x] Starting individual containers won't start their dependents' log stream.
- [ ] The ALT modifier key on Mac is not working
- [ ] Auto-scroll logs to the latest entry
- [x] Display the project name, directory and the compose file name _somewhere_
- [ ] > Add an info panel for containers with CPU, memory, volumes, networks, etc.
- [ ] Rework the Keys section because it looks horrible (wrap around!)
- [ ] cleanup
- [ ] enable Up/Down on alternate screen (currently it's weird because the not rendered list should manage the state of it..)
