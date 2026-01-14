//! Keyboard input.

use roc_integration::roc;

/// A press or release of a keyboard key.
#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyboardEvent {
    key: KeyboardKey,
    state: KeyState,
}

/// A key on a keyboard.
#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyboardKey {
    Letter(LetterKey),
    Number(NumberKey),
    Arrow(ArrowKey),
    Modifier(ModifierKey),
    Whitespace(WhitespaceKey),
    Control(ControlKey),
    Symbol(SymbolKey),
    Numpad(NumpadKey),
    Function(FunctionKey),
    Lock(LockKey),
    Navigation(NavigationKey),
}

#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LetterKey {
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
}

#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumberKey {
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
}

#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArrowKey {
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
}

#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModifierKey {
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    SuperLeft,
    SuperRight,
}

#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WhitespaceKey {
    Space,
    Tab,
    Enter,
}

#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlKey {
    Escape,
    Backspace,
    Delete,
}

#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKey {
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    Backslash,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
    Backquote,
}

#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumpadKey {
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadEnter,
    NumpadDecimal,
}

#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FunctionKey {
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockKey {
    CapsLock,
    NumLock,
}

#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavigationKey {
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
}

/// The state of a key following a key event.
#[roc(parents = "Input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyState {
    /// The key was pressed.
    Pressed,
    /// The key is being held down, emitting repeated events.
    Held,
    /// The key was released.
    Released,
}

impl KeyboardEvent {
    /// Returns a `KeyboardEvent` corresponding to the given `winit`
    /// `KeyEvent`, or [`None`] if the `KeyEvent` has no analogous
    /// `KeyboardEvent`.
    #[cfg(feature = "window")]
    pub fn from_winit(event: winit::event::KeyEvent) -> Option<Self> {
        let winit::keyboard::PhysicalKey::Code(code) = event.physical_key else {
            return None;
        };
        let key = KeyboardKey::from_winit(code)?;
        let state = KeyState::from_winit(event.state, event.repeat);
        Some(Self { key, state })
    }
}

