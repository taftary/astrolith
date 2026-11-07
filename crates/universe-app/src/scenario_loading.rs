//! Opt-in streamed loading sidecar; bounded file size and sticky non-success.

use bevy::prelude::*;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use universe_render::{LoadingDiagnostics, ScenarioRun};

const OUTPUT_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Collection {
    Off,
    Loading,
    Measure,
}

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
            return Err("loading sidecar exceeds 256 MiB output limit".into());
        }
        writeln!(self.writer, "{line}")
            .map_err(|e| format!("cannot write loading evidence: {e}"))?;
        self.bytes += line.len() + 1;
        Ok(())
    }

    fn drain(&mut self, diagnostics: &LoadingDiagnostics) -> Result<(), String> {
        let drain = diagnostics.drain_lines(&mut self.lines);
        // Reuse the bounded output buffer. Partial rows precede a sticky loss.
        let mut lines = std::mem::take(&mut self.lines);
        let result = lines.drain(..).try_for_each(|line| self.append(&line));
        self.lines = lines;
        self.writer
            .flush()
            .map_err(|e| format!("cannot flush loading evidence: {e}"))?;
        result.and(drain)?;
        diagnostics.problem().map_or(Ok(()), Err)
    }
}

#[derive(Resource, Clone)]
pub(crate) struct LoadingOutput(Arc<Mutex<Output>>);

impl LoadingOutput {
    pub(crate) fn create(root: &Path) -> Result<Self, String> {
        let file = std::fs::File::create_new(root.join("loading-v1.tsv"))
            .map_err(|e| format!("cannot create loading sidecar: {e}"))?;
        let mut output = Output {
            writer: BufWriter::new(file),
            lines: Vec::with_capacity(universe_render::LOADING_EVENT_LIMIT),
            bytes: 0,
            problem: None,
            finalized: false,
        };
        output.append("LOAD-RUN\tversion=1\tevent_limit=8192\tresource_limit=8192\toutput_limit_bytes=268435456\tscope=app-session-through-seal\tshutdown_idle_or_gpu_release_claim=false")?;
        output.writer.flush().map_err(|e| e.to_string())?;
        Ok(Self(Arc::new(Mutex::new(output))))
    }

    pub(crate) fn problem(&self) -> Option<String> {
        self.0.lock().map_or_else(
            |_| Some("loading output lock poisoned".into()),
            |o| o.problem.clone(),
        )
    }

    pub(crate) fn finish(
        &self,
        diagnostics: &LoadingDiagnostics,
        scenario: &Result<(), String>,
    ) -> Result<(), String> {
        let boundary = diagnostics.seal()?;
        let mut output = self.0.lock().map_err(|_| "loading output lock poisoned")?;
        if output.finalized {
            return Err("loading output already finalized".into());
        }
        output.finalized = true;
        let drained = output.drain(diagnostics);
        let problem = output.problem.clone().or_else(|| drained.err());
        let complete = scenario.is_ok() && problem.is_none();
        output.append(&format!("LOAD-END\tversion=1\tcomplete={complete}\tmain_frame={}\trecords={}\telapsed_ns={}\tall_jobs_or_retirements_finished=not-claimed", boundary.main_frame, boundary.records, boundary.elapsed_ns))?;
        output.writer.flush().map_err(|e| e.to_string())?;
        problem.map_or(Ok(()), Err)
    }
}

pub(crate) fn stream(
    diagnostics: Res<LoadingDiagnostics>,
    output: Res<LoadingOutput>,
    _run: Res<ScenarioRun>,
) {
    if let Ok(mut output) = output.0.lock()
        && output.problem.is_none()
        && let Err(reason) = output.drain(&diagnostics)
    {
        output.problem = Some(reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_output_refuses_overwrite_and_cannot_hide_an_output_limit() {
        let root =
            std::env::temp_dir().join(format!("astrolith-loading-output-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        let output = LoadingOutput::create(&root).unwrap();
        let file = root.join("loading-v1.tsv");
        let original = std::fs::read(&file).unwrap();
        assert!(LoadingOutput::create(&root).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), original);
        {
            let mut value = output.0.lock().unwrap();
            value.bytes = OUTPUT_BYTES;
            assert!(value.append("required-record").is_err());
            assert!(value.append("LOAD-END\tcomplete=true").is_err());
        }
        assert_eq!(std::fs::read(&file).unwrap(), original);
        drop(output);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn loading_output_cannot_replace_scenario_failure_or_a_sticky_collection_problem() {
        let root = std::env::temp_dir().join(format!(
            "astrolith-loading-completion-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        for (name, scenario, problem) in [
            ("scenario", Err("unmet required observation".into()), None),
            (
                "collection",
                Ok(()),
                Some("required record unavailable".to_string()),
            ),
        ] {
            let sub = root.join(name);
            std::fs::create_dir(&sub).unwrap();
            let output = LoadingOutput::create(&sub).unwrap();
            output.0.lock().unwrap().problem = problem.clone();
            let result = output.finish(&LoadingDiagnostics::default(), &scenario);
            assert_eq!(result.is_err(), problem.is_some());
            let text = std::fs::read_to_string(sub.join("loading-v1.tsv")).unwrap();
            assert!(text.contains("LOAD-END\tversion=1\tcomplete=false"));
            assert!(
                output
                    .finish(&LoadingDiagnostics::default(), &Ok(()))
                    .is_err()
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
