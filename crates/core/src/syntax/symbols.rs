//! 声明、导入、引用与保守调用事实（计划 D18、T05）。
//!
//! 身份规范：
//! - `lexicalPath`：容器链 + 名称，例如 `LedgerService.query`；匿名符号使用
//!   父实体 + AST 结构路径（`parent#$0`），不使用行号。
//! - `normalizedSignature`：声明的语法 token 序列（去空白与注释，保留标识符与
//!   字面量），字符串与数值不做求值。

use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Tree};

use crate::syntax::parser::LanguageKind;

/// 符号种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Interface,
    TypeAlias,
    Enum,
    EnumMember,
    Variable,
    Property,
    Module,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Method => "method",
            SymbolKind::Class => "class",
            SymbolKind::Interface => "interface",
            SymbolKind::TypeAlias => "typeAlias",
            SymbolKind::Enum => "enum",
            SymbolKind::EnumMember => "enumMember",
            SymbolKind::Variable => "variable",
            SymbolKind::Property => "property",
            SymbolKind::Module => "module",
        }
    }
}

/// 符号事实。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolFact {
    pub name: String,
    pub symbol_kind: SymbolKind,
    pub lexical_path: String,
    pub normalized_signature: String,
    pub container: Option<String>,
    /// 名称标识符的字节区间。
    pub start_byte: u64,
    pub end_byte: u64,
    pub start_line: u64,
    pub end_line: u64,
}

/// 导入事实。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFact {
    /// 原始模块文本（TS/JS 为模块说明符，Python 为模块名）。
    pub module: String,
    /// 导入的名称（可能为空）。
    pub names: Vec<String>,
    pub start_byte: u64,
    pub end_byte: u64,
    pub start_line: u64,
    pub end_line: u64,
}

/// 调用事实（保守引用）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallFact {
    /// 被调用者的简单名称。
    pub callee: String,
    /// 被调用者的原始文本（成员表达式保留对象部分）。
    pub callee_text: String,
    /// 调用点所在的词法路径（外层符号）。
    #[serde(default)]
    pub container: Option<String>,
    pub start_byte: u64,
    pub end_byte: u64,
    pub start_line: u64,
    pub end_line: u64,
}

/// 一个文件的全部语法事实。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileFacts {
    #[serde(default)]
    pub symbols: Vec<SymbolFact>,
    #[serde(default)]
    pub imports: Vec<ImportFact>,
    #[serde(default)]
    pub calls: Vec<CallFact>,
}

/// 从语法树提取事实。
pub fn extract_facts(tree: &Tree, source: &str, kind: LanguageKind) -> FileFacts {
    let mut facts = FileFacts::default();
    let mut collector = Collector {
        source,
        kind,
        facts: &mut facts,
        container: Vec::new(),
        anonymous_counter: Vec::new(),
    };
    collector.walk(tree.root_node());
    facts.symbols.sort_by_key(|left| left.start_byte);
    facts.imports.sort_by_key(|left| left.start_byte);
    facts.calls.sort_by_key(|left| left.start_byte);
    facts
}

struct Collector<'a> {
    source: &'a str,
    kind: LanguageKind,
    facts: &'a mut FileFacts,
    container: Vec<String>,
    anonymous_counter: Vec<usize>,
}

impl<'a> Collector<'a> {
    fn walk(&mut self, node: Node<'_>) {
        if let Some(symbol) = self.symbol_of(node) {
            self.facts.symbols.push(symbol);
        }
        if let Some(import) = self.import_of(node) {
            self.facts.imports.push(import);
        }
        if let Some(call) = self.call_of(node) {
            self.facts.calls.push(call);
        }

        let is_container = matches!(
            node.kind(),
            "class_declaration"
                | "class_definition"
                | "interface_declaration"
                | "enum_declaration"
                | "module"
                | "namespace_declaration"
        );
        if is_container {
            let name = node
                .child_by_field_name("name")
                .and_then(|name| self.text(name))
                .unwrap_or_else(|| self.next_anonymous_name());
            self.container.push(name);
            self.anonymous_counter.push(0);
            for child in node.children(&mut node.walk()) {
                self.walk(child);
            }
            self.container.pop();
            self.anonymous_counter.pop();
            return;
        }

        for child in node.children(&mut node.walk()) {
            self.walk(child);
        }
    }

    fn next_anonymous_name(&mut self) -> String {
        let depth = self.anonymous_counter.len();
        let index = self
            .anonymous_counter
            .get_mut(depth.saturating_sub(1))
            .copied()
            .unwrap_or(0);
        if let Some(counter) = self.anonymous_counter.get_mut(depth.saturating_sub(1)) {
            *counter += 1;
        }
        format!("#{index}")
    }

    fn symbol_of(&mut self, node: Node<'_>) -> Option<SymbolFact> {
        let symbol_kind = symbol_kind_of(node.kind())?;
        let name_node = node.child_by_field_name("name");
        let (name, lexical_path) = match name_node {
            Some(name_node) => {
                let name = self.text(name_node)?;
                let mut path = self.container.clone();
                path.push(name.clone());
                (name, path.join("."))
            }
            None => {
                // 匿名符号：父实体 + AST 结构路径，不使用行号。
                let name = self.next_anonymous_name();
                let mut path = self.container.clone();
                path.push(name.clone());
                (name, path.join("."))
            }
        };

        let name_range = name_node.unwrap_or(node);
        let normalized_signature = self.normalized_signature(node);
        let container = self.container.last().cloned();
        Some(SymbolFact {
            name,
            symbol_kind,
            lexical_path,
            normalized_signature,
            container,
            start_byte: name_range.start_byte() as u64,
            end_byte: name_range.end_byte() as u64,
            start_line: name_range.start_position().row as u64 + 1,
            end_line: name_range.end_position().row as u64 + 1,
        })
    }

