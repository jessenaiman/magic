//! Synthetic `egui` 0.37 pointer, scroll, keyboard, text, and focus event injection for `--control`
//! (`ui.click`, `ui.move`, `ui.drag`, `ui.scroll`, `ui.key`, `ui.type`, `ui.focus`).

use egui::{
    Event, Key, Modifiers, MouseWheelSource, MouseWheelUnit, PointerButton, Pos2, TouchPhase, Vec2,
};
use magic_ui_egui::AutomationRegistry;
use serde_json::{Value, json};

pub fn parse_modifiers(raw: Option<&Value>) -> Result<Modifiers, String> {
    let Some(val) = raw else {
        return Ok(Modifiers::NONE);
    };
    if val.is_null() {
        return Ok(Modifiers::NONE);
    }
    if let Some(obj) = val.as_object() {
        for key in obj.keys() {
            if !matches!(key.as_str(), "shift" | "alt" | "command" | "ctrl") {
                return Err(format!("unknown modifier `{key}` (expected shift, alt, command, ctrl)"));
            }
        }
        let shift = obj.get("shift").and_then(Value::as_bool).unwrap_or(false);
        let alt = obj.get("alt").and_then(Value::as_bool).unwrap_or(false);
        let command = obj.get("command").and_then(Value::as_bool).unwrap_or(false);
        let ctrl_flag = obj.get("ctrl").and_then(Value::as_bool).unwrap_or(false);
        let on_mac = cfg!(target_os = "macos");
        return Ok(Modifiers {
            alt,
            ctrl: ctrl_flag || (!on_mac && command),
            shift,
            mac_cmd: on_mac && command,
            command,
        });
    }
    if let Some(arr) = val.as_array() {
        let mut mods = Modifiers::NONE;
        let on_mac = cfg!(target_os = "macos");
        for item in arr {
            match item.as_str().unwrap_or_default() {
                "Shift" | "shift" => mods.shift = true,
                "Alt" | "alt" => mods.alt = true,
                "Ctrl" | "Control" | "ctrl" => {
                    mods.ctrl = true;
                    if !on_mac {
                        mods.command = true;
                    }
                }
                "Meta" | "Command" | "Cmd" | "command" => {
                    mods.command = true;
                    if on_mac {
                        mods.mac_cmd = true;
                    } else {
                        mods.ctrl = true;
                    }
                }
                other => return Err(format!("unknown modifier `{other}`")),
            }
        }
        return Ok(mods);
    }
    Err("modifiers must be an object `{shift?, alt?, command?, ctrl?}`".into())
}

pub fn resolve_point(reg: &AutomationRegistry, params: &Value, method: &str) -> Result<Pos2, String> {
    if let Some(id) = params.get("id").and_then(Value::as_str) {
        let elem = reg
            .get(id)
            .ok_or_else(|| format!("{method}: widget `{id}` is missing or off-screen"))?;
        if !elem.enabled {
            return Err(format!("{method}: widget `{id}` is disabled"));
        }
        let fx = parse_fraction(params.get("fx"), 0.5, "fx")?;
        let fy = parse_fraction(params.get("fy"), 0.5, "fy")?;
        let x = elem.rect.min.x + elem.rect.width() * fx;
        let y = elem.rect.min.y + elem.rect.height() * fy;
        return Ok(Pos2::new(x, y));
    }
    let x = params
        .get("x")
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{method}: provide `id` or finite `x` and `y` coordinates"))?;
    let y = params
        .get("y")
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{method}: provide `id` or finite `x` and `y` coordinates"))?;
    Ok(Pos2::new(x as f32, y as f32))
}

fn parse_fraction(raw: Option<&Value>, default: f32, name: &str) -> Result<f32, String> {
    let Some(val) = raw.filter(|v| !v.is_null()) else {
        return Ok(default);
    };
    let f = val
        .as_f64()
        .filter(|n| n.is_finite() && (0.0..=1.0).contains(n))
        .ok_or_else(|| format!("{name} must be a number in 0.0..=1.0"))?;
    Ok(f as f32)
}

