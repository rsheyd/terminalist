use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use terminalist::ui::components::SidebarComponent;
use terminalist::ui::core::actions::{Action, NavigationCounts, SidebarSelection};
use terminalist::ui::core::Component;
use uuid::Uuid;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn label_named(name: &str) -> terminalist::entities::label::Model {
    terminalist::entities::label::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::new_v4(),
        remote_id: name.to_string(),
        name: name.to_string(),
        order_index: 0,
        is_favorite: false,
    }
}

#[test]
fn test_sidebar_component_creation() {
    // Test that SidebarComponent can be created without panicking
    let _sidebar = SidebarComponent::new();
}

#[test]
fn bracket_keys_navigate_sidebar_items() {
    let mut sidebar = SidebarComponent::new();
    let first = label_named("First");
    let second = label_named("Second");
    sidebar.update_data(
        Vec::new(),
        vec![first.clone(), second.clone()],
        NavigationCounts::default(),
        0,
    );

    assert!(matches!(
        sidebar.handle_key_events(key(KeyCode::Char(']'))),
        Action::NavigateToSidebar(SidebarSelection::Label(uuid)) if uuid == first.uuid
    ));
    assert!(matches!(
        sidebar.handle_key_events(key(KeyCode::Char('['))),
        Action::NavigateToSidebar(SidebarSelection::Label(uuid)) if uuid == second.uuid
    ));
}

#[test]
fn uppercase_navigation_keys_remain_available() {
    let mut sidebar = SidebarComponent::new();
    let first = label_named("First");
    let second = label_named("Second");
    sidebar.update_data(
        Vec::new(),
        vec![first.clone(), second.clone()],
        NavigationCounts::default(),
        0,
    );

    assert!(matches!(
        sidebar.handle_key_events(key(KeyCode::Char('J'))),
        Action::NavigateToSidebar(SidebarSelection::Label(uuid)) if uuid == first.uuid
    ));
    assert!(matches!(
        sidebar.handle_key_events(key(KeyCode::Char('K'))),
        Action::NavigateToSidebar(SidebarSelection::Label(uuid)) if uuid == second.uuid
    ));
}
