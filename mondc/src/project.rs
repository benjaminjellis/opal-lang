use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct ProjectAnalysis {
    pub module_exports: HashMap<String, Vec<String>>,
    pub module_type_decls: HashMap<String, Vec<crate::ast::TypeDecl>>,
    pub all_module_schemes: HashMap<String, crate::typecheck::TypeEnv>,
    pub module_aliases: HashMap<String, String>,
}

#[derive(Clone, Debug, Default)]
pub struct ResolvedImports {
    pub imports: HashMap<String, String>,
    pub import_origins: HashMap<String, String>,
    pub imported_schemes: crate::typecheck::TypeEnv,
    pub imported_type_decls: Vec<crate::ast::TypeDecl>,
    pub imported_field_indices: HashMap<String, usize>,
    pub module_aliases: HashMap<String, String>,
}

pub fn build_project_analysis(
    std_mods: &[(String, String, String)],
    src_module_sources: &[(String, String)],
) -> Result<ProjectAnalysis, String> {
    let mut module_exports = HashMap::new();
    let mut module_type_decls = HashMap::new();

    for (user_name, _, source) in std_mods {
        module_exports.insert(user_name.clone(), crate::exported_names(source));
        module_type_decls.insert(user_name.clone(), crate::exported_type_decls(source));
    }
    for (module_name, source) in src_module_sources {
        module_exports.insert(module_name.clone(), crate::exported_names(source));
        module_type_decls.insert(module_name.clone(), crate::exported_type_decls(source));
    }

    let module_aliases: HashMap<String, String> = std_mods
        .iter()
        .map(|(user_name, erlang_name, _)| (user_name.clone(), erlang_name.clone()))
        .collect();

    let mut all_module_schemes: HashMap<String, crate::typecheck::TypeEnv> = HashMap::new();
    for (user_name, _, source) in std_mods {
        let imports = resolve_imports_for_source(
            source,
            &module_exports,
            &ProjectAnalysis {
                module_exports: module_exports.clone(),
                module_type_decls: module_type_decls.clone(),
                all_module_schemes: all_module_schemes.clone(),
                module_aliases: module_aliases.clone(),
            },
        );
        let schemes = crate::infer_module_exports(
            user_name,
            source,
            imports.imports,
            &module_exports,
            &imports.imported_type_decls,
            &imports.imported_schemes,
        );
        all_module_schemes.insert(user_name.clone(), schemes);
    }

    let ordered_module_sources = ordered_module_sources(src_module_sources)?;
    for (module_name, source) in &ordered_module_sources {
        let imports = resolve_imports_for_source(
            source,
            &module_exports,
            &ProjectAnalysis {
                module_exports: module_exports.clone(),
                module_type_decls: module_type_decls.clone(),
                all_module_schemes: all_module_schemes.clone(),
                module_aliases: module_aliases.clone(),
            },
        );
        let schemes = crate::infer_module_exports(
            module_name,
            source,
            imports.imports,
            &module_exports,
            &imports.imported_type_decls,
            &imports.imported_schemes,
        );
        all_module_schemes.insert(module_name.clone(), schemes);
    }

    Ok(ProjectAnalysis {
        module_exports,
        module_type_decls,
        all_module_schemes,
        module_aliases,
    })
}

pub fn alias_package_root_module(
    analysis: &mut ProjectAnalysis,
    package_name: &str,
) -> Result<(), String> {
    const LIB_MODULE_NAME: &str = "lib";

    if package_name == LIB_MODULE_NAME {
        return Ok(());
    }

    let Some(lib_exports) = analysis.module_exports.get(LIB_MODULE_NAME).cloned() else {
        return Ok(());
    };

    if analysis.module_exports.contains_key(package_name)
        || analysis.module_type_decls.contains_key(package_name)
        || analysis.all_module_schemes.contains_key(package_name)
    {
        return Err(format!(
            "module name collision: package `{package_name}` conflicts with an existing module name; cannot alias `src/lib.mond` as `{package_name}`"
        ));
    }

    analysis
        .module_exports
        .insert(package_name.to_string(), lib_exports);
    analysis.module_type_decls.insert(
        package_name.to_string(),
        analysis
            .module_type_decls
            .get(LIB_MODULE_NAME)
            .cloned()
            .unwrap_or_default(),
    );
    analysis.all_module_schemes.insert(
        package_name.to_string(),
        analysis
            .all_module_schemes
            .get(LIB_MODULE_NAME)
            .cloned()
            .unwrap_or_default(),
    );
    analysis
        .module_aliases
        .insert(package_name.to_string(), LIB_MODULE_NAME.to_string());

    Ok(())
}

