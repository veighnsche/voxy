use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use gtk::{prelude::*, Application};
use tokio::{runtime::Runtime, sync::mpsc};
use voxy_audio::{AudioInput, NoopAudioInput};
use voxy_core::{AppEvent, AppState, CoreCommand, CoreModel};
use voxy_stt::{DummyStreamingTranscriber, StreamingTranscriber};

use crate::ui::{self, ViewModel, Widgets};

const APP_ID: &str = "com.vince.voxy";
const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Default)]
struct WindowFlags {
    pinned: bool,
}

pub fn run() {
    let runtime = Arc::new(Runtime::new().expect("failed to create tokio runtime"));

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(move |app| activate(app, Arc::clone(&runtime)));

    app.run();
}

fn activate(app: &Application, runtime: Arc<Runtime>) {
    let widgets = ui::build(app);
    let model = Rc::new(RefCell::new(CoreModel::default()));
    let window_flags = Rc::new(RefCell::new(WindowFlags::default()));
    let applying_text_update = Rc::new(Cell::new(false));

    let (event_tx, event_rx) = mpsc::channel::<AppEvent>(64);
    let event_rx = Rc::new(RefCell::new(event_rx));

    let transcriber = Arc::new(DummyStreamingTranscriber::new(event_tx.clone()));
    let audio_input = Arc::new(NoopAudioInput);

    wire_ui_signals(
        widgets.clone(),
        Rc::clone(&model),
        Rc::clone(&window_flags),
        Rc::clone(&applying_text_update),
        event_tx.clone(),
    );

    start_event_loop(
        widgets.clone(),
        Rc::clone(&model),
        Rc::clone(&window_flags),
        Rc::clone(&applying_text_update),
        Rc::clone(&event_rx),
        event_tx,
        Arc::clone(&transcriber),
        Arc::clone(&audio_input),
        runtime,
    );

    render_view(&widgets, &model, &window_flags, &applying_text_update);
    widgets.window.present();
}

fn wire_ui_signals(
    widgets: Widgets,
    model: Rc<RefCell<CoreModel>>,
    window_flags: Rc<RefCell<WindowFlags>>,
    applying_text_update: Rc<Cell<bool>>,
    event_tx: mpsc::Sender<AppEvent>,
) {
    {
        let event_tx = event_tx.clone();
        widgets.mic_button.connect_clicked(move |_| {
            let _ = event_tx.try_send(AppEvent::MicToggled);
        });
    }

    {
        let event_tx = event_tx.clone();
        widgets.reset_button.connect_clicked(move |_| {
            let _ = event_tx.try_send(AppEvent::ResetRequested);
        });
    }

    {
        let widgets = widgets.clone();
        let model = Rc::clone(&model);
        let window_flags = Rc::clone(&window_flags);
        let applying_text_update = Rc::clone(&applying_text_update);

        // Keep-above integration is intentionally deferred at scaffold stage.
        widgets.pin_button.connect_clicked(move |_| {
            {
                let mut flags = window_flags.borrow_mut();
                flags.pinned = !flags.pinned;
            }
            render_view(&widgets, &model, &window_flags, &applying_text_update);
        });
    }

    {
        let model = Rc::clone(&model);
        let applying_text_update = Rc::clone(&applying_text_update);

        widgets.text_buffer.connect_changed(move |buffer| {
            if applying_text_update.get() {
                return;
            }

            let text = buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), false)
                .to_string();

            model.borrow_mut().apply_user_edit(text);
        });
    }
}

fn start_event_loop(
    widgets: Widgets,
    model: Rc<RefCell<CoreModel>>,
    window_flags: Rc<RefCell<WindowFlags>>,
    applying_text_update: Rc<Cell<bool>>,
    event_rx: Rc<RefCell<mpsc::Receiver<AppEvent>>>,
    event_tx: mpsc::Sender<AppEvent>,
    transcriber: Arc<DummyStreamingTranscriber>,
    audio_input: Arc<NoopAudioInput>,
    runtime: Arc<Runtime>,
) {
    gtk4::glib::timeout_add_local(EVENT_POLL_INTERVAL, move || {
        loop {
            let event = match event_rx.borrow_mut().try_recv() {
                Ok(event) => event,
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    return gtk4::glib::ControlFlow::Break
                }
            };

            handle_event(
                event,
                &model,
                &event_tx,
                &transcriber,
                &audio_input,
                &runtime,
            );
        }

        render_view(&widgets, &model, &window_flags, &applying_text_update);
        gtk4::glib::ControlFlow::Continue
    });
}

fn handle_event(
    event: AppEvent,
    model: &Rc<RefCell<CoreModel>>,
    event_tx: &mpsc::Sender<AppEvent>,
    transcriber: &Arc<DummyStreamingTranscriber>,
    audio_input: &Arc<NoopAudioInput>,
    runtime: &Arc<Runtime>,
) {
    let commands = model.borrow_mut().reduce(event);
    execute_commands(commands, event_tx, transcriber, audio_input, runtime);
}

fn execute_commands(
    commands: Vec<CoreCommand>,
    event_tx: &mpsc::Sender<AppEvent>,
    transcriber: &Arc<DummyStreamingTranscriber>,
    audio_input: &Arc<NoopAudioInput>,
    runtime: &Arc<Runtime>,
) {
    for command in commands {
        match command {
            CoreCommand::StartAudioInput => audio_input.start(),
            CoreCommand::StopAudioInput => audio_input.stop(),
            CoreCommand::StartTranscriber => {
                let transcriber = Arc::clone(transcriber);
                runtime.spawn(async move {
                    transcriber.start().await;
                });
            }
            CoreCommand::StopTranscriber => {
                let transcriber = Arc::clone(transcriber);
                runtime.spawn(async move {
                    transcriber.stop().await;
                });
            }
            CoreCommand::StopTranscriberThenEmit(event) => {
                let transcriber = Arc::clone(transcriber);
                let event_tx = event_tx.clone();
                runtime.spawn(async move {
                    transcriber.stop().await;
                    let _ = event_tx.send(event).await;
                });
            }
            CoreCommand::EmitEvent(event) => {
                let event_tx = event_tx.clone();
                runtime.spawn(async move {
                    let _ = event_tx.send(event).await;
                });
            }
        }
    }
}

fn render_view(
    widgets: &Widgets,
    model: &Rc<RefCell<CoreModel>>,
    window_flags: &Rc<RefCell<WindowFlags>>,
    applying_text_update: &Rc<Cell<bool>>,
) {
    let pinned = window_flags.borrow().pinned;
    let model = model.borrow();

    let view_model = build_view_model(&model, pinned);
    ui::render(widgets, &view_model, applying_text_update);
}

fn build_view_model(model: &CoreModel, pinned: bool) -> ViewModel {
    let mic_on = matches!(model.app_state, AppState::Recording);

    let state_text = match &model.app_state {
        AppState::Idle => "Idle".to_owned(),
        AppState::Recording => "Recording".to_owned(),
        AppState::Processing => "Processing".to_owned(),
        AppState::Error(message) => format!("Error({message})"),
    };

    let pin_text = if pinned { "Pinned" } else { "Unpinned" };

    ViewModel {
        text: model.buffer.full_text(),
        mic_on,
        pinned,
        status_text: format!("State: {state_text} | Window: {pin_text}"),
    }
}
