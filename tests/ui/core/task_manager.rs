use terminalist::entities::task;
use terminalist::ui::core::task_manager::TaskManager;
use terminalist::ui::core::{Action, AiExecutionReport, AiTaskProposal};
use uuid::Uuid;

fn example_task() -> task::Model {
    task::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::new_v4(),
        remote_id: "remote".to_string(),
        content: "Example task".to_string(),
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

#[test]
fn test_task_manager_creation() {
    // Test that TaskManager can be created without panicking
    let _task_manager = TaskManager::new();
}

#[tokio::test]
async fn task_operations_expose_blocking_status_immediately() {
    let (mut manager, _receiver) = TaskManager::new();
    manager.spawn_task_operation(|| async { Ok("done".to_string()) }, "Completing 2 tasks".to_string());

    assert!(manager.has_blocking_work());
    assert_eq!(manager.processing_description().as_deref(), Some("Completing 2 tasks"));
    manager.cancel_all_tasks();
}

#[tokio::test]
async fn non_blocking_task_operations_keep_input_responsive() {
    let (mut manager, _receiver) = TaskManager::new();
    let task_uuid = uuid::Uuid::new_v4();
    manager.spawn_non_blocking_task_operation(
        task_uuid,
        || async { Ok("done".to_string()) },
        "Complete task".to_string(),
    );

    assert!(!manager.has_blocking_work());
    assert!(manager.has_pending_operation_for_task(&task_uuid));
    assert_eq!(manager.processing_description().as_deref(), Some("Complete task"));
    manager.cancel_all_tasks();
}

#[tokio::test]
async fn ai_operation_reports_partial_results_back_to_the_dialog() {
    let (mut manager, mut receiver) = TaskManager::new();
    let task = example_task();
    let proposal = AiTaskProposal::mock_for(&task, "Handled", Some(Uuid::new_v4()));
    let expected_report = AiExecutionReport::failed(
        vec!["Add completion note".to_string()],
        "Create successor failed".to_string(),
    );
    let report_for_operation = expected_report.clone();

    manager.spawn_ai_assist_operation(Box::new(task), proposal, move || async move { report_for_operation });

    assert!(manager.has_blocking_work());
    assert!(matches!(receiver.recv().await, Some(Action::RefreshData)));
    assert!(matches!(
        receiver.recv().await,
        Some(Action::AiAssistFinished { report, .. }) if report == expected_report
    ));
}

#[tokio::test]
async fn ai_generation_reports_proposal_back_to_the_dialog() {
    let (mut manager, mut receiver) = TaskManager::new();
    let task = example_task();
    let proposal = AiTaskProposal::mock_for(&task, "Handled", Some(Uuid::new_v4()));
    let expected = proposal.clone();

    manager.spawn_ai_proposal_generation(Box::new(task), move || async move { Ok(proposal) });

    assert!(manager.has_blocking_work());
    assert!(matches!(
        receiver.recv().await,
        Some(Action::AiProposalGenerated { proposal, .. }) if proposal == expected
    ));
}

#[tokio::test]
async fn failed_ai_revision_restores_the_previous_proposal() {
    let (mut manager, mut receiver) = TaskManager::new();
    let task = example_task();
    let previous = AiTaskProposal::mock_for(&task, "Handled", Some(Uuid::new_v4()));
    let expected = previous.clone();

    manager.spawn_ai_proposal_revision(Box::new(task), previous, || async {
        anyhow::bail!("revision unavailable")
    });

    assert!(matches!(
        receiver.recv().await,
        Some(Action::AiProposalRevisionFailed {
            proposal,
            message,
            ..
        }) if proposal == expected && message.contains("revision unavailable")
    ));
}