fn parse_button(raw: Option<&Value>) -> Result<PointerButton, String> {
    match raw.and_then(Value::as_str).unwrap_or("left") {
        "left" | "primary" => Ok(PointerButton::Primary),
        "right" | "secondary" => Ok(PointerButton::Secondary),
        "middle" => Ok(PointerButton::Middle),
        other => Err(format!("unknown pointer button `{other}` (expected left, right, or middle)")),
    }
}

pub fn build_move(reg: &AutomationRegistry, params: &Value) -> Result<(Vec<Event>, Value), String> {
    super::only_keys(params, "ui.move", &["id", "fx", "fy", "x", "y"])?;
    let pos = resolve_point(reg, params, "ui.move")?;
    Ok((
        vec![Event::PointerMoved(pos)],
        json!({"ok": true, "x": pos.x, "y": pos.y}),
    ))
}

pub fn build_click(reg: &AutomationRegistry, params: &Value) -> Result<(Vec<Event>, Value), String> {
    super::only_keys(params, "ui.click", &["id", "fx", "fy", "x", "y", "button", "count", "modifiers"])?;
    let count = match params.get("count") {
        None | Some(Value::Null) => 1u32,
        Some(v) => v
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .filter(|&n| n <= 10)
            .ok_or("ui.click: count must be an integer from 0 to 10")?,
    };
    let button = parse_button(params.get("button"))?;
    let modifiers = parse_modifiers(params.get("modifiers"))?;
    let pos = resolve_point(reg, params, "ui.click")?;
    let mut events = vec![Event::PointerMoved(pos)];
    for _ in 0..count {
        events.push(Event::PointerButton {
            pos,
            button,
            pressed: true,
            modifiers,
        });
        events.push(Event::PointerButton {
            pos,
            button,
            pressed: false,
            modifiers,
        });
    }
    Ok((
        events,
        json!({"ok": true, "x": pos.x, "y": pos.y, "count": count}),
    ))
}

pub fn build_drag(reg: &AutomationRegistry, params: &Value) -> Result<(Vec<Event>, Value), String> {
    super::only_keys(params, "ui.drag", &["from", "to", "button", "steps", "modifiers"])?;
    let from_obj = params.get("from").ok_or("ui.drag requires `from`")?;
    let to_obj = params.get("to").ok_or("ui.drag requires `to`")?;
    let from = resolve_point(reg, from_obj, "ui.drag.from")?;
    let to = resolve_point(reg, to_obj, "ui.drag.to")?;
    let raw_steps = match params.get("steps") {
        None | Some(Value::Null) => 12u32,
        Some(v) => v
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .filter(|&n| n <= 1024)
            .ok_or("ui.drag: steps must be an integer from 0 to 1024")?,
    };
    let steps = raw_steps.max(2);
    let button = parse_button(params.get("button"))?;
    let modifiers = parse_modifiers(params.get("modifiers"))?;
    let mut events = Vec::with_capacity((steps as usize) + 4);
    events.push(Event::PointerMoved(from));
    events.push(Event::PointerButton {
        pos: from,
        button,
        pressed: true,
        modifiers,
    });
    for i in 1..=steps {
        let t = (i as f32) / (steps as f32);
        let p = Pos2::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
        events.push(Event::PointerMoved(p));
    }
    events.push(Event::PointerButton {
        pos: to,
        button,
        pressed: false,
        modifiers,
    });
    Ok((
        events,
        json!({"ok": true, "from": [from.x, from.y], "to": [to.x, to.y], "steps": steps}),
    ))
}

