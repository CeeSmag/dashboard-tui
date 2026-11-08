//! `q` / `^Q` / `Ctrl+C` exit semantics and text-input modals (headless).

use tui_rs::core::{Color, PrioritySegment};
use tui_rs::dashboard::{
    AddonRegistry, DashboardBackend, DashboardEngine, EngineAction, KeyBindings, KeyCode, KeyPress,
    ModalContext, OptionOutcome, OptionSpec, SectionLayoutMode,
};
use tui_rs::universal::{ConfirmModal, EditModal};

#[derive(Default)]
struct App {
    submitted: String,
}

impl DashboardBackend for App {
    fn app_title(&self) -> &'static str {
        "Exit semantics"
    }
    fn status_badge(&self) -> (&'static str, Color) {
        ("OK", Color::Green)
    }
    fn header_segments(&self) -> Vec<PrioritySegment<'_>> {
        Vec::new()
    }
}

fn engine() -> DashboardEngine<App> {
    AddonRegistry::<App>::new()
        .with_keys(KeyBindings::default().quit("^q").cancel('q'))
        .with_page(0, "Main", "F1", SectionLayoutMode::Auto)
        .with_menu_section(0, 0, "Actions", "1-2", 5, 8, true)
        .with_option(
            0,
            0,
            OptionSpec::new(1, "Edit", |_ctx: &mut ModalContext<'_, App>| {
                OptionOutcome::OpenModal(Box::new(EditModal::new(
                    "Edit",
                    "Value:",
                    "",
                    |app: &mut App, value: &str| {
                        value.clone_into(&mut app.submitted);
                        Ok(String::new())
                    },
                )))
            }),
        )
        .with_option(
            0,
            0,
            OptionSpec::new(2, "Confirm", |_ctx: &mut ModalContext<'_, App>| {
                OptionOutcome::OpenModal(Box::new(ConfirmModal::new(
                    "Confirm",
                    "Sure?",
                    |_app: &mut App| Ok(String::new()),
                )))
            }),
        )
        .build_engine(App::default())
}

#[test]
fn q_closes_a_popup_then_goes_back_at_the_root() {
    let mut engine = engine();
    let _ = engine.handle_key(KeyPress::char('2'));
    assert!(engine.has_modal());

    assert_eq!(
        engine.handle_key(KeyPress::char('q')),
        EngineAction::Continue
    );
    assert!(!engine.has_modal(), "q closes the popup");

    assert_eq!(engine.handle_key(KeyPress::char('q')), EngineAction::Back);
}

#[test]
fn ctrl_q_quits_everything_even_with_a_popup_open() {
    let mut engine = engine();
    let _ = engine.handle_key(KeyPress::char('2'));
    assert_eq!(
        engine.handle_key(KeyPress::char('q').with_ctrl(true)),
        EngineAction::Quit
    );
}

#[test]
fn ctrl_c_only_goes_back() {
    let mut engine = engine();
    assert_eq!(
        engine.handle_key(KeyPress::char('c').with_ctrl(true)),
        EngineAction::Back
    );
}

#[test]
fn text_input_receives_the_letter_q_instead_of_closing() {
    let mut engine = engine();
    let _ = engine.handle_key(KeyPress::char('1'));
    assert!(engine.has_modal());

    for ch in ['a', 'q', 'b'] {
        let _ = engine.handle_key(KeyPress::char(ch));
        assert!(
            engine.has_modal(),
            "typing `{ch}` must not close the text popup"
        );
    }
    let _ = engine.handle_key(KeyPress::new(KeyCode::Enter));
    assert!(!engine.has_modal());
    assert_eq!(engine.backend().submitted, "aqb");
}

#[test]
fn esc_still_cancels_a_text_popup() {
    let mut engine = engine();
    let _ = engine.handle_key(KeyPress::char('1'));
    let _ = engine.handle_key(KeyPress::new(KeyCode::Esc));
    assert!(!engine.has_modal());
}

#[test]
fn getters_report_focus_and_sub_view_state() {
    let engine = engine();
    assert_eq!(engine.focused_section(), 0);
    assert!(!engine.has_sub_view());
}
