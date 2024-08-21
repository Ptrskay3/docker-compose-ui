use std::{collections::HashMap, error, process::Stdio};

use bollard::{
    container::{ListContainersOptions, LogsOptions},
    Docker,
};
use docker_compose_types::Compose;
use futures::StreamExt;
use ratatui::widgets::{ListState, ScrollbarState};
use tokio::process::{Child, Command};

use crate::handler::QueueType;

bitflags::bitflags! {
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub struct DockerModifier: u8 {
        const BUILD = 1 << 1;
        const FORCE_RECREATE = 1 << 2;
        const PULL_ALWAYS = 1 << 3;
        const ABORT_ON_CONTAINER_FAILURE = 1 << 4;
        const NO_DEPS = 1 << 5;
    }
}

impl DockerModifier {
    pub fn to_args(&self) -> Vec<&str> {
        let mut args = vec![];
        if self.contains(DockerModifier::BUILD) {
            args.push("--build");
        }
        if self.contains(DockerModifier::FORCE_RECREATE) {
            args.push("--force-recreate");
        }
        if self.contains(DockerModifier::PULL_ALWAYS) {
            args.extend(["--pull", "always"]);
        }
        if self.contains(DockerModifier::ABORT_ON_CONTAINER_FAILURE) {
            args.push("--abort-on-container-exit");
        }
        if self.contains(DockerModifier::NO_DEPS) {
            args.push("--no-deps");
        }
        args
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
    pub show_popup: bool,
    pub vertical_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
}

#[derive(Debug)]
pub struct ComposeList {
    pub compose: Compose,
    pub state: ListState,
    pub start_queued: Queued,
    pub stop_queued: Queued,
    pub modifiers: DockerModifier,
    pub log_area_content: Option<String>,
    pub error_msg: Option<String>,
}

#[derive(Debug, Default)]
pub struct Queued {
    pub state: Vec<usize>,
    pub names: HashMap<usize, String>,
}

impl App {
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
                start_queued: Default::default(),
                stop_queued: Default::default(),
                modifiers: DockerModifier::empty(),
                log_area_content: None,
                error_msg: None,
            },
            show_popup: false,
            running: true,
            running_container_names,
            docker,
            target,
            vertical_scroll_state: ScrollbarState::default(),
            vertical_scroll: 0,
        }
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn set_error_log(&mut self, error: String) {
        self.compose_content.error_msg = Some(error);
    }

    pub fn set_container_log(&mut self, content: String) {
        self.compose_content.log_area_content = Some(content);
    }

    pub fn clear_latest_error_log(&mut self) {
        self.compose_content.error_msg = None;
    }

    pub fn toggle_modifier(&mut self, modifier: char) {
        // SAFETY: The caller only passes numeric chars.
        let code = 1 << (modifier as u8);
        self.compose_content
            .modifiers
            .toggle(DockerModifier::from_bits_truncate(code));
    }

    pub fn up(&mut self) {
        self.compose_content.state.select_previous();
    }

    pub fn up_first(&mut self) {
        self.compose_content.state.select_first();
    }

    pub fn down(&mut self) {
        self.compose_content.state.select_next();
    }
    pub fn down_last(&mut self) {
        self.compose_content.state.select_last();
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
                    let key = &self.compose_content.compose.services.0.keys()[selected];
                    self.compose_content
                        .stop_queued
                        .names
                        .insert(selected, key.clone());

                    self.compose_content.stop_queued.state.push(selected);
                    self.compose_content.stop_queued.state.dedup();
                }
                QueueType::Start => {
                    let key = &self.compose_content.compose.services.0.keys()[selected];
                    self.compose_content
                        .start_queued
                        .names
                        .insert(selected, key.clone());

                    self.compose_content.start_queued.state.push(selected);
                    self.compose_content.start_queued.state.dedup();
                }
            }
        }
    }
    pub fn queue_all(&mut self, queue_type: QueueType) {
        match queue_type {
            QueueType::Start => {
                self.compose_content.start_queued.names = self
                    .compose_content
                    .compose
                    .services
                    .0
                    .iter()
                    .enumerate()
                    .map(|(i, (k, _))| (i, k.clone()))
                    .collect();

                self.compose_content.start_queued.state.clear();
                let all = self.compose_content.compose.services.0.len();
                self.compose_content.start_queued.state.extend(0..all);
            }
            QueueType::Stop => {
                self.compose_content.stop_queued.names = self
                    .compose_content
                    .compose
                    .services
                    .0
                    .iter()
                    .enumerate()
                    .map(|(i, (k, _))| (i, k.clone()))
                    .collect();
                self.compose_content.stop_queued.state.clear();
                let all = self.compose_content.compose.services.0.len();
                self.compose_content.stop_queued.state.extend(0..all);
            }
        }
    }

    pub fn dc(&mut self, up: bool) -> Option<Child> {
        let selected = self.compose_content.state.selected()?;
        let key = &self.compose_content.compose.services.0.keys()[selected];

        let args = &self.compose_content.modifiers.to_args();

        let child = if up {
            Command::new("docker")
                .args(["compose", "-f", &self.target, "up", &key, "-d"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null())
                .args(args)
                .spawn()
                .unwrap()
        } else {
            Command::new("docker")
                .args(["compose", "-f", &self.target, "down", &key])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null())
                .args(args)
                .spawn()
                .unwrap()
        };
        Some(child)
    }

    pub fn all(&mut self) -> Child {
        let args = &self.compose_content.modifiers.to_args();

        let child = Command::new("docker")
            .args(["compose", "-f", &self.target, "up", "-d"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .args(args)
            .spawn()
            .unwrap();

        child
    }
    pub fn restart(&mut self) -> Option<Child> {
        let selected = self.compose_content.state.selected()?;
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

        let clear_start =
            self.running_container_names
                .iter()
                .enumerate()
                .fold(vec![], |mut acc, (_, name)| {
                    if let Some(index) = self
                        .compose_content
                        .start_queued
                        .names
                        .iter()
                        .find_map(|(k, n)| if n == name { Some(k) } else { None })
                        .cloned()
                    {
                        acc.push(index);
                    }
                    acc
                });

        let clear_stop =
            self.running_container_names
                .iter()
                .enumerate()
                .fold(vec![], |mut acc, (_, name)| {
                    if let Some(index) = self
                        .compose_content
                        .stop_queued
                        .names
                        .iter()
                        .find_map(|(k, n)| if n == name { Some(k) } else { None })
                        .cloned()
                    {
                        acc.push(index);
                    }
                    acc
                });

        // Whatever is already running, we should clear from the start_queued.
        self.compose_content
            .start_queued
            .state
            .retain(|i| !clear_start.contains(&i));
        self.compose_content
            .start_queued
            .names
            .retain(|i, _| !clear_start.contains(&i));

        // Whatever is not running, we should clear from the stop_queued.
        self.compose_content
            .stop_queued
            .state
            .retain(|i| clear_stop.contains(&i));
        self.compose_content
            .stop_queued
            .names
            .retain(|i, _| clear_stop.contains(&i));

        Ok(())
    }

    pub async fn stream_container_logs(&self) -> Option<String> {
        if let Some(_selected) = self.compose_content.state.selected() {
            // TODO: Needs work: do it in the background, and a lot of unwraps.
            // let key = &self.compose_content.compose.services.0.keys()[selected];
            // let mut list_container_filters = HashMap::new();
            // list_container_filters.insert("status", vec!["running"]);
            // let containers = &self
            //     .docker
            //     .list_containers(Some(ListContainersOptions {
            //         all: true,
            //         filters: list_container_filters,
            //         ..Default::default()
            //     }))
            //     .await
            //     .unwrap();

            // let c = containers
            //     .iter()
            //     .cloned()
            //     .flat_map(|c| c.names)
            //     .flatten()
            //     .map(|name| name.trim_start_matches("/").into())
            //     .collect::<Vec<String>>();

            // TODO: Needs work: match those to their real names.. probably we should do this at the startup

            let key = "docker-ratatui-redis-1";
            let options = Some(LogsOptions::<String> {
                stdout: true,
                timestamps: false,
                since: 0,
                ..Default::default()
            });

            let mut logs = self.docker.logs(key, options);
            let mut output = vec![];

            while let Some(Ok(value)) = logs.next().await {
                let data = value.to_string();
                if !data.trim().is_empty() {
                    output.push(data);
                }
            }

            Some(output.join(""))
        } else {
            None
        }
        //     let options = Some(LogsOptions {
        //         stdout: true,
        //         stderr: false,
        //         tail: String::from("all"),
        //         ..Default::default()
        //     });
        //     let logs = self
        //         .docker
        //         .logs(&key, options.clone())
        //         .try_collect::<Vec<_>>()
        //         .await
        //         .unwrap();
        //     let logs = logs.first().unwrap();

        //     if let LogOutput::StdOut { message } = logs {
        //         return Some(String::from_utf8_lossy(message).into());
        //     } else {
        //         None
        //     }
        // } else {
        //     None
        // }
    }
}
