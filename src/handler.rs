use crate::app::{App, AppResult};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::Sender;

pub enum DockerEvent {
    Refresh,
    ErrorLog(String),
    ContainerLog(String),
}

pub enum QueueType {
    Stop,
    Start,
}

/// Handles the key events and updates the state of [`App`].
pub async fn handle_key_events(
    key_event: KeyEvent,
    app: &mut App,
    tx: Sender<DockerEvent>,
) -> AppResult<()> {
    match key_event.code {
        // Exit application on `ESC` or `q`
        KeyCode::Esc | KeyCode::Char('q') => {
            if app.show_popup {
                app.show_popup = false;
            } else {
                app.quit();
            }
        }
        // Exit application on `Ctrl-C`
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit();
            }
        }

        KeyCode::Up => {
            if key_event.modifiers == KeyModifiers::SHIFT {
                app.up_first();
                return Ok(());
            }
            app.up();
        }

        KeyCode::Down => {
            if key_event.modifiers == KeyModifiers::SHIFT {
                app.down_last();
                return Ok(());
            }
            app.down();
        }

        KeyCode::Enter => {
            if app.show_popup {
                app.show_popup = false;
                return Ok(());
            }
            app.clear_latest_error_log();

            if let Some(child) = app.dc(true) {
                app.queue(QueueType::Start);
                tokio::spawn(async move {
                    let op = child.wait_with_output().await.unwrap();
                    if op.status.success() {
                        tx.send(DockerEvent::Refresh).await.unwrap()
                    } else {
                        tx.send(DockerEvent::ErrorLog(
                            String::from_utf8_lossy(&op.stderr).into(),
                        ))
                        .await
                        .unwrap()
                    }
                });
            }
        }
        KeyCode::Char('s') => {
            app.clear_latest_error_log();

            if let Some(child) = app.dc(false) {
                app.queue(QueueType::Stop);
                tokio::spawn(async move {
                    let op = child.wait_with_output().await.unwrap();
                    if op.status.success() {
                        tx.send(DockerEvent::Refresh).await.unwrap()
                    } else {
                        tx.send(DockerEvent::ErrorLog(
                            String::from_utf8_lossy(&op.stderr).into(),
                        ))
                        .await
                        .unwrap()
                    }
                });
            }
        }

        KeyCode::Char('f') => {
            app.refresh().await?;
        }

        KeyCode::Char('a') => {
            app.clear_latest_error_log();
            let child = app.all();
            app.queue_all(QueueType::Start);
            tokio::spawn(async move {
                let op = child.wait_with_output().await.unwrap();
                if op.status.success() {
                    tx.send(DockerEvent::Refresh).await.unwrap()
                } else {
                    tx.send(DockerEvent::ErrorLog(
                        String::from_utf8_lossy(&op.stderr).into(),
                    ))
                    .await
                    .unwrap()
                }
            });
        }
        KeyCode::Char('x') => {
            app.clear_latest_error_log();
            let child = app.down_all();
            app.queue_all(QueueType::Stop);
            tokio::spawn(async move {
                let op = child.wait_with_output().await.unwrap();
                if op.status.success() {
                    tx.send(DockerEvent::Refresh).await.unwrap()
                } else {
                    tx.send(DockerEvent::ErrorLog(
                        String::from_utf8_lossy(&op.stderr).into(),
                    ))
                    .await
                    .unwrap()
                }
            });
        }
        KeyCode::Char('r') => {
            app.clear_latest_error_log();
            if let Some(child) = app.restart() {
                app.queue(QueueType::Start);
                tokio::spawn(async move {
                    let op = child.wait_with_output().await.unwrap();
                    if op.status.success() {
                        tx.send(DockerEvent::Refresh).await.unwrap()
                    } else {
                        tx.send(DockerEvent::ErrorLog(
                            String::from_utf8_lossy(&op.stderr).into(),
                        ))
                        .await
                        .unwrap()
                    }
                });
            }
        }
        KeyCode::Char(c) if ['1', '2', '3', '4', '5'].contains(&c) => {
            app.toggle_modifier(c);
        }

        KeyCode::Char('l') => {
            if let Some(logs) = app.stream_container_logs().await {
                tx.send(DockerEvent::ContainerLog(logs)).await.unwrap();
            }
        }

        // Other handlers you could add here.
        _ => {}
    }
    Ok(())
}
