use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use terminalist::ui::components::SidebarComponent;
use terminalist::ui::core::actions::{Action, NavigationCounts, SidebarSelection};
use terminalist::ui::core::Component;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn test_sidebar_component_creation() {
    // Test that SidebarComponent can be created without panicking
    let _sidebar = SidebarComponent::new();
}

#[test]
fn bracket_keys_navigate_sidebar_items() {
    let mut sidebar = SidebarComponent::new();
    sidebar.update_data(Vec::new(), Vec::new(), NavigationCounts::default(), 0);

    assert!(matches!(
        sidebar.handle_key_events(key(KeyCode::Char(']'))),
        Action::NavigateToSidebar(SidebarSelection::Agenda)
    ));
    assert!(matches!(
        sidebar.handle_key_events(key(KeyCode::Char('['))),
        Action::NavigateToSidebar(SidebarSelection::Today)
    ));
}

#[test]
fn uppercase_navigation_keys_remain_available() {
    let mut sidebar = SidebarComponent::new();
    sidebar.update_data(Vec::new(), Vec::new(), NavigationCounts::default(), 0);

    assert!(matches!(
        sidebar.handle_key_events(key(KeyCode::Char('J'))),
        Action::NavigateToSidebar(SidebarSelection::Agenda)
    ));
    assert!(matches!(
        sidebar.handle_key_events(key(KeyCode::Char('K'))),
        Action::NavigateToSidebar(SidebarSelection::Today)
    ));
}
