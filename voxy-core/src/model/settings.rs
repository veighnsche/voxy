use crate::{
    clamp_silence_auto_stop_seconds, clamp_silence_gate_threshold, clamp_vad_silence_duration_ms,
    TranscriptionModelId,
};

use super::{CoreCommand, CoreModel};

impl CoreModel {
    pub(super) fn reduce_settings_toggled(&mut self) -> Vec<CoreCommand> {
        self.ui_prefs.settings_open = !self.ui_prefs.settings_open;
        self.log_line = if self.ui_prefs.settings_open {
            "Settings opened".to_owned()
        } else {
            "Settings closed".to_owned()
        };
        Vec::new()
    }

    pub(super) fn reduce_transcription_model_changed(
        &mut self,
        model: TranscriptionModelId,
    ) -> Vec<CoreCommand> {
        self.ui_prefs.transcription_model = model;
        self.log_line = format!("Transcription model set to {}", model.as_label());
        Vec::new()
    }

    pub(super) fn reduce_silence_auto_stop_seconds_changed(
        &mut self,
        seconds: u64,
    ) -> Vec<CoreCommand> {
        self.ui_prefs.silence_auto_stop_seconds = clamp_silence_auto_stop_seconds(seconds);
        self.log_line = if self.ui_prefs.silence_auto_stop_seconds == 0 {
            "Silence auto-stop disabled".to_owned()
        } else {
            format!(
                "Silence auto-stop set to {}s",
                self.ui_prefs.silence_auto_stop_seconds
            )
        };
        Vec::new()
    }

    pub(super) fn reduce_vad_silence_duration_ms_changed(&mut self, ms: u32) -> Vec<CoreCommand> {
        self.ui_prefs.vad_silence_duration_ms = clamp_vad_silence_duration_ms(ms);
        self.log_line = format!(
            "VAD pause set to {}ms",
            self.ui_prefs.vad_silence_duration_ms
        );
        Vec::new()
    }

    pub(super) fn reduce_silence_gate_threshold_changed(
        &mut self,
        threshold: f32,
    ) -> Vec<CoreCommand> {
        self.ui_prefs.silence_gate_threshold = clamp_silence_gate_threshold(threshold);
        self.log_line = format!(
            "Silence gate threshold set to {:.2}",
            self.ui_prefs.silence_gate_threshold
        );
        Vec::new()
    }
}
