use ratatui::{backend::Backend, backend::TestBackend, Terminal};
use terminalist::entities::task;
use terminalist::ui::components::dialogs::task_dialogs::{
    render_ai_task_management_dialog, render_task_details_dialog, render_task_time_dialog,
};
use terminalist::ui::core::{
    AiAssistStage, AiExecutionReport, AiProposalPage, AiProposedAction, AiProposedActionKind, AiTaskProposal,
};
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
    assert!(rendered.contains("m manage with AI"));
    assert!(rendered.contains("j/k or ↑/↓ scroll"));
}

#[test]
fn ai_context_cursor_follows_wrapped_pasted_text() {
    let backend = TestBackend::new(60, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let task = task::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::new_v4(),
        remote_id: "remote".to_string(),
        content: "Example".to_string(),
        description: None,
        project_uuid: Uuid::new_v4(),
        section_uuid: None,
        parent_uuid: None,
        priority: 1,
        order_index: 0,
        due_date: None,
        due_datetime: None,
        is_recurring: false,
        deadline: None,
        duration: None,
        is_completed: false,
        completed_at: None,
        is_deleted: false,
        deleted_at: None,
    };
    let input = "This pasted context is intentionally long enough to wrap across several visual rows.";

    terminal
        .draw(|frame| {
            render_ai_task_management_dialog(
                frame,
                frame.area(),
                &task,
                AiAssistStage::ContextEntry,
                "",
                0,
                None,
                AiProposalPage::Recommendation,
                0,
                0,
                None,
                None,
            );
        })
        .unwrap();
    let initial_cursor = terminal.backend_mut().get_cursor_position().unwrap();

    terminal
        .draw(|frame| {
            render_ai_task_management_dialog(
                frame,
                frame.area(),
                &task,
                AiAssistStage::ContextEntry,
                input,
                input.chars().count(),
                None,
                AiProposalPage::Recommendation,
                0,
                0,
                None,
                None,
            );
        })
        .unwrap();
    let wrapped_cursor = terminal.backend_mut().get_cursor_position().unwrap();

    assert!(wrapped_cursor.y > initial_cursor.y);
}

#[test]
fn ai_task_management_dialog_renders_ai_proposal_and_warning() {
    let backend = TestBackend::new(110, 36);
    let mut terminal = Terminal::new(backend).unwrap();
    let task = task::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::new_v4(),
        remote_id: "remote".to_string(),
        content: "Help Mom with ChatGPT".to_string(),
        description: None,
        project_uuid: Uuid::new_v4(),
        section_uuid: None,
        parent_uuid: None,
        priority: 1,
        order_index: 0,
        due_date: None,
        due_datetime: None,
        is_recurring: false,
        deadline: None,
        duration: None,
        is_completed: false,
        completed_at: None,
        is_deleted: false,
        deleted_at: None,
    };
    let proposal = AiTaskProposal::mock_for(&task, "The original need is handled.", Some(Uuid::new_v4()));

    terminal
        .draw(|frame| {
            render_ai_task_management_dialog(
                frame,
                frame.area(),
                &task,
                AiAssistStage::ProposalReview,
                "",
                0,
                Some(&proposal),
                AiProposalPage::Actions,
                0,
                0,
                None,
                None,
            );
        })
        .unwrap();

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("AI Task Management"));
    assert!(rendered.contains("Proposed Actions · 2/2"));
    assert!(rendered.contains("Proposed actions (3)"));
    assert!(
        rendered.contains("Complete the original task"),
        "selected action was not rendered:\n{rendered}"
    );
    assert!(rendered.contains("Action 1 of 3"));
    assert!(rendered.contains("More actions below"));
}

