use std::collections::{HashMap, HashSet};

pub(crate) struct ResolveContext<'a> {
    pub all_module_schemes: &'a HashMap<String, zierc::typecheck::TypeEnv>,
    pub module_type_decls: &'a HashMap<String, Vec<zierc::ast::TypeDecl>>,
    pub std_aliases: &'a HashMap<String, String>,
}

pub(crate) struct ResolvedImports {
    pub imports: HashMap<String, String>,
    pub imported_schemes: zierc::typecheck::TypeEnv,
    pub imported_type_decls: Vec<zierc::ast::TypeDecl>,
    pub module_aliases: HashMap<String, String>,
}

impl<'a> ResolveContext<'a> {
    pub(crate) fn resolve_for_source(
        &self,
        source: &str,
        visible_exports: &HashMap<String, Vec<String>>,
    ) -> ResolvedImports {
        let mut imports: HashMap<String, String> = HashMap::new();
        let mut imported_schemes: zierc::typecheck::TypeEnv = HashMap::new();

        for (_, mod_name, unqualified) in zierc::used_modules(source) {
            let erlang_name = self
                .std_aliases
                .get(&mod_name)
                .cloned()
                .unwrap_or_else(|| mod_name.clone());

            if let Some(exports) = visible_exports.get(&mod_name) {
                for fn_name in exports {
                    if unqualified.includes(fn_name) {
                        imports.insert(fn_name.clone(), erlang_name.clone());
                    }
                }
            }

            if let Some(mod_schemes) = self.all_module_schemes.get(&mod_name) {
                for (fn_name, scheme) in mod_schemes {
                    if unqualified.includes(fn_name) {
                        imported_schemes.insert(fn_name.clone(), scheme.clone());
                    }
                    imported_schemes.insert(format!("{mod_name}/{fn_name}"), scheme.clone());
                }
            }
        }

        let imported_type_decls: Vec<zierc::ast::TypeDecl> = referenced_modules(source)
            .iter()
            .flat_map(|mod_name| {
                self.module_type_decls
                    .get(mod_name)
                    .cloned()
                    .unwrap_or_default()
            })
            .collect();

        ResolvedImports {
            imports,
            imported_schemes,
            imported_type_decls,
            module_aliases: self.std_aliases.clone(),
        }
    }
}

pub(crate) fn referenced_modules(source: &str) -> HashSet<String> {
    let mut referenced: HashSet<String> = zierc::used_modules(source)
        .into_iter()
        .map(|(_, mod_name, _)| mod_name)
        .collect();
    for tok in zierc::lexer::Lexer::new(source).lex() {
        if let zierc::lexer::TokenKind::QualifiedIdent((module, _)) = tok.kind {
            referenced.insert(module);
        }
    }
    referenced
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolver_qualified_only_use_adds_no_unqualified_imports() {
        let mut exports = HashMap::new();
        exports.insert("io".to_string(), vec!["println".to_string()]);

        let ctx = ResolveContext {
            all_module_schemes: &HashMap::new(),
            module_type_decls: &HashMap::new(),
            std_aliases: &HashMap::new(),
        };
        let resolved = ctx.resolve_for_source("(use std/io)\n(let main {} ())", &exports);
        assert!(resolved.imports.is_empty());
    }

    #[test]
    fn resolver_wildcard_imports_all_exports() {
        let mut exports = HashMap::new();
        exports.insert(
            "math".to_string(),
            vec!["inc".to_string(), "dec".to_string()],
        );

        let ctx = ResolveContext {
            all_module_schemes: &HashMap::new(),
            module_type_decls: &HashMap::new(),
            std_aliases: &HashMap::new(),
        };
        let resolved = ctx.resolve_for_source("(use math [*])\n(let main {} (inc 1))", &exports);
        assert_eq!(resolved.imports.get("inc"), Some(&"math".to_string()));
        assert_eq!(resolved.imports.get("dec"), Some(&"math".to_string()));
    }

    #[test]
    fn referenced_modules_include_qualified_calls_without_use() {
        let refs = referenced_modules("(let main {} (io/println \"x\"))");
        assert!(refs.contains("io"));
    }
}
