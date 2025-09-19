# Hash: 9eb9adaa71d83ca286a669fb0c6f77acde32d0abcc4f95619bf2a9edc2dc4cef
# Generated: 2025-09-19T14:54:30+00:00
# Rust type: impact::input::key::KeyboardKey
# Type category: Inline
# Commit: fc08276f (dirty)
module [
    KeyboardKey,
    write_bytes,
    from_bytes,
]

import Input.ArrowKey
import Input.ControlKey
import Input.FunctionKey
import Input.LetterKey
import Input.LockKey
import Input.ModifierKey
import Input.NavigationKey
import Input.NumberKey
import Input.NumpadKey
import Input.SymbolKey
import Input.WhitespaceKey

## A key on a keyboard.
KeyboardKey : [
    Letter Input.LetterKey.LetterKey,
    Number Input.NumberKey.NumberKey,
    Arrow Input.ArrowKey.ArrowKey,
    Modifier Input.ModifierKey.ModifierKey,
    Whitespace Input.WhitespaceKey.WhitespaceKey,
    Control Input.ControlKey.ControlKey,
    Symbol Input.SymbolKey.SymbolKey,
    Numpad Input.NumpadKey.NumpadKey,
    Function Input.FunctionKey.FunctionKey,
    Lock Input.LockKey.LockKey,
    Navigation Input.NavigationKey.NavigationKey,
]

## Serializes a value of [KeyboardKey] into the binary representation
## expected by the engine and appends the bytes to the list.
write_bytes : List U8, KeyboardKey -> List U8
write_bytes = |bytes, value|
    when value is
        Letter(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(0)
            |> Input.LetterKey.write_bytes(val)

        Number(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(1)
            |> Input.NumberKey.write_bytes(val)

        Arrow(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(2)
            |> Input.ArrowKey.write_bytes(val)

        Modifier(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(3)
            |> Input.ModifierKey.write_bytes(val)

        Whitespace(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(4)
            |> Input.WhitespaceKey.write_bytes(val)

        Control(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(5)
            |> Input.ControlKey.write_bytes(val)

        Symbol(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(6)
            |> Input.SymbolKey.write_bytes(val)

        Numpad(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(7)
            |> Input.NumpadKey.write_bytes(val)

        Function(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(8)
            |> Input.FunctionKey.write_bytes(val)

        Lock(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(9)
            |> Input.LockKey.write_bytes(val)

        Navigation(val) ->
            bytes
            |> List.reserve(2)
            |> List.append(10)
            |> Input.NavigationKey.write_bytes(val)

## Deserializes a value of [KeyboardKey] from its bytes in the
## representation used by the engine.
from_bytes : List U8 -> Result KeyboardKey _
from_bytes = |bytes|
    if List.len(bytes) != 2 then
        Err(InvalidNumberOfBytes)
    else
        when bytes is
            [0, .. as data_bytes] ->
                Ok(
                    Letter(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Input.LetterKey.from_bytes?,
                    ),
                )

            [1, .. as data_bytes] ->
                Ok(
                    Number(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Input.NumberKey.from_bytes?,
                    ),
                )

            [2, .. as data_bytes] ->
                Ok(
                    Arrow(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Input.ArrowKey.from_bytes?,
                    ),
                )

            [3, .. as data_bytes] ->
                Ok(
                    Modifier(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Input.ModifierKey.from_bytes?,
                    ),
                )

            [4, .. as data_bytes] ->
                Ok(
                    Whitespace(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Input.WhitespaceKey.from_bytes?,
                    ),
                )

            [5, .. as data_bytes] ->
                Ok(
                    Control(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Input.ControlKey.from_bytes?,
                    ),
                )

            [6, .. as data_bytes] ->
                Ok(
                    Symbol(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Input.SymbolKey.from_bytes?,
                    ),
                )

            [7, .. as data_bytes] ->
                Ok(
                    Numpad(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Input.NumpadKey.from_bytes?,
                    ),
                )

            [8, .. as data_bytes] ->
                Ok(
                    Function(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Input.FunctionKey.from_bytes?,
                    ),
                )

            [9, .. as data_bytes] ->
                Ok(
                    Lock(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Input.LockKey.from_bytes?,
                    ),
                )

            [10, .. as data_bytes] ->
                Ok(
                    Navigation(
                        data_bytes |> List.sublist({ start: 0, len: 1 }) |> Input.NavigationKey.from_bytes?,
                    ),
                )

            [] -> Err(MissingDiscriminant)
            [discr, ..] -> Err(InvalidDiscriminant(discr))
