use super::*;
use crate::entities::{project, task};
use crate::storage::{remove_test_database, LocalStorage};
use chrono::Local;
use ratatui::{backend::TestBackend, buffer::Buffer, style::Color, Terminal};
use std::{fmt::Write as _, path::Path, process::Command, sync::Arc};
use tokio::sync::Mutex;

fn color_hex(color: Color, fallback: &str) -> String {
    match color {
        Color::Black => "#1e2030".into(),
        Color::Red | Color::LightRed => "#ed8796".into(),
        Color::Green | Color::LightGreen => "#a6da95".into(),
        Color::Yellow | Color::LightYellow => "#eed49f".into(),
        Color::Blue | Color::LightBlue => "#8aadf4".into(),
        Color::Magenta | Color::LightMagenta => "#c6a0f6".into(),
        Color::Cyan | Color::LightCyan => "#8bd5ca".into(),
        Color::Gray | Color::White => "#cad3f5".into(),
        Color::DarkGray => "#6e738d".into(),
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Indexed(index) => format!("rgb({0},{0},{0})", 32 + index.saturating_mul(2)),
        Color::Reset => fallback.into(),
    }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn buffer_to_png(buffer: &Buffer, path: &Path) {
    const CELL_WIDTH: u16 = 10;
    const CELL_HEIGHT: u16 = 20;
    let width = buffer.area.width * CELL_WIDTH;
    let height = buffer.area.height * CELL_HEIGHT;
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}"><rect width="100%" height="100%" fill="#24273a"/><g font-family="Menlo, Monaco, monospace" font-size="16" xml:space="preserve">"##
    );

    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            let background = color_hex(cell.bg, "#24273a");
            if background != "#24273a" {
                let _ = write!(
                    svg,
                    r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#,
                    x * CELL_WIDTH,
                    y * CELL_HEIGHT,
                    CELL_WIDTH,
                    CELL_HEIGHT,
                    background
                );
            }
            if cell.symbol().trim().is_empty() {
                continue;
            }
            let foreground = color_hex(cell.fg, "#cad3f5");
            let weight = if cell.modifier.contains(ratatui::style::Modifier::BOLD) {
                "bold"
            } else {
                "normal"
            };
            let _ = write!(
                svg,
                r#"<text x="{}" y="{}" fill="{}" font-weight="{}">{}</text>"#,
                x * CELL_WIDTH,
                y * CELL_HEIGHT + 16,
                foreground,
                weight,
                xml_escape(cell.symbol())
            );
        }
    }
    svg.push_str("</g></svg>");

    let svg_path = path.with_extension("svg");
    std::fs::write(&svg_path, svg).unwrap();
    let font = std::env::var("TERMINALIST_SCREENSHOT_FONT").unwrap_or_else(|_| {
        [
            "/System/Library/Fonts/Monaco.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        ]
        .into_iter()
        .find(|candidate| Path::new(candidate).exists())
        .expect("set TERMINALIST_SCREENSHOT_FONT to a monospace font file")
        .into()
    });
    let status = Command::new("magick")
        .arg("-font")
        .arg(font)
        .arg(&svg_path)
        .arg(path)
        .status()
        .unwrap();
    assert!(status.success(), "ImageMagick failed to render {}", path.display());
    std::fs::remove_file(svg_path).unwrap();
}

fn project(name: &str, backend_uuid: Uuid, order_index: i32) -> project::Model {
    project::Model {
        uuid: Uuid::new_v4(),
        backend_uuid,
        remote_id: format!("project-{order_index}"),
        name: name.into(),
        is_favorite: order_index < 2,
        is_inbox_project: order_index == 0,
        order_index,
        parent_uuid: None,
    }
}