pub fn build_scroll(reg: &AutomationRegistry, params: &Value) -> Result<(Vec<Event>, Value), String> {
    super::only_keys(
        params,
        "ui.scroll",
        &["id", "fx", "fy", "x", "y", "dx", "dy", "unit", "source", "modifiers"],
    )?;
    let dx = params.get("dx").and_then(Value::as_f64).unwrap_or(0.0) as f32;
    let dy = params.get("dy").and_then(Value::as_f64).unwrap_or(0.0) as f32;
    if !dx.is_finite() || !dy.is_finite() {
        return Err("ui.scroll: dx and dy must be finite numbers".into());
    }
    let unit = match params.get("unit").and_then(Value::as_str).unwrap_or("point") {
        "point" | "pixel" => MouseWheelUnit::Point,
        "line" => MouseWheelUnit::Line,
        other => return Err(format!("ui.scroll: unknown unit `{other}` (expected point or line)")),
    };
    let source = match params.get("source").and_then(Value::as_str) {
        Some("trackpad") => MouseWheelSource::Trackpad,
        Some("wheel") => MouseWheelSource::Wheel,
        None if matches!(unit, MouseWheelUnit::Line) => MouseWheelSource::Wheel,
        None => MouseWheelSource::Trackpad,
        Some(other) => return Err(format!("ui.scroll: unknown source `{other}` (expected wheel or trackpad)")),
    };
    let modifiers = parse_modifiers(params.get("modifiers"))?;
    let mut events = Vec::new();
    if params.get("id").is_some() || params.get("x").is_some() {
        let pos = resolve_point(reg, params, "ui.scroll")?;
        events.push(Event::PointerMoved(pos));
    }
    events.push(Event::MouseWheel {
        unit,
        delta: Vec2::new(dx, dy),
        modifiers,
        phase: TouchPhase::Move,
        source,
    });
    Ok((events, json!({"ok": true, "dx": dx, "dy": dy})))
}

pub fn build_key(params: &Value) -> Result<(Vec<Event>, Value), String> {
    super::only_keys(params, "ui.key", &["key", "modifiers"])?;
    let raw_key = params
        .get("key")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or("ui.key requires non-empty `key`")?;
    let mut modifiers = parse_modifiers(params.get("modifiers"))?;
    let mut key_part = raw_key;
    if raw_key.contains('+') {
        for part in raw_key.split('+') {
            match part.trim() {
                "Cmd" | "Meta" | "Command" => {
                    modifiers.command = true;
                    if cfg!(target_os = "macos") {
                        modifiers.mac_cmd = true;
                    } else {
                        modifiers.ctrl = true;
                    }
                }
                "Ctrl" | "Control" => {
                    modifiers.ctrl = true;
                    if !cfg!(target_os = "macos") {
                        modifiers.command = true;
                    }
                }
                "Shift" => modifiers.shift = true,
                "Alt" | "Option" => modifiers.alt = true,
                "Physical" => {}
                tail => key_part = tail,
            }
        }
    }
    let key = map_egui_key(key_part).ok_or_else(|| format!("ui.key: unrecognized key `{key_part}`"))?;
    let events = vec![
        Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        },
        Event::Key {
            key,
            physical_key: Some(key),
            pressed: false,
            repeat: false,
            modifiers,
        },
    ];
    Ok((events, json!({"ok": true, "key": raw_key})))
}

fn map_egui_key(name: &str) -> Option<Key> {
    let trimmed = name.strip_prefix("Key").unwrap_or(name);
    match trimmed {
        "Enter" | "Return" => Some(Key::Enter),
        "Tab" => Some(Key::Tab),
        "Escape" | "Esc" => Some(Key::Escape),
        "Backspace" => Some(Key::Backspace),
        "Delete" => Some(Key::Delete),
        "Space" | " " => Some(Key::Space),
        "ArrowUp" | "Up" => Some(Key::ArrowUp),
        "ArrowDown" | "Down" => Some(Key::ArrowDown),
        "ArrowLeft" | "Left" => Some(Key::ArrowLeft),
        "ArrowRight" | "Right" => Some(Key::ArrowRight),
        "Home" => Some(Key::Home),
        "End" => Some(Key::End),
        "PageUp" => Some(Key::PageUp),
        "PageDown" => Some(Key::PageDown),
        other => Key::from_name(other),
    }
}
