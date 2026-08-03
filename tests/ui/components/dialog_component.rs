use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use terminalist::entities::{project, task};
use terminalist::ui::components::DialogComponent;
use terminalist::ui::core::{Action, AiAssistStage, AiProposalPage, Component, DialogType};
use uuid::Uuid;

fn search_task(content: &str) -> task::Model {
    task::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::new_v4(),
        remote_id: Uuid::new_v4().to_string(),
        content: content.to_string(),
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
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn follow_up_project() -> project::Model {
    project::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::new_v4(),
        remote_id: "follow-up".to_string(),
        name: "Follow-up".to_string(),
        is_favorite: false,
        is_inbox_project: false,
        order_index: 0,
        parent_uuid: None,
    }
}

#[test]
fn test_dialog_component_creation() {
    // Test that DialogComponent can be created without panicking
    let _dialog = DialogComponent::new();
}

#[test]
fn task_creation_collects_a_separate_recurring_schedule() {
    let mut dialog = DialogComponent::new();
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
        default_due_date: Some("2026-08-03".to_string()),
        default_label_uuid: None,
    }));

    for character in "Water plants".chars() {
        dialog.handle_key_events(key(KeyCode::Char(character)));
    }
    dialog.handle_key_events(key(KeyCode::Down));
    for character in "every Saturday".chars() {
        dialog.handle_key_events(key(KeyCode::Char(character)));
    }

    let action = dialog.handle_key_events(key(KeyCode::Enter));

    assert!(matches!(
        action,
        Action::CreateTask {
            content,
            due_string: Some(due_string),
            due_date: None,
            ..
        } if content == "Water plants" && due_string == "every Saturday"
    ));
}

#[test]
fn task_creation_keeps_the_default_date_when_schedule_is_empty() {
    let mut dialog = DialogComponent::new();
    dialog.update(Action::ShowDialog(DialogType::TaskCreation {
        default_project_uuid: None,
        default_due_date: Some("2026-08-03".to_string()),
        default_label_uuid: None,
    }));
    dialog.handle_key_events(key(KeyCode::Char('x')));

    let action = dialog.handle_key_events(key(KeyCode::Enter));

    assert!(matches!(
        action,
        Action::CreateTask {
            due_string: None,
            due_date: Some(due_date),
            ..
        } if due_date == "2026-08-03"
    ));
}

#[test]
fn test_search_result_navigation_is_bounded() {
    let mut dialog = DialogComponent::new();
    dialog.dialog_type = Some(DialogType::TaskSearch);
    dialog.search_results = vec![search_task("first"), search_task("second")];

    dialog.handle_key_events(key(KeyCode::Down));
    assert!(dialog.search_results_focused);
    assert_eq!(dialog.search_selected_index, 0);
    dialog.handle_key_events(key(KeyCode::Down));
    assert_eq!(dialog.search_selected_index, 1);
    dialog.handle_key_events(key(KeyCode::Down));
    assert_eq!(dialog.search_selected_index, 1);

    dialog.handle_key_events(key(KeyCode::Up));
    assert_eq!(dialog.search_selected_index, 0);
    dialog.handle_key_events(key(KeyCode::Up));
    assert!(!dialog.search_results_focused);
    assert_eq!(dialog.search_selected_index, 0);
}

#[test]
fn test_all_letters_are_entered_while_search_input_is_focused() {
    let mut dialog = DialogComponent::new();
    dialog.dialog_type = Some(DialogType::TaskSearch);

    for character in 'a'..='z' {
        let action = dialog.handle_key_events(key(KeyCode::Char(character)));
        assert!(matches!(action, Action::SearchTasks(_)));
    }

    assert_eq!(dialog.input_buffer, "abcdefghijklmnopqrstuvwxyz");
    assert_eq!(dialog.cursor_position, 26);
}

#[test]
fn test_t_sets_selected_search_result_due_today() {
    let mut dialog = DialogComponent::new();
    dialog.dialog_type = Some(DialogType::TaskSearch);
    dialog.search_results = vec![search_task("first"), search_task("second")];
    dialog.search_selected_index = 1;
    dialog.search_results_focused = true;
    let selected_uuid = dialog.search_results[1].uuid;

    let action = dialog.handle_key_events(key(KeyCode::Char('t')));

    assert!(matches!(action, Action::SetTaskDueToday(uuid) if uuid == selected_uuid));
}

