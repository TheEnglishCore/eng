use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use eframe::egui;

use crate::ast::{Statement, WidgetKind};
use crate::error::{EnglingError, Result};
use crate::vm::VM;

pub struct UiState {
    pub windows: HashMap<String, WindowState>,
    pub handlers: Vec<(String, String)>,
}

pub struct WindowState {
    pub title: String,
    pub widgets: Vec<WidgetState>,
}

pub enum WidgetState {
    Button {
        label: String,
    },
    Label {
        label: String,
        name: String,
        text: String,
    },
    TextField {
        label: String,
        text: String,
    },
}

static UI_STATE: Mutex<Option<UiState>> = Mutex::new(None);

/// Initialize the UI registry. The VM is registered separately via
/// [`register_vm`] after the program has finished running and its globals
/// (function definitions, etc.) have been populated.
pub fn init_ui() {
    *UI_STATE.lock().unwrap() = Some(UiState {
        windows: HashMap::new(),
        handlers: Vec::new(),
    });
}

/// Register the live interpreter VM so that button-click handlers can call
/// functions defined in the user's program.
pub fn register_vm(vm: &VM) {
    // Snapshot the current globals into a fresh VM wrapped in Arc<Mutex>.
    // Subsequent event handlers will call functions in this VM's scope.
    SHARED_VM
        .lock()
        .unwrap()
        .replace(Arc::new(Mutex::new(clone_vm(vm))));
}

static SHARED_VM: Mutex<Option<Arc<Mutex<VM>>>> = Mutex::new(None);

fn clone_vm(vm: &VM) -> VM {
    // The VM has no interior mutability beyond its internal state, so a
    // shallow clone via Default + manual sync would be wrong. Instead, take
    // ownership of a fresh VM and copy the scopes (globals, locals).
    let mut new_vm = VM::new();
    for (name, value) in vm.scopes_mut().globals() {
        new_vm.scopes_mut().set_global(name.clone(), value.clone());
    }
    new_vm
}

pub fn process_ui_statement(stmt: &Statement) -> Result<()> {
    let mut state = UI_STATE.lock().unwrap();
    let state = state
        .as_mut()
        .ok_or_else(|| EnglingError::runtime("UI not initialized"))?;

    match stmt {
        Statement::WindowDecl { name, title } => {
            state.windows.insert(
                name.clone(),
                WindowState {
                    title: title.clone(),
                    widgets: Vec::new(),
                },
            );
        }
        Statement::WidgetDecl {
            window,
            kind,
            label,
        } => {
            let win = state.windows.get_mut(window).ok_or_else(|| {
                EnglingError::runtime(format!("Window '{window}' is not defined"))
            })?;
            let widget = match kind {
                WidgetKind::Button => WidgetState::Button {
                    label: label.clone(),
                },
                WidgetKind::Label => WidgetState::Label {
                    label: label.clone(),
                    name: label.clone(),
                    text: label.clone(),
                },
                WidgetKind::TextField => WidgetState::TextField {
                    label: label.clone(),
                    text: String::new(),
                },
            };
            win.widgets.push(widget);
        }
        Statement::EventHandler {
            button_label,
            function,
        } => {
            state
                .handlers
                .push((button_label.clone(), function.clone()));
        }
        _ => {}
    }
    Ok(())
}

pub fn update_label_text(name: &str, text: String) -> Result<()> {
    let mut state = UI_STATE.lock().unwrap();
    let state = state
        .as_mut()
        .ok_or_else(|| EnglingError::runtime("UI not initialized"))?;
    for win in state.windows.values_mut() {
        for widget in win.widgets.iter_mut() {
            if let WidgetState::Label {
                name: n, text: t, ..
            } = widget
            {
                if n == name {
                    *t = text;
                }
            }
        }
    }
    Ok(())
}

pub fn run_event_loop() -> Result<()> {
    let state = UI_STATE
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| EnglingError::runtime("No UI windows to display"))?;

    let window_name = state
        .windows
        .keys()
        .next()
        .cloned()
        .ok_or_else(|| EnglingError::runtime("No windows defined"))?;

    let window_state = state.windows.get(&window_name).unwrap().clone();
    let handlers = state.handlers.clone();
    let vm =
        SHARED_VM.lock().unwrap().clone().ok_or_else(|| {
            EnglingError::runtime("UI VM not registered; call register_vm() first")
        })?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_title(window_state.title.clone()),
        ..Default::default()
    };

    eframe::run_simple_native(&window_state.title, options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(&window_state.title);
            ui.separator();

            for widget in &window_state.widgets {
                match widget {
                    WidgetState::Button { label } => {
                        if ui.button(label).clicked() {
                            for (btn_label, func_name) in &handlers {
                                if btn_label == label {
                                    if let Ok(mut vm) = vm.lock() {
                                        if let Err(e) = vm.call_function(func_name, vec![]) {
                                            eprintln!("Event handler error: {e}");
                                        }
                                    }
                                }
                            }
                        }
                    }
                    WidgetState::Label { text, .. } => {
                        ui.label(text);
                    }
                    WidgetState::TextField { label, text } => {
                        ui.horizontal(|ui| {
                            ui.label(label);
                            ui.text_edit_singleline(text);
                        });
                    }
                }
            }
        });
    })
    .map_err(|e| EnglingError::runtime(format!("UI error: {e}")))?;

    Ok(())
}
