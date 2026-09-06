use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::thread_local;

use raptor_lib::common::errors::{ErrorSeverity, IError};
use raptor_lib::common::span::Span;
use raptor_lib::frontend::lexer::lazy_stream_reader::LazyStreamReader;
use raptor_lib::frontend::lexer::lexer::{Lexer, LexerOptions};
use raptor_lib::frontend::parser::{IParser, Parser};
use raptor_lib::frontend::tokens::{TokenCategory, TokenValue};
use raptor_lib::import_resolver::ImportResolver;
use raptor_lib::semantic::semantic_checker::checker::{HoverInfo, SemanticChecker};

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

thread_local! {
    static LEXER_WARNINGS: RefCell<Vec<Box<dyn IError>>> = RefCell::new(Vec::new());
    static FILENAME_CACHE: RefCell<HashMap<Url, &'static str>> = RefCell::new(HashMap::new());
    static URI_CACHE: RefCell<HashMap<&'static str, Url>> = RefCell::new(HashMap::new());
}

fn on_warning(warning: Box<dyn IError>) {
    LEXER_WARNINGS.with(|w| w.borrow_mut().push(warning));
}

/// Resolves (and caches) the canonical, leaked `&'static str` filename for a given `Url`,
/// also recording the reverse mapping so diagnostics for that file (even when it's an
/// imported file that was never opened directly) can be routed back to a `Url`.
fn filename_for_uri(uri: &Url) -> &'static str {
    FILENAME_CACHE.with(|cache| {
        if let Some(name) = cache.borrow().get(uri) {
            return *name;
        }

        let raw_path = uri
            .to_file_path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| uri.path().to_string());

        let path_string = fix_wsl_drive_prefix(raw_path);

        let leaked: &'static str = Box::leak(path_string.into_boxed_str());
        cache.borrow_mut().insert(uri.clone(), leaked);
        URI_CACHE.with(|u| u.borrow_mut().insert(leaked, uri.clone()));
        leaked
    })
}

/// Finds a `Url` for a filename that a span points to. This covers files that were
/// never opened as an LSP document (e.g. transitively imported files) by falling back
/// to building a `file://` URL directly from the path.
fn uri_for_filename(filename: &str) -> Option<Url> {
    URI_CACHE.with(|cache| {
        if let Some(uri) = cache.borrow().get(filename) {
            return Some(uri.clone());
        }
        Url::from_file_path(filename).ok()
    })
}

#[cfg(not(windows))]
fn fix_wsl_drive_prefix(path: String) -> String {
    let bytes = path.as_bytes();
    let looks_like_drive_letter =
        bytes.len() >= 4 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b':' && (bytes[3] == b'/' || bytes[3] == b'\\');

    if looks_like_drive_letter {
        let drive = (bytes[1] as char).to_ascii_lowercase();
        format!("/mnt/{}{}", drive, &path[3..])
    } else {
        path
    }
}

#[cfg(windows)]
fn fix_wsl_drive_prefix(path: String) -> String {
    path
}

struct DocumentState {
    text: String,
    hovers: Vec<HoverInfo>,
}

struct Backend {
    client: Client,
    documents: Mutex<HashMap<Url, DocumentState>>,
    /// For each "entry" document we've validated, remembers which other files (by
    /// filename) had diagnostics published last time, so that on the next validation
    /// we can explicitly clear diagnostics for files that no longer have any.
    published_files: Mutex<HashMap<Url, HashSet<&'static str>>>,
}

