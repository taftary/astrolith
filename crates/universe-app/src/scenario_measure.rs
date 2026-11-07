//! Buffered bounded measurement sidecar; real-input exit remains authoritative.

use bevy::prelude::*;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use universe_render::MeasureDiagnostics;

const OUTPUT_BYTES: usize = 256 * 1024 * 1024;

struct Output {
    writer: BufWriter<std::fs::File>,
    lines: Vec<String>,
    bytes: usize,
    problem: Option<String>,
    finalized: bool,
}

impl Output {
    fn append(&mut self, line: &str) -> Result<(), String> {
        if self.bytes.saturating_add(line.len() + 1) > OUTPUT_BYTES {
            return Err("input measurement sidecar exceeds 256 MiB output limit".into());
        }
        writeln!(self.writer, "{line}")
            .map_err(|e| format!("cannot write input measurement: {e}"))?;
        self.bytes += line.len() + 1;
        Ok(())
    }

    fn drain(&mut self, diagnostics: &MeasureDiagnostics) -> Result<(), String> {
        let drained = diagnostics.drain_lines(&mut self.lines);
        let mut lines = std::mem::take(&mut self.lines);
        let written = lines.drain(..).try_for_each(|line| self.append(&line));
        self.lines = lines;
        self.writer
            .flush()
            .map_err(|e| format!("cannot flush input measurement: {e}"))?;
        written.and(drained)
    }
}

#[derive(Resource, Clone)]
pub(crate) struct MeasureOutput(Arc<Mutex<Output>>);

impl MeasureOutput {
    pub(crate) fn create(directory: &Path) -> Result<Self, String> {
        let file = std::fs::File::create_new(directory.join("input-measure-v1.tsv"))
            .map_err(|e| format!("cannot create input measurement: {e}"))?;
        let mut output = Output {
            writer: BufWriter::new(file),
            lines: Vec::with_capacity(universe_render::MEASURE_EVENT_LIMIT),
            bytes: 0,
            problem: None,
            finalized: false,
        };
        output.append("MEASURE-RUN\tversion=1\tevent_limit=8192\toutput_limit_bytes=268435456\tclock=shared-input-loading-monotonic\tinterval_scope=First-to-First\tstate_source=actual-scenario-observation-not-requested-assertions\tprocess_clock_link=bracketed-wall-observations-with-drift-check\tshutdown_idle_or_gpu_release_claim=false")?;
        output.writer.flush().map_err(|e| e.to_string())?;
        Ok(Self(Arc::new(Mutex::new(output))))
    }

    pub(crate) fn problem(&self) -> Option<String> {
        self.0.lock().map_or_else(
            |_| Some("input measurement output lock poisoned".into()),
            |o| o.problem.clone(),
        )
    }

    pub(crate) fn finish(
        &self,
        diagnostics: &MeasureDiagnostics,
        scenario_ok: bool,
    ) -> Result<(), String> {
        let end = diagnostics.seal_line(scenario_ok && self.problem().is_none())?;
        let mut output = self
            .0
            .lock()
            .map_err(|_| "input measurement output lock poisoned")?;
        if output.finalized {
            return Err("input measurement already finalized".into());
        }
        output.finalized = true;
        let drained = output.drain(diagnostics);
        let problem = output.problem.clone().or_else(|| drained.err());
        if problem.is_none() {
            output.append(&end)?;
        } else {
            output.append(&end.replace("complete=true", "complete=false"))?;
        }
        output.writer.flush().map_err(|e| e.to_string())?;
        problem.map_or(Ok(()), Err)
    }
}

pub(crate) fn stream(diagnostics: Res<MeasureDiagnostics>, output: Res<MeasureOutput>) {
    if let Ok(mut output) = output.0.lock()
        && output.problem.is_none()
        && let Err(reason) = output.drain(&diagnostics)
    {
        output.problem = Some(reason);
    }
}
