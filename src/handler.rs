use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use tokio::sync::mpsc::Sender;

#[derive(Debug)]
pub enum DockerEvent {
    Refresh,
    ErrorLog(String),
}

pub enum QueueType {
    Stop,
    Start,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlternateScreenContent {
    Help,
    ContainerDetails(SplitScreen),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SplitScreen {
    UpperLeft,
    LowerLeft,
    UpperRight,
    LowerRight,
}

impl SplitScreen {
    fn transition(self) -> Self {
        match self {
            SplitScreen::UpperLeft => SplitScreen::LowerLeft,
            SplitScreen::LowerLeft => SplitScreen::UpperRight,
            SplitScreen::UpperRight => SplitScreen::LowerRight,
            SplitScreen::LowerRight => SplitScreen::UpperLeft,
        }
    }
    fn transition_back(self) -> Self {
        match self {
            SplitScreen::LowerLeft => SplitScreen::UpperLeft,
            SplitScreen::UpperRight => SplitScreen::LowerLeft,
            SplitScreen::LowerRight => SplitScreen::UpperRight,
            SplitScreen::UpperLeft => SplitScreen::LowerRight,
        }
    }
}

/// Handles the key events and updates the state of [`App`].
pub async fn handle_key_events(
    key_event: KeyEvent,
    app: &mut App,
    tx: Sender<DockerEvent>,
) -> anyhow::Result<()> {
    match key_event.code {
        // Exit application on `ESC` or `q`
        KeyCode::Esc | KeyCode::Char('q') => {
            match app.alternate_screen_content {
                AlternateScreenContent::Help | AlternateScreenContent::ContainerDetails(_) => {
                    app.alternate_screen_content = AlternateScreenContent::None;
                    return Ok(());
                }
                e @ AlternateScreenContent::None => e,
            };
            if app.show_popup {
                app.show_popup = false;
                app.reset_popup_scroll();
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
                app.up_first(tx.clone());
                return Ok(());
            }
            app.up(tx.clone());
            app.reset_scroll();
        }

        KeyCode::Down => {
            if key_event.modifiers == KeyModifiers::SHIFT {
                app.down_last(tx.clone());
                return Ok(());
            }
            app.down(tx.clone());
            app.reset_scroll();
        }

        KeyCode::Enter => {
            match app.alternate_screen_content {
                AlternateScreenContent::Help | AlternateScreenContent::ContainerDetails(_) => {
                    app.alternate_screen_content = AlternateScreenContent::None;
                    return Ok(());
                }
                _ => {}
            };
            if app.show_popup {
                app.show_popup = false;
                app.reset_popup_scroll();
                return Ok(());
            }
            app.clear_latest_error_log();

            if let Some(child) = app.dc(true) {
                app.queue(QueueType::Start);
                tokio::spawn(async move {
                    let op = child.wait_with_output().await.unwrap();
                    if !op.status.success() {
                        tx.send(DockerEvent::ErrorLog(
                            String::from_utf8_lossy(&op.stderr).into(),
                        ))
                        .await
                        .unwrap()
                    }
                    tx.send(DockerEvent::Refresh).await.unwrap()
                });
            }
        }
        KeyCode::Char('s') => {
            app.clear_latest_error_log();

            if let Some(child) = app.dc(false) {
                app.queue(QueueType::Stop);
                tokio::spawn(async move {
                    let op = child.wait_with_output().await.unwrap();
                    if !op.status.success() {
                        tx.send(DockerEvent::ErrorLog(
                            String::from_utf8_lossy(&op.stderr).into(),
                        ))
                        .await
                        .unwrap()
                    }
                    tx.send(DockerEvent::Refresh).await.unwrap()
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
                if !op.status.success() {
                    tx.send(DockerEvent::ErrorLog(
                        String::from_utf8_lossy(&op.stderr).into(),
                    ))
                    .await
                    .unwrap()
                }
                tx.send(DockerEvent::Refresh).await.unwrap();
            });
        }
        KeyCode::Char('l') if key_event.modifiers == KeyModifiers::CONTROL => {
            app.clear_current_log();
        }
        KeyCode::Char('x') => {
            app.clear_latest_error_log();
            let child = app.down_all();
            app.queue_all(QueueType::Stop);
            tokio::spawn(async move {
                let op = child.wait_with_output().await.unwrap();
                if !op.status.success() {
                    tx.send(DockerEvent::ErrorLog(
                        String::from_utf8_lossy(&op.stderr).into(),
                    ))
                    .await
                    .unwrap()
                }
                tx.send(DockerEvent::Refresh).await.unwrap();
            });
        }
        KeyCode::Char('r') => {
            app.clear_latest_error_log();
            if let Some(child) = app.restart() {
                app.queue(QueueType::Start);
                tokio::spawn(async move {
                    let op = child.wait_with_output().await.unwrap();
                    if !op.status.success() {
                        tx.send(DockerEvent::ErrorLog(
                            String::from_utf8_lossy(&op.stderr).into(),
                        ))
                        .await
                        .unwrap()
                    }
                    tx.send(DockerEvent::Refresh).await.unwrap()
                });
            }
        }
        KeyCode::Char(c) if ['1', '2', '3', '4', '5'].contains(&c) => {
            app.toggle_modifier(c);
        }

        KeyCode::Char('j') | KeyCode::PageUp => scroll_up(app, 1),
        KeyCode::Char('k') | KeyCode::PageDown => scroll_down(app, 1),

        KeyCode::Char('w') if key_event.modifiers == KeyModifiers::CONTROL => {
            app.clear_current_log();
            app.remove_container(true, tx.clone()).await?;
        }
        KeyCode::Char('w')
            if key_event.modifiers == (KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            app.clear_current_log();
            app.wipe(true, tx.clone()).await?;
        }
        KeyCode::Char('h') => {
            if app.alternate_screen_content != AlternateScreenContent::Help {
                app.alternate_screen_content = AlternateScreenContent::Help;
            } else {
                app.alternate_screen_content = AlternateScreenContent::None;
            }
        }
        KeyCode::Char('e') => {
            if !matches!(
                app.alternate_screen_content,
                AlternateScreenContent::ContainerDetails(_)
            ) {
                app.alternate_screen_content =
                    AlternateScreenContent::ContainerDetails(SplitScreen::UpperLeft);
            } else {
                app.alternate_screen_content = AlternateScreenContent::None;
            }
        }
        KeyCode::BackTab => {
            if let AlternateScreenContent::ContainerDetails(state) = app.alternate_screen_content {
                app.alternate_screen_content =
                    AlternateScreenContent::ContainerDetails(state.transition_back());
            }
        }
        KeyCode::Tab => {
            if let AlternateScreenContent::ContainerDetails(state) = app.alternate_screen_content {
                app.alternate_screen_content =
                    AlternateScreenContent::ContainerDetails(state.transition());
            }
        }

        _ => {}
    }
    Ok(())
}

pub async fn handle_mouse_events(
    mouse_event: MouseEvent,
    app: &mut App,
    _tx: Sender<DockerEvent>,
) -> anyhow::Result<()> {
    match mouse_event.kind {
        MouseEventKind::ScrollUp => scroll_up(app, 5),
        MouseEventKind::ScrollDown => scroll_down(app, 5),
        _ => {}
    }
    Ok(())
}

fn scroll_up(app: &mut App, amount: usize) {
    if app.show_popup {
        app.popup_scroll = app.popup_scroll.saturating_sub(amount);
        app.popup_scroll_state = app.popup_scroll_state.position(app.popup_scroll);
    } else if let AlternateScreenContent::ContainerDetails(split_screen) =
        app.alternate_screen_content
    {
        match split_screen {
            SplitScreen::UpperLeft => {
                app.alternate_screen.upper_left_scroll = app
                    .alternate_screen
                    .upper_left_scroll
                    .saturating_sub(amount);
                app.alternate_screen.upper_left_scroll_state = app
                    .alternate_screen
                    .upper_left_scroll_state
                    .position(app.alternate_screen.upper_left_scroll);
            }
            SplitScreen::LowerLeft => {
                app.alternate_screen.lower_left_scroll = app
                    .alternate_screen
                    .lower_left_scroll
                    .saturating_sub(amount);
                app.alternate_screen.lower_left_scroll_state = app
                    .alternate_screen
                    .lower_left_scroll_state
                    .position(app.alternate_screen.lower_left_scroll);
            }
            SplitScreen::UpperRight => {
                app.alternate_screen.upper_right_scroll = app
                    .alternate_screen
                    .upper_right_scroll
                    .saturating_sub(amount);
                app.alternate_screen.upper_right_scroll_state = app
                    .alternate_screen
                    .upper_right_scroll_state
                    .position(app.alternate_screen.upper_right_scroll);
            }
            SplitScreen::LowerRight => {
                app.alternate_screen.lower_right_scroll = app
                    .alternate_screen
                    .lower_right_scroll
                    .saturating_sub(amount);
                app.alternate_screen.lower_right_scroll_state = app
                    .alternate_screen
                    .lower_right_scroll_state
                    .position(app.alternate_screen.lower_right_scroll);
            }
        }
    } else {
        app.vertical_scroll = app.vertical_scroll.saturating_sub(amount);
        app.vertical_scroll_state = app.vertical_scroll_state.position(app.vertical_scroll);
    }
}

fn scroll_down(app: &mut App, amount: usize) {
    if app.show_popup {
        app.popup_scroll = app.popup_scroll.saturating_add(amount);
        app.popup_scroll_state = app.popup_scroll_state.position(app.popup_scroll);
    } else if let AlternateScreenContent::ContainerDetails(split_screen) =
        app.alternate_screen_content
    {
        match split_screen {
            SplitScreen::UpperLeft => {
                app.alternate_screen.upper_left_scroll = app
                    .alternate_screen
                    .upper_left_scroll
                    .saturating_add(amount);
                app.alternate_screen.upper_left_scroll_state = app
                    .alternate_screen
                    .upper_left_scroll_state
                    .position(app.alternate_screen.upper_left_scroll);
            }
            SplitScreen::LowerLeft => {
                app.alternate_screen.lower_left_scroll = app
                    .alternate_screen
                    .lower_left_scroll
                    .saturating_add(amount);
                app.alternate_screen.lower_left_scroll_state = app
                    .alternate_screen
                    .lower_left_scroll_state
                    .position(app.alternate_screen.lower_left_scroll);
            }
            SplitScreen::UpperRight => {
                app.alternate_screen.upper_right_scroll = app
                    .alternate_screen
                    .upper_right_scroll
                    .saturating_add(amount);
                app.alternate_screen.upper_right_scroll_state = app
                    .alternate_screen
                    .upper_right_scroll_state
                    .position(app.alternate_screen.upper_right_scroll);
            }
            SplitScreen::LowerRight => {
                app.alternate_screen.lower_right_scroll = app
                    .alternate_screen
                    .lower_right_scroll
                    .saturating_add(amount);
                app.alternate_screen.lower_right_scroll_state = app
                    .alternate_screen
                    .lower_right_scroll_state
                    .position(app.alternate_screen.lower_right_scroll);
            }
        }
    } else {
        app.vertical_scroll = app.vertical_scroll.saturating_add(amount);
        app.vertical_scroll_state = app.vertical_scroll_state.position(app.vertical_scroll);
    }
}
