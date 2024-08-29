use std::{
    collections::HashMap,
    error,
    hash::Hash,
    process::Stdio,
    sync::{Arc, Mutex},
};

use bollard::{
    container::{ListContainersOptions, LogsOptions, RemoveContainerOptions},
    secret::ContainerInspectResponse,
    Docker,
};
use docker_compose_types::Compose;
use futures::{Stream, StreamExt};
use indexmap::IndexMap;
use tokio::{sync::mpsc::Sender, task::JoinHandle};

use ratatui::widgets::{ListState, ScrollbarState};
use tokio::process::{Child, Command};

use crate::handler::{DockerEvent, FullScreenContent, QueueType};

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
    pub project_name: String,
    pub running: bool,
    pub compose_content: ComposeList,
    pub running_container_names: Vec<String>,
    pub docker: Docker,
    pub target: String,
    pub show_popup: bool,
    pub popup_scroll: usize,
    pub popup_scroll_state: ScrollbarState,
    pub vertical_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
    pub container_name_mapping: IndexMap<usize, String>,
    pub container_info: IndexMap<usize, Option<ContainerInspectResponse>>,
    pub full_path: std::path::PathBuf,
    pub docker_version: String,
    pub full_screen_content: FullScreenContent,
    pub alternate_screen: AlternateScreen,
}

#[derive(Debug)]
pub struct AlternateScreen {
    pub upper_scroll_state: ScrollbarState,
    pub upper_scroll: usize,
    pub lower_scroll_state: ScrollbarState,
    pub lower_scroll: usize,
}

impl AlternateScreen {
    pub fn new() -> Self {
        Self {
            upper_scroll_state: ScrollbarState::default(),
            upper_scroll: 0,
            lower_scroll_state: ScrollbarState::default(),
            lower_scroll: 0,
        }
    }

    pub fn reset_scrolls(&mut self) {
        self.upper_scroll = 0;
        self.upper_scroll_state = self.upper_scroll_state.position(0);
        self.lower_scroll = 0;
        self.lower_scroll_state = self.lower_scroll_state.position(0);
    }
}

#[derive(Debug, Clone)]
pub struct StreamOptions {
    pub tail: String,
    pub all: bool,
}

impl Default for StreamOptions {
    fn default() -> Self {
        Self {
            tail: "50".into(),
            all: false,
        }
    }
}

impl From<StreamOptions> for LogsOptions<String> {
    fn from(val: StreamOptions) -> Self {
        let mut opts = LogsOptions::<String> {
            follow: true,
            stdout: true,
            stderr: true,
            tail: val.tail,
            ..Default::default()
        };

        if val.all {
            opts.tail = "all".into()
        }

        opts
    }
}

pub fn get_log_stream(
    id: &str,
    docker: &bollard::Docker,
    stream_options: StreamOptions,
) -> impl Stream<Item = String> {
    let logstream = docker
        .logs(id, Some(stream_options.into()))
        .filter_map(|res| async move {
            Some(match res {
                Ok(r) => format!("{r}"),
                Err(_err) => String::default(),
            })
        });

    Box::pin(logstream)
}

#[derive(Debug)]
pub struct ComposeList {
    pub compose: Compose,
    pub state: ListState,
    pub start_queued: Queued,
    pub stop_queued: Queued,
    pub modifiers: DockerModifier,
    pub log_streamer_handle: Arc<Mutex<IndexMap<usize, JoinHandle<()>>>>,
    pub logs: Arc<Mutex<IndexMap<usize, Vec<String>>>>,
    pub error_msg: Option<String>,
    pub stream_options: StreamOptions,
}