#[test]
fn selected_third_action_remains_visible_after_wrapped_actions() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let task = task::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::new_v4(),
        remote_id: "remote".to_string(),
        content: "Example".to_string(),
        description: None,
        project_uuid: Uuid::new_v4(),
        section_uuid: None,
        parent_uuid: None,
        priority: 1,
        order_index: 0,
        due_date: None,
        due_datetime: None,
        is_recurring: false,
        deadline: None,
        duration: None,
        is_completed: false,
        completed_at: None,
        is_deleted: false,
        deleted_at: None,
    };
    let long_note = "This deliberately long action wraps across several terminal rows so the selected action below it must be located using rendered rows rather than its list index.".repeat(3);
    let proposal = AiTaskProposal {
        summary: "Review the actions.".to_string(),
        actions: vec![
            AiProposedAction {
                enabled: true,
                kind: AiProposedActionKind::AddCompletionNote {
                    content: long_note.clone(),
                },
            },
            AiProposedAction {
                enabled: true,
                kind: AiProposedActionKind::AddCompletionNote { content: long_note },
            },
            AiProposedAction {
                enabled: true,
                kind: AiProposedActionKind::CompleteTask,
            },
            AiProposedAction {
                enabled: true,
                kind: AiProposedActionKind::CreateProject {
                    reference: "later".to_string(),
                    name: "Potential Future Tasks".to_string(),
                },
            },
        ],
    };

    terminal
        .draw(|frame| {
            render_ai_task_management_dialog(
                frame,
                frame.area(),
                &task,
                AiAssistStage::ProposalReview,
                "",
                0,
                Some(&proposal),
                AiProposalPage::Actions,
                0,
                2,
                None,
                None,
            );
        })
        .unwrap();

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(
        rendered.contains("Complete the original task"),
        "selected action was not rendered:\n{rendered}"
    );
    assert!(rendered.contains("Action 3 of 4"));
}

#[test]
fn ai_recommendation_page_uses_the_full_review_area() {
    let backend = TestBackend::new(90, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let task = task::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::new_v4(),
        remote_id: "remote".to_string(),
        content: "Example".to_string(),
        description: None,
        project_uuid: Uuid::new_v4(),
        section_uuid: None,
        parent_uuid: None,
        priority: 1,
        order_index: 0,
        due_date: None,
        due_datetime: None,
        is_recurring: false,
        deadline: None,
        duration: None,
        is_completed: false,
        completed_at: None,
        is_deleted: false,
        deleted_at: None,
    };
    let mut proposal = AiTaskProposal::mock_for(&task, "Handled", Some(Uuid::new_v4()));
    proposal.summary =
        "First paragraph explaining the recommendation in detail.\n\nFinal recommendation remains visible.".to_string();

    terminal
        .draw(|frame| {
            render_ai_task_management_dialog(
                frame,
                frame.area(),
                &task,
                AiAssistStage::ProposalReview,
                "",
                0,
                Some(&proposal),
                AiProposalPage::Recommendation,
                0,
                0,
                None,
                None,
            );
        })
        .unwrap();
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("Recommendation · 1/2"));
    assert!(rendered.contains("Final recommendation remains visible."));
}

#[test]
fn ai_task_management_dialog_renders_partial_failure_report() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let task = task::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::new_v4(),
        remote_id: "remote".to_string(),
        content: "Help Mom with ChatGPT".to_string(),
        description: None,
        project_uuid: Uuid::new_v4(),
        section_uuid: None,
        parent_uuid: None,
        priority: 1,
        order_index: 0,
        due_date: None,
        due_datetime: None,
        is_recurring: false,
        deadline: None,
        duration: None,
        is_completed: false,
        completed_at: None,
        is_deleted: false,
        deleted_at: None,
    };
    let proposal = AiTaskProposal::mock_for(&task, "Handled", Some(Uuid::new_v4()));
    let report = AiExecutionReport::failed(
        vec!["Add completion note".to_string()],
        "Create successor: network unavailable".to_string(),
    );

    terminal
        .draw(|frame| {
            render_ai_task_management_dialog(
                frame,
                frame.area(),
                &task,
                AiAssistStage::Result,
                "",
                0,
                Some(&proposal),
                AiProposalPage::Recommendation,
                0,
                0,
                None,
                Some(&report),
            );
        })
        .unwrap();

    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("stopped before all actions completed"));
    assert!(rendered.contains("Add completion note"));
    assert!(rendered.contains("network unavailable"));
}
