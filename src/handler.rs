use crate::app::{App, AppResult};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::Sender;

pub enum DockerEvent {
    Refresh,
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
            app.quit();
        }
        // Exit application on `Ctrl-C`
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit();
            }
        }

        KeyCode::Up => {
            app.up();
        }

        KeyCode::Down => {
            app.down();
        }

        KeyCode::Enter => {
            if let Some(mut child) = app.dc(true) {
                app.queue(QueueType::Start);
                tokio::spawn(async move {
                    let status = child.wait().await.unwrap();
                    if status.success() {
                        tx.send(DockerEvent::Refresh).await.unwrap()
                    } else {
                    }
                });
            }
        }
        KeyCode::Char('s') => {
            if let Some(mut child) = app.dc(false) {
                app.queue(QueueType::Stop);
                tokio::spawn(async move {
                    let status = child.wait().await.unwrap();
                    if status.success() {
                        tx.send(DockerEvent::Refresh).await.unwrap()
                    }
                });
            }
        }

        KeyCode::Char('f') => {
            app.refresh().await?;
        }

        KeyCode::Char('a') => {
            let mut child = app.all();
            app.queue_all(QueueType::Start);
            tokio::spawn(async move {
                let status = child.wait().await.unwrap();
                if status.success() {
                    tx.send(DockerEvent::Refresh).await.unwrap()
                }
            });
        }
        KeyCode::Char('x') => {
            let mut child = app.down_all();
            app.queue_all(QueueType::Stop);
            tokio::spawn(async move {
                let status = child.wait().await.unwrap();
                if status.success() {
                    tx.send(DockerEvent::Refresh).await.unwrap()
                }
            });
        }
        KeyCode::Char('r') => {
            if let Some(mut child) = app.restart() {
                app.queue(QueueType::Start);
                tokio::spawn(async move {
                    let status = child.wait().await.unwrap();
                    if status.success() {
                        tx.send(DockerEvent::Refresh).await.unwrap()
                    }
                });
            }
        }

        // Other handlers you could add here.
        _ => {}
    }
    Ok(())
}