#[test]
fn test_space_toggles_selected_incomplete_search_result() {
    let mut dialog = DialogComponent::new();
    dialog.dialog_type = Some(DialogType::TaskSearch);
    dialog.search_results = vec![search_task("first"), search_task("second")];
    dialog.search_selected_index = 1;
    dialog.search_results_focused = true;
    let selected_uuid = dialog.search_results[1].uuid;

    let action = dialog.handle_key_events(key(KeyCode::Char(' ')));

    assert!(matches!(action, Action::ToggleTasks(tasks) if tasks == vec![(selected_uuid, false)]));
}

#[test]
fn test_space_toggles_selected_completed_search_result() {
    let mut dialog = DialogComponent::new();
    dialog.dialog_type = Some(DialogType::TaskSearch);
    let mut completed = search_task("completed");
    completed.is_completed = true;
    let selected_uuid = completed.uuid;
    dialog.search_results = vec![completed];
    dialog.search_results_focused = true;

    let action = dialog.handle_key_events(key(KeyCode::Char(' ')));

    assert!(matches!(action, Action::ToggleTasks(tasks) if tasks == vec![(selected_uuid, true)]));
}

#[test]
fn test_space_is_search_text_while_query_is_focused() {
    let mut dialog = DialogComponent::new();
    dialog.dialog_type = Some(DialogType::TaskSearch);

    let action = dialog.handle_key_events(key(KeyCode::Char(' ')));

    assert!(matches!(action, Action::SearchTasks(query) if query == " "));
    assert_eq!(dialog.input_buffer, " ");
}

#[test]
fn test_enter_has_no_search_action() {
    let mut dialog = DialogComponent::new();
    dialog.dialog_type = Some(DialogType::TaskSearch);

    assert!(matches!(dialog.handle_key_events(key(KeyCode::Enter)), Action::None));
}

#[test]
fn test_e_edits_title_from_task_details() {
    let mut dialog = DialogComponent::new();
    let task = search_task("original title");
    let task_uuid = task.uuid;
    let project_uuid = task.project_uuid;
    dialog.dialog_type = Some(DialogType::TaskDetails { task: Box::new(task) });

    let action = dialog.handle_key_events(key(KeyCode::Char('e')));

    assert!(matches!(
        action,
        Action::ShowDialog(DialogType::TaskEdit {
            task_uuid: actual_task_uuid,
            content,
            project_uuid: actual_project_uuid,
        }) if actual_task_uuid == task_uuid
            && content == "original title"
            && actual_project_uuid == project_uuid
    ));
}

#[test]
fn test_d_edits_description_from_task_details() {
    let mut dialog = DialogComponent::new();
    let mut task = search_task("original title");
    task.description = Some("original description".to_string());
    let task_uuid = task.uuid;
    dialog.dialog_type = Some(DialogType::TaskDetails { task: Box::new(task) });

    let action = dialog.handle_key_events(key(KeyCode::Char('d')));

    assert!(matches!(
        action,
        Action::ShowDialog(DialogType::TaskDescriptionEdit {
            task_uuid: actual_task_uuid,
            description,
        }) if actual_task_uuid == task_uuid && description == "original description"
    ));
}

#[test]
fn test_description_edit_is_prefilled_and_can_be_cleared() {
    let mut dialog = DialogComponent::new();
    let task_uuid = Uuid::new_v4();
    dialog.update(Action::ShowDialog(DialogType::TaskDescriptionEdit {
        task_uuid,
        description: "old description".to_string(),
    }));

    assert_eq!(dialog.input_buffer, "old description");
    for _ in 0.."old description".chars().count() {
        dialog.handle_key_events(key(KeyCode::Backspace));
    }
    let action = dialog.handle_key_events(key(KeyCode::Enter));

    assert!(matches!(
        action,
        Action::EditTaskDescription {
            task_uuid: actual_task_uuid,
            description,
        } if actual_task_uuid == task_uuid && description.is_empty()
    ));
}

