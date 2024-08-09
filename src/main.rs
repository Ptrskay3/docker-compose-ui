use bollard::container::ListContainersOptions;
use bollard::Docker;
use dcr::app::{App, AppResult};
use dcr::event::{Event, EventHandler};
use dcr::handler::{handle_key_events, DockerEvent};
use dcr::tui::Tui;
use docker_compose_types::Compose;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::collections::HashMap;
use std::io;

#[tokio::main]
async fn main() -> AppResult<()> {
    #[cfg(unix)]
    let docker = Docker::connect_with_socket_defaults()?;

    let mut list_container_filters = HashMap::new();
    list_container_filters.insert("status", vec!["running"]);

    let containers = &docker
        .list_containers(Some(ListContainersOptions {
            all: true,
            filters: list_container_filters,
            ..Default::default()
        }))
        .await?;

    let running_container_names = containers
        .iter()
        .cloned()
        .flat_map(|c| c.names)
        .flatten()
        .map(|name| name.trim_start_matches("/").into())
        .collect::<Vec<String>>();

    let file = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "docker-compose.yml".to_string());
    let file_payload = std::fs::read_to_string(&file)?;
    let compose_content = match serde_yaml::from_str::<Compose>(&file_payload) {
        Ok(c) => c,
        Err(e) => panic!("Failed to parse docker-compose file: {}", e),
    };

    let mut app = App::new(compose_content, running_container_names, docker, file);

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    // Start the main loop.
    while app.running {
        // Render the user interface.

        tui.draw(&mut app)?;

        // Handle events.
        match tui.events.next().await? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, &mut app, tx.clone()).await?,
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }
        if let Ok(docker_event) = rx.try_recv() {
            match docker_event {
                DockerEvent::Refresh => app.refresh().await?,
            }
        }
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(())
}