fn std_function_completions() -> Vec<CompletionItem> {
    let functions = [
        "print",
        "println",
        "input",
        "read_file",
        "write_file",
        "append_file",
        "delete_file",
        "exists_file",
        "tcp_accept",
        "tcp_close",
        "tcp_listen",
        "tcp_read",
        "tcp_write",
        "str_len",
        "sleep_ms",
        "vector_push",
        "vector_size",
        "vector_stringify",
    ];

    functions
        .iter()
        .map(|name| CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            ..Default::default()
        })
        .collect()
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client.log_message(MessageType::INFO, "raptor-lsp initialized").await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.validate(params.text_document.uri, params.text_document.text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.validate(params.text_document.uri, change.text).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            self.validate(params.text_document.uri, text).await;
        }
    }

    async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;

        let source = self.documents.lock().await.get(&uri).map(|doc| doc.text.clone()).unwrap_or_default();

        let filename = filename_for_uri(&uri);

        let mut items = keyword_completions();

        items.extend(std_function_completions());
        items.extend(identifier_completions(&source, filename));

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let documents = self.documents.lock().await;

        let Some(doc) = documents.get(&uri) else {
            return Ok(None);
        };

        let best = doc
            .hovers
            .iter()
            .filter(|h| span_contains_position(&h.span, position))
            .min_by_key(|h| span_len(&h.span));

        Ok(best.map(|h| Hover {
            contents: HoverContents::Scalar(MarkedString::String(h.contents.clone())),
            range: Some(span_to_range(&h.span)),
        }))
    }
}

impl Backend {
    async fn validate(&self, uri: Url, text: String) {
        let filename = filename_for_uri(&uri);
        let (diagnostics_by_file, hovers) = analyze(&text, filename);

        self.documents.lock().await.insert(uri.clone(), DocumentState { text, hovers });

        let mut current_files: HashSet<&'static str> = HashSet::new();

        // Always publish for the entry document itself, even if it has no
        // diagnostics right now (this clears any previously shown errors on it).
        let own_diagnostics = diagnostics_by_file.get(filename).cloned().unwrap_or_default();
        self.client.publish_diagnostics(uri.clone(), own_diagnostics, None).await;
        current_files.insert(filename);

        // Diagnostics that landed on other files (e.g. via imports) get published
        // under their own Url.
        for (other_filename, diags) in diagnostics_by_file.iter() {
            if *other_filename == filename {
                continue;
            }

            if let Some(other_uri) = uri_for_filename(other_filename) {
                self.client.publish_diagnostics(other_uri, diags.clone(), None).await;
                current_files.insert(other_filename);
            }
        }

        // Clear diagnostics for files that had them last time around but don't
        // anymore (e.g. a cyclic import that got fixed).
        let mut published_files = self.published_files.lock().await;
        if let Some(previous_files) = published_files.get(&uri) {
            for stale_filename in previous_files.difference(&current_files) {
                if let Some(stale_uri) = uri_for_filename(stale_filename) {
                    self.client.publish_diagnostics(stale_uri, vec![], None).await;
                }
            }
        }
        published_files.insert(uri, current_files);
    }
}

fn analyze(source: &str, filename: &'static str) -> (HashMap<&'static str, Vec<Diagnostic>>, Vec<HoverInfo>) {
    let mut diagnostics: HashMap<&'static str, Vec<Diagnostic>> = HashMap::new();

    LEXER_WARNINGS.with(|w| w.borrow_mut().clear());

    let cursor = Cursor::new(source.as_bytes().to_vec());
    let reader = LazyStreamReader::new(cursor, Some(filename));

    let lexer_options = LexerOptions {
        max_comment_length: 500,
        max_identifier_length: 100,
    };

    let lexer = match Lexer::new(reader, lexer_options.clone(), on_warning) {
        Ok(lexer) => lexer,
        Err(err) => {
            push_error(&mut diagnostics, err.as_ref(), DiagnosticSeverity::ERROR, filename);
            return (diagnostics, vec![]);
        }
    };

    LEXER_WARNINGS.with(|w| {
        for warning in w.borrow().iter() {
            push_error(&mut diagnostics, warning.as_ref(), DiagnosticSeverity::WARNING, filename);
        }
    });

    let mut parser = Parser::new(lexer);

    let program = match parser.parse() {
        Ok(program) => program,
        Err(err) => {
            push_error(&mut diagnostics, err.as_ref(), DiagnosticSeverity::ERROR, filename);
            return (diagnostics, vec![]);
        }
    };

    let mut import_resolver = ImportResolver::new(lexer_options, on_warning);
    let import_resolved_program = match import_resolver.resolve(filename, program) {
        Ok(program) => program,
        Err(err) => {
            push_error(&mut diagnostics, err.as_ref(), DiagnosticSeverity::ERROR, filename);
            return (diagnostics, vec![]);
        }
    };

    let mut semantic_checker = match SemanticChecker::new(&import_resolved_program) {
        Ok(checker) => checker,
        Err(err) => {
            push_error(&mut diagnostics, err.as_ref(), DiagnosticSeverity::ERROR, filename);
            return (diagnostics, vec![]);
        }
    };

    semantic_checker.check();

    for error in &semantic_checker.errors {
        let severity = match error.get_severity() {
            ErrorSeverity::HIGH => DiagnosticSeverity::ERROR,
            ErrorSeverity::LOW => DiagnosticSeverity::WARNING,
        };
        push_error(&mut diagnostics, error.as_ref(), severity, filename);
    }

    (diagnostics, semantic_checker.hovers)
}

