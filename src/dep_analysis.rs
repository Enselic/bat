/*  To exercise this code, run this:

  cargo run -- cache --build --source assets --blank --target assets | tee /tmp/out.txt

*/

use std::collections::{
    HashSet,
    HashMap,
};
use syntect::parsing::syntax_definition::{
    ContextReference,
    SyntaxDefinition,
    MatchOperation,
    Pattern,
};
use syntect::parsing::{
    SyntaxSet,
    SyntaxSetBuilder,
};

use serde::{Deserialize, Serialize};


// Offset into a binary blob where the start of a syntax set can be found
// Size is the size.
#[derive(Debug, Eq, PartialEq, Clone, Copy, Deserialize, Serialize, Hash)]
pub struct OffsetAndSize {
    pub offset: u64,
    pub size: u64,
}



#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyntaxDefinitionWithDeps {
    syntax_definition: SyntaxDefinition,
    deps: Vec<ContextReference>,
}


// TODO: Use references instead of copies
fn construct_direct_dependency_map(
    syntax_defs_with_deps: &mut [SyntaxDefinitionWithDeps],
) -> Vec<(ContextReference, SyntaxDefinitionWithDeps)> {
    // This can be a HashMap<ContextReference, SyntaxDefinitionWithDeps>
    // when/if ContextReference starts deriving from Hash
    let mut context_ref_to_syntax_def: Vec<(ContextReference, SyntaxDefinitionWithDeps)> = vec![]; // HashMap::new();

    for syntax_def_with_deps in syntax_defs_with_deps {
        let SyntaxDefinition {
            name,
            scope,
            contexts,
            ..
        } = &syntax_def_with_deps.syntax_definition;

        eprintln!("name={} scope={} deps:", name, scope.build_string());

        // Look for variants of:
        // - embed: scope:source.c
        // - include: C.sublime-syntax
        for context in contexts.values() {
            for pattern in &context.patterns {
                match *pattern {
                    Pattern::Include(ref context_reference) => {
                        handle_context_reference(&mut syntax_def_with_deps.deps, context_reference);
                    },
                    Pattern::Match(ref match_ref) => {
                        if let MatchOperation::Push(ref context_references) = match_ref.operation {
                            for context_reference in context_references {
                                handle_context_reference(&mut syntax_def_with_deps.deps, context_reference);
                            }
                        }
                    },
                }
            }
        }

        // TODO: Use references instead of clones
        context_ref_to_syntax_def.push((
            ContextReference::File { name: name.clone(), sub_context: None },
            syntax_def_with_deps.clone(),
        ));

        context_ref_to_syntax_def.push((
            ContextReference::ByScope { scope: *scope, sub_context: None },
            syntax_def_with_deps.clone(),
        ));
    }

    context_ref_to_syntax_def
}

fn handle_context_reference(
    deps: &mut Vec<ContextReference>,
    context_reference: &ContextReference
) {
    match *context_reference {
        ContextReference::File { ref name, .. } => {
            eprintln!("    {}", name);
            deps.push(context_reference.clone());
        },
        ContextReference::ByScope { ref scope, .. } => {
            eprintln!("    {}", scope.build_string());
            deps.push(context_reference.clone());
        },
        _ => {},
    }
}


/// Returns a vec of indepdent [`SyntaxSet`]s.
/// Enables improved startup time for some projects.
/// Implemented with an ugly brute force agorithm for prototyping purposes.
pub fn build_independent(syntax_set_builder: &SyntaxSetBuilder) -> Vec<SyntaxSet> {
    let mut result = vec![];

    let mut syntax_defs_with_deps = syntax_set_builder.syntaxes().iter().map(|syntax_definition| {
        SyntaxDefinitionWithDeps {
            syntax_definition: syntax_definition.clone(),
            deps: vec![],
        }
    }).collect::<Vec<SyntaxDefinitionWithDeps>>();

    let context_ref_to_syntax_def = construct_direct_dependency_map(&mut syntax_defs_with_deps);

    // Second pass: Transitively group dependencies and build SyntaxSets from them
    for syn_def_and_deps in syntax_defs_with_deps {

        eprintln!("Figuring out transitively deps for {}", syn_def_and_deps.syntax_definition.name);

        let mut builder = SyntaxSetBuilder::new();

        // We definitely need ourselves...
        let mut added_names = HashSet::new();
        builder.add(syn_def_and_deps.syntax_definition.clone());
        added_names.insert(syn_def_and_deps.syntax_definition.name);


        let mut deps_left = syn_def_and_deps.deps.clone();

        while deps_left.len() > 0 {
            let dep = deps_left.pop().unwrap();

            let syntax_for_dep = context_ref_to_syntax_def.iter().find(|x| x.0 == dep);
            match syntax_for_dep {
                Some(syntax_for_dep) => {
                    let syntax_definitiom_with_deps = &syntax_for_dep.1;
                    let syntax_definition = &syntax_definitiom_with_deps.syntax_definition;
                    let deps = &syntax_definitiom_with_deps.deps;
                    if added_names.contains(&syntax_definition.name) {
                        eprintln!("    not adding {}, already added", syntax_definition.name);
                    } else {
                        eprintln!("    adding {} to SyntaxSetBuilder", syntax_definition.name);
                        builder.add(syntax_definition.clone());
                        for dep in deps {
                            deps_left.push(dep.clone());
                        }
                        added_names.insert(syntax_definition.name.clone());
                    }
                },
                None => {
                    eprintln!("    syntax for {:?} not found, ignoring and hoping for the best", dep);
                },
            }
        }

        result.push(builder.build())
    }

    result
}
