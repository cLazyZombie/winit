use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{ElementState, Ime, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{ImeSurroundingText, Window, WindowId};

#[path = "util/fill.rs"]
mod fill;

fn main() -> Result<(), impl std::error::Error> {
    let event_loop = EventLoop::new().unwrap();
    let mut app = ImeTextbox::default();
    event_loop.run_app(&mut app)
}

#[derive(Default)]
struct ImeTextbox {
    window: Option<Window>,
    text: String,
    cursor: usize,
    preedit: String,
    last_event: String,
    modifiers: ModifiersState,
    focused: bool,
}

impl ImeTextbox {
    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let attributes = Window::default_attributes()
            .with_title("IME textbox")
            .with_inner_size(LogicalSize::new(900.0, 220.0));

        let window = event_loop.create_window(attributes).unwrap();
        window.set_ime_allowed(true);
        window.set_ime_cursor_area(LogicalPosition::new(24.0, 84.0), LogicalSize::new(20.0, 28.0));

        self.window = Some(window);
        self.last_event = "focus this window and type Korean".into();
        self.sync_surrounding_text();
        self.update_title();
        self.request_redraw();

        println!("IME textbox ready.");
        println!("Focus the window, switch to a Korean IME, and type directly into it.");
        println!(
            "The committed buffer is shown in the window title and every IME event is logged here."
        );
        println!("Press Escape or close the window to quit.");
    }

    fn sync_surrounding_text(&self) {
        let Some(window) = &self.window else {
            return;
        };

        let surrounding_text =
            ImeSurroundingText::new(self.text.clone(), self.cursor, self.cursor).unwrap();
        window.set_ime_surrounding_text(surrounding_text);
    }

    fn update_title(&self) {
        let Some(window) = &self.window else {
            return;
        };

        let focus = if self.focused { "focused" } else { "click window" };
        window.set_title(&format!(
            "IME textbox ({focus}) | text: \"{}\" | preedit: \"{}\" | last: {}",
            self.visible_text(),
            truncate_for_title(&self.preedit),
            self.last_event,
        ));
    }

    fn visible_text(&self) -> String {
        let mut text = String::new();
        let cursor = floor_char_boundary(&self.text, self.cursor);
        text.push_str(&self.text[..cursor]);
        text.push('|');
        text.push_str(&self.text[cursor..]);
        truncate_for_title(&text)
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn insert_text(&mut self, text: &str, source: &str) {
        if text.chars().any(|ch| ch.is_control()) {
            return;
        }

        self.cursor = floor_char_boundary(&self.text, self.cursor);
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
        self.preedit.clear();
        self.last_event = format!("{source} {text:?}");
        println!("{} -> text={:?}", self.last_event, self.text);
        self.sync_surrounding_text();
        self.update_title();
        self.request_redraw();
    }

    fn delete_surrounding(&mut self, before_bytes: usize, after_bytes: usize) {
        self.cursor = floor_char_boundary(&self.text, self.cursor);
        let start = floor_char_boundary(&self.text, self.cursor.saturating_sub(before_bytes));
        let end = ceil_char_boundary(
            &self.text,
            self.cursor.saturating_add(after_bytes).min(self.text.len()),
        );

        if start < end {
            self.text.replace_range(start..end, "");
            self.cursor = start;
        }

        self.last_event = format!("delete_surrounding before={before_bytes} after={after_bytes}");
        println!("{} -> text={:?}", self.last_event, self.text);
        self.sync_surrounding_text();
        self.update_title();
        self.request_redraw();
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        self.cursor = floor_char_boundary(&self.text, self.cursor);
        let start = previous_char_boundary(&self.text, self.cursor);
        self.text.replace_range(start..self.cursor, "");
        self.cursor = start;
        self.last_event = "backspace".into();
        println!("{} -> text={:?}", self.last_event, self.text);
        self.sync_surrounding_text();
        self.update_title();
        self.request_redraw();
    }

    fn delete_forward(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }

        self.cursor = floor_char_boundary(&self.text, self.cursor);
        let end = next_char_boundary(&self.text, self.cursor);
        self.text.replace_range(self.cursor..end, "");
        self.last_event = "delete".into();
        println!("{} -> text={:?}", self.last_event, self.text);
        self.sync_surrounding_text();
        self.update_title();
        self.request_redraw();
    }

    fn move_cursor_left(&mut self) {
        self.cursor = previous_char_boundary(&self.text, self.cursor);
        self.last_event = "cursor_left".into();
        self.sync_surrounding_text();
        self.update_title();
    }

    fn move_cursor_right(&mut self) {
        self.cursor = next_char_boundary(&self.text, self.cursor);
        self.last_event = "cursor_right".into();
        self.sync_surrounding_text();
        self.update_title();
    }
}

