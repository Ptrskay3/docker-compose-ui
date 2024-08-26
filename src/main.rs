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
use std::path::Path;

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

    // Try to load the .env from the same directory as the docker-compose file.
    let sfile = Path::new(&file).canonicalize()?;
    let dotenv_file = sfile.parent().expect("a directory").join(".env");
    dotenvy::from_path(dotenv_file).ok();

    let project_name = std::env::var("COMPOSE_PROJECT_NAME").unwrap_or_else(|_| {
        let components = sfile.components().collect::<Vec<_>>();
        if let Some(component) = components.get(components.len().saturating_sub(2)) {
            component.as_os_str().to_string_lossy().into_owned()
        } else {
            // TODO: This shouldn't happen ever.
            "docker".to_string()
        }
    });

    let mut container_name_mapping = HashMap::new();
    for (i, (service_name, info)) in compose_content.services.clone().0.iter().enumerate() {
        let service_name = if let Some(info) = info {
            if let Some(container_name) = &info.container_name {
                container_name.clone()
            } else {
                // We don't scale services, the 1 index should be fine.
                format!("{}-{}-1", project_name, service_name)
            }
        } else {
            format!("{}-{}-1", project_name, service_name)
        };
        container_name_mapping.insert(i, service_name.clone());
    }
    let mut app = App::new(
        project_name,
        compose_content,
        container_name_mapping,
        running_container_names,
        docker.clone(),
        file,
    );

    for (i, service_name) in &app.container_name_mapping {
        app.compose_content
            .start_log_stream(*i, service_name, docker.clone())
            .await?;
    }

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
                DockerEvent::ErrorLog(log) => {
                    app.set_error_log(log);
                    app.show_popup = true;
                    app.clear_starting();
                }
            }
        }
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(())
}
