use std::time::{Duration, Instant};

use gilrs::{Axis, Button, EventType, Gilrs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
    Menu,
    PreviousSection,
    NextSection,
}

pub struct Gamepad {
    gilrs: Gilrs,
    name: String,
    stick_x: bool,
    stick_y: bool,
    last_repeat: Instant,
    held: Option<Nav>,
}

const PRESS: f32 = 0.55;
const RELEASE: f32 = 0.35;
const REPEAT: Duration = Duration::from_millis(180);

impl Gamepad {
    pub fn new() -> Option<Self> {
        let gilrs = Gilrs::new().ok()?;
        let name = gilrs
            .gamepads()
            .next()
            .map(|(_, pad)| pad.name().to_string())
            .unwrap_or_default();
        Some(Self {
            gilrs,
            name,
            stick_x: false,
            stick_y: false,
            last_repeat: Instant::now(),
            held: None,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Mapeado común para mandos XInput/DualShock/DualSense.
    pub fn poll(&mut self) -> Vec<Nav> {
        let mut navs = Vec::new();
        while let Some(event) = self.gilrs.next_event() {
            match event.event {
                EventType::ButtonPressed(button, _) => {
                    if let Some(nav) = map_button(button) {
                        navs.push(nav);
                    }
                }
                EventType::AxisChanged(axis, value, _) => {
                    self.handle_axis(axis, value, &mut navs);
                }
                _ => {}
            }
        }
        self.repeat(&mut navs);
        navs
    }

    fn handle_axis(&mut self, axis: Axis, value: f32, navs: &mut Vec<Nav>) {
        let (positive, negative, state) = match axis {
            Axis::LeftStickX => (Nav::Right, Nav::Left, &mut self.stick_x),
            Axis::LeftStickY => (Nav::Down, Nav::Up, &mut self.stick_y),
            _ => return,
        };
        let nav = if value > PRESS { Some(positive) } else if value < -PRESS { Some(negative) } else { None };
        match nav {
            Some(nav) if !*state => {
                *state = true;
                self.held = Some(nav);
                self.last_repeat = Instant::now();
                navs.push(nav);
            }
            None if value.abs() < RELEASE => *state = false,
            _ => {}
        }
    }

    fn repeat(&mut self, navs: &mut Vec<Nav>) {
        let Some(nav) = self.held else { return };
        if self.last_repeat.elapsed() < REPEAT {
            return;
        }
        if self.stick_x || self.stick_y {
            navs.push(nav);
            self.last_repeat = Instant::now();
        } else {
            self.held = None;
        }
    }
}

fn map_button(button: Button) -> Option<Nav> {
    match button {
        Button::DPadUp => Some(Nav::Up),
        Button::DPadDown => Some(Nav::Down),
        Button::DPadLeft => Some(Nav::Left),
        Button::DPadRight => Some(Nav::Right),
        Button::South => Some(Nav::Confirm),
        Button::East => Some(Nav::Back),
        Button::Start => Some(Nav::Menu),
        Button::LeftTrigger => Some(Nav::PreviousSection),
        Button::RightTrigger => Some(Nav::NextSection),
        _ => None,
    }
}
