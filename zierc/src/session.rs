use std::collections::HashMap;

use codespan_reporting::{
    diagnostic::{Diagnostic, Severity},
    files::SimpleFiles,
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};

use crate::resolve;

#[derive(Debug, Clone)]
pub struct SessionOptions {
    pub emit_diagnostics: bool,
    pub emit_warnings: bool,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            emit_diagnostics: false,
            emit_warnings: true,
        }
    }
}

#[derive(Default)]
pub struct SessionCaches {
    pub symbol_table: Option<resolve::SymbolTable>,
}

pub struct CompilerSession {
    pub options: SessionOptions,
    pub caches: SessionCaches,
    pub emitted_errors: usize,
    pub emitted_warnings: usize,
    pub emitted_notes: usize,
}

pub struct CompileReport {
    pub output: Option<String>,
    pub files: SimpleFiles<String, String>,
    pub diagnostics: Vec<Diagnostic<usize>>,
}

fn default_color_choice() -> ColorChoice {
    ColorChoice::Auto
}

impl CompileReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| matches!(d.severity, Severity::Bug | Severity::Error))
    }
}

impl Default for CompilerSession {
    fn default() -> Self {
        Self::new(SessionOptions::default())
    }
}

impl CompilerSession {
    pub fn new(options: SessionOptions) -> Self {
        Self {
            options,
            caches: SessionCaches::default(),
            emitted_errors: 0,
            emitted_warnings: 0,
            emitted_notes: 0,
        }
    }

    pub fn symbol_table<'a>(
        &'a mut self,
        module_exports: &HashMap<String, Vec<String>>,
    ) -> &'a resolve::SymbolTable {
        self.caches
            .symbol_table
            .get_or_insert_with(|| resolve::SymbolTable::from_module_exports(module_exports))
    }

    pub fn emit(&mut self, files: &SimpleFiles<String, String>, diag: &Diagnostic<usize>) {
        if !self.options.emit_diagnostics {
            return;
        }
        if !self.options.emit_warnings
            && diag.severity == codespan_reporting::diagnostic::Severity::Warning
        {
            return;
        }
        match diag.severity {
            codespan_reporting::diagnostic::Severity::Bug
            | codespan_reporting::diagnostic::Severity::Error => self.emitted_errors += 1,
            codespan_reporting::diagnostic::Severity::Warning => self.emitted_warnings += 1,
            _ => self.emitted_notes += 1,
        }
        let writer = StandardStream::stderr(default_color_choice());
        let config = codespan_reporting::term::Config::default();
        term::emit_to_write_style(&mut writer.lock(), &config, files, diag).unwrap();
    }
}

pub fn emit_compile_report(report: &CompileReport, emit_warnings: bool) {
    emit_compile_report_with_color(report, emit_warnings, default_color_choice());
}

pub fn emit_compile_report_with_color(
    report: &CompileReport,
    emit_warnings: bool,
    color_choice: ColorChoice,
) {
    let writer = StandardStream::stderr(color_choice);
    let config = codespan_reporting::term::Config::default();
    for diag in &report.diagnostics {
        if !emit_warnings && diag.severity == Severity::Warning {
            continue;
        }
        term::emit_to_write_style(&mut writer.lock(), &config, &report.files, diag).unwrap();
    }
}