// TODO: Auto-scroll
impl ComposeList {
    pub async fn start_log_stream(
        &mut self,
        idx: usize,
        id: &str,
        docker: bollard::Docker,
    ) -> AppResult<()> {
        let mut logs_stream = get_log_stream(id, &docker, self.stream_options.clone());

        let log_messages = self.logs.clone();
        let mut guard = self.log_streamer_handle.lock().unwrap();
        if let Some(handle) = guard.shift_remove(&idx) {
            handle.abort();
        }
        guard.insert(
            idx,
            tokio::spawn(async move {
                while let Some(v) = logs_stream.next().await {
                    {
                        log_messages.lock().unwrap().entry(idx).or_default().push(v);
                    }
                }
            }),
        );

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct Queued {
    pub state: Vec<usize>,
    pub names: IndexMap<usize, String>,
}

impl App {
    pub fn new(
        project_name: String,
        compose: Compose,
        container_name_mapping: IndexMap<usize, String>,
        running_container_names: Vec<String>,
        docker: Docker,
        target: String,
        full_path: impl AsRef<std::path::Path>,
        docker_version: String,
    ) -> Self {
        let mut state = ListState::default();
        state.select_first();
        Self {
            project_name,
            compose_content: ComposeList {
                compose,
                state,
                start_queued: Default::default(),
                stop_queued: Default::default(),
                modifiers: DockerModifier::empty(),
                log_streamer_handle: Arc::new(Mutex::new(IndexMap::new())),
                logs: Arc::new(Mutex::new(IndexMap::new())),
                error_msg: None,
                stream_options: StreamOptions::default(),
            },
            container_name_mapping,
            show_popup: false,
            running: true,
            running_container_names,
            docker,
            target,
            vertical_scroll: 0,
            vertical_scroll_state: ScrollbarState::default(),
            popup_scroll: 0,
            popup_scroll_state: ScrollbarState::default(),
            container_info: IndexMap::new(),
            full_path: full_path.as_ref().to_path_buf(),
            docker_version,
            full_screen_content: FullScreenContent::None,
            alternate_screen: AlternateScreen::new(),
        }
    }

    pub async fn fetch_all_container_info(&mut self) -> AppResult<()> {
        for (i, name) in &self.container_name_mapping {
            if let Ok(info) = self
                .docker
                .inspect_container(name, Default::default())
                .await
            {
                self.container_info.insert(*i, Some(info));
            } else {
                self.container_info.insert(*i, None);
            }
        }

        Ok(())
    }

    pub fn reset_scroll(&mut self) {
        self.vertical_scroll = 0;
        self.vertical_scroll_state = self.vertical_scroll_state.position(0);
        self.alternate_screen.reset_scrolls();
    }

    pub fn reset_popup_scroll(&mut self) {
        self.popup_scroll_state = self.popup_scroll_state.position(0);
        self.popup_scroll = 0;
    }

    pub fn clear_current_log(&mut self) {
        if let Some(selected) = self.compose_content.state.selected() {
            *self
                .compose_content
                .logs
                .lock()
                .unwrap()
                .entry(selected)
                .or_default() = Vec::new();
        }
    }

    pub async fn restart_log_streaming(&mut self) -> AppResult<()> {
        let Some(selected) = self.compose_content.state.selected() else {
            return Ok(());
        };
        let Some(container_name) = self.container_name_mapping.get(&selected) else {
            return Ok(());
        };
        self.compose_content
            .start_log_stream(selected, container_name, self.docker.clone())
            .await?;

        Ok(())
    }

    pub async fn restart_all_log_streaming(&mut self) -> AppResult<()> {
        for (selected, container_name) in &self.container_name_mapping {
            self.compose_content
                .start_log_stream(*selected, container_name, self.docker.clone())
                .await?;
        }

        Ok(())
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

    // TODO: we may wrap around: https://docs.rs/ratatui/latest/src/demo2/tabs/recipe.rs.html#105
    pub fn up(&mut self, _tx: Sender<DockerEvent>) {
        self.compose_content.state.select_previous();
    }

    pub fn up_first(&mut self, _tx: Sender<DockerEvent>) {
        self.compose_content.state.select_first();
    }

    pub fn down(&mut self, _tx: Sender<DockerEvent>) {
        self.compose_content.state.select_next();
    }

    pub fn down_last(&mut self, _tx: Sender<DockerEvent>) {
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
                    let key = self
                        .container_name_mapping
                        .get(&selected)
                        .expect("to be set");

                    self.compose_content
                        .stop_queued
                        .names
                        .insert(selected, key.clone());

                    self.compose_content.stop_queued.state.push(selected);
                    self.compose_content.stop_queued.state.dedup();
                }
                QueueType::Start => {
                    let key = self
                        .container_name_mapping
                        .get(&selected)
                        .expect("to be set");

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
                self.compose_content.start_queued.names = self.container_name_mapping.clone();
                self.compose_content.start_queued.state.clear();
                let all = self.compose_content.compose.services.0.len();
                self.compose_content.start_queued.state.extend(0..all);
            }
            QueueType::Stop => {
                self.compose_content.start_queued.names = self.container_name_mapping.clone();
                self.compose_content.stop_queued.state.clear();
                let all = self.compose_content.compose.services.0.len();
                self.compose_content.stop_queued.state.extend(0..all);
            }
        }
    }

    pub fn dc(&mut self, up: bool) -> Option<Child> {
        let selected = self.compose_content.state.selected()?;
        let key = &self.compose_content.compose.services.0.keys()[selected];

        let child = if up {
            Command::new("docker")
                .args(["compose", "-f", &self.target, "up", key, "-d"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null())
                .args(self.compose_content.modifiers.to_args())
                .spawn()
                .unwrap()
        } else {
            Command::new("docker")
                .args(["compose", "-f", &self.target, "down", key])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null())
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
        self.compose_content
            .logs
            .lock()
            .unwrap()
            .shift_remove(&selected);

        let child = Command::new("docker")
            .args(["compose", "-f", &self.target, "restart", key])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
            .unwrap();

        Some(child)
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
                        .find_map(|(k, n)| if name == n { Some(k) } else { None })
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
                        .find_map(|(k, n)| if name == n { Some(k) } else { None })
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
            .retain(|i| !clear_start.contains(i));
        self.compose_content
            .start_queued
            .names
            .retain(|i, _| !clear_start.contains(i));

        // Whatever is not running, we should clear from the stop_queued.
        self.compose_content
            .stop_queued
            .state
            .retain(|i| clear_stop.contains(i));
        self.compose_content
            .stop_queued
            .names
            .retain(|i, _| clear_stop.contains(i));

        self.restart_all_log_streaming().await?;
        self.fetch_all_container_info().await?;

        Ok(())
    }

    // FIXME: Should run prune, not remove
    pub async fn remove_container(&mut self, v: bool, tx: Sender<DockerEvent>) -> AppResult<()> {
        let Some(selected) = self.compose_content.state.selected() else {
            return Ok(());
        };
        let container_name = &self.container_name_mapping[&selected];
        if let Err(e) = self
            .docker
            .remove_container(
                container_name,
                Some(RemoveContainerOptions {
                    v,
                    force: true,
                    ..Default::default()
                }),
            )
            .await
        {
            tx.send(DockerEvent::ErrorLog(e.to_string())).await?;
        }
        tx.send(DockerEvent::Refresh).await?;

        Ok(())
    }

    // FIXME: Should run prune, not remove
    pub async fn wipe(&mut self, v: bool, tx: Sender<DockerEvent>) -> AppResult<()> {
        let result =
            futures::future::join_all(self.container_name_mapping.values().map(|container_name| {
                let docker = self.docker.clone();

                async move {
                    docker
                        .remove_container(
                            container_name,
                            Some(RemoveContainerOptions {
                                v,
                                force: true,
                                ..Default::default()
                            }),
                        )
                        .await
                }
            }))
            .await;
        let errors = result
            .iter()
            .filter_map(|r| r.as_ref().err())
            .map(|e| e.to_string())
            .collect::<Vec<String>>();
        if !errors.is_empty() {
            tx.send(DockerEvent::ErrorLog(errors.join("\n"))).await?;
        }

        tx.send(DockerEvent::Refresh).await?;

        Ok(())
    }

    pub fn clear_starting(&mut self) {
        self.compose_content.start_queued.state.clear();
        self.compose_content.start_queued.names.clear();
    }
}