    fn normalized_signature(&self, node: Node<'_>) -> String {
        let mut parts: Vec<String> = Vec::new();
        collect_tokens(node, self.source.as_bytes(), &mut parts);
        let mut signature = String::new();
        for part in parts {
            if signature.len() + part.len() > 512 {
                break;
            }
            signature.push_str(&part);
        }
        signature
    }

    fn import_of(&self, node: Node<'_>) -> Option<ImportFact> {
        match node.kind() {
            "import_statement" => {
                let source_node = node.child_by_field_name("source")?;
                let module = self.text(source_node)?;
                let module = module.trim_matches(|c| c == '"' || c == '\'').to_string();
                Some(ImportFact {
                    module,
                    names: self.imported_names(node),
                    start_byte: node.start_byte() as u64,
                    end_byte: node.end_byte() as u64,
                    start_line: node.start_position().row as u64 + 1,
                    end_line: node.end_position().row as u64 + 1,
                })
            }
            "import_from_statement" => {
                let module_node = node.child_by_field_name("module_name")?;
                let module = self.text(module_node)?;
                Some(ImportFact {
                    module,
                    names: Vec::new(),
                    start_byte: node.start_byte() as u64,
                    end_byte: node.end_byte() as u64,
                    start_line: node.start_position().row as u64 + 1,
                    end_line: node.end_position().row as u64 + 1,
                })
            }
            _ => None,
        }
    }

    fn imported_names(&self, node: Node<'_>) -> Vec<String> {
        let mut names = Vec::new();
        for child in node.children(&mut node.walk()) {
            if child.kind() == "import_clause" {
                for entry in child.children(&mut child.walk()) {
                    if entry.kind() == "import_specifier" {
                        if let Some(alias) = entry.child_by_field_name("alias")
                            && let Some(text) = self.text(alias)
                        {
                            names.push(text);
                            continue;
                        }
                        if let Some(name) = entry.child_by_field_name("name")
                            && let Some(text) = self.text(name)
                        {
                            names.push(text);
                        }
                    } else if entry.kind() == "identifier"
                        && let Some(text) = self.text(entry)
                    {
                        names.push(text);
                    }
                }
            }
        }
        names
    }

    fn call_of(&self, node: Node<'_>) -> Option<CallFact> {
        let (function_node, is_call) = match self.kind {
            LanguageKind::Python => {
                if node.kind() != "call" {
                    return None;
                }
                (node.child_by_field_name("function")?, true)
            }
            _ => {
                if node.kind() != "call_expression" {
                    return None;
                }
                (node.child_by_field_name("function")?, true)
            }
        };
        if !is_call {
            return None;
        }
        let callee_text = self.text(function_node)?;
        let callee = simple_name(&callee_text);
        if callee.is_empty() {
            return None;
        }
        Some(CallFact {
            callee,
            callee_text,
            container: self.container.last().cloned(),
            start_byte: function_node.start_byte() as u64,
            end_byte: function_node.end_byte() as u64,
            start_line: function_node.start_position().row as u64 + 1,
            end_line: function_node.end_position().row as u64 + 1,
        })
    }

    fn text(&self, node: Node<'_>) -> Option<String> {
        self.source
            .get(node.start_byte()..node.end_byte())
            .map(|value| value.to_string())
    }
}

fn symbol_kind_of(kind: &str) -> Option<SymbolKind> {
    match kind {
        "function_declaration"
        | "function_definition"
        | "function_expression"
        | "generator_function_declaration"
        | "arrow_function" => Some(SymbolKind::Function),
        "method_definition" | "method_signature" => Some(SymbolKind::Method),
        "class_declaration" | "class_definition" => Some(SymbolKind::Class),
        "interface_declaration" => Some(SymbolKind::Interface),
        "type_alias_declaration" => Some(SymbolKind::TypeAlias),
        "enum_declaration" => Some(SymbolKind::Enum),
        "enum_assignment" => Some(SymbolKind::EnumMember),
        "lexical_declaration" | "variable_declaration" => Some(SymbolKind::Variable),
        "variable_declarator" => Some(SymbolKind::Variable),
        "public_field_definition" | "field_definition" => Some(SymbolKind::Property),
        _ => None,
    }
}

/// 收集子树的语法 token（跳过注释），用于规范化签名。
fn collect_tokens(node: Node<'_>, source: &[u8], out: &mut Vec<String>) {
    let is_leaf = node.child_count() == 0;
    if is_leaf {
        let kind = node.kind();
        if kind != "comment" && kind != "line_comment" && kind != "block_comment" {
            out.push(node.utf8_text(source).unwrap_or_default().to_string());
        }
        return;
    }
    for child in node.children(&mut node.walk()) {
        collect_tokens(child, source, out);
    }
}

/// 简单名称：成员表达式取最后一段。
pub fn simple_name(text: &str) -> String {
    let trimmed = text.trim();
    let last = trimmed
        .rsplit(['.', ':', '(', ')', ' '])
        .find(|part| !part.is_empty())
        .unwrap_or("");
    let cleaned: String = last
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
        .collect();
    cleaned
}
