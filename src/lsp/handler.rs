use std::path::{Path, PathBuf};
use std::fs;
use std::{thread, time::Duration};
use crate::solc::manager::SolcManager;
use crate::solc::versions::SolcList;

use lsp_types::{
    Diagnostic, DiagnosticSeverity, InitializeResult, PublishDiagnosticsParams, Range,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
    GotoDefinitionResponse, Location, TextDocumentPositionParams, OneOf,
};
use serde_json::{json, Value};

use crate::project::remappings::{parse_remappings, Remapping};
use crate::project::root::find_project_root;
use crate::util::fs::run_solc;
use crate::util::log::log_to_file;

use crate::analysis::definitions::DEFINITION_MAP;
use crate::util::position::{byte_offset_to_position, position_to_byte_offset};

use crate::util::text::extract_identifier_at;
use once_cell::sync::OnceCell;
use std::sync::Arc;

pub static SOLC_MANAGER: OnceCell<Arc<SolcManager>> = OnceCell::new();

pub fn handle_request(request: &str) -> Option<String> {
    let parsed: Value = serde_json::from_str(request).ok()?;
    let method = parsed.get("method")?.as_str()?;

    match method {
        "initialize" => {
            let id = parsed.get("id")?.clone();

            // Spawn background sync of latest solc versions
            thread::spawn(|| {
                let cache_dir = dirs::cache_dir()
                    .unwrap_or_else(|| PathBuf::from(".cache"))
                    .join("emacs-solidity-server/solc");
                std::fs::create_dir_all(&cache_dir)
                    .expect("Failed to create cache directory");

                let list_path = cache_dir.join("list.json");

                // Download list.json if not present
                let url = "https://binaries.soliditylang.org/linux-amd64/list.json";

                loop {
                    match crate::solc::fetch::download_to_file(url, &list_path) {
                        Ok(_) => break, // success: exit loop
                        Err(e) => {
                            log_to_file(&format!("[solc-sync] Failed to download list.json, retrying: {:?}", e));
                            thread::sleep(Duration::from_secs(5)); // retry after delay
                        }
                    }
                }

                if let Ok(list) = SolcList::from_file(&list_path) {
                    let manager = Arc::new(SolcManager::new(cache_dir.clone(), list));

                    if let Err(err) = manager.ensure_latest_versions() {
                        log_to_file(&format!("[solc-sync] Error ensuring solc versions: {:?}", err));
                    } else {
                        log_to_file("[solc-sync] Successfully ensured latest solc versions");
                    }

                    if SOLC_MANAGER.set(manager.clone()).is_err() {
                        log_to_file("[solc-sync] SOLC_MANAGER already set");
                    }
                }
            });

            let result = InitializeResult {
                capabilities: ServerCapabilities {
                    text_document_sync: Some(TextDocumentSyncCapability::Kind(
                        TextDocumentSyncKind::FULL,
                    )),
                    definition_provider: Some(OneOf::Left(true)),
                    ..Default::default()
                },
                server_info: Some(lsp_types::ServerInfo {
                    name: "emacs-solidity-server".into(),
                    version: Some("0.1.0".into()),
                }),
            };
            return Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string());
        }

        "textDocument/didOpen"
        | "textDocument/didChange"
        | "textDocument/didSave" =>
        {
            let params = parsed.get("params")?;
            let uri = params
                .get("textDocument")?
                .get("uri")?
                .as_str()?;

            let source_code = if method == "textDocument/didChange" {
                params
                    .get("contentChanges")?
                    .as_array()?
                    .get(0)?
                    .get("text")?
                    .as_str()?
            } else {
                params
                    .get("textDocument")?
                    .get("text")?
                    .as_str()?
            };

            return handle_and_publish(uri, source_code);
        }

        "textDocument/definition" => {
            return handle_definition(&parsed);
        }

        "shutdown" => {
            let id = parsed.get("id")?.clone();
            return Some(json!({ "jsonrpc": "2.0", "id": id, "result": null }).to_string());
        }
        "exit" => std::process::exit(0),

        _ => None,
    }
}

fn handle_and_publish(uri: &str, source_code: &str) -> Option<String> {
    log_to_file("Reached handle_and_publish");

    let source_path = Url::parse(uri).ok()?.to_file_path().ok()?;
    let project_root = find_project_root(&source_path)
        .unwrap_or_else(|| source_path.parent().unwrap_or(Path::new("/")).to_path_buf());

    log_to_file(&format!("Project root: {}", project_root.display()));
    let remappings: Vec<Remapping> = parse_remappings(&project_root);

    let output = run_solc(&source_path, source_code, &remappings, &project_root).ok()?;

    if let Ok(stderr) = String::from_utf8(output.stderr.clone()) {
        if !stderr.trim().is_empty() {
            log_to_file(&format!("solc stderr:\n{}", stderr));
        }
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let parsed_out: Value = serde_json::from_str(&stdout).unwrap_or_default();
    let errors = parsed_out["errors"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let diagnostics: Vec<Diagnostic> = errors
        .iter()
        .filter_map(|e| {
            let msg = e.get("message")?.as_str()?.to_owned();
            let severity = match e.get("severity")?.as_str()? {
                "error" => Some(DiagnosticSeverity::ERROR),
                "warning" => Some(DiagnosticSeverity::WARNING),
                _ => None,
            };

            let loc = e.get("sourceLocation")?;
            let start = loc.get("start")?.as_u64()? as usize;
            let end = loc.get("end")?.as_u64()? as usize;

            Some(Diagnostic {
                range: Range {
                    start: byte_offset_to_position(source_code, start),
                    end: byte_offset_to_position(source_code, end),
                },
                severity,
                message: msg,
                ..Default::default()
            })
        })
        .collect();

    let publish = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": PublishDiagnosticsParams {
            uri: Url::parse(uri).ok()?,
            diagnostics,
            version: None,
        }
    });

    Some(publish.to_string())
}

pub fn handle_definition(req: &Value) -> Option<String> {
    let params: TextDocumentPositionParams =
        serde_json::from_value(req.get("params")?.clone()).ok()?;
    let uri = params.text_document.uri.clone();
    let file_path = uri.to_file_path().ok()?;
    let pos = params.position;

    let content = fs::read_to_string(&file_path).ok()?;
    let offset = position_to_byte_offset(&content, pos)?;

    let ident = extract_identifier_at(&content, offset)?;
    log_to_file(&format!("Looking up definition for '{}'", ident));

    let map = DEFINITION_MAP.lock().ok()?;
    let matches = map
        .values()
        .flat_map(|index| index.get(&ident))
        .next();

    let result = if let Some(defs) = matches {
        let locations: Vec<Location> = defs.iter().map(|d| {
            log_to_file(&format!(
                "- [{}] {} at {:?}",
                d.kind, d.name, d.location.range
            ));
            d.location.clone()
        }).collect();

        GotoDefinitionResponse::Array(locations)
    } else {
        log_to_file(&format!("No definition found for '{}'", ident));
        GotoDefinitionResponse::Array(vec![])
    };

    Some(json!({
        "jsonrpc": "2.0",
        "id": req.get("id")?,
        "result": result,
    }).to_string())
}