/// Adds a diagnostic to the bucket matching the file its span actually points to,
/// falling back to `default_filename` if the span carries no filename of its own.
fn push_error(
    diagnostics: &mut HashMap<&'static str, Vec<Diagnostic>>,
    err: &dyn IError,
    severity: DiagnosticSeverity,
    default_filename: &'static str,
) {
    let span_filename = err.get_span().start().filename.unwrap_or(default_filename);
    diagnostics.entry(span_filename).or_default().push(error_to_diagnostic(err, severity));
}

fn keyword_completions() -> Vec<CompletionItem> {
    let keywords = [
        "for", "while", "if", "else", "as", "fn", "true", "false", "return", "switch", "break", "continue", "import", "extern",
    ];
    let types = ["bool", "str", "i64", "f64", "void"];

    let mut items: Vec<CompletionItem> = keywords
        .iter()
        .map(|kw| CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        })
        .collect();

    items.extend(types.iter().map(|ty| CompletionItem {
        label: ty.to_string(),
        kind: Some(CompletionItemKind::TYPE_PARAMETER),
        ..Default::default()
    }));

    items
}

fn identifier_completions(source: &str, filename: &'static str) -> Vec<CompletionItem> {
    let cursor = Cursor::new(source.as_bytes().to_vec());
    let reader = LazyStreamReader::new(cursor, Some(filename));
    let lexer_options = LexerOptions {
        max_comment_length: 500,
        max_identifier_length: 100,
    };

    let mut names = std::collections::HashSet::new();

    if let Ok(mut lexer) = Lexer::new(reader, lexer_options, |_| {}) {
        while let Ok(token) = lexer.generate_token() {
            if token.category == TokenCategory::Identifier {
                if let TokenValue::String(name) = &token.value {
                    names.insert(name.clone());
                }
            }
            if token.category == TokenCategory::ETX {
                break;
            }
        }
    }

    names
        .into_iter()
        .map(|name| CompletionItem {
            label: name,
            kind: Some(CompletionItemKind::VARIABLE),
            ..Default::default()
        })
        .collect()
}

fn error_to_diagnostic(err: &dyn IError, severity: DiagnosticSeverity) -> Diagnostic {
    let span = err.get_span();
    let range = span_to_range(&span);

    Diagnostic {
        range,
        severity: Some(severity),
        source: Some("raptor".to_string()),
        message: err.message(),
        ..Default::default()
    }
}

fn span_to_range(span: &Span) -> Range {
    Range::new(
        Position::new(span.start().line - 1, span.start().column - 1),
        Position::new(span.end().line - 1, span.end().column - 1),
    )
}

fn span_contains_position(span: &Span, pos: Position) -> bool {
    let range = span_to_range(span);
    (pos.line > range.start.line || (pos.line == range.start.line && pos.character >= range.start.character))
        && (pos.line < range.end.line || (pos.line == range.end.line && pos.character <= range.end.character))
}

fn span_len(span: &Span) -> (u32, u32) {
    let start = (span.start().line, span.start().column);
    let end = (span.end().line, span.end().column);
    (end.0.saturating_sub(start.0), end.1.saturating_sub(start.1))
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: Mutex::new(HashMap::new()),
        published_files: Mutex::new(HashMap::new()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
