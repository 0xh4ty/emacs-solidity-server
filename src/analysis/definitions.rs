use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;

use lsp_types::{Location, Url};
use once_cell::sync::Lazy;
use serde_json::Value;

use crate::util::position::byte_offset_to_position;
use std::fs;

/// Structure for a single definition
#[derive(Debug, Clone)]
pub struct Definition {
    pub name: String,
    pub location: Location,
    pub kind: String, // Contract, Function, Variable, Struct, etc.
}

/// Map from identifier name → list of definitions
pub type DefinitionIndex = HashMap<String, Vec<Definition>>;

/// Global map: file URI → DefinitionIndex
pub static DEFINITION_MAP: Lazy<Mutex<HashMap<String, DefinitionIndex>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Recursively walk AST and extract definitions into the index
pub fn build_definition_index(ast: &Value, file_uri: &str) -> DefinitionIndex {
    let mut index = DefinitionIndex::new();
    visit_node(ast, file_uri, &mut index);
    index
}

/// Visit AST node recursively
fn visit_node(node: &Value, file_uri: &str, index: &mut DefinitionIndex) {
    if let Some(obj) = node.as_object() {
        if let Some(node_type) = obj.get("nodeType").and_then(|v| v.as_str()) {
            match node_type {
                "ContractDefinition"
                | "InterfaceDefinition"
                | "LibraryDefinition"
                | "FunctionDefinition"
                | "ModifierDefinition"
                | "EventDefinition"
                | "ErrorDefinition"
                | "StructDefinition"
                | "EnumDefinition"
                | "EnumValue"
                | "UserDefinedValueTypeDefinition"
                | "VariableDeclaration" => {
                    if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                        if let Some(src) = obj.get("src").and_then(|v| v.as_str()) {
                            if let Some(location) = parse_solc_src(src, file_uri) {
                                let def = Definition {
                                    name: name.to_string(),
                                    location,
                                    kind: node_type.to_string(),
                                };
                                index.entry(name.to_string()).or_default().push(def);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // recurse into all children
        for value in obj.values() {
            visit_node(value, file_uri, index);
        }
    } else if let Some(array) = node.as_array() {
        for value in array {
            visit_node(value, file_uri, index);
        }
    }
}

/// Parse solc-style `src`: "start:length:fileIndex" into LSP Location
fn parse_solc_src(src: &str, file_uri: &str) -> Option<Location> {
    let parts: Vec<&str> = src.split(':').collect();
    if parts.len() != 3 {
        return None;
    }

    let start = parts[0].parse::<usize>().ok()?;
    let length = parts[1].parse::<usize>().ok()?;
    let path = file_uri.strip_prefix("file://")?;
    let content = fs::read_to_string(path).ok()?;

    let start_pos = byte_offset_to_position(&content, start);
    let end_pos = byte_offset_to_position(&content, start + length);

    Some(Location {
        uri: file_uri.parse().ok()?,
        range: lsp_types::Range {
            start: start_pos,
            end: end_pos,
        },
    })
}

/// Extract AST from `solc` JSON output and build per-file definition indices
pub fn extract_definitions_from_solc_json(json: &Value, project_root: &Path) -> HashMap<String, DefinitionIndex> {
    let mut defs_per_file = HashMap::new();

    if let Some(sources) = json.get("sources").and_then(|v| v.as_object()) {
        for (file_name, file_data) in sources {
            if let Some(ast) = file_data.get("ast") {
                // Resolve relative to project root
                let joined = project_root.join(file_name);
                let abs_path = joined.canonicalize().unwrap_or(joined);
                let uri = Url::from_file_path(&abs_path)
                    .map(|u| u.to_string())
                    .unwrap_or_else(|_| format!("file://{}", abs_path.to_string_lossy()));

                let index = build_definition_index(ast, &uri);
                defs_per_file.insert(uri, index);
            }
        }
    }

    defs_per_file
}
