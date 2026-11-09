use rstest::rstest;
use serde_json::{Value, json};

use super::*;

fn key(code: &str, mods: &[Modifier], kind: KeyKind) -> InputEvent {
    InputEvent::Key(Key { code: code.to_owned(), mods: mods.to_vec(), kind })
}

fn mouse(kind: MouseKind, button: Button, (x, y): (u16, u16), mods: &[Modifier]) -> InputEvent {
    InputEvent::Mouse(Mouse { kind, button, x, y, mods: mods.to_vec() })
}

fn written(event: &InputEvent) -> Value {
    serde_json::to_value(event).expect("an input event serializes")
}

fn read(line: Value) -> InputEvent {
    serde_json::from_value(line).expect("an input event")
}

mod an_event_is_written {
    use super::*;

    #[rstest]
    #[case::a_plain_key(key("enter", &[], KeyKind::Press), json!({"key": {"code": "enter"}}))]
    #[case::a_key_with_ctrl(key("c", &[Modifier::Ctrl], KeyKind::Press), json!({"key": {"code": "c", "mods": ["ctrl"]}}))]
    #[case::a_key_with_every_modifier(
        key("up", &[Modifier::Ctrl, Modifier::Alt, Modifier::Shift, Modifier::Super], KeyKind::Press),
        json!({"key": {"code": "up", "mods": ["ctrl", "alt", "shift", "super"]}})
    )]
    #[case::a_repeat(key("backspace", &[], KeyKind::Repeat), json!({"key": {"code": "backspace", "kind": "repeat"}}))]
    #[case::a_release(key("esc", &[], KeyKind::Release), json!({"key": {"code": "esc", "kind": "release"}}))]
    #[case::text(InputEvent::Text("ls -la".to_owned()), json!({"text": "ls -la"}))]
    #[case::a_paste(InputEvent::Paste("one\ntwo".to_owned()), json!({"paste": "one\ntwo"}))]
    #[case::a_scroll(
        mouse(MouseKind::ScrollUp, Button::Left, (3, 7), &[]),
        json!({"mouse": {"kind": "scroll-up", "x": 3, "y": 7}})
    )]
    #[case::a_right_click_with_shift(
        mouse(MouseKind::Down, Button::Right, (0, 0), &[Modifier::Shift]),
        json!({"mouse": {"kind": "down", "button": "right", "x": 0, "y": 0, "mods": ["shift"]}})
    )]
    fn in_cornercases_shape_without_its_defaults(#[case] event: InputEvent, #[case] line: Value) {
        assert_eq!(written(&event), line);
    }

    #[rstest]
    #[case::up(MouseKind::Up, "up")]
    #[case::drag(MouseKind::Drag, "drag")]
    #[case::moved(MouseKind::Moved, "moved")]
    #[case::scroll_down(MouseKind::ScrollDown, "scroll-down")]
    #[case::scroll_left(MouseKind::ScrollLeft, "scroll-left")]
    #[case::scroll_right(MouseKind::ScrollRight, "scroll-right")]
    fn with_its_mouse_kind_in_kebab_case(#[case] kind: MouseKind, #[case] name: &str) {
        let event = mouse(kind, Button::Middle, (1, 2), &[]);
        assert_eq!(written(&event), json!({"mouse": {"kind": name, "button": "middle", "x": 1, "y": 2}}));
    }
}

mod an_event_is_read {
    use super::*;

    #[test]
    fn as_cornercase_writes_it_with_every_field() {
        let line = json!({"key": {"code": "c", "mods": ["ctrl"], "kind": "press"}});
        assert_eq!(read(line), key("c", &[Modifier::Ctrl], KeyKind::Press));
    }

    #[test]
    fn with_its_defaults_when_the_fields_are_left_out() {
        assert_eq!(read(json!({"key": {"code": "tab"}})), key("tab", &[], KeyKind::Press));
        assert_eq!(
            read(json!({"mouse": {"kind": "scroll-down"}})),
            mouse(MouseKind::ScrollDown, Button::Left, (0, 0), &[])
        );
    }

    #[test]
    fn back_from_what_it_wrote() {
        let event = mouse(MouseKind::Drag, Button::Right, (40, 12), &[Modifier::Alt]);
        assert_eq!(read(written(&event)), event);
    }
}
