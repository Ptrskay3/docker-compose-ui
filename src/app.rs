use std::{collections::HashMap, error, process::Stdio};

use bollard::{container::ListContainersOptions, Docker};
use docker_compose_types::Compose;
use ratatui::widgets::ListState;
use tokio::process::{Child, Command};

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    pub compose_content: ComposeList,
    pub running_container_names: Vec<String>,
    pub docker: Docker,
    pub target: String,
}

#[derive(Debug)]
pub struct ComposeList {
    pub compose: Compose,
    pub state: ListState,
    pub queued: Vec<usize>,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(
        compose: Compose,
        running_container_names: Vec<String>,
        docker: Docker,
        target: String,
    ) -> Self {
        let mut state = ListState::default();
        state.select_first();
        Self {
            compose_content: ComposeList {
                compose,
                state,
                queued: vec![],
            },
            running: true,
            running_container_names,
            docker,
            target,
        }
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn up(&mut self) {
        self.compose_content.state.select_previous();
    }

    pub fn down(&mut self) {
        self.compose_content.state.select_next();
    }

    pub fn queue(&mut self) {
        if let Some(selected) = self.compose_content.state.selected() {
            self.compose_content.queued.push(selected);
            self.compose_content.queued.dedup();
        }
    }
    pub fn queue_all(&mut self) {
        self.compose_content.queued.clear();
        let all = self.compose_content.compose.services.0.keys().count();
        self.compose_content.queued.extend(0..all);
    }

    pub fn dc(&mut self, up: bool) -> Option<Child> {
        if let Some(selected) = self.compose_content.state.selected() {
            let key = &self.compose_content.compose.services.0.keys()[selected];

            let child = if up {
                Command::new("docker")
                    .args(["compose", "-f", &self.target, "up", &key, "-d"])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .stdin(Stdio::null())
                    .spawn()
                    .unwrap()
            } else {
                Command::new("docker")
                    .args(["compose", "-f", &self.target, "down", &key])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .stdin(Stdio::null())
                    .spawn()
                    .unwrap()
            };
            return Some(child);
        }
        None
    }

    pub fn all(&mut self) -> Child {
        let child = Command::new("docker")
            .args(["compose", "-f", &self.target, "up", "-d"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
            .unwrap();

        child
    }

    pub async fn refresh(&mut self) -> AppResult<()> {
        let mut list_container_filters = HashMap::new();
        list_container_filters.insert("status", vec!["running"]);

        let containers = &self
            .docker
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters: list_container_filters,
                ..Default::default()
            }))
            .await?;

        self.running_container_names = containers
            .iter()
            .cloned()
            .flat_map(|c| c.names)
            .flatten()
            .map(|name| name.trim_start_matches("/").into())
            .collect::<Vec<String>>();
        self.compose_content.queued = vec![];
        Ok(())
    }
}