impl ApplicationHandler for ImeTextbox {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.create_window(event_loop);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                self.last_event = format!("focused={focused}");
                self.update_title();
            },
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            },
            WindowEvent::Ime(event) => match event {
                Ime::Enabled => {
                    self.last_event = "ime_enabled".into();
                    println!("{}", self.last_event);
                    self.sync_surrounding_text();
                    self.update_title();
                },
                Ime::Preedit(text, caret) => {
                    self.preedit = text.clone();
                    self.last_event = format!("preedit {text:?} caret={caret:?}");
                    println!("{}", self.last_event);
                    self.update_title();
                    self.request_redraw();
                },
                Ime::Commit(text) => {
                    self.insert_text(&text, "commit");
                },
                Ime::DeleteSurrounding { before_bytes, after_bytes } => {
                    self.delete_surrounding(before_bytes, after_bytes);
                },
                Ime::Disabled => {
                    self.last_event = "ime_disabled".into();
                    println!("{}", self.last_event);
                    self.update_title();
                },
            },
            WindowEvent::KeyboardInput {
                event: KeyEvent { logical_key, text, state: ElementState::Pressed, .. },
                ..
            } => {
                if self.handle_shortcut_or_edit_key(event_loop, logical_key.as_ref()) {
                    return;
                }

                if self.modifiers.control_key() || self.modifiers.super_key() {
                    return;
                }

                if let Some(text) = text {
                    self.insert_text(&text, "key");
                }
            },
            WindowEvent::RedrawRequested => {
                if let Some(window) = &self.window {
                    window.pre_present_notify();
                    fill::fill_window(window);
                }
            },
            _ => (),
        }
    }
}

impl ImeTextbox {
    fn handle_shortcut_or_edit_key(
        &mut self,
        event_loop: &ActiveEventLoop,
        key: Key<&str>,
    ) -> bool {
        match key {
            Key::Named(NamedKey::Escape) => {
                event_loop.exit();
                true
            },
            Key::Named(NamedKey::Backspace) => {
                self.backspace();
                true
            },
            Key::Named(NamedKey::Delete) => {
                self.delete_forward();
                true
            },
            Key::Named(NamedKey::ArrowLeft) => {
                self.move_cursor_left();
                true
            },
            Key::Named(NamedKey::ArrowRight) => {
                self.move_cursor_right();
                true
            },
            Key::Named(NamedKey::Home) => {
                self.cursor = 0;
                self.last_event = "cursor_home".into();
                self.sync_surrounding_text();
                self.update_title();
                true
            },
            Key::Named(NamedKey::End) => {
                self.cursor = self.text.len();
                self.last_event = "cursor_end".into();
                self.sync_surrounding_text();
                self.update_title();
                true
            },
            Key::Character("q") | Key::Character("Q")
                if self.modifiers.control_key() || self.modifiers.super_key() =>
            {
                event_loop.exit();
                true
            },
            _ => false,
        }
    }
}

fn truncate_for_title(text: &str) -> String {
    const MAX_CHARS: usize = 80;
    let mut truncated: String = text.chars().take(MAX_CHARS).collect();
    if text.chars().count() > MAX_CHARS {
        truncated.push_str("...");
    }
    truncated
}

fn previous_char_boundary(text: &str, index: usize) -> usize {
    let mut cursor = floor_char_boundary(text, index);
    if cursor == 0 {
        return 0;
    }
    cursor -= 1;
    floor_char_boundary(text, cursor)
}

fn next_char_boundary(text: &str, index: usize) -> usize {
    let mut cursor = index.min(text.len());
    if cursor == text.len() {
        return cursor;
    }
    cursor += 1;
    ceil_char_boundary(text, cursor)
}

fn floor_char_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_char_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index += 1;
    }
    index
}