#[test]
fn test_m_opens_ai_task_management_from_task_details() {
    let mut dialog = DialogComponent::new();
    let task = search_task("Help Mom with ChatGPT");
    let task_uuid = task.uuid;
    dialog.dialog_type = Some(DialogType::TaskDetails { task: Box::new(task) });

    let action = dialog.handle_key_events(key(KeyCode::Char('m')));

    assert!(matches!(
        action,
        Action::ShowDialog(DialogType::AiTaskManagement {
            task,
            stage: AiAssistStage::ContextEntry,
            proposal: None,
            report: None,
        }) if task.uuid == task_uuid
    ));
}

#[test]
fn test_ai_context_submission_requests_generation_and_accepts_result() {
    let mut dialog = DialogComponent::new();
    let task = search_task("Help Mom with ChatGPT");
    dialog.projects = vec![follow_up_project()];
    dialog.update(Action::ShowDialog(DialogType::AiTaskManagement {
        task: Box::new(task),
        stage: AiAssistStage::ContextEntry,
        proposal: None,
        report: None,
    }));

    for character in "The original need is handled.".chars() {
        dialog.handle_key_events(key(KeyCode::Char(character)));
    }
    let action = dialog.handle_key_events(key(KeyCode::Enter));
    assert!(matches!(action, Action::AiAssist { .. }));

    let Action::AiAssist { task, .. } = action else {
        panic!("expected AI assist action");
    };
    let proposal =
        terminalist::ui::core::AiTaskProposal::mock_for(&task, "The original need is handled.", Some(Uuid::new_v4()));
    let follow_up = dialog.update(Action::AiProposalGenerated { task, proposal });
    assert!(matches!(follow_up, Action::None));
    assert!(matches!(
        dialog.dialog_type,
        Some(DialogType::AiTaskManagement {
            stage: AiAssistStage::ProposalReview,
            proposal: Some(ref proposal),
            ..
        }) if proposal.actions.len() == 3
    ));
}

#[test]
fn test_ai_context_up_and_down_follow_wrapped_rows() {
    let mut dialog = DialogComponent::new();
    dialog.dialog_type = Some(DialogType::AiTaskManagement {
        task: Box::new(search_task("Example")),
        stage: AiAssistStage::ContextEntry,
        proposal: None,
        report: None,
    });
    dialog.input_buffer = "a".repeat(80);
    dialog.cursor_position = 80;

    dialog.handle_key_events(key(KeyCode::Up));
    assert_eq!(dialog.cursor_position, 6);
    dialog.handle_key_events(key(KeyCode::Down));
    assert_eq!(dialog.cursor_position, 80);
}

#[test]
fn test_ai_proposal_actions_can_be_toggled() {
    let mut dialog = DialogComponent::new();
    let task = search_task("Help Mom with ChatGPT");
    let proposal =
        terminalist::ui::core::AiTaskProposal::mock_for(&task, "The original need is handled.", Some(Uuid::new_v4()));
    dialog.dialog_type = Some(DialogType::AiTaskManagement {
        task: Box::new(task),
        stage: AiAssistStage::ProposalReview,
        proposal: Some(proposal),
        report: None,
    });
    dialog.ai_proposal_page = AiProposalPage::Actions;

    dialog.handle_key_events(key(KeyCode::Down));
    assert_eq!(dialog.ai_selected_action_index, 1);
    dialog.handle_key_events(key(KeyCode::Char(' ')));

    assert!(matches!(
        dialog.dialog_type,
        Some(DialogType::AiTaskManagement {
            proposal: Some(ref proposal),
            ..
        }) if !proposal.actions[1].enabled
    ));
}