fn task(content: &str, project_uuid: Uuid, backend_uuid: Uuid, order_index: i32) -> task::Model {
    task::Model {
        uuid: Uuid::new_v4(),
        backend_uuid,
        remote_id: format!("task-{order_index}"),
        content: content.into(),
        description: (order_index == 0).then(|| "Keep the release steps accurate and repeatable.".into()),
        project_uuid,
        section_uuid: None,
        parent_uuid: None,
        priority: match order_index % 4 {
            0 => 4,
            1 => 3,
            2 => 2,
            _ => 1,
        },
        order_index,
        due_date: Some(Local::now().format("%Y-%m-%d").to_string()),
        due_datetime: None,
        is_recurring: order_index == 4,
        deadline: None,
        duration: None,
        is_completed: false,
        completed_at: None,
        is_deleted: false,
        deleted_at: None,
    }
}

#[tokio::test]
#[ignore = "writes README screenshot assets on explicit request"]
async fn generate_readme_screenshots() {
    let output_dir = std::env::var("TERMINALIST_SCREENSHOT_DIR")
        .expect("set TERMINALIST_SCREENSHOT_DIR to the README image directory");
    std::fs::create_dir_all(&output_dir).unwrap();

    let db_path = std::env::temp_dir().join(format!("terminalist-screenshots-{}.db", Uuid::new_v4()));
    let storage = Arc::new(Mutex::new(LocalStorage::new_at(db_path.clone()).await.unwrap()));
    let sync_service = SyncService::new_for_test(storage.clone(), Uuid::new_v4());
    let mut app = AppComponent::new(sync_service, Config::default());
    let backend_uuid = Uuid::new_v4();
    let projects = vec![
        project("Inbox", backend_uuid, 0),
        project("Terminalist", backend_uuid, 1),
        project("Personal", backend_uuid, 2),
        project("Reading", backend_uuid, 3),
    ];
    let tasks = vec![
        task("Review the release checklist", projects[1].uuid, backend_uuid, 0),
        task(
            "Plan the next Terminalist improvement",
            projects[1].uuid,
            backend_uuid,
            1,
        ),
        task("Reply to project feedback", projects[0].uuid, backend_uuid, 2),
        task("Schedule a focused work block", projects[2].uuid, backend_uuid, 3),
        task("Weekly planning review", projects[2].uuid, backend_uuid, 4),
        task("Read the saved Rust article", projects[3].uuid, backend_uuid, 5),
    ];
    let mut navigation_counts = NavigationCounts {
        today: tasks.len(),
        tomorrow: 2,
        upcoming: 5,
        trash: 1,
        ..NavigationCounts::default()
    };
    for project in &projects {
        navigation_counts.projects.insert(
            project.uuid,
            tasks.iter().filter(|task| task.project_uuid == project.uuid).count(),
        );
    }
    app.apply_snapshot(ViewSnapshot {
        generation: 1,
        selection: SidebarSelection::Today,
        is_initial: false,
        projects: projects.clone(),
        labels: Vec::new(),
        sections: Vec::new(),
        tasks: tasks.clone(),
        all_tasks: tasks.clone(),
        navigation_counts,
    });
    app.sync_component_data();

    let backend = TestBackend::new(134, 39);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.render(frame, frame.area())).unwrap();
    buffer_to_png(
        terminal.backend().buffer(),
        &Path::new(&output_dir).join("screenshot1.png"),
    );

    app.dialog.dialog_type = Some(DialogType::TaskDetails {
        task: Box::new(tasks[0].clone()),
    });
    let backend = TestBackend::new(134, 39);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.render(frame, frame.area())).unwrap();
    buffer_to_png(
        terminal.backend().buffer(),
        &Path::new(&output_dir).join("screenshot2.png"),
    );

    app.task_manager.cancel_all_tasks_and_wait().await;
    drop(app);
    let storage = match Arc::try_unwrap(storage) {
        Ok(storage) => storage.into_inner(),
        Err(storage) => panic!("screenshot storage still has {} owners", Arc::strong_count(&storage)),
    };
    storage.conn.close().await.unwrap();
    remove_test_database(db_path).await.unwrap();
}
