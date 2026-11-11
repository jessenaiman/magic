//! Native file and message dialogs on the `eframe` main thread via [`craft_dialogs::Dialogs`],
//! inspectable and scriptable over `--control` (`ui.dialog.state` and `ui.dialog.override`).

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use craft_dialogs::{Answer, Dialogs, FileDialog, Kind, MessageResult, Override, Request};
use magic_ui_egui::PickerTarget;
use serde_json::{Value, json};

pub struct DialogHost {
    dialogs: Dialogs<PickerTarget>,
    pending: VecDeque<Value>,
    last: Value,
    overridden: bool,
}

impl DialogHost {
    pub fn new(ctx: &egui::Context) -> Self {
        let wake_ctx = ctx.clone();
        let dialogs = Dialogs::new(Arc::new(move || wake_ctx.request_repaint()));
        Self {
            dialogs,
            pending: VecDeque::new(),
            last: Value::Null,
            overridden: false,
        }
    }

    pub fn is_open(&self) -> bool {
        self.dialogs.is_open()
    }

    pub fn open_picker(
        &mut self,
        target: PickerTarget,
        title: &str,
        extensions: &[String],
        directory: bool,
        save: bool,
    ) {
        let mut dialog = match (directory, save) {
            (true, _) => FileDialog::folder(),
            (false, true) => FileDialog::save(),
            (false, false) => FileDialog::open(),
        }
        .title(title);
        if !extensions.is_empty() {
            let ext_refs: Vec<&str> = extensions.iter().map(String::as_str).collect();
            dialog = dialog.filter(title, &ext_refs);
        }
        let req = Request::File(dialog.clone());
        self.pending.push_back(describe_request(&req));
        self.dialogs.ask(dialog, target);
    }

    pub fn drain_answers(&mut self) -> Vec<(PickerTarget, Answer)> {
        let answers = self.dialogs.take_answers();
        let mut out = Vec::with_capacity(answers.len());
        for (target, answer) in answers {
            let req = self.pending.pop_front().unwrap_or(Value::Null);
            self.last = json!({
                "request": req,
                "answer": describe_answer(&answer),
            });
            out.push((target, answer));
        }
        out
    }

    pub fn show_pending(&mut self, frame: &eframe::Frame) {
        self.dialogs.show_pending(Some(frame));
    }

    pub fn state_json(&self) -> Value {
        json!({
            "open": self.dialogs.is_open(),
            "queued": self.dialogs.queued(),
            "override": self.overridden,
            "pending": self.pending,
            "last": self.last,
        })
    }

    pub fn apply_override(&mut self, params: &Value) -> Result<Value, String> {
        let scripted = parse_scripted(params)?;
        let ov = override_for(scripted);
        self.overridden = ov.is_some();
        self.dialogs.set_override(ov);
        Ok(json!({"ok": true, "override": self.overridden}))
    }
}

fn describe_request(request: &Request) -> Value {
    match request {
        Request::File(dialog) => json!({
            "kind": match dialog.kind {
                Kind::Open { multiple: false } => "open",
                Kind::Open { multiple: true } => "openMany",
                Kind::Folder => "folder",
                Kind::Save => "save",
            },
            "title": dialog.title,
            "filters": dialog.filters.iter().map(|f| json!({"name": f.name, "extensions": f.extensions})).collect::<Vec<_>>(),
            "directory": dialog.directory,
            "fileName": dialog.file_name,
        }),
        Request::Message(message) => json!({
            "kind": "message",
            "level": format!("{:?}", message.level).to_lowercase(),
            "title": message.title,
            "description": message.description,
            "buttons": format!("{:?}", message.buttons),
        }),
    }
}

fn describe_answer(answer: &Answer) -> Value {
    match answer {
        Answer::Cancelled => json!({"cancelled": true}),
        Answer::Paths(paths) => json!({"paths": paths}),
        Answer::Message(result) => json!({"button": match result {
            MessageResult::Ok => "ok",
            MessageResult::Cancel => "cancel",
            MessageResult::Yes => "yes",
            MessageResult::No => "no",
            MessageResult::Custom(label) => label,
        }}),
        Answer::Unavailable(why) => json!({"unavailable": why}),
    }
}

enum Scripted {
    Native,
    Answers {
        paths: Option<Vec<PathBuf>>,
        button: Option<MessageResult>,
    },
    Cancelled,
    Unavailable(String),
}

const OVERRIDE_FORMS: &str =
    "ui.dialog.override takes exactly one of: paths and/or button, cancelled: true, unavailable: reason, clear: true";

fn parse_scripted(params: &Value) -> Result<Scripted, String> {
    super::only_keys(params, "ui.dialog.override", &["paths", "button", "cancelled", "unavailable", "clear"])?;
    let paths = params.get("paths");
    let button = params.get("button");
    let forms = ["cancelled", "unavailable", "clear"]
        .iter()
        .filter(|key| params.get(**key).is_some())
        .count();
    if forms + usize::from(paths.is_some() || button.is_some()) != 1 {
        return Err(OVERRIDE_FORMS.into());
    }
    if let Some(flag) = params.get("cancelled").or_else(|| params.get("clear")) {
        if flag != true {
            return Err(OVERRIDE_FORMS.into());
        }
        return Ok(if params.get("clear").is_some() {
            Scripted::Native
        } else {
            Scripted::Cancelled
        });
    }
    if let Some(why) = params.get("unavailable") {
        return why
            .as_str()
            .map(|w| Scripted::Unavailable(w.to_owned()))
            .ok_or_else(|| "unavailable must be a string".to_string());
    }
    let paths = paths
        .map(|raw| {
            let list = raw
                .as_array()
                .filter(|l| !l.is_empty())
                .ok_or("paths must be a non-empty array of absolute paths")?;
            list.iter()
                .map(|p| {
                    p.as_str()
                        .map(PathBuf::from)
                        .filter(|pb| pb.is_absolute())
                        .ok_or("paths must be a non-empty array of absolute paths")
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    let button = button
        .map(|b| match b.as_str() {
            Some("ok") => Ok(MessageResult::Ok),
            Some("cancel") => Ok(MessageResult::Cancel),
            Some("yes") => Ok(MessageResult::Yes),
            Some("no") => Ok(MessageResult::No),
            Some(label) => Ok(MessageResult::Custom(label.to_owned())),
            None => Err("button must be a string"),
        })
        .transpose()?;
    Ok(Scripted::Answers { paths, button })
}

fn override_for(scripted: Scripted) -> Option<Override> {
    match scripted {
        Scripted::Native => None,
        Scripted::Cancelled => Some(Box::new(|request| match request {
            Request::File(_) => Answer::Cancelled,
            Request::Message(_) => Answer::Message(MessageResult::Cancel),
        })),
        Scripted::Unavailable(why) => Some(Box::new(move |_| Answer::Unavailable(why.clone()))),
        Scripted::Answers { paths, button } => Some(Box::new(move |request| match request {
            Request::File(_) => paths
                .clone()
                .map_or_else(|| Answer::Unavailable("the override has no paths for a file dialog".into()), Answer::Paths),
            Request::Message(_) => button
                .clone()
                .map_or_else(|| Answer::Unavailable("the override has no button for a message box".into()), Answer::Message),
        })),
    }
}
