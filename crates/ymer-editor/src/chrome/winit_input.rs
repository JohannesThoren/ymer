//! Från winit till `ymer-ui`.
//!
//! Gränssnittsbiblioteket vet inget om fönstersystem – det tar färdigt
//! tolkade tecken och en handfull redigeringstangenter. Översättningen
//! hör därför hemma här, hos den som faktiskt har ett fönster.
//!
//! Händelser kommer in när de kommer, men gränssnittet vill se dem en
//! gång per frame. [`InputPump`] samlar ihop dem och nollställer det som
//! bara gäller en frame – nedtryck, släpp, hjul, tecken – i [`take`].
//! Det som varar, som var pekaren är och om knappen hålls nere, står
//! kvar.

use std::time::{Duration, Instant};

use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key as WinitKey, NamedKey};
use ymer_ui::{Input, Key, Vec2};

/// Två klick inom den här tiden, nära varandra, är ett dubbelklick.
/// Samma tal som de flesta skrivbordsmiljöer använder.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);
/// Hur långt pekaren får ha flyttat sig mellan klicken.
const DOUBLE_CLICK_SLOP: f32 = 4.0;
/// En hjulrad i pixlar. winit rapporterar rader på mus och pixlar på
/// styrplatta; listan vill ha pixlar.
const LINE: f32 = 40.0;

pub struct InputPump {
    input: Input,
    last_click: Option<(Instant, Vec2)>,
}

impl Default for InputPump {
    fn default() -> Self {
        Self::new()
    }
}

impl InputPump {
    pub fn new() -> Self {
        Self {
            input: Input::default(),
            last_click: None,
        }
    }

    pub fn position(&self) -> Vec2 {
        self.input.pointer.position
    }

    /// Framens input. Nollställer det som bara gäller en frame.
    pub fn take(&mut self) -> Input {
        let input = self.input.clone();
        self.input.pointer.pressed = false;
        self.input.pointer.released = false;
        self.input.pointer.double = false;
        self.input.pointer.scroll = Vec2::ZERO;
        self.input.text.clear();
        self.input.keys.clear();
        input
    }

    /// Matar in en fönsterhändelse. Sant om den rörde gränssnittet.
    pub fn accept(&mut self, event: &WindowEvent, scale: f32) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.input.pointer.position =
                    Vec2::new(position.x as f32 / scale, position.y as f32 / scale);
                true
            }

            WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Left,
                ..
            } => {
                match state {
                    ElementState::Pressed => {
                        self.input.pointer.down = true;
                        self.input.pointer.pressed = true;
                        self.input.pointer.double = self.is_double_click();
                    }
                    ElementState::Released => {
                        self.input.pointer.down = false;
                        self.input.pointer.released = true;
                    }
                }
                true
            }

            WindowEvent::MouseWheel { delta, .. } => {
                // Nedåt i innehållet är positivt, alltså tvärtom mot
                // winit, som rapporterar hur hjulet rullade.
                let pixels = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2::new(-x * LINE, -y * LINE),
                    MouseScrollDelta::PixelDelta(position) => {
                        Vec2::new(-position.x as f32, -position.y as f32)
                    }
                };
                self.input.pointer.scroll = self.input.pointer.scroll + pixels;
                true
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return false;
                }
                if let Some(key) = translate(&event.logical_key) {
                    self.input.keys.push(key);
                }
                // Tecken kommer färdigtolkade från fönstersystemet, med
                // tangentbordslayout och allt. Att bygga dem ur
                // tangentkoder hade gjort å, ä och ö till fel bokstäver.
                if let WinitKey::Character(text) = &event.logical_key {
                    self.input.text.push_str(text);
                } else if event.logical_key == WinitKey::Named(NamedKey::Space) {
                    self.input.text.push(' ');
                }
                true
            }

            _ => false,
        }
    }

    fn is_double_click(&mut self) -> bool {
        let now = Instant::now();
        let position = self.input.pointer.position;
        let double = self.last_click.is_some_and(|(when, where_)| {
            now.duration_since(when) < DOUBLE_CLICK
                && (where_ - position).x.abs() < DOUBLE_CLICK_SLOP
                && (where_ - position).y.abs() < DOUBLE_CLICK_SLOP
        });
        // Ett dubbelklick startar inte ett trippelklick.
        self.last_click = if double { None } else { Some((now, position)) };
        double
    }
}

fn translate(key: &WinitKey) -> Option<Key> {
    let WinitKey::Named(named) = key else {
        return None;
    };
    Some(match named {
        NamedKey::Backspace => Key::Backspace,
        NamedKey::Delete => Key::Delete,
        NamedKey::ArrowLeft => Key::Left,
        NamedKey::ArrowRight => Key::Right,
        NamedKey::ArrowUp => Key::Up,
        NamedKey::ArrowDown => Key::Down,
        NamedKey::Home => Key::Home,
        NamedKey::End => Key::End,
        NamedKey::Enter => Key::Enter,
        NamedKey::Tab => Key::Tab,
        NamedKey::Escape => Key::Escape,
        _ => return None,
    })
}
