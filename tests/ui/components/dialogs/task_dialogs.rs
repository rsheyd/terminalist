use ratatui::{backend::Backend, backend::TestBackend, Terminal};
use terminalist::entities::task;
use terminalist::ui::components::dialogs::task_dialogs::{render_task_details_dialog, render_task_time_dialog};
use uuid::Uuid;

#[test]
fn time_dialog_renders_input_and_places_cursor_after_it() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| render_task_time_dialog(frame, frame.area(), "2pm", 3))
        .unwrap();

    let cursor = terminal.backend_mut().get_cursor_position().unwrap();
    let buffer = terminal.backend().buffer();
    assert!(cursor.x >= 3, "cursor must leave room for the typed value");
    let before_cursor = (cursor.x - 3..cursor.x)
        .map(|x| buffer[(x, cursor.y)].symbol())
        .collect::<String>();

    assert_eq!(
        before_cursor, "2pm",
        "typed value must be visible immediately before the cursor"
    );
}

#[test]
fn details_dialog_renders_full_title_and_metadata() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let task = task::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::new_v4(),
        remote_id: "remote".to_string(),
        content: "A full task title that should remain visible".to_string(),
        description: Some("Useful description".to_string()),
        project_uuid: Uuid::new_v4(),
        section_uuid: None,
        parent_uuid: None,
        priority: 4,
        order_index: 0,
        due_date: Some("2026-07-24".to_string()),
        due_datetime: None,
        is_recurring: false,
        deadline: None,
        duration: Some("30m".to_string()),
        is_completed: false,
        completed_at: None,
        is_deleted: false,
        deleted_at: None,
    };
    let mut scrollbar_state = ratatui::widgets::ScrollbarState::new(0);

    terminal
        .draw(|frame| render_task_details_dialog(frame, frame.area(), &task, "Terminalist", 0, &mut scrollbar_state))
        .unwrap();

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("A full task title that should remain visible"));
    assert!(rendered.contains("Project: Terminalist"));
    assert!(rendered.contains("Useful description"));
}
