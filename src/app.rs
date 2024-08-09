use std::{collections::HashMap, error, process::Stdio};

use bollard::{container::ListContainersOptions, Docker};
use docker_compose_types::Compose;
use ratatui::widgets::ListState;
use tokio::process::{Child, Command};

use crate::handler::QueueType;

bitflags::bitflags! {
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub struct DockerModifier: u8 {
        const BUILD = 1 << 0;
        const FORCE_RECREATE = 1 << 1;
        const NO_DEPS = 1 << 2;
        const PULL_ALWAYS = 1 << 3;
        const ABORT_ON_CONTAINER_FAILURE = 1 << 4;
    }
}

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
    pub start_queued: Vec<usize>,
    pub stop_queued: Vec<usize>,
    pub modifiers: DockerModifier,
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
                start_queued: vec![],
                stop_queued: vec![],
                modifiers: DockerModifier::empty(),
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

    pub fn down_all(&mut self) -> Child {
        let child = Command::new("docker")
            .args(["compose", "-f", &self.target, "down"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
            .unwrap();

        child
    }

    pub fn queue(&mut self, queue_type: QueueType) {
        if let Some(selected) = self.compose_content.state.selected() {
            match queue_type {
                QueueType::Stop => {
                    self.compose_content.stop_queued.push(selected);
                    self.compose_content.stop_queued.dedup();
                }
                QueueType::Start => {
                    self.compose_content.start_queued.push(selected);
                    self.compose_content.start_queued.dedup();
                }
            }
        }
    }
    pub fn queue_all(&mut self, queue_type: QueueType) {
        match queue_type {
            QueueType::Start => {
                self.compose_content.start_queued.clear();
                let all = self.compose_content.compose.services.0.keys().count();
                self.compose_content.start_queued.extend(0..all);
            }
            QueueType::Stop => {
                self.compose_content.stop_queued.clear();
                let all = self.compose_content.compose.services.0.keys().count();
                self.compose_content.stop_queued.extend(0..all);
            }
        }
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
    pub fn restart(&mut self) -> Option<Child> {
        if let Some(selected) = self.compose_content.state.selected() {
            let key = &self.compose_content.compose.services.0.keys()[selected];

            let child = Command::new("docker")
                .args(["compose", "-f", &self.target, "restart", &key])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null())
                .spawn()
                .unwrap();

            return Some(child);
        }
        None
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
        // TODO: Don't clear everything, there may be valid pending stuff in there.
        self.compose_content.start_queued = vec![];
        self.compose_content.stop_queued = vec![];
        Ok(())
    }
}