pub fn resolve_imports_for_source(
    source: &str,
    visible_exports: &HashMap<String, Vec<String>>,
    project: &ProjectAnalysis,
) -> ResolvedImports {
    let mut imports = HashMap::new();
    let mut import_origins = HashMap::new();
    let mut imported_schemes = HashMap::new();
    let mut imported_type_decls = Vec::new();
    let mut imported_field_indices: HashMap<String, usize> = HashMap::new();
    let mut imported_type_keys: HashSet<(String, String)> = HashSet::new();

    for (_, mod_name, unqualified) in crate::used_modules(source) {
        let erlang_name = project
            .module_aliases
            .get(&mod_name)
            .cloned()
            .unwrap_or_else(|| mod_name.clone());

        if let Some(exports) = visible_exports.get(&mod_name) {
            for fn_name in exports {
                if unqualified.includes(fn_name) {
                    imports.insert(fn_name.clone(), erlang_name.clone());
                    import_origins.insert(fn_name.clone(), mod_name.clone());
                }
            }
        }

        if let Some(mod_schemes) = project.all_module_schemes.get(&mod_name) {
            for (fn_name, scheme) in mod_schemes {
                if unqualified.includes(fn_name) {
                    imported_schemes.insert(fn_name.clone(), scheme.clone());
                }
                imported_schemes.insert(format!("{mod_name}/{fn_name}"), scheme.clone());
            }
        }

        if let Some(type_decls) = project.module_type_decls.get(&mod_name) {
            // Field accessors (for example `:value`) should remain usable when a module is
            // referenced, even if constructors require explicit unqualified type import.
            for type_decl in type_decls {
                if matches!(type_decl, crate::ast::TypeDecl::Record { .. }) {
                    let accessor_schemes = crate::typecheck::constructor_schemes(type_decl);
                    for (name, scheme) in accessor_schemes {
                        if name.starts_with(':') {
                            imported_schemes.insert(name, scheme);
                        }
                    }
                    if let crate::ast::TypeDecl::Record { fields, .. } = type_decl {
                        for (i, (field_name, _)) in fields.iter().enumerate() {
                            imported_field_indices.insert(field_name.clone(), i + 2);
                        }
                    }
                }
            }

            match &unqualified {
                crate::ast::UnqualifiedImports::None => {}
                crate::ast::UnqualifiedImports::Wildcard => {
                    for type_decl in type_decls {
                        let type_name = match type_decl {
                            crate::ast::TypeDecl::Record { name, .. } => name.clone(),
                            crate::ast::TypeDecl::Variant { name, .. } => name.clone(),
                        };
                        let key = (mod_name.clone(), type_name);
                        if imported_type_keys.insert(key) {
                            imported_type_decls.push(type_decl.clone());
                        }
                    }
                }
                crate::ast::UnqualifiedImports::Specific(names) => {
                    for type_decl in type_decls {
                        let type_name = match type_decl {
                            crate::ast::TypeDecl::Record { name, .. } => name,
                            crate::ast::TypeDecl::Variant { name, .. } => name,
                        };
                        if !names.iter().any(|n| n == type_name) {
                            continue;
                        }
                        let key = (mod_name.clone(), type_name.clone());
                        if imported_type_keys.insert(key) {
                            imported_type_decls.push(type_decl.clone());
                        }
                    }
                }
            }
        }
    }

    ResolvedImports {
        imports,
        import_origins,
        imported_schemes,
        imported_type_decls,
        imported_field_indices,
        module_aliases: project.module_aliases.clone(),
    }
}

pub fn referenced_modules(source: &str) -> HashSet<String> {
    let mut referenced: HashSet<String> = crate::used_modules(source)
        .into_iter()
        .map(|(_, mod_name, _)| mod_name)
        .collect();
    for tok in crate::lexer::Lexer::new(source).lex() {
        if let crate::lexer::TokenKind::QualifiedIdent((module, _)) = tok.kind {
            referenced.insert(module);
        }
    }
    referenced
}

pub fn ordered_module_sources(
    module_sources: &[(String, String)],
) -> Result<Vec<(String, String)>, String> {
    let source_by_name: BTreeMap<String, String> = module_sources
        .iter()
        .map(|(name, src)| (name.clone(), src.clone()))
        .collect();
    if source_by_name.len() != module_sources.len() {
        return Err(
            "duplicate module names found in src/: module file stems must be unique".into(),
        );
    }

    let graph = local_module_graph(&source_by_name);

    let order = topo_sort_modules(&graph)?;
    Ok(order
        .into_iter()
        .filter_map(|name| source_by_name.get(&name).cloned().map(|src| (name, src)))
        .collect())
}

