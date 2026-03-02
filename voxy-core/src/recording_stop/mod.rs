use std::time::{Duration, Instant};

use crate::clamp_silence_gate_threshold;

pub const SILENCE_GATE_RELEASE_HYSTERESIS: f32 = 0.05;
pub const SILENCE_RESET_DEBOUNCE: Duration = Duration::from_millis(300);

const MIN_LEVEL: f32 = 0.0;
const MAX_LEVEL: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingStopReason {
    SilenceAutoStop,
    MaxRecordingDuration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RecordingStopDecision {
    pub silence_seconds_remaining: Option<u64>,
    pub stop_reason: Option<RecordingStopReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RecordingStopPolicyState {
    recording_started_at: Option<Instant>,
    max_duration_triggered: bool,
    below_gate_started_at: Option<Instant>,
    above_gate_started_at: Option<Instant>,
    silence_duration_triggered: bool,
}

impl RecordingStopPolicyState {
    pub fn evaluate(
        &mut self,
        now: Instant,
        is_recording: bool,
        raw_input_level: f32,
        silence_timeout_seconds: u64,
        gate_threshold: f32,
        max_recording_duration: Option<Duration>,
    ) -> RecordingStopDecision {
        if !is_recording {
            self.reset();
            return RecordingStopDecision::default();
        }

        if self.recording_started_at.is_none() {
            self.recording_started_at = Some(now);
            self.max_duration_triggered = false;
        }

        let mut decision = RecordingStopDecision::default();
        self.apply_silence_policy(
            &mut decision,
            now,
            raw_input_level,
            silence_timeout_seconds,
            gate_threshold,
        );
        self.apply_max_duration_policy(&mut decision, now, max_recording_duration);
        decision
    }

    pub fn reset(&mut self) {
        self.recording_started_at = None;
        self.max_duration_triggered = false;
        self.below_gate_started_at = None;
        self.above_gate_started_at = None;
        self.silence_duration_triggered = false;
    }

    fn apply_silence_policy(
        &mut self,
        decision: &mut RecordingStopDecision,
        now: Instant,
        raw_input_level: f32,
        silence_timeout_seconds: u64,
        gate_threshold: f32,
    ) {
        if silence_timeout_seconds == 0 {
            self.below_gate_started_at = None;
            self.above_gate_started_at = None;
            self.silence_duration_triggered = false;
            decision.silence_seconds_remaining = None;
            return;
        }

        let duration = Duration::from_secs(silence_timeout_seconds);
        let visual_level = visual_input_level(raw_input_level);
        let gate_threshold = clamp_silence_gate_threshold(gate_threshold);
        let gate_release_threshold =
            clamp_silence_gate_threshold(gate_threshold + SILENCE_GATE_RELEASE_HYSTERESIS);

        if self.below_gate_started_at.is_none() {
            if visual_level < gate_threshold {
                self.below_gate_started_at = Some(now);
                self.above_gate_started_at = None;
                self.silence_duration_triggered = false;
            }
        } else if visual_level >= gate_release_threshold {
            if let Some(above_started_at) = self.above_gate_started_at {
                if now.saturating_duration_since(above_started_at) >= SILENCE_RESET_DEBOUNCE {
                    self.below_gate_started_at = None;
                    self.above_gate_started_at = None;
                    self.silence_duration_triggered = false;
                }
            } else {
                self.above_gate_started_at = Some(now);
            }
        } else {
            self.above_gate_started_at = None;
        }

        if let Some(started_at) = self.below_gate_started_at {
            let elapsed = now.saturating_duration_since(started_at);
            if !self.silence_duration_triggered && elapsed >= duration {
                self.silence_duration_triggered = true;
                decision.stop_reason = Some(RecordingStopReason::SilenceAutoStop);
                decision.silence_seconds_remaining = Some(0);
            } else if !self.silence_duration_triggered {
                let remaining = duration.saturating_sub(elapsed);
                decision.silence_seconds_remaining = Some(remaining.as_secs().max(1));
            }
        } else {
            decision.silence_seconds_remaining = None;
        }
    }

    fn apply_max_duration_policy(
        &mut self,
        decision: &mut RecordingStopDecision,
        now: Instant,
        max_recording_duration: Option<Duration>,
    ) {
        if decision.stop_reason.is_some() {
            return;
        }

        let Some(duration) = max_recording_duration else {
            self.max_duration_triggered = false;
            return;
        };

        let Some(started_at) = self.recording_started_at else {
            return;
        };

        if !self.max_duration_triggered && now.saturating_duration_since(started_at) >= duration {
            self.max_duration_triggered = true;
            decision.stop_reason = Some(RecordingStopReason::MaxRecordingDuration);
        }
    }
}

pub fn visual_input_level(raw_level: f32) -> f32 {
    let level = raw_level.clamp(MIN_LEVEL, MAX_LEVEL);
    if level <= 0.000_01 {
        return 0.0;
    }

    // Map linear PCM peak into a dB range so normal speech isn't stuck near zero.
    let db = 20.0 * level.log10();
    let normalized_db = ((db + 54.0) / 54.0).clamp(MIN_LEVEL, MAX_LEVEL);
    normalized_db.powf(0.8)
}

#[cfg(test)]
mod tests;
