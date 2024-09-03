use anyhow::Context;
use bollard::container::ListContainersOptions;
use bollard::Docker;
use clap::Parser;
use dcr::app::App;
use dcr::event::{Event, EventHandler};
use dcr::handler::{handle_key_events, handle_mouse_events, DockerEvent};
use dcr::tui::Tui;
use dcr::{LIGHT_MODE, MAX_PATH_CHARS};
use docker_compose_types::Compose;
use indexmap::IndexMap;
use miette::LabeledSpan;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::collections::HashMap;
use std::io;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(default_value_t = String::from("docker-compose.yml"))]
    compose_file: String,

    /// Set the maximum path length to display without truncating.
    #[arg(env, long, default_value_t = 40)]
    max_path_len: usize,

    /// Enable light mode.
    #[arg(env = "DCR_LIGHT_MODE", long)]
    light: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .context_lines(3)
                .terminal_links(true)
                .build(),
        )
    }))?;
    #[cfg(unix)]
    let docker =
        Docker::connect_with_socket_defaults().context("Failed to connect to Docker daemon")?;

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
        .filter_map(|c| c.names)
        .flatten()
        .map(|name| name.trim_start_matches('/').into())
        .collect::<Vec<String>>();

    let Args {
        compose_file: file,
        max_path_len,
        light,
    } = Args::parse();
    MAX_PATH_CHARS.set(max_path_len).unwrap();
    LIGHT_MODE.set(light).unwrap();
    let full_path = Path::new(&file).canonicalize()?;

    let file_payload =
        std::fs::read_to_string(&file).with_context(|| format!("file '{file}' not found"))?;
    let deserializer = serde_yaml::Deserializer::from_str(&file_payload);
    let compose_content = match serde_path_to_error::deserialize::<'_, _, Compose>(deserializer) {
        Ok(c) => c,
        Err(e) => {
            let inner = e.into_inner();
            let Some(location) = inner.location() else {
                anyhow::bail!("Failed to deserialize compose file.")
            };
            let report = miette::miette!(
                labels = vec![LabeledSpan::at(location.index(), inner.to_string())],
                "Failed to deserialize compose file at {}",
                full_path.display()
            )
            .with_source_code(file_payload);
            anyhow::bail!("{report:?}");
        }
    };

    // Try to load the .env from the same directory as the docker-compose file.
    let dotenv_file = full_path.parent().expect("a directory").join(".env");
    dotenvy::from_path(dotenv_file).ok();

    let project_name = std::env::var("COMPOSE_PROJECT_NAME").unwrap_or_else(|_| {
        let components = full_path.components().collect::<Vec<_>>();
        components
            .get(components.len().saturating_sub(2))
            .expect("Failed to determine project name.")
            .as_os_str()
            .to_string_lossy()
            .into_owned()
    });

    let mut container_name_mapping = IndexMap::new();
    for (i, (service_name, info)) in compose_content.services.clone().0.iter().enumerate() {
        let service_name = if let Some(info) = info {
            if let Some(container_name) = &info.container_name {
                container_name.clone()
            } else {
                // We don't scale services, the 1 index should be fine.
                format!("{project_name}-{service_name}-1")
            }
        } else {
            format!("{project_name}-{service_name}-1")
        };
        container_name_mapping.insert(i, service_name.clone());
    }

    let docker_version = docker
        .version()
        .await?
        .version
        .unwrap_or_else(|| "unknown".to_string());

    let mut app = App::new(
        project_name,
        compose_content,
        container_name_mapping,
        running_container_names,
        docker.clone(),
        file,
        full_path,
        docker_version,
    );

    app.start_all_log_streaming().await?;
    app.fetch_all_container_info().await?;

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    // We may send 2 messages in one frame, so we need that to be buffered to avoid waiting indefinitely on the sender side.
    let (tx, mut rx) = tokio::sync::mpsc::channel(2);

    while app.running {
        tui.draw(&mut app)?;

        match tui.events.next().await? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, &mut app, tx.clone()).await?,
            Event::Mouse(mouse_event) => {
                handle_mouse_events(mouse_event, &mut app, tx.clone()).await?;
            }
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

    tui.exit()?;
    Ok(())
}