pub fn reachable_module_sources(
    module_sources: &[(String, String)],
    roots: &[String],
) -> Result<Vec<(String, String)>, String> {
    let source_by_name: BTreeMap<String, String> = module_sources
        .iter()
        .map(|(name, src)| (name.clone(), src.clone()))
        .collect();
    if source_by_name.len() != module_sources.len() {
        return Err(
            "duplicate module names found in src/: module file stems must be unique".into(),
        );
    }

    let graph = local_module_graph(&source_by_name);
    let order = topo_sort_modules(&graph)?;
    let reachable = reachable_modules(&graph, roots)?;

    Ok(order
        .into_iter()
        .filter(|name| reachable.contains(name))
        .filter_map(|name| source_by_name.get(&name).cloned().map(|src| (name, src)))
        .collect())
}

pub fn std_modules_from_sources(
    module_sources: &[(String, String)],
) -> Result<Vec<(String, String, String)>, String> {
    let ordered = ordered_module_sources(module_sources)?;
    Ok(ordered
        .into_iter()
        .map(|(user_name, source)| {
            let erlang_name = format!("mond_{user_name}");
            (user_name, erlang_name, source)
        })
        .collect())
}

fn topo_sort_modules(graph: &BTreeMap<String, Vec<String>>) -> Result<Vec<String>, String> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Mark {
        Visiting,
        Done,
    }

    fn dfs(
        node: &str,
        graph: &BTreeMap<String, Vec<String>>,
        marks: &mut HashMap<String, Mark>,
        stack: &mut Vec<String>,
        out: &mut Vec<String>,
    ) -> Result<(), String> {
        match marks.get(node).copied() {
            Some(Mark::Done) => return Ok(()),
            Some(Mark::Visiting) => {
                let start = stack.iter().position(|n| n == node).unwrap_or(0);
                let mut cycle: Vec<String> = stack[start..].to_vec();
                cycle.push(node.to_string());
                return Err(format!(
                    "cyclic module dependency detected: {}",
                    cycle.join(" -> ")
                ));
            }
            None => {}
        }
        marks.insert(node.to_string(), Mark::Visiting);
        stack.push(node.to_string());
        for dep in graph.get(node).cloned().unwrap_or_default() {
            dfs(&dep, graph, marks, stack, out)?;
        }
        stack.pop();
        marks.insert(node.to_string(), Mark::Done);
        out.push(node.to_string());
        Ok(())
    }

    let mut marks = HashMap::new();
    let mut out = Vec::new();
    let mut stack = Vec::new();
    for node in graph.keys() {
        dfs(node, graph, &mut marks, &mut stack, &mut out)?;
    }
    Ok(out)
}

fn local_module_graph(source_by_name: &BTreeMap<String, String>) -> BTreeMap<String, Vec<String>> {
    let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (module_name, source) in source_by_name {
        let mut deps: BTreeSet<String> = BTreeSet::new();
        for (namespace, dep, _) in crate::used_modules(source) {
            if namespace.is_empty() && source_by_name.contains_key(&dep) {
                deps.insert(dep);
            }
        }
        graph.insert(module_name.clone(), deps.into_iter().collect());
    }
    graph
}

