use std::collections::HashSet;
use cgmath::{Vector2, Zero};
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[derive(Debug)]
pub struct Input {
    keys_held: HashSet<KeyCode>, // fxHashSet??
    keys_held_last_frame: HashSet<KeyCode>,
    mouse_buttons_held: HashSet<MouseButton>,
    mouse_buttons_held_last_frame: HashSet<MouseButton>,

    pub mouse_delta: Vector2<f64>
}

impl Input {
    pub fn new() -> Self {
        Self{
            keys_held: HashSet::default(),
            keys_held_last_frame: HashSet::default(),
            mouse_buttons_held: HashSet::default(),
            mouse_buttons_held_last_frame: HashSet::default(),

            mouse_delta: Vector2::new(0.0, 0.0),
        }
    }

    pub fn reset(&mut self) {
        self.keys_held_last_frame = self.keys_held.clone();
        self.mouse_buttons_held_last_frame = self.mouse_buttons_held.clone();
        self.mouse_delta = Vector2::zero();

    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) -> bool {

        match event {
            WindowEvent::KeyboardInput {
                event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(key_code),
                    state,
                    ..
                },
                ..
            } => {
                match state {
                    ElementState::Pressed => {
                        self.keys_held.insert(*key_code);
                    }
                    ElementState::Released => {
                        self.keys_held.remove(key_code);
                    }
                }
                true
            }
            WindowEvent::MouseInput { button, state, .. } => {
                match state {
                    ElementState::Pressed => {
                        self.mouse_buttons_held.insert(*button);
                    }
                    ElementState::Released => {
                        self.mouse_buttons_held.remove(button);
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub fn handle_device_event(&mut self, event: &DeviceEvent) -> bool {
        
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.mouse_delta.x += delta.0;
                self.mouse_delta.y += delta.1;
                true
            }
            _ => false,
        }
    }

    pub fn is_key_down(&self, key_code: KeyCode) -> bool {
        self.keys_held.contains(&key_code)
    }

    pub fn is_key_just_pressed(&self, key_code: KeyCode) -> bool {
        self.keys_held.contains(&key_code) && !self.keys_held_last_frame.contains(&key_code)
    }

    pub fn is_key_just_released(&self, key_code: KeyCode) -> bool {
        self.keys_held_last_frame.contains(&key_code) && !self.keys_held.contains(&key_code)
    }

    pub fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        self.mouse_buttons_held.contains(&button)
    }

    pub fn is_mouse_button_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_held.contains(&button)
            && !self.mouse_buttons_held_last_frame.contains(&button)
    }

    pub fn is_mouse_button_just_released(&self, button: MouseButton) -> bool {
        self.mouse_buttons_held_last_frame.contains(&button)
            && !self.mouse_buttons_held.contains(&button)
    }

    pub fn mouse_delta(&self) -> Vector2<f64> {
        self.mouse_delta
    }

    pub fn mouse_delta_f32(&self) -> Vector2<f32> {
        Vector2::new(self.mouse_delta.x as f32, self.mouse_delta.y as f32)
    }
}