impl KeyboardKey {
    /// Returns the `KeyboardKey` corresponding to the given `winit` `KeyCode`,
    /// or [`None`] if the `KeyCode` is not supported.
    #[allow(clippy::enum_glob_use)]
    #[cfg(feature = "window")]
    pub fn from_winit(code: winit::keyboard::KeyCode) -> Option<Self> {
        use KeyboardKey::*;
        use winit::keyboard::KeyCode::*;

        Some(match code {
            KeyA => Letter(LetterKey::KeyA),
            KeyB => Letter(LetterKey::KeyB),
            KeyC => Letter(LetterKey::KeyC),
            KeyD => Letter(LetterKey::KeyD),
            KeyE => Letter(LetterKey::KeyE),
            KeyF => Letter(LetterKey::KeyF),
            KeyG => Letter(LetterKey::KeyG),
            KeyH => Letter(LetterKey::KeyH),
            KeyI => Letter(LetterKey::KeyI),
            KeyJ => Letter(LetterKey::KeyJ),
            KeyK => Letter(LetterKey::KeyK),
            KeyL => Letter(LetterKey::KeyL),
            KeyM => Letter(LetterKey::KeyM),
            KeyN => Letter(LetterKey::KeyN),
            KeyO => Letter(LetterKey::KeyO),
            KeyP => Letter(LetterKey::KeyP),
            KeyQ => Letter(LetterKey::KeyQ),
            KeyR => Letter(LetterKey::KeyR),
            KeyS => Letter(LetterKey::KeyS),
            KeyT => Letter(LetterKey::KeyT),
            KeyU => Letter(LetterKey::KeyU),
            KeyV => Letter(LetterKey::KeyV),
            KeyW => Letter(LetterKey::KeyW),
            KeyX => Letter(LetterKey::KeyX),
            KeyY => Letter(LetterKey::KeyY),
            KeyZ => Letter(LetterKey::KeyZ),

            Digit0 => Number(NumberKey::Digit0),
            Digit1 => Number(NumberKey::Digit1),
            Digit2 => Number(NumberKey::Digit2),
            Digit3 => Number(NumberKey::Digit3),
            Digit4 => Number(NumberKey::Digit4),
            Digit5 => Number(NumberKey::Digit5),
            Digit6 => Number(NumberKey::Digit6),
            Digit7 => Number(NumberKey::Digit7),
            Digit8 => Number(NumberKey::Digit8),
            Digit9 => Number(NumberKey::Digit9),

            ArrowUp => Arrow(ArrowKey::ArrowUp),
            ArrowDown => Arrow(ArrowKey::ArrowDown),
            ArrowLeft => Arrow(ArrowKey::ArrowLeft),
            ArrowRight => Arrow(ArrowKey::ArrowRight),

            ShiftLeft => Modifier(ModifierKey::ShiftLeft),
            ShiftRight => Modifier(ModifierKey::ShiftRight),
            ControlLeft => Modifier(ModifierKey::ControlLeft),
            ControlRight => Modifier(ModifierKey::ControlRight),
            AltLeft => Modifier(ModifierKey::AltLeft),
            AltRight => Modifier(ModifierKey::AltRight),
            SuperLeft => Modifier(ModifierKey::SuperLeft),
            SuperRight => Modifier(ModifierKey::SuperRight),

            Space => Whitespace(WhitespaceKey::Space),
            Tab => Whitespace(WhitespaceKey::Tab),
            Enter => Whitespace(WhitespaceKey::Enter),

            Escape => Control(ControlKey::Escape),
            Backspace => Control(ControlKey::Backspace),
            Delete => Control(ControlKey::Delete),

            Minus => Symbol(SymbolKey::Minus),
            Equal => Symbol(SymbolKey::Equal),
            BracketLeft => Symbol(SymbolKey::BracketLeft),
            BracketRight => Symbol(SymbolKey::BracketRight),
            Backslash => Symbol(SymbolKey::Backslash),
            Semicolon => Symbol(SymbolKey::Semicolon),
            Quote => Symbol(SymbolKey::Quote),
            Comma => Symbol(SymbolKey::Comma),
            Period => Symbol(SymbolKey::Period),
            Slash => Symbol(SymbolKey::Slash),
            Backquote => Symbol(SymbolKey::Backquote),

            Numpad0 => Numpad(NumpadKey::Numpad0),
            Numpad1 => Numpad(NumpadKey::Numpad1),
            Numpad2 => Numpad(NumpadKey::Numpad2),
            Numpad3 => Numpad(NumpadKey::Numpad3),
            Numpad4 => Numpad(NumpadKey::Numpad4),
            Numpad5 => Numpad(NumpadKey::Numpad5),
            Numpad6 => Numpad(NumpadKey::Numpad6),
            Numpad7 => Numpad(NumpadKey::Numpad7),
            Numpad8 => Numpad(NumpadKey::Numpad8),
            Numpad9 => Numpad(NumpadKey::Numpad9),
            NumpadAdd => Numpad(NumpadKey::NumpadAdd),
            NumpadSubtract => Numpad(NumpadKey::NumpadSubtract),
            NumpadMultiply => Numpad(NumpadKey::NumpadMultiply),
            NumpadDivide => Numpad(NumpadKey::NumpadDivide),
            NumpadEnter => Numpad(NumpadKey::NumpadEnter),
            NumpadDecimal => Numpad(NumpadKey::NumpadDecimal),

            F1 => Function(FunctionKey::F1),
            F2 => Function(FunctionKey::F2),
            F3 => Function(FunctionKey::F3),
            F4 => Function(FunctionKey::F4),
            F5 => Function(FunctionKey::F5),
            F6 => Function(FunctionKey::F6),
            F7 => Function(FunctionKey::F7),
            F8 => Function(FunctionKey::F8),
            F9 => Function(FunctionKey::F9),
            F10 => Function(FunctionKey::F10),
            F11 => Function(FunctionKey::F11),
            F12 => Function(FunctionKey::F12),

            CapsLock => Lock(LockKey::CapsLock),
            NumLock => Lock(LockKey::NumLock),

            Insert => Navigation(NavigationKey::Insert),
            Home => Navigation(NavigationKey::Home),
            End => Navigation(NavigationKey::End),
            PageUp => Navigation(NavigationKey::PageUp),
            PageDown => Navigation(NavigationKey::PageDown),

            _ => return None,
        })
    }
}

#[cfg(feature = "window")]
impl KeyState {
    #[cfg(feature = "window")]
    pub fn from_winit(state: winit::event::ElementState, repeat: bool) -> Self {
        match (state, repeat) {
            (winit::event::ElementState::Pressed, false) => Self::Pressed,
            (winit::event::ElementState::Pressed, true) => Self::Held,
            (winit::event::ElementState::Released, _) => Self::Released,
        }
    }
}
