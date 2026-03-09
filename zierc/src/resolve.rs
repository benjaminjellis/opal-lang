use std::collections::{HashMap, HashSet};

use crate::{ast::Declaration, typecheck::TypeEnv};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleId(String);

impl ModuleId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FunctionId(String);

impl FunctionId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolId {
    pub module: Option<ModuleId>,
    pub function: FunctionId,
}

impl SymbolId {
    pub fn qualified(module: ModuleId, function: FunctionId) -> Self {
        Self {
            module: Some(module),
            function,
        }
    }

    pub fn to_env_key(&self) -> String {
        match &self.module {
            Some(m) => format!("{}/{}", m.as_str(), self.function.as_str()),
            None => self.function.as_str().to_string(),
        }
    }
}

pub struct SymbolTable {
    exports: HashMap<ModuleId, Vec<FunctionId>>,
}

impl SymbolTable {
    pub fn from_module_exports(module_exports: &HashMap<String, Vec<String>>) -> Self {
        let exports = module_exports
            .iter()
            .map(|(m, fns)| {
                (
                    ModuleId::new(m.clone()),
                    fns.iter().cloned().map(FunctionId::new).collect(),
                )
            })
            .collect();
        Self { exports }
    }

    pub fn used_modules(decls: &[Declaration]) -> HashSet<ModuleId> {
        decls
            .iter()
            .filter_map(|d| {
                if let Declaration::Use {
                    path: (_, module), ..
                } = d
                {
                    Some(ModuleId::new(module.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn qualified_symbols_for_used_modules(&self, decls: &[Declaration]) -> Vec<SymbolId> {
        let used = Self::used_modules(decls);
        let mut out: Vec<SymbolId> = self
            .exports
            .iter()
            .filter(|(m, _)| used.contains(*m))
            .flat_map(|(m, fns)| {
                fns.iter()
                    .cloned()
                    .map(move |f| SymbolId::qualified(m.clone(), f))
            })
            .collect();
        out.sort_by_key(|s| s.to_env_key());
        out
    }
}

pub fn unresolved_env_names(
    decls: &[Declaration],
    import_names: impl IntoIterator<Item = String>,
    env: &TypeEnv,
    symbols: &SymbolTable,
) -> Vec<String> {
    let mut names: Vec<String> = import_names.into_iter().collect();
    names.extend(
        symbols
            .qualified_symbols_for_used_modules(decls)
            .into_iter()
            .map(|s| s.to_env_key()),
    );
    names.into_iter().filter(|n| !env.contains_key(n)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lower, sexpr, typecheck};

    fn parse_decls(src: &str) -> Vec<Declaration> {
        let mut lowerer = lower::Lowerer::new();
        let tokens = crate::lexer::Lexer::new(src).lex();
        let file_id = lowerer.add_file("scan.zier".into(), src.into());
        let sexprs = sexpr::SExprParser::new(tokens, file_id)
            .parse()
            .expect("parse");
        lowerer.lower_file(file_id, &sexprs)
    }

    #[test]
    fn used_modules_deduplicates_repeated_use() {
        let decls = parse_decls("(use std/io)\n(use std/io)\n(let main {} ())");
        let used = SymbolTable::used_modules(&decls);
        assert_eq!(used.len(), 1);
        assert!(used.contains(&ModuleId::new("io")));
    }

    #[test]
    fn qualified_symbols_only_for_used_modules() {
        let mut module_exports = HashMap::new();
        module_exports.insert("io".to_string(), vec!["println".to_string()]);
        module_exports.insert("math".to_string(), vec!["inc".to_string()]);
        let table = SymbolTable::from_module_exports(&module_exports);
        let decls = parse_decls("(use std/io)\n(let main {} ())");
        let names: Vec<String> = table
            .qualified_symbols_for_used_modules(&decls)
            .into_iter()
            .map(|s| s.to_env_key())
            .collect();
        assert_eq!(names, vec!["io/println".to_string()]);
    }

    #[test]
    fn unresolved_env_names_excludes_seeded_entries() {
        let mut module_exports = HashMap::new();
        module_exports.insert("io".to_string(), vec!["println".to_string()]);
        let table = SymbolTable::from_module_exports(&module_exports);
        let decls = parse_decls("(use std/io)\n(let main {} ())");
        let mut env = typecheck::primitive_env();
        env.extend(typecheck::import_env(&["io/println".to_string()]));
        let unresolved = unresolved_env_names(&decls, ["println".to_string()], &env, &table);
        assert_eq!(unresolved, vec!["println".to_string()]);
    }
}