fn reachable_modules(
    graph: &BTreeMap<String, Vec<String>>,
    roots: &[String],
) -> Result<BTreeSet<String>, String> {
    let mut reachable = BTreeSet::new();
    let mut stack: Vec<String> = Vec::new();

    for root in roots {
        if !graph.contains_key(root) {
            return Err(format!(
                "module `{root}` was selected as a build root but does not exist in src/"
            ));
        }
        stack.push(root.clone());
    }

    while let Some(module_name) = stack.pop() {
        if !reachable.insert(module_name.clone()) {
            continue;
        }
        for dep in graph.get(&module_name).into_iter().flatten() {
            stack.push(dep.clone());
        }
    }

    Ok(reachable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_module_sources_respects_dependencies() {
        let modules = vec![
            (
                "main".to_string(),
                "(use util)\n(let main {} (util_fn))".to_string(),
            ),
            ("util".to_string(), "(let util_fn {} 1)".to_string()),
            ("other".to_string(), "(let other {} 2)".to_string()),
        ];
        let ordered = ordered_module_sources(&modules).expect("topo order");
        let names: Vec<String> = ordered.into_iter().map(|(n, _)| n).collect();
        let pos_main = names.iter().position(|n| n == "main").expect("main");
        let pos_util = names.iter().position(|n| n == "util").expect("util");
        assert!(pos_util < pos_main, "dependency must come first: {names:?}");
    }

    #[test]
    fn ordered_module_sources_rejects_cycles() {
        let modules = vec![
            ("a".to_string(), "(use b)\n(let a {} 1)".to_string()),
            ("b".to_string(), "(use a)\n(let b {} 2)".to_string()),
        ];
        let err = ordered_module_sources(&modules).expect_err("expected cycle error");
        assert!(err.contains("cyclic module dependency detected"));
        assert!(err.contains("a -> b -> a") || err.contains("b -> a -> b"));
    }

    #[test]
    fn reachable_module_sources_only_keeps_transitive_dependencies_of_roots() {
        let modules = vec![
            (
                "main".to_string(),
                "(use util)\n(let main {} (util_fn))".to_string(),
            ),
            (
                "util".to_string(),
                "(use helper)\n(let util_fn {} (helper_fn))".to_string(),
            ),
            ("helper".to_string(), "(let helper_fn {} 1)".to_string()),
            ("unused".to_string(), "(let ignore_me {} 2)".to_string()),
        ];

        let ordered = reachable_module_sources(&modules, &["main".to_string()]).expect("roots");
        let names: Vec<String> = ordered.into_iter().map(|(n, _)| n).collect();

        assert_eq!(names, vec!["helper", "util", "main"]);
    }

    #[test]
    fn std_modules_from_sources_discovers_files_without_root_reexports() {
        let modules = vec![
            ("io".to_string(), "(let println {x} x)".to_string()),
            ("extra".to_string(), "(let helper {} 1)".to_string()),
            ("std".to_string(), "(let hello {} 1)".to_string()),
        ];
        let discovered = std_modules_from_sources(&modules).expect("std modules");
        let names: Vec<String> = discovered.into_iter().map(|(name, _, _)| name).collect();
        assert!(names.contains(&"io".to_string()));
        assert!(names.contains(&"extra".to_string()));
        assert!(names.contains(&"std".to_string()));
    }

    #[test]
    fn resolve_imports_supports_root_and_submodule_imports() {
        let mut exports = HashMap::new();
        exports.insert("std".to_string(), vec!["hello".to_string()]);
        exports.insert("io".to_string(), vec!["println".to_string()]);

        let mut module_aliases = HashMap::new();
        module_aliases.insert("std".to_string(), "mond_std".to_string());
        module_aliases.insert("io".to_string(), "mond_io".to_string());

        let resolved = resolve_imports_for_source(
            "(use std [hello])\n(use std/io)\n(let main {} (hello))",
            &exports,
            &ProjectAnalysis {
                module_exports: exports.clone(),
                module_type_decls: HashMap::new(),
                all_module_schemes: HashMap::new(),
                module_aliases,
            },
        );

        assert_eq!(resolved.imports.get("hello"), Some(&"mond_std".to_string()));
        assert!(!resolved.imports.contains_key("println"));
    }

    #[test]
    fn alias_package_root_module_maps_package_name_to_lib() {
        let mut analysis = ProjectAnalysis {
            module_exports: HashMap::from([
                ("lib".to_string(), vec!["now".to_string()]),
                ("util".to_string(), vec!["helper".to_string()]),
            ]),
            module_type_decls: HashMap::new(),
            all_module_schemes: HashMap::new(),
            module_aliases: HashMap::new(),
        };

        alias_package_root_module(&mut analysis, "time").expect("alias package root module");

        assert!(analysis.module_exports.contains_key("time"));
        assert_eq!(
            analysis.module_aliases.get("time").map(String::as_str),
            Some("lib")
        );
    }

    #[test]
    fn alias_package_root_module_rejects_name_collisions() {
        let mut analysis = ProjectAnalysis {
            module_exports: HashMap::from([
                ("lib".to_string(), vec!["now".to_string()]),
                ("time".to_string(), vec!["from_time".to_string()]),
            ]),
            module_type_decls: HashMap::new(),
            all_module_schemes: HashMap::new(),
            module_aliases: HashMap::new(),
        };

        let err =
            alias_package_root_module(&mut analysis, "time").expect_err("expected alias collision");
        assert!(
            err.contains("module name collision"),
            "unexpected error: {err}"
        );
    }
}