#[test]
fn test_ai_proposal_requires_confirmation_before_apply() {
    let mut dialog = DialogComponent::new();
    let task = search_task("Help Mom with ChatGPT");
    let proposal =
        terminalist::ui::core::AiTaskProposal::mock_for(&task, "The original need is handled.", Some(Uuid::new_v4()));
    dialog.dialog_type = Some(DialogType::AiTaskManagement {
        task: Box::new(task),
        stage: AiAssistStage::ProposalReview,
        proposal: Some(proposal),
        report: None,
    });
    dialog.ai_proposal_page = AiProposalPage::Actions;

    assert!(matches!(dialog.handle_key_events(key(KeyCode::Enter)), Action::None));
    assert!(matches!(
        dialog.dialog_type,
        Some(DialogType::AiTaskManagement {
            stage: AiAssistStage::ApplyConfirmation,
            ..
        })
    ));

    assert!(matches!(
        dialog.handle_key_events(key(KeyCode::Enter)),
        Action::AiApplyProposal { .. }
    ));
}

#[test]
fn test_ai_proposal_can_continue_when_unavailable_mock_destination_is_disabled() {
    let mut dialog = DialogComponent::new();
    let task = search_task("Help Mom with ChatGPT");
    let proposal = terminalist::ui::core::AiTaskProposal::mock_for(&task, "The original need is handled.", None);
    dialog.dialog_type = Some(DialogType::AiTaskManagement {
        task: Box::new(task),
        stage: AiAssistStage::ProposalReview,
        proposal: Some(proposal),
        report: None,
    });
    dialog.ai_proposal_page = AiProposalPage::Actions;

    assert!(matches!(dialog.handle_key_events(key(KeyCode::Enter)), Action::None));
    assert!(matches!(
        dialog.dialog_type,
        Some(DialogType::AiTaskManagement {
            stage: AiAssistStage::ApplyConfirmation,
            ..
        })
    ));
    assert!(dialog.ai_error_message.is_none());
}

#[test]
fn test_ai_proposal_pages_navigate_with_arrows_and_enter() {
    let mut dialog = DialogComponent::new();
    let task = search_task("Example");
    let proposal = terminalist::ui::core::AiTaskProposal::mock_for(&task, "Handled", Some(Uuid::new_v4()));
    dialog.dialog_type = Some(DialogType::AiTaskManagement {
        task: Box::new(task),
        stage: AiAssistStage::ProposalReview,
        proposal: Some(proposal),
        report: None,
    });

    assert_eq!(dialog.ai_proposal_page, AiProposalPage::Recommendation);
    dialog.handle_key_events(key(KeyCode::Right));
    assert_eq!(dialog.ai_proposal_page, AiProposalPage::Actions);
    dialog.handle_key_events(key(KeyCode::Left));
    assert_eq!(dialog.ai_proposal_page, AiProposalPage::Recommendation);
    dialog.handle_key_events(key(KeyCode::Enter));
    assert_eq!(dialog.ai_proposal_page, AiProposalPage::Actions);
}

#[test]
fn test_ai_proposal_revision_preserves_context_and_current_proposal() {
    let mut dialog = DialogComponent::new();
    let task = search_task("Example");
    dialog.dialog_type = Some(DialogType::AiTaskManagement {
        task: Box::new(task.clone()),
        stage: AiAssistStage::ContextEntry,
        proposal: None,
        report: None,
    });
    for character in "Original context".chars() {
        dialog.handle_key_events(key(KeyCode::Char(character)));
    }
    let _ = dialog.handle_key_events(key(KeyCode::Enter));

    let proposal = terminalist::ui::core::AiTaskProposal::mock_for(&task, "Handled", Some(Uuid::new_v4()));
    dialog.dialog_type = Some(DialogType::AiTaskManagement {
        task: Box::new(task),
        stage: AiAssistStage::ProposalReview,
        proposal: Some(proposal.clone()),
        report: None,
    });
    dialog.ai_proposal_page = AiProposalPage::Actions;
    dialog.handle_key_events(key(KeyCode::Char('r')));
    assert!(matches!(
        dialog.dialog_type,
        Some(DialogType::AiTaskManagement {
            stage: AiAssistStage::RevisionEntry,
            ..
        })
    ));
    for character in "Use an existing project".chars() {
        dialog.handle_key_events(key(KeyCode::Char(character)));
    }
    let action = dialog.handle_key_events(key(KeyCode::Enter));
    assert!(matches!(
        action,
        Action::AiReviseProposal {
            original_context,
            proposal: returned,
            revision,
            ..
        } if original_context == "Original context"
            && returned == proposal
            && revision == "Use an existing project"
    ));
}
