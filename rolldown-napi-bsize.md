# cargo bsize: librolldown_binding.dylib

> Only propose source-code changes. Do not propose configuration changes.

Contents: [Summary](#summary) · [Dependencies](#dependencies) · [Functions and data symbols](#functions-and-data-symbols) · [Generic code, by the types it is instantiated over](#generic-code-by-the-types-it-is-instantiated-over) · [Panic, format, and unwind overhead](#panic-format-and-unwind-overhead) · [Duplicate read-only data](#duplicate-read-only-data) · [Dynamic dispatch](#dynamic-dispatch) · [Reference graph](#reference-graph) · [Derives](#derives) · [Largest types](#largest-types) · [Inlined code](#inlined-code) · [Assembly](#assembly) · [Constant data](#constant-data) · [Dynamic relocations](#dynamic-relocations)

## Summary

- Binary: `/Users/boshen/github/rolldown/rolldown/target/bsize/release/librolldown_binding.dylib` (macho)
- Total 23.1 MiB, shipped 16.0 MiB (without symbols and debug info). Shares below are of the shipped size.

|     Size | Share | Category |
|---------:|------:|---|
| 11.8 MiB | 73.7% | code |
|  7.1 MiB |     - | symbols (not shipped) |
|  2.2 MiB | 13.9% | unwind |
|  1.9 MiB | 12.1% | read-only data |
| 24.6 KiB |  0.2% | data |
| 24.7 KiB |  0.2% | overhead (headers, padding, code signature) |

### Sections

|      Size | Share | Section |
|----------:|------:|---|
|  11.7 MiB | 73.2% | `__TEXT,__text` |
|   7.1 MiB |     - | `__LINKEDIT` |
|   1.5 MiB |  9.2% | `__TEXT,__eh_frame` |
|   1.2 MiB |  7.3% | `__TEXT,__const` |
| 647.4 KiB |  4.0% | `__DATA_CONST,__const` |
| 524.3 KiB |  3.2% | `__TEXT,__gcc_except_tab` |
| 245.2 KiB |  1.5% | `__TEXT,__unwind_info` |
| 130.8 KiB |  0.8% | `__TEXT,__cstring` |
|  70.1 KiB |  0.4% | `__TEXT,__text_startup` |
|  21.3 KiB |  0.1% | `__DATA,__data` |
|   1.7 KiB | <0.1% | `__TEXT,__stub_helper` |
|   1.7 KiB | <0.1% | `__TEXT,__stubs` |
|   1.1 KiB | <0.1% | `__DATA,__la_symbol_ptr` |
|     936 B | <0.1% | `__DATA,__thread_vars` |
|     896 B | <0.1% | `__DATA_CONST,__mod_init_func` |
|     328 B | <0.1% | `__DATA,__thread_data` |
|      80 B | <0.1% | `__DATA_CONST,__got` |

### Coverage

_what the symbol table names, and what it leaves anonymous_

|      Size | Share | Bytes |
|----------:|------:|---|
|  11.8 MiB | 73.7% | code in 24764 named symbols |
| 474.3 KiB |  2.9% | read-only data in 489 named symbols |
|   1.5 MiB |  9.2% | in those sections, named by no symbol |
|  34.1 KiB |  0.2% | code no compile unit claims (built without debug info: C, assembly, std's backtrace crates) |

## Dependencies

### Duplicate versions (7)

_the same crate at several versions; each ships its own copy of the code, costed here from the compile units when the debug info was read_

| Crate | Version |     Code | Used by |
|---|---|---------:|---|
| `compact_str` | 0.9.1 |    248 B | oxc_resolver 11.24.2 |
|  | 0.10.0 |  3.4 KiB | oxc_span 0.146.0, oxc_str 0.146.0, oxc_transformer 0.146.0 |
| `embedded-io` | 0.4.0 |          | postcard 1.1.3 |
|  | 0.6.1 |          | postcard 1.1.3 |
| `foldhash` | 0.1.5 |          | hashbrown 0.15.5 |
|  | 0.2.0 |  1.4 KiB | hashbrown 0.16.1, hashbrown 0.17.1 |
| `getrandom` | 0.3.4 |          | ahash 0.8.12, tempfile 3.27.0 |
|  | 0.4.3 |    220 B | uuid 1.24.1 |
| `hashbrown` | 0.14.5 |     80 B | dashmap 6.2.1 |
|  | 0.15.5 |          | petgraph 0.8.3 |
|  | 0.16.1 |          | halfbrown 0.4.0, regress 0.11.1 |
|  | 0.17.1 |     80 B | indexmap 2.14.0, oxc_allocator 0.146.0, oxc_str 0.146.0, referencing 0.49.9 |
| `miniz_oxide` | 0.8.9 |          | flate2 1.1.9 |
|  | 0.9.1 |          | oxc-browserslist 5.0.1 |
| `num-bigint` | 0.4.8 |          | num 0.4.3, num-rational 0.4.2 |
|  | 0.5.1 | 58.5 KiB | oxc_ecmascript 0.146.0, oxc_parser 0.146.0, rolldown_common 1.2.5 |

### Features

_each linked dependency's resolved features and who asked for them; the bytes are the crate's whole code, so a shorter feature list returns some part of them_

|      Size | Share | Crate | Features | Requested by |
|----------:|------:|---|---|---|
| 318.7 KiB |  2.0% | `oxc_parser 0.146.0` | default, regular_expression | oxc (default features; regular_expression), oxc_minifier (default features; regular_expression), oxc_minify_napi (default features; regular_expression), oxc_transformer_plugins (default features; regular_expression) |
| 298.1 KiB |  1.8% | `oxc_ast 0.146.0` | default, serialize | oxc (default features), oxc_ast_visit (default features), oxc_codegen (default features), oxc_ecmascript (default features), oxc_isolated_declarations (default features), oxc_mangler (default features), oxc_minifier (default features), oxc_napi (default features), oxc_parser (default features), oxc_semantic (default features), oxc_transformer (default features), oxc_transformer_plugins (default features), oxc_traverse (default features) |
| 271.0 KiB |  1.7% | `regex-automata 0.4.18` | alloc, dfa, dfa-build, dfa-onepass, dfa-search, hybrid, meta, nfa, nfa-backtrack, nfa-pikevm, nfa-thompson, perf, perf-inline, perf-literal, perf-literal-multisubstring, perf-literal-substring, std, syntax, unicode, unicode-age, unicode-bool, unicode-case, unicode-gencat, unicode-perl, unicode-script, unicode-segment, unicode-word-boundary | fancy-regex (alloc, syntax, meta, nfa, dfa, hybrid), globset (std, perf, syntax, meta, nfa, hybrid), ignore (std, perf, syntax, meta, nfa, hybrid, dfa-onepass), regex (alloc, syntax, meta, nfa-pikevm) |
| 244.7 KiB |  1.5% | `oxc_resolver 11.24.2` | default, pnp, yarn_pnp | generator (default features; yarn_pnp), oxc_resolver_napi (default features), rolldown_binding (default features; yarn_pnp), rolldown_common (default features; yarn_pnp), rolldown_error (default features; yarn_pnp), rolldown_fs (default features; yarn_pnp), rolldown_plugin_vite_resolve (default features; yarn_pnp), rolldown_plugin_vite_transform (default features; yarn_pnp), rolldown_resolver (default features; yarn_pnp) |
| 192.1 KiB |  1.2% | `oxc_codegen 0.146.0` | sourcemap | oxc, oxc_minify_napi (sourcemap) |
| 172.7 KiB |  1.1% | `oxc_semantic 0.146.0` | default, serialize | oxc (default features), oxc_codegen (default features), oxc_mangler (default features), oxc_minifier (default features), oxc_transformer (default features), oxc_transformer_plugins (default features), oxc_traverse (default features) |
| 172.4 KiB |  1.1% | `regex-syntax 0.8.11` | default, std, unicode, unicode-age, unicode-bool, unicode-case, unicode-gencat, unicode-perl, unicode-script, unicode-segment | fancy-regex, globset (std), jsonschema-regex (default features), regex, regex-automata |
| 169.2 KiB |  1.0% | `oxc_resolver_napi 11.24.2` | yarn_pnp | rolldown_binding (yarn_pnp) |
| 110.4 KiB |  0.7% | `oxc_transform_napi 0.146.0` | default | rolldown_binding (default features) |
| 107.6 KiB |  0.7% | `regress 0.11.1` | backend-pikevm, default, std | oxc_resolver_napi (default features), pnp (default features), rolldown_utils (default features) |
|  91.4 KiB |  0.6% | `tokio 1.53.1` | default, fs, libc, macros, mio, rt, rt-multi-thread, signal, signal-hook-registry, sync, time, tokio-macros, windows-sys | async-scoped (default features; rt-multi-thread, macros, sync), bench (rt, rt-multi-thread), criterion2 (rt), napi (default features; rt, sync), rolldown (rt, macros, sync, time), rolldown_common, rolldown_dev (rt, macros, sync, time), rolldown_plugin (sync), rolldown_plugin_utils (fs), rolldown_testing (rt, macros, sync, rt-multi-thread), rolldown_utils, rolldown_watcher (rt, macros, sync, time) |
|  89.5 KiB |  0.5% | `aho-corasick 1.1.4` | default, perf-literal, std | globset (default features), regex, regex-automata |
|  86.9 KiB |  0.5% | `oxc_minify_napi 0.146.0` | default | rolldown_binding (default features) |
|  85.8 KiB |  0.5% | `oxc_parser_napi 0.146.0` | default | rolldown_binding (default features) |
|  79.5 KiB |  0.5% | `napi 3.12.1` | anyhow, async, default, dyn-symbols, indexmap, napi1, napi2, napi3, napi4, object_indexmap, serde, serde-json, serde_json, tokio, tokio_rt, tracing | oxc_minify_napi (default features), oxc_napi (default features), oxc_parser_napi (default features), oxc_resolver_napi (napi3, serde-json), oxc_sourcemap (default features), oxc_transform_napi (default features), rolldown_binding (default features; async, anyhow, tracing, object_indexmap), rolldown_error (default features; async, anyhow, tracing, object_indexmap) |
|  61.2 KiB |  0.4% | `oxc_sourcemap 8.1.2` | default, napi | oxc_codegen (default features), oxc_minify_napi (default features; napi), oxc_transform_napi (default features; napi), rolldown_binding (default features), rolldown_common (default features), rolldown_ecmascript (default features), rolldown_sourcemap (default features), string_wizard (default features) |
|  58.5 KiB |  0.4% | `num-bigint 0.5.1` | default, std | oxc_ecmascript (default features), oxc_parser (default features), rolldown_common (default features) |
|  57.9 KiB |  0.4% | `oxc_ast_visit 0.146.0` | default, serialize | oxc (default features), oxc_isolated_declarations (default features), oxc_minifier (default features), oxc_napi (default features; serialize), oxc_semantic (default features), oxc_transformer (default features), oxc_transformer_plugins (default features), oxc_traverse (default features) |
|  54.6 KiB |  0.3% | `zlib-rs 0.6.6` | rust-allocator, std | flate2 (std, rust-allocator) |
|  41.3 KiB |  0.3% | `url 2.5.8` | default, std | rolldown (default features), rolldown_binding (default features), rolldown_plugin_vite_resolve (default features) |

## Functions and data symbols

### Largest functions

|     Size | Share | Function | Defined at |
|---------:|------:|---|---|
| 44.8 KiB |  0.3% | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}::{closure#0}` | `crates/rolldown/src/module_loader/module_loader.rs:379` |
| 37.1 KiB |  0.2% | `rolldown_binding::utils::normalize_binding_options::normalize_binding_options` | `crates/rolldown_binding/src/utils/normalize_binding_options.rs:460` |
| 35.7 KiB |  0.2% | `<rolldown::stages::link_stage::LinkStage>::link` | `crates/rolldown/src/stages/link_stage/mod.rs:235` |
| 31.2 KiB |  0.2% | `core::ptr::drop_glue::<hashbrown::scopeguard::ScopeGuard<&mut hashbrown::raw::RawTableInner, <hashbrown::raw::RawTableInner>::rehash_in_place::{closure#0}>>` (33×) | `library/core/src/ptr/mod.rs:825` |
| 30.7 KiB |  0.2% | `<regress::classicalbacktrack::MatchAttempter<regress::indexing::Utf8Input>>::run_lookaround::<regress::cursor::Backward>` (4×) | `regress-0.11.1/src/classicalbacktrack.rs:458` |
| 29.3 KiB |  0.2% | `rolldown::utils::prepare_build_context::prepare_build_context` | `crates/rolldown/src/utils/prepare_build_context.rs:163` |
| 28.9 KiB |  0.2% | `<rolldown::module_finalizers::ScopeHoistingFinalizer as oxc_ast_visit::generated::visit_js_mut::VisitJsMut>::visit_program` | `crates/rolldown/src/module_finalizers/impl_visit_mut.rs:179` |
| 28.7 KiB |  0.2% | `<regress::parse::Parser<core::iter::adapters::map::Map<core::str::iter::Chars, <u32 as core::convert::From<char>>::from>>>::consume_term` (3×) | `regress-0.11.1/src/parse.rs:513` |
| 28.2 KiB |  0.2% | `<regress::classicalbacktrack::MatchAttempter<regress::indexing::Utf8Input>>::try_at_pos::<regress::cursor::Forward>` (4×) | `regress-0.11.1/src/classicalbacktrack.rs:637` |
| 27.2 KiB |  0.2% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` | `crates/rolldown/src/ecmascript/ecma_generator.rs:48` |
| 25.9 KiB |  0.2% | `<rolldown::hmr::hmr_stage::HmrStage<rolldown_fs::os::OsFileSystem>>::compute_hmr_update_for_file_changes::{closure#0}` | `crates/rolldown/src/hmr/hmr_stage.rs:102` |
| 24.9 KiB |  0.2% | `<rolldown::stages::generate_stage::GenerateStage>::optimize_dynamic_entry_bits` | `crates/rolldown/src/stages/generate_stage/dynamic_already_loaded.rs:55` |
| 23.2 KiB |  0.1% | `<rolldown::stages::link_stage::LinkStage>::bind_imports_and_exports` | `crates/rolldown/src/stages/link_stage/bind_imports_and_exports.rs:179` |
| 22.8 KiB |  0.1% | `napi_sys::functions::napi1::load` | `napi-sys-3.3.0/src/lib.rs:51` |
| 21.4 KiB |  0.1% | `<oxc_ast::ast::js::Expression as oxc_estree::serialize::ESTree>::serialize::<&mut oxc_estree::serialize::ESTreeSerializer<oxc_estree::serialize::config::ConfigFixes, oxc_estree::serialize::formatter::CompactFormatter>>` | `oxc_ast-0.146.0/src/generated/derive_estree.rs:23` |
| 20.7 KiB |  0.1% | `rolldown::utils::chunk::deconflict_chunk_symbols::deconflict_chunk_symbols` | `crates/rolldown/src/utils/chunk/deconflict_chunk_symbols.rs:19` |
| 20.7 KiB |  0.1% | `rolldown_plugin_bundle_analyzer::render_markdown::render_markdown` | `crates/rolldown_plugin_bundle_analyzer/src/render_markdown.rs:8` |
| 20.7 KiB |  0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}` | `crates/rolldown/src/stages/generate_stage/mod.rs:145` |
| 17.4 KiB |  0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunks::{closure#0}::{closure#0}` | `crates/rolldown/src/stages/generate_stage/code_splitting.rs:53` |
| 17.3 KiB |  0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunk_name_and_preliminary_filenames::{closure#0}::{closure#0}` | `crates/rolldown/src/stages/generate_stage/mod.rs:255` |

### Largest data symbols

_≤ marks an upper bound: the size runs to the next symbol, so it also counts the unnamed constants in between_

|        Size | Share | Symbol |
|------------:|------:|---|
| ≤ 295.0 KiB |  1.8% | `__mi_page_empty` |
|    17.2 KiB |  0.1% | `zmij::STATIC_DATA` |
|    12.7 KiB |  0.1% | `unicode_linebreak::BREAK_PROP_TRIE_DATA` |
|  ≤ 11.6 KiB |  0.1% | `__mi_theap_empty_wrong` |
|  ≤ 10.2 KiB |  0.1% | `core::num::imp::dec2flt::table::POWER_OF_FIVE_128` |
|     9.7 KiB |  0.1% | `dragonbox_ecma::cache::CACHE` |
|     8.0 KiB | <0.1% | `zlib_rs::crc32::braid::CRC32_WORD_TABLE` |
|   ≤ 7.9 KiB | <0.1% | `__mi_theap_empty` |
|     7.8 KiB | <0.1% | `unicode_id_start::tables::LEAF` |
|     5.8 KiB | <0.1% | `unicode_width::tables::WIDTH_LEAVES` |
|     5.6 KiB | <0.1% | `unicode_linebreak::BREAK_PROP_TRIE_INDEX` |
|   ≤ 4.0 KiB | <0.1% | `core::unicode::unicode_data::uppercase::BITSET_CHUNKS_MAP` |
|     4.0 KiB | <0.1% | `num_bigint::biguint::convert::get_half_radix_base::BASES` |
|     4.0 KiB | <0.1% | `num_bigint::biguint::convert::get_radix_base::BASES` |
|   ≤ 3.2 KiB | <0.1% | `core::unicode::unicode_data::alphabetic::SHORT_OFFSET_RUNS` |
|     2.4 KiB | <0.1% | `serde_json::de::POW10` |
|     2.3 KiB | <0.1% | `unicode_linebreak::PAIR_TABLE` |
|     2.0 KiB | <0.1% | `oxc_parser::lexer::byte_handlers::byte_handler_tables::NO_TOKENS` |
|     1.8 KiB | <0.1% | `unicode_id_start::tables::TRIE_CONTINUE` |
|   ≤ 1.8 KiB | <0.1% | `core::unicode::unicode_data::conversions::UPPERCASE_LUT` |

### By pattern

_a symbol can match several patterns, so these do not sum to the total; the ↳ rows are the largest single offenders_

|      Size | Share | Pattern | Symbols |
|----------:|------:|---|--------:|
|   2.7 MiB | 16.7% | closures |    4145 |
| 227.8 KiB |  1.4% | ↳ `core::ptr::drop_glue` |         |
| 129.4 KiB |  0.8% | ↳ `core::slice::sort::unstable::quicksort::quicksort` |         |
| 123.8 KiB |  0.8% | ↳ `core::slice::sort::stable::quicksort::quicksort` |         |
|   1.1 MiB |  7.0% | drop glue |    5767 |
|   1.1 MiB |  7.0% | ↳ `core::ptr::drop_glue` |         |
|   1.5 KiB | <0.1% | ↳ `core::ptr::drop_in_place` |         |
| 541.5 KiB |  3.3% | iterators |     972 |
|  45.7 KiB |  0.3% | ↳ `rayon::iter::plumbing::bridge_producer_consumer::helper` |         |
|  44.5 KiB |  0.3% | ↳ `core::ptr::drop_glue` |         |
|  38.5 KiB |  0.2% | ↳ `rayon_core::join::join_context::{closure#0}` |         |
| 404.7 KiB |  2.5% | formatting |    2832 |
|  10.9 KiB |  0.1% | ↳ `<std::thread::local::LocalKey<core::cell::RefCell<alloc::string::String>>>::with` |         |
|  10.9 KiB |  0.1% | ↳ `<alloc::string::String as core::fmt::Write>::write_char` |         |
|  10.4 KiB |  0.1% | ↳ `<&regress::insn::Insn as core::fmt::Debug>::fmt` |         |
| 130.3 KiB |  0.8% | serde |     180 |
|  12.9 KiB |  0.1% | ↳ `<&mut serde_json::de::Deserializer<serde_json::read::SliceRead> as serde_core::de::Deserializer>::deserialize_struct` |         |
|  12.1 KiB |  0.1% | ↳ `<serde_json::value::Value as serde_core::de::Deserialize>::deserialize` |         |
|  10.9 KiB |  0.1% | ↳ `pnp::manifest::deserialize_package_registry_data` |         |
|  31.1 KiB |  0.2% | panic paths |     130 |
|  10.4 KiB |  0.1% | ↳ `core::ptr::drop_glue` |         |
|   7.0 KiB | <0.1% | ↳ `<regex_automata::util::pool::inner::Pool<regex_automata::meta::regex::Cache, alloc::boxed::Box<dyn core::ops::function::Fn<(), Output = regex_automata::meta::regex::Cache> + core::marker::Sync + core::panic::unwind_safe::UnwindSafe + core::panic::unwind_safe::RefUnwindSafe + core::marker::Send>>>::put_value` |         |
|   5.0 KiB | <0.1% | ↳ `<regex_automata::util::pool::inner::Pool<regex_automata::meta::regex::Cache, alloc::boxed::Box<dyn core::ops::function::Fn<(), Output = regex_automata::meta::regex::Cache> + core::marker::Sync + core::panic::unwind_safe::UnwindSafe + core::panic::unwind_safe::RefUnwindSafe + core::marker::Send>>>::get_slow` |         |

### By trait method, every impl combined

_attribution — one method summed over every impl, so where the bytes sit, not what one change removes_

|      Size | Share | Trait method | Impls |
|----------:|------:|---|------:|
| 253.5 KiB |  1.6% | `<core::fmt::Debug>::fmt` |  1982 |
| 245.0 KiB |  1.5% | `<napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` |   171 |
| 143.2 KiB |  0.9% | `<oxc_estree::serialize::ESTree>::serialize` |    68 |
| 117.5 KiB |  0.7% | `<oxc_codegen::gen::Gen>::gen` |   125 |
| 100.2 KiB |  0.6% | `<napi::bindgen_runtime::js_values::ToNapiValue>::to_napi_value` |   133 |
|  76.7 KiB |  0.5% | `<core::iter::traits::iterator::Iterator>::next` |    89 |
|  72.4 KiB |  0.4% | `<core::convert::From>::from` |   142 |
|  68.0 KiB |  0.4% | `<oxc_allocator::clone_in::CloneIn>::clone_in_impl` |   103 |
|  62.7 KiB |  0.4% | `<core::clone::Clone>::clone` |    85 |
|  52.0 KiB |  0.3% | `<core::fmt::Display>::fmt` |   401 |
|  47.4 KiB |  0.3% | `<core::convert::TryFrom>::try_from` |    38 |
|  38.0 KiB |  0.2% | `<rayon_core::job::Job>::execute` |    88 |
|  35.5 KiB |  0.2% | `<oxc_ast_visit::generated::visit_js_mut::VisitJsMut>::visit_expression` |    10 |
|  35.5 KiB |  0.2% | `<oxc_span::cmp::ContentEq>::content_eq` |   119 |
|  33.7 KiB |  0.2% | `<oxc_estree::serialize::structs::StructSerializer>::serialize_field` |    52 |
|  33.2 KiB |  0.2% | `<rolldown_plugin::plugin::Plugin>::resolve_id` |    17 |
|  32.3 KiB |  0.2% | `<oxc_ast_visit::generated::visit_js_mut::VisitJsMut>::visit_program` |     4 |
|  29.7 KiB |  0.2% | `<oxc_ecmascript::side_effects::MayHaveSideEffects>::may_have_side_effects` |    43 |
|  28.7 KiB |  0.2% | `<core::convert::From::from>::consume_term` |     3 |
|  27.2 KiB |  0.2% | `<rolldown::types::generator::Generator>::instantiate_chunk` |     1 |

### By trait, every method of every impl combined

_one axis coarser than above; the ↳ rows are the trait's largest single impls — the concrete targets_

|      Size | Share | Trait | Methods |
|----------:|------:|---|--------:|
| 253.5 KiB |  1.6% | `core::fmt::Debug` |    1982 |
|  10.4 KiB |  0.1% | ↳ `<&regress::insn::Insn as core::fmt::Debug>::fmt` |         |
|   7.4 KiB | <0.1% | ↳ `<oxc_resolver::error::ResolveError as core::fmt::Debug>::fmt` |         |
|   2.6 KiB | <0.1% | ↳ `<&[u8; 16] as core::fmt::Debug>::fmt` |         |
| 245.0 KiB |  1.5% | `napi::bindgen_runtime::js_values::FromNapiValue` |     171 |
|  11.9 KiB |  0.1% | ↳ `<rolldown_binding::options::binding_output_options::BindingOutputOptions as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` |         |
|  10.1 KiB |  0.1% | ↳ `<rolldown_binding::options::binding_input_options::BindingInputOptions as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` |         |
|   9.0 KiB |  0.1% | ↳ `<rolldown_binding::options::plugin::binding_plugin_options::BindingPluginOptions as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` |         |
| 185.6 KiB |  1.1% | `oxc_ast_visit::generated::visit_js_mut::VisitJsMut` |     280 |
|  28.9 KiB |  0.2% | ↳ `<rolldown::module_finalizers::ScopeHoistingFinalizer as oxc_ast_visit::generated::visit_js_mut::VisitJsMut>::visit_program` |         |
|   7.7 KiB | <0.1% | ↳ `<rolldown::module_finalizers::ScopeHoistingFinalizer as oxc_ast_visit::generated::visit_js_mut::VisitJsMut>::visit_expression` |         |
|   5.1 KiB | <0.1% | ↳ `<rolldown::utils::tweak_ast_for_scanning::PreProcessor as oxc_ast_visit::generated::visit_js_mut::VisitJsMut>::visit_expression` |         |
| 178.6 KiB |  1.1% | `rolldown_plugin::plugin::Plugin` |     139 |
|  10.5 KiB |  0.1% | ↳ `<rolldown_plugin_vite_resolve::vite_resolve_plugin::ViteResolvePlugin as rolldown_plugin::plugin::Plugin>::resolve_id::{closure#0}` |         |
|   8.1 KiB | <0.1% | ↳ `<rolldown_plugin_vite_manifest::ViteManifestPlugin as rolldown_plugin::plugin::Plugin>::generate_bundle::{closure#0}` |         |
|   6.3 KiB | <0.1% | ↳ `<rolldown_binding::options::plugin::js_plugin::JsPlugin as rolldown_plugin::plugin::Plugin>::load::{closure#0}` |         |
| 143.2 KiB |  0.9% | `oxc_estree::serialize::ESTree` |      68 |
|  21.4 KiB |  0.1% | ↳ `<oxc_ast::ast::js::Expression as oxc_estree::serialize::ESTree>::serialize` |         |
|  16.7 KiB |  0.1% | ↳ `<oxc_ast::ast::js::Statement as oxc_estree::serialize::ESTree>::serialize` |         |
|  13.2 KiB |  0.1% | ↳ `<oxc_ast::ast::ts::TSType as oxc_estree::serialize::ESTree>::serialize` |         |
| 141.8 KiB |  0.9% | `oxc_ast_visit::generated::visit_js::VisitJs` |     232 |
|  12.0 KiB |  0.1% | ↳ `<rolldown::ast_scanner::AstScanner as oxc_ast_visit::generated::visit_js::VisitJs>::visit_statement` |         |
|   7.3 KiB | <0.1% | ↳ `<rolldown::ast_scanner::AstScanner as oxc_ast_visit::generated::visit_js::VisitJs>::visit_identifier_reference` |         |
|   3.8 KiB | <0.1% | ↳ `<oxc_minifier::keep_var::KeepVar as oxc_ast_visit::generated::visit_js::VisitJs>::visit_expression` |         |
| 119.8 KiB |  0.7% | `oxc_ast_visit::generated::visit::Visit` |     202 |
|   3.5 KiB | <0.1% | ↳ `<oxc_semantic::stats::Counter as oxc_ast_visit::generated::visit::Visit>::visit_expression` |         |
|   2.9 KiB | <0.1% | ↳ `<oxc_minifier::property_mangler::PropertyCollector as oxc_ast_visit::generated::visit::Visit>::visit_expression` |         |
|   2.9 KiB | <0.1% | ↳ `<oxc_semantic::builder::SemanticBuilder as oxc_ast_visit::generated::visit::Visit>::visit_ts_enum_declaration` |         |
| 119.0 KiB |  0.7% | `oxc_codegen::gen::Gen` |     130 |
|   7.5 KiB | <0.1% | ↳ `<oxc_ast::ast::js::Statement as oxc_codegen::gen::Gen>::gen` |         |
|   3.9 KiB | <0.1% | ↳ `<oxc_ast::ast::js::ImportDeclaration as oxc_codegen::gen::Gen>::gen` |         |
|   3.2 KiB | <0.1% | ↳ `<oxc_ast::ast::js::IfStatement as oxc_codegen::gen::Gen>::gen` |         |
| 111.0 KiB |  0.7% | `oxc_traverse::generated::traverse::Traverse` |      78 |
|  10.5 KiB |  0.1% | ↳ `<oxc_transformer::es2018::async_generator_functions::AsyncGeneratorFunctions as oxc_traverse::generated::traverse::Traverse<oxc_transformer::state::TransformState>>::enter_statement` |         |
|   9.4 KiB |  0.1% | ↳ `<rolldown::hmr::hmr_ast_finalizer::HmrAstFinalizer as oxc_traverse::generated::traverse::Traverse<()>>::enter_program` |         |
|   8.6 KiB |  0.1% | ↳ `<oxc_transformer::typescript::TypeScript as oxc_traverse::generated::traverse::Traverse<oxc_transformer::state::TransformState>>::enter_class` |         |
| 108.3 KiB |  0.7% | `core::iter::traits::iterator::Iterator` |     151 |
|  12.2 KiB |  0.1% | ↳ `<core::slice::iter::IterMut<rolldown_common::chunk::Chunk> as core::iter::traits::iterator::Iterator>::for_each` |         |
|   6.9 KiB | <0.1% | ↳ `<oxc_diagnostics::handlers::graphical::line::CharWidthIterator as core::iter::traits::iterator::Iterator>::next` |         |
|   3.5 KiB | <0.1% | ↳ `<core::iter::adapters::filter_map::FilterMap<indexmap::map::iter::Iter<rolldown_utils::bitset::BitSet, rolldown_common::types::chunk_idx::ChunkIdx>, <rolldown::stages::generate_stage::GenerateStage>::try_insert_common_module_to_exist_chunk::{closure#1}> as core::iter::traits::iterator::Iterator>::next` |         |
| 100.2 KiB |  0.6% | `napi::bindgen_runtime::js_values::ToNapiValue` |     133 |
|   5.6 KiB | <0.1% | ↳ `<rolldown_binding::transform::BindingCompilerOptions as napi::bindgen_runtime::js_values::ToNapiValue>::to_napi_value` |         |
|   4.2 KiB | <0.1% | ↳ `<alloc::vec::Vec<alloc::string::String> as napi::bindgen_runtime::js_values::ToNapiValue>::to_napi_value` |         |
|   3.1 KiB | <0.1% | ↳ `<oxc_parser_napi::types::EcmaScriptModule as napi::bindgen_runtime::js_values::ToNapiValue>::to_napi_value` |         |
|  94.2 KiB |  0.6% | `core::convert::From::from` |      54 |
|  28.7 KiB |  0.2% | ↳ `<regress::parse::Parser<core::iter::adapters::map::Map<core::str::iter::Chars, <u32 as core::convert::From<char>>::from>>>::consume_term` |         |
|  16.5 KiB |  0.1% | ↳ `<regress::parse::Parser<core::iter::adapters::map::Map<core::str::iter::Chars, <u32 as core::convert::From<char>>::from>>>::try_escape_unicode_sequence` |         |
|   9.3 KiB |  0.1% | ↳ `<regress::parse::Parser<core::iter::adapters::map::Map<core::str::iter::Chars, <u32 as core::convert::From<char>>::from>>>::consume_class_set_expression` |         |
|  72.8 KiB |  0.4% | `oxc_ast_visit::generated::visit_mut::VisitMut` |     158 |
|   3.8 KiB | <0.1% | ↳ `<oxc_minifier::property_mangler::PropertyRewriter as oxc_ast_visit::generated::visit_mut::VisitMut>::visit_expression` |         |
|   3.6 KiB | <0.1% | ↳ `<oxc_transformer::es2022::class_properties::private_method::PrivateMethodVisitor as oxc_ast_visit::generated::visit_mut::VisitMut>::visit_expression` |         |
|   3.4 KiB | <0.1% | ↳ `<oxc_transformer::es2022::class_properties::static_block_and_prop_init::StaticVisitor as oxc_ast_visit::generated::visit_mut::VisitMut>::visit_expression` |         |
|  72.4 KiB |  0.4% | `core::convert::From` |     142 |
|   3.9 KiB | <0.1% | ↳ `<oxc_parser_napi::types::EcmaScriptModule as core::convert::From<&oxc_syntax::module_record::ModuleRecord>>::from` |         |
|   3.7 KiB | <0.1% | ↳ `<rolldown_binding::transform::BindingCompilerOptions as core::convert::From<oxc_resolver::tsconfig::CompilerOptions>>::from` |         |
|   2.6 KiB | <0.1% | ↳ `<rolldown_plugin_vite_dynamic_import_vars::ViteDynamicImportVarsPlugin as core::convert::From<rolldown_binding::options::plugin::config::binding_vite_dynamic_import_vars_plugin_config::BindingViteDynamicImportVarsPluginConfig>>::from::{closure#0}::{closure#0}::{closure#0}` |         |
|  68.0 KiB |  0.4% | `oxc_allocator::clone_in::CloneIn` |     103 |
|   6.8 KiB | <0.1% | ↳ `<oxc_ast::ast::js::Expression as oxc_allocator::clone_in::CloneIn>::clone_in_impl` |         |
|   3.7 KiB | <0.1% | ↳ `<oxc_ast::ast::ts::TSType as oxc_allocator::clone_in::CloneIn>::clone_in_impl` |         |
|   3.5 KiB | <0.1% | ↳ `<oxc_ast::ast::js::Statement as oxc_allocator::clone_in::CloneIn>::clone_in_impl` |         |
|  66.0 KiB |  0.4% | `core::clone::Clone` |      90 |
|  12.2 KiB |  0.1% | ↳ `<rolldown_common::module::normal_module::NormalModule as core::clone::Clone>::clone` |         |
|   8.9 KiB |  0.1% | ↳ `<oxc_resolver::error::ResolveError as core::clone::Clone>::clone` |         |
|   3.0 KiB | <0.1% | ↳ `<oxc_resolver::tsconfig::TsConfig as core::clone::Clone>::clone` |         |
|  52.0 KiB |  0.3% | `core::fmt::Display` |     401 |
|   2.9 KiB | <0.1% | ↳ `<rustc_demangle::legacy::Demangle as core::fmt::Display>::fmt` |         |
|   1.9 KiB | <0.1% | ↳ `<core::str::iter::EscapeDebug as core::fmt::Display>::fmt` |         |
|   1.7 KiB | <0.1% | ↳ `<yansi::paint::Painted<&&str> as core::fmt::Display>::fmt` |         |
|  49.8 KiB |  0.3% | `rolldown_plugin::pluginable::Pluginable` |     732 |
|     296 B | <0.1% | ↳ `<rolldown_plugin_oxc_runtime::OxcRuntimePlugin as rolldown_plugin::pluginable::Pluginable>::call_load` |         |
|     256 B | <0.1% | ↳ `<rolldown_plugin_oxc_runtime::OxcRuntimePlugin as rolldown_plugin::pluginable::Pluginable>::call_transform_ast` |         |
|     200 B | <0.1% | ↳ `<rolldown_plugin_oxc_runtime::OxcRuntimePlugin as rolldown_plugin::pluginable::Pluginable>::call_augment_chunk_hash` |         |
|  47.4 KiB |  0.3% | `core::convert::TryFrom` |      38 |
|   6.1 KiB | <0.1% | ↳ `<oxc_resolver::options::Restriction as core::convert::TryFrom<oxc_resolver_napi::options::Restriction>>::try_from` |         |
|   5.0 KiB | <0.1% | ↳ `<alloc::sync::Arc<dyn rolldown_plugin::pluginable::Pluginable> as core::convert::TryFrom<rolldown_binding::options::plugin::binding_builtin_plugin::BindingBuiltinPlugin>>::try_from` |         |
|   3.8 KiB | <0.1% | ↳ `<oxc_resolver::options::Restriction as core::convert::TryFrom<oxc_resolver_napi::options::Restriction>>::try_from::{closure#1}` |         |
|  38.0 KiB |  0.2% | `rayon_core::job::Job` |      88 |
|     564 B | <0.1% | ↳ `<rayon_core::job::StackJob<rayon_core::latch::SpinLatch, <rayon_core::registry::Registry>::in_worker_cross<rayon_core::join::join_context<rayon::iter::plumbing::bridge_producer_consumer::helper<rayon::iter::enumerate::EnumerateProducer<rayon::vec::DrainProducer<rolldown_common::types::instantiated_chunk::InstantiatedChunk>>, rayon::iter::map::MapConsumer<rayon::iter::collect::consumer::CollectConsumer<rolldown_common::types::asset::Asset>, rolldown::utils::chunk::finalize_chunks::finalize_assets::{closure#0}::{closure#0}::{closure#6}>>::{closure#0}, rayon::iter::plumbing::bridge_producer_consumer::helper<rayon::iter::enumerate::EnumerateProducer<rayon::vec::DrainProducer<rolldown_common::types::instantiated_chunk::InstantiatedChunk>>, rayon::iter::map::MapConsumer<rayon::iter::collect::consumer::CollectConsumer<rolldown_common::types::asset::Asset>, rolldown::utils::chunk::finalize_chunks::finalize_assets::{closure#0}::{closure#0}::{closure#6}>>::{closure#1}, rayon::iter::collect::consumer::CollectResult<rolldown_common::types::asset::Asset>, rayon::iter::collect::consumer::CollectResult<rolldown_common::types::asset::Asset>>::{closure#0}, (rayon::iter::collect::consumer::CollectResult<rolldown_common::types::asset::Asset>, rayon::iter::collect::consumer::CollectResult<rolldown_common::types::asset::Asset>)>::{closure#0}, (rayon::iter::collect::consumer::CollectResult<rolldown_common::types::asset::Asset>, rayon::iter::collect::consumer::CollectResult<rolldown_common::types::asset::Asset>)> as rayon_core::job::Job>::execute` |         |
|     564 B | <0.1% | ↳ `<rayon_core::job::StackJob<rayon_core::latch::SpinLatch, <rayon_core::registry::Registry>::in_worker_cross<rayon_core::join::join_context<rayon::iter::plumbing::bridge_producer_consumer::helper<rayon::vec::DrainProducer<rolldown::hmr::hmr_stage::ModuleRenderInput>, rayon::iter::map::MapConsumer<rayon::iter::collect::consumer::CollectConsumer<(rolldown_common::ecmascript::module_idx::ModuleIdx, alloc::string::String)>, <rolldown::hmr::hmr_stage::HmrStage<rolldown_fs::os::OsFileSystem>>::compute_hmr_update_for_file_changes::{closure#0}::{closure#7}>>::{closure#0}, rayon::iter::plumbing::bridge_producer_consumer::helper<rayon::vec::DrainProducer<rolldown::hmr::hmr_stage::ModuleRenderInput>, rayon::iter::map::MapConsumer<rayon::iter::collect::consumer::CollectConsumer<(rolldown_common::ecmascript::module_idx::ModuleIdx, alloc::string::String)>, <rolldown::hmr::hmr_stage::HmrStage<rolldown_fs::os::OsFileSystem>>::compute_hmr_update_for_file_changes::{closure#0}::{closure#7}>>::{closure#1}, rayon::iter::collect::consumer::CollectResult<(rolldown_common::ecmascript::module_idx::ModuleIdx, alloc::string::String)>, rayon::iter::collect::consumer::CollectResult<(rolldown_common::ecmascript::module_idx::ModuleIdx, alloc::string::String)>>::{closure#0}, (rayon::iter::collect::consumer::CollectResult<(rolldown_common::ecmascript::module_idx::ModuleIdx, alloc::string::String)>, rayon::iter::collect::consumer::CollectResult<(rolldown_common::ecmascript::module_idx::ModuleIdx, alloc::string::String)>)>::{closure#0}, (rayon::iter::collect::consumer::CollectResult<(rolldown_common::ecmascript::module_idx::ModuleIdx, alloc::string::String)>, rayon::iter::collect::consumer::CollectResult<(rolldown_common::ecmascript::module_idx::ModuleIdx, alloc::string::String)>)> as rayon_core::job::Job>::execute` |         |
|     564 B | <0.1% | ↳ `<rayon_core::job::StackJob<rayon_core::latch::SpinLatch, rayon_core::join::join_context::call_b<rayon::iter::collect::consumer::CollectResult<(alloc::string::String, u128)>, rayon::iter::plumbing::bridge_producer_consumer::helper<rayon::range::IterProducer<usize>, rayon::iter::map::MapConsumer<rayon::iter::collect::consumer::CollectConsumer<(alloc::string::String, u128)>, rolldown::utils::chunk::finalize_chunks::finalize_assets::{closure#0}::{closure#0}::{closure#4}>>::{closure#1}>::{closure#0}, rayon::iter::collect::consumer::CollectResult<(alloc::string::String, u128)>> as rayon_core::job::Job>::execute` |         |

### By crate, where the code is defined

|      Size | Share | Crate | Symbols |
|----------:|------:|---|--------:|
|   2.1 MiB | 13.3% | `core` |    8525 |
|   1.2 MiB |  7.8% | `rolldown` |     563 |
| 806.9 KiB |  4.9% | `rolldown_binding` |     813 |
| 472.3 KiB |  2.9% | `napi` |     656 |
| 415.7 KiB |  2.5% | `hashbrown` |     930 |
| 400.8 KiB |  2.5% | `oxc_transformer` |     478 |
| 399.3 KiB |  2.4% | `oxc_ast_visit` |     515 |
| 296.5 KiB |  1.8% | `oxc_minifier` |     372 |
| 285.3 KiB |  1.7% | `oxc_ast` |     503 |
| 278.5 KiB |  1.7% | `regress` |     177 |
| 278.3 KiB |  1.7% | `oxc_parser` |     332 |
| 250.7 KiB |  1.5% | `std` |     758 |
| 227.7 KiB |  1.4% | `regex_automata` |     344 |
| 226.9 KiB |  1.4% | `alloc` |    1242 |
| 182.6 KiB |  1.1% | `rolldown_plugin` |     853 |
| 181.7 KiB |  1.1% | `oxc_codegen` |     203 |
| 176.7 KiB |  1.1% | `oxc_traverse` |     216 |
| 172.7 KiB |  1.1% | `tokio` |     513 |
| 158.8 KiB |  1.0% | `oxc_resolver` |     153 |
| 156.8 KiB |  1.0% | `oxc_ecmascript` |     219 |

### By workspace file, where the code is defined

_2.9 MiB (18.0%) in 3012 functions defined in this workspace; code inlined away is charged to its caller's file, and a generated file shows the full cost of what it generates_

|     Size | Share | File | Functions |
|---------:|------:|---|----------:|
| 85.6 KiB |  0.5% | `crates/rolldown_binding/src/utils/normalize_binding_options.rs` |        47 |
| 73.2 KiB |  0.4% | `crates/rolldown/src/module_loader/module_loader.rs` |        10 |
| 71.7 KiB |  0.4% | `crates/rolldown/src/stages/generate_stage/compute_cross_chunk_links.rs` |        16 |
| 68.5 KiB |  0.4% | `crates/rolldown_binding/src/types/binding_magic_string.rs` |        99 |
| 64.5 KiB |  0.4% | `crates/rolldown_binding/src/options/plugin/js_plugin.rs` |        23 |
| 59.5 KiB |  0.4% | `crates/rolldown/src/stages/link_stage/bind_imports_and_exports.rs` |        10 |
| 55.2 KiB |  0.3% | `crates/rolldown/src/stages/generate_stage/mod.rs` |         9 |
| 49.8 KiB |  0.3% | `crates/rolldown_plugin/src/pluginable.rs` |       732 |
| 46.8 KiB |  0.3% | `crates/rolldown/src/stages/generate_stage/order_wrapping.rs` |        15 |
| 46.6 KiB |  0.3% | `crates/rolldown/src/stages/generate_stage/code_splitting.rs` |        14 |
| 45.6 KiB |  0.3% | `crates/rolldown_binding/src/binding_dev_engine.rs` |        38 |
| 45.1 KiB |  0.3% | `crates/rolldown/src/module_finalizers/impl_visit_mut.rs` |         9 |
| 41.7 KiB |  0.3% | `crates/rolldown/src/stages/link_stage/mod.rs` |         2 |
| 40.3 KiB |  0.2% | `crates/rolldown_plugin/src/plugin_driver/build_hooks.rs` |        15 |
| 38.4 KiB |  0.2% | `crates/rolldown/src/esm_init_obligations.rs` |        19 |
| 34.7 KiB |  0.2% | `crates/rolldown/src/stages/generate_stage/order_analysis.rs` |        15 |
| 34.0 KiB |  0.2% | `crates/rolldown/src/stages/generate_stage/chunk_optimizer.rs` |        17 |
| 33.7 KiB |  0.2% | `crates/rolldown/src/ast_scanner/impl_visit.rs` |        17 |
| 31.2 KiB |  0.2% | `crates/rolldown/src/hmr/hmr_stage.rs` |         8 |
| 30.8 KiB |  0.2% | `crates/rolldown_common/src/file_emitter.rs` |        15 |

### By workspace directory

|      Size | Share | Directory | Functions |
|----------:|------:|---|----------:|
| 394.4 KiB |  2.4% | `crates/rolldown/src/stages/generate_stage` |       128 |
| 176.4 KiB |  1.1% | `crates/rolldown_binding/src/types` |       275 |
| 154.4 KiB |  0.9% | `crates/rolldown_binding/src/options/plugin` |        99 |
| 126.0 KiB |  0.8% | `crates/rolldown/src/stages/link_stage` |        25 |
| 122.4 KiB |  0.7% | `crates/rolldown/src/module_loader` |        21 |
| 122.4 KiB |  0.7% | `crates/rolldown_binding/src` |       130 |
|  99.4 KiB |  0.6% | `crates/rolldown_binding/src/utils` |        62 |
|  87.1 KiB |  0.5% | `crates/rolldown/src/utils` |        31 |
|  77.6 KiB |  0.5% | `crates/rolldown/src/module_finalizers` |        38 |
|  74.2 KiB |  0.5% | `crates/rolldown_plugin/src/plugin_driver` |        44 |
|  64.3 KiB |  0.4% | `crates/rolldown/src/hmr` |        18 |
|  61.7 KiB |  0.4% | `crates/rolldown/src/utils/chunk` |        17 |
|  57.1 KiB |  0.3% | `crates/rolldown_plugin_vite_resolve/src` |        39 |
|  54.7 KiB |  0.3% | `crates/rolldown_utils/src` |        63 |
|  52.0 KiB |  0.3% | `crates/rolldown_plugin/src` |       778 |
|  51.2 KiB |  0.3% | `crates/rolldown/src/ast_scanner` |        36 |
|  48.1 KiB |  0.3% | `crates/rolldown_binding/src/options/plugin/config` |        44 |
|  47.1 KiB |  0.3% | `crates/rolldown_common/src/types` |       106 |
|  46.3 KiB |  0.3% | `crates/rolldown_watcher/src` |        23 |
|  45.9 KiB |  0.3% | `crates/rolldown_binding/src/options/plugin/types` |        49 |

### Generic families

_every instantiation of one generic summed; recoverable = the total less its largest instance, what collapsing the family onto one copy would return_

|      Size | Share | Family | Instances |     Each | Recoverable | Defined at |
|----------:|------:|---|----------:|---------:|------------:|---|
|   1.1 MiB |  7.0% | `core::ptr::drop_glue` |      5748 |    204 B |    ~1.1 MiB | `library/core/src/ptr/mod.rs:825` |
| 177.2 KiB |  1.1% | `core::slice::sort::unstable::quicksort::quicksort` |        63 |  2.8 KiB |  ~167.5 KiB | `library/core/src/slice/sort/unstable/quicksort.rs:21` |
| 136.6 KiB |  0.8% | `core::slice::sort::stable::quicksort::quicksort` |        40 |  3.4 KiB |  ~127.9 KiB | `library/core/src/slice/sort/stable/quicksort.rs:16` |
|  81.5 KiB |  0.5% | `napi::bindgen_runtime::js_values::object::from_raw_optional_field` |       129 |    646 B |   ~78.3 KiB | `napi-3.12.1/src/bindgen_runtime/js_values/object.rs:774` |
|  74.8 KiB |  0.5% | `napi::threadsafe_function::call_js_cb` |        43 |  1.7 KiB |   ~67.7 KiB | `napi-3.12.1/src/threadsafe_function.rs:805` |
|  66.9 KiB |  0.4% | `core::slice::sort::stable::drift::sort` |        40 |  1.7 KiB |   ~63.7 KiB | `library/core/src/slice/sort/stable/drift.rs:20` |
|  45.7 KiB |  0.3% | `rayon::iter::plumbing::bridge_producer_consumer::helper` |        39 |  1.2 KiB |   ~41.4 KiB | `rayon-1.12.0/src/iter/plumbing/mod.rs:393` |
|  45.3 KiB |  0.3% | `<hashbrown::raw::RawTable<usize>>::reserve_rehash` |        39 |  1.2 KiB |   ~43.9 KiB | `hashbrown-0.17.1/src/raw.rs:948` |
|  41.3 KiB |  0.3% | `tokio::runtime::task::raw::poll` |        63 |    671 B |   ~40.2 KiB | `tokio-1.53.1/src/runtime/task/raw.rs:341` |
|  38.5 KiB |  0.2% | `rayon_core::join::join_context::{closure#0}` |        42 |    938 B |   ~37.2 KiB | `rayon-core-1.13.0/src/join/mod.rs:132` |
|  34.3 KiB |  0.2% | `<regress::classicalbacktrack::MatchAttempter<regress::indexing::Utf8Input>>::run_lookaround` |         8 |  4.3 KiB |   ~26.3 KiB | `regress-0.11.1/src/classicalbacktrack.rs:458` |
|  33.7 KiB |  0.2% | `<oxc_estree::serialize::structs::ESTreeStructSerializer<oxc_estree::serialize::config::ConfigFixes, oxc_estree::serialize::formatter::CompactFormatter> as oxc_estree::serialize::structs::StructSerializer>::serialize_field` |        52 |    663 B |   ~29.8 KiB | `oxc_estree-0.146.0/src/serialize/structs.rs:89` |
|  33.1 KiB |  0.2% | `<rolldown::stages::generate_stage::GenerateStage>::compute_cross_chunk_link_state` |         2 | 16.6 KiB |   ~16.6 KiB | `crates/rolldown/src/stages/generate_stage/compute_cross_chunk_links.rs:483` |
|  33.1 KiB |  0.2% | `core::slice::sort::shared::smallsort::small_sort_general` |        27 |  1.2 KiB |   ~31.3 KiB | `library/core/src/slice/sort/shared/smallsort.rs:205` |
|  30.7 KiB |  0.2% | `<regress::classicalbacktrack::MatchAttempter<regress::indexing::Utf8Input>>::with_scm_loop_impl` |         8 |  3.8 KiB |   ~26.6 KiB | `regress-0.11.1/src/classicalbacktrack.rs:236` |
|  30.5 KiB |  0.2% | `core::slice::sort::shared::pivot::median3_rec` |        95 |    329 B |   ~29.4 KiB | `library/core/src/slice/sort/shared/pivot.rs:55` |
|  29.9 KiB |  0.2% | `core::slice::sort::unstable::ipnsort` |        63 |    486 B |   ~28.9 KiB | `library/core/src/slice/sort/unstable/mod.rs:114` |
|  28.7 KiB |  0.2% | `<regress::parse::Parser<core::iter::adapters::map::Map<core::str::iter::Chars, <u32 as core::convert::From<char>>::from>>>::consume_term` |         3 |  9.6 KiB |   ~19.1 KiB | `regress-0.11.1/src/parse.rs:513` |
|  28.2 KiB |  0.2% | `<regress::classicalbacktrack::MatchAttempter<regress::indexing::Utf8Input>>::try_at_pos` |         4 |  7.1 KiB |   ~21.1 KiB | `regress-0.11.1/src/classicalbacktrack.rs:637` |
|  25.7 KiB |  0.2% | `oxc_ast_visit::generated::visit_js_mut::walk_js_mut::walk_expression` |         7 |  3.7 KiB |   ~20.3 KiB | `oxc_ast_visit-0.146.0/src/generated/visit_js_mut.rs:1070` |

### By crate, which one caused the instantiation

_generic code from the families above, re-attributed to the crate that instantiated it — not additional_

|      Size | Share | Crate | Symbols |
|----------:|------:|---|--------:|
|   1.3 MiB |  8.4% | `rolldown_binding` |    3340 |
| 550.7 KiB |  3.4% | `rolldown` |    2067 |
| 105.8 KiB |  0.6% | `rolldown_utils` |     224 |
| 104.1 KiB |  0.6% | `oxc_resolver_napi` |     229 |
| 103.5 KiB |  0.6% | `pnp` |     190 |
|  81.4 KiB |  0.5% | `std` |     141 |
|  71.6 KiB |  0.4% | `rolldown_common` |     306 |
|  62.1 KiB |  0.4% | `rolldown_watcher` |     281 |
|  57.4 KiB |  0.4% | `oxc_resolver` |     209 |
|  57.1 KiB |  0.3% | `rolldown_plugin_vite_resolve` |     304 |
|  56.2 KiB |  0.3% | `rolldown_plugin_vite_import_glob` |     211 |
|  52.9 KiB |  0.3% | `rolldown_dev` |     256 |
|  47.2 KiB |  0.3% | `rolldown_devtools` |     164 |
|  42.8 KiB |  0.3% | `oxc_transformer` |     120 |
|  38.6 KiB |  0.2% | `oxc_minifier` |     126 |
|  37.1 KiB |  0.2% | `oxc_transform_napi` |     120 |
|  36.4 KiB |  0.2% | `rolldown_plugin` |     193 |
|  27.1 KiB |  0.2% | `oxc_minify_napi` |      90 |
|  26.7 KiB |  0.2% | `regex_automata` |     131 |
|  22.5 KiB |  0.1% | `rolldown_error` |      66 |

## Generic code, by the types it is instantiated over

_a turbofish names the types a generic was specialized to; bytes count toward every crate those types name, so rows overlap — the ↳ rows are the largest generic families within each_

- 6.6 MiB (41.3%) in 10796 symbols and 332829 inlined instances

|      Size | Share | Argument crate | Instantiations |
|----------:|------:|---|---------------:|
|   1.0 MiB |  6.6% | `rolldown` |          10891 |
| 144.5 KiB |  0.9% | ↳ `core::ptr::drop_glue` |                |
|  93.3 KiB |  0.6% | ↳ `core::slice::sort::unstable::quicksort::quicksort` |                |
|  69.1 KiB |  0.4% | ↳ `core::slice::sort::stable::quicksort::quicksort` |                |
|   1.0 MiB |  6.4% | `rolldown_common` |          12090 |
| 316.8 KiB |  1.9% | ↳ `core::ptr::drop_glue` |                |
| 105.0 KiB |  0.6% | ↳ `core::slice::sort::unstable::quicksort::quicksort` |                |
|  34.7 KiB |  0.2% | ↳ `rayon::iter::plumbing::bridge_producer_consumer::helper` |                |
| 484.1 KiB |  3.0% | `rolldown_binding` |           4749 |
| 152.3 KiB |  0.9% | ↳ `core::ptr::drop_glue` |                |
|  74.8 KiB |  0.5% | ↳ `napi::threadsafe_function::call_js_cb` |                |
|  40.9 KiB |  0.3% | ↳ `napi::bindgen_runtime::js_values::object::from_raw_optional_field` |                |
| 465.8 KiB |  2.9% | `(primitives)` |           1276 |
| 159.5 KiB |  1.0% | ↳ `core::ptr::copy_nonoverlapping` |                |
|  36.9 KiB |  0.2% | ↳ `core::sync::atomic::atomic_sub` |                |
|  35.7 KiB |  0.2% | ↳ `core::sync::atomic::atomic_load` |                |
| 442.9 KiB |  2.7% | `rustc_hash` |           5837 |
| 132.5 KiB |  0.8% | ↳ `core::ptr::drop_glue` |                |
|   9.6 KiB |  0.1% | ↳ `rayon_core::join::join_context::{closure#0}` |                |
|   7.3 KiB | <0.1% | ↳ `napi::bindgen_runtime::js_values::object::from_raw_optional_field` |                |
| 436.4 KiB |  2.7% | `napi` |           3922 |
| 137.4 KiB |  0.8% | ↳ `core::ptr::drop_glue` |                |
|  74.8 KiB |  0.5% | ↳ `napi::threadsafe_function::call_js_cb` |                |
|  56.8 KiB |  0.3% | ↳ `napi::bindgen_runtime::js_values::object::from_raw_optional_field` |                |
| 362.9 KiB |  2.2% | `alloc` |           2703 |
| 179.2 KiB |  1.1% | ↳ `core::ptr::drop_glue` |                |
|  11.8 KiB |  0.1% | ↳ `<oxc_diagnostics::handlers::graphical::handler::GraphicalReportHandler>::render_context` |                |
|  11.8 KiB |  0.1% | ↳ `napi::bindgen_runtime::js_values::object::from_raw_optional_field` |                |
| 311.7 KiB |  1.9% | `oxc_transformer` |           2950 |
|  12.5 KiB |  0.1% | ↳ `core::ptr::drop_glue` |                |
|  11.2 KiB |  0.1% | ↳ `oxc_ast_visit::generated::visit_js_mut::walk_js_mut::walk_expression` |                |
|   9.6 KiB |  0.1% | ↳ `oxc_ast_visit::generated::visit_mut::walk_mut::walk_ts_type` |                |
| 304.8 KiB |  1.9% | `hashbrown` |           3845 |
|  78.8 KiB |  0.5% | ↳ `core::ptr::drop_glue` |                |
|   5.1 KiB | <0.1% | ↳ `<hashbrown::raw::RawTable<(alloc::string::String, ())>>::reserve_rehash` |                |
|   2.0 KiB | <0.1% | ↳ `core::ptr::copy` |                |
| 297.6 KiB |  1.8% | `core` |           2259 |
|  81.5 KiB |  0.5% | ↳ `core::ptr::drop_glue` |                |
|  35.9 KiB |  0.2% | ↳ `core::str::validations::next_code_point` |                |
|  30.3 KiB |  0.2% | ↳ `core::slice::sort::unstable::quicksort::quicksort` |                |
| 253.0 KiB |  1.5% | `oxc_minifier` |           2056 |
|   8.9 KiB |  0.1% | ↳ `oxc_minifier::generated::walk::walk_expression` |                |
|   8.3 KiB |  0.1% | ↳ `oxc_minifier::generated::walk::walk_statement` |                |
|   7.4 KiB | <0.1% | ↳ `core::slice::sort::unstable::quicksort::quicksort` |                |
| 221.3 KiB |  1.4% | `oxc_str` |           2190 |
|  50.7 KiB |  0.3% | ↳ `core::ptr::drop_glue` |                |
|  24.3 KiB |  0.1% | ↳ `core::slice::sort::unstable::quicksort::quicksort` |                |
|  10.9 KiB |  0.1% | ↳ `<hashbrown::raw::RawTable<usize>>::reserve_rehash` |                |
| 185.8 KiB |  1.1% | `tokio` |           3363 |
|  81.9 KiB |  0.5% | ↳ `core::ptr::drop_glue` |                |
|  41.3 KiB |  0.3% | ↳ `tokio::runtime::task::raw::poll` |                |
|   9.4 KiB |  0.1% | ↳ `tokio::runtime::task::raw::drop_join_handle_slow` |                |
| 179.4 KiB |  1.1% | `rayon` |           2552 |
|  45.7 KiB |  0.3% | ↳ `rayon::iter::plumbing::bridge_producer_consumer::helper` |                |
|  38.5 KiB |  0.2% | ↳ `rayon_core::join::join_context::{closure#0}` |                |
|  23.2 KiB |  0.1% | ↳ `core::ptr::drop_glue` |                |
| 175.5 KiB |  1.1% | `oxc_ast` |           2686 |
|  39.2 KiB |  0.2% | ↳ `<oxc_estree::serialize::structs::ESTreeStructSerializer<oxc_estree::serialize::config::ConfigFixes, oxc_estree::serialize::formatter::CompactFormatter> as oxc_estree::serialize::structs::StructSerializer>::serialize_field` |                |
|  19.7 KiB |  0.1% | ↳ `<oxc_ast::ast::js::Expression as oxc_traverse::ast_operations::gather_node_parts::GatherNodeParts>::gather` |                |
|  15.2 KiB |  0.1% | ↳ `core::ptr::write` |                |
| 167.5 KiB |  1.0% | `oxc_estree` |            685 |
|  21.4 KiB |  0.1% | ↳ `<oxc_ast::ast::js::Expression as oxc_estree::serialize::ESTree>::serialize` |                |
|  16.7 KiB |  0.1% | ↳ `<oxc_ast::ast::js::Statement as oxc_estree::serialize::ESTree>::serialize` |                |
|  13.2 KiB |  0.1% | ↳ `<oxc_ast::ast::ts::TSType as oxc_estree::serialize::ESTree>::serialize` |                |
| 156.5 KiB |  1.0% | `rolldown_fs` |           1457 |
|  41.7 KiB |  0.3% | ↳ `core::ptr::drop_glue` |                |
|  12.0 KiB |  0.1% | ↳ `rolldown::module_loader::resolve_utils::resolve_dependencies::{closure#0}` |                |
|   8.3 KiB |  0.1% | ↳ `core::slice::sort::unstable::quicksort::quicksort` |                |
| 155.3 KiB |  1.0% | `indexmap` |           1967 |
|  45.3 KiB |  0.3% | ↳ `<hashbrown::raw::RawTable<usize>>::reserve_rehash` |                |
|  45.0 KiB |  0.3% | ↳ `core::ptr::drop_glue` |                |
|   9.4 KiB |  0.1% | ↳ `core::slice::sort::unstable::quicksort::quicksort` |                |
| 152.3 KiB |  0.9% | `std` |           1582 |
|  72.6 KiB |  0.4% | ↳ `core::ptr::drop_glue` |                |
|   7.6 KiB | <0.1% | ↳ `<std::sys::sync::once_box::OnceBox<std::sys::pal::unix::sync::mutex::Mutex>>::initialize` |                |
|   4.6 KiB | <0.1% | ↳ `core::mem::replace` |                |
| 138.4 KiB |  0.8% | `regress` |            440 |
|  34.3 KiB |  0.2% | ↳ `<regress::classicalbacktrack::MatchAttempter<regress::indexing::Utf8Input>>::run_lookaround` |                |
|  30.7 KiB |  0.2% | ↳ `<regress::classicalbacktrack::MatchAttempter<regress::indexing::Utf8Input>>::with_scm_loop_impl` |                |
|  28.2 KiB |  0.2% | ↳ `<regress::classicalbacktrack::MatchAttempter<regress::indexing::Utf8Input>>::try_at_pos` |                |

## Panic, format, and unwind overhead

_infrastructure the code views only hint at; panic="abort" drops the unwind tables, -Zbuild-std with panic_immediate_abort strips the panic locations, disabling tracing removes the callsite metadata_

|     Size | Share | What | Symbols |
|---------:|------:|---|--------:|
|  2.2 MiB | 13.9% | unwind and exception tables |         |
| 31.0 KiB |  0.2% | tracing metadata |     268 |
|  1.5 KiB | <0.1% | panic |       1 |

## Duplicate read-only data

_byte-identical constants under different names; the linker's --icf or sharing one const collapses them — 480 B recoverable from 4 groups (8 symbols)_

| Recoverable | Share | Constants | Copies |  Each |
|------------:|------:|---|-------:|------:|
|       256 B | <0.1% | `oxc_estree::serialize::strings::ESCAPE` ≡ `serde_json::ser::ESCAPE` |      2 | 256 B |
|       200 B | <0.1% | `dragonbox_ecma::to_chars::RADIX_100_TABLE` ≡ `itoa::DECIMAL_PAIRS` |      2 | 200 B |
|        16 B | <0.1% | `oxc_estree::serialize::strings::write_char_escape::HEX_DIGITS` ≡ `serde_json::ser::Formatter::write_char_escape::HEX_DIGITS` |      2 |  16 B |
|         8 B | <0.1% | `crossbeam_epoch::guard::unprotected::UNPROTECTED` ≡ `regex_automata::util::pool::inner::THREAD_ID_UNOWNED` |      2 |   8 B |

## Dynamic dispatch

### Named vtables and shims

_a proxy: the few vtables and fn-pointer shims that carry a symbol; most vtables are anonymous and counted below_

- 17.5 KiB (0.1%) in 123 vtables; 0 B (0.0%) in 0 coercion and drop shims.

|  Size | Share | Symbol |
|------:|------:|---|
| 572 B | <0.1% | `<<napi::threadsafe_function::ThreadsafeFunction<napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_plugin_context::BindingPluginContext, rolldown_binding::options::plugin::types::binding_shared_string::BindingSharedString, rolldown_binding::types::binding_rendered_chunk::BindingRenderedChunk, rolldown_binding::types::binding_normalized_options::BindingNormalizedOptions, rolldown_binding::options::plugin::types::binding_render_chunk_meta_chunks::BindingRenderedChunkMeta)>, napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_render_chunk_output::BindingHookRenderChunkOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_render_chunk_output::BindingHookRenderChunkOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_plugin_context::BindingPluginContext, rolldown_binding::options::plugin::types::binding_shared_string::BindingSharedString, rolldown_binding::types::binding_rendered_chunk::BindingRenderedChunk, rolldown_binding::types::binding_normalized_options::BindingNormalizedOptions, rolldown_binding::options::plugin::types::binding_render_chunk_meta_chunks::BindingRenderedChunkMeta)>, napi::status::Status, false, true>>::call_async_catch::{closure#0}::{closure#0}::{closure#0} as core::ops::function::FnOnce<(core::result::Result<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_render_chunk_output::BindingHookRenderChunkOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_render_chunk_output::BindingHookRenderChunkOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::error::Error>, napi::env::Env)>>::call_once::{shim:vtable#0}` |
| 512 B | <0.1% | `<<napi::threadsafe_function::ThreadsafeFunction<napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_load_context::BindingLoadPluginContext, alloc::string::String)>, napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_load_output::BindingHookLoadOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_load_output::BindingHookLoadOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_load_context::BindingLoadPluginContext, alloc::string::String)>, napi::status::Status, false, true>>::call_async_catch::{closure#0}::{closure#0}::{closure#0} as core::ops::function::FnOnce<(core::result::Result<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_load_output::BindingHookLoadOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_load_output::BindingHookLoadOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::error::Error>, napi::env::Env)>>::call_once::{shim:vtable#0}` |
| 512 B | <0.1% | `<<napi::threadsafe_function::ThreadsafeFunction<napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_transform_context::BindingTransformPluginContext, rolldown_binding::options::plugin::types::binding_shared_string::BindingSharedString, alloc::string::String, rolldown_binding::options::plugin::types::binding_plugin_transform_extra_args::BindingTransformHookExtraArgs)>, napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_transform_output::BindingHookTransformOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_transform_output::BindingHookTransformOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_transform_context::BindingTransformPluginContext, rolldown_binding::options::plugin::types::binding_shared_string::BindingSharedString, alloc::string::String, rolldown_binding::options::plugin::types::binding_plugin_transform_extra_args::BindingTransformHookExtraArgs)>, napi::status::Status, false, true>>::call_async_catch::{closure#0}::{closure#0}::{closure#0} as core::ops::function::FnOnce<(core::result::Result<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_transform_output::BindingHookTransformOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_transform_output::BindingHookTransformOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::error::Error>, napi::env::Env)>>::call_once::{shim:vtable#0}` |
| 464 B | <0.1% | `<<napi::threadsafe_function::ThreadsafeFunction<(), napi::bindgen_runtime::js_values::either::Either<alloc::vec::Vec<rolldown_binding::types::defer_sync_scan_data::BindingDeferSyncScanData>, rolldown_binding::types::js_callback::InvalidReturnValue>, (), napi::status::Status, false, true>>::call_async_catch::{closure#0}::{closure#0}::{closure#0} as core::ops::function::FnOnce<(core::result::Result<napi::bindgen_runtime::js_values::either::Either<alloc::vec::Vec<rolldown_binding::types::defer_sync_scan_data::BindingDeferSyncScanData>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::error::Error>, napi::env::Env)>>::call_once::{shim:vtable#0}` |
| 432 B | <0.1% | `<std::thread::lifecycle::spawn_unchecked<<notify::poll::PollWatcher>::run::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}` |
| 416 B | <0.1% | `<std::thread::lifecycle::spawn_unchecked<<rayon_core::registry::DefaultSpawn as rayon_core::registry::ThreadSpawn>::spawn::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}` |
| 416 B | <0.1% | `<std::thread::lifecycle::spawn_unchecked<<rolldown::stages::scan_stage::ScanStage<rolldown_fs::os::OsFileSystem>>::create_sourcemap_channel::{closure#0}, std::collections::hash::map::HashMap<rolldown_common::ecmascript::module_idx::ModuleIdx, alloc::vec::Vec<rolldown_common::types::sourcemap_chain_element::SourcemapChainElement>, rustc_hash::FxBuildHasher>>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}` |
| 412 B | <0.1% | `<std::thread::lifecycle::spawn_unchecked<notify_debouncer_full::new_debouncer_opt<rolldown_fs_watcher::utils::DebounceEventHandlerAdapter<rolldown_dev::watcher_event_handler::WatcherEventHandler>, notify::poll::PollWatcher, notify_debouncer_full::file_id_map::FileIdMap>::{closure#1}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}` |
| 396 B | <0.1% | `<std::thread::lifecycle::spawn_unchecked<notify_debouncer_full::new_debouncer_opt<rolldown_fs_watcher::utils::DebounceEventHandlerAdapter<rolldown_watcher::task_fs_event_handler::TaskFsEventHandler>, notify::poll::PollWatcher, notify_debouncer_full::file_id_map::FileIdMap>::{closure#1}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}` |
| 372 B | <0.1% | `<<napi::threadsafe_function::ThreadsafeFunction<(), napi::bindgen_runtime::js_values::either::Either<std::collections::hash::map::HashMap<alloc::string::String, alloc::string::String, rustc_hash::FxBuildHasher>, rolldown_binding::types::js_callback::InvalidReturnValue>, (), napi::status::Status, false, true>>::call_async_catch::{closure#0}::{closure#0}::{closure#0} as core::ops::function::FnOnce<(core::result::Result<napi::bindgen_runtime::js_values::either::Either<std::collections::hash::map::HashMap<alloc::string::String, alloc::string::String, rustc_hash::FxBuildHasher>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::error::Error>, napi::env::Env)>>::call_once::{shim:vtable#0}` |
| 364 B | <0.1% | `<std::thread::lifecycle::spawn_unchecked<<notify::fsevent::FsEventWatcher>::run::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}` |
| 344 B | <0.1% | `<std::thread::lifecycle::spawn_unchecked<<tokio::runtime::blocking::pool::Spawner>::spawn_thread::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}` |
| 344 B | <0.1% | `<std::thread::lifecycle::spawn_unchecked<rolldown_devtools::writer::LOG_WRITER_TX::{closure#0}::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}` |
| 340 B | <0.1% | `<<napi::threadsafe_function::ThreadsafeFunction<(), napi::bindgen_runtime::js_values::either::Either<rolldown_binding::types::binding_plugin_timings::BindingPluginTimingsMeasurement, rolldown_binding::types::js_callback::InvalidReturnValue>, (), napi::status::Status, false, true>>::call_async_catch::{closure#0}::{closure#0}::{closure#0} as core::ops::function::FnOnce<(core::result::Result<napi::bindgen_runtime::js_values::either::Either<rolldown_binding::types::binding_plugin_timings::BindingPluginTimingsMeasurement, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::error::Error>, napi::env::Env)>>::call_once::{shim:vtable#0}` |
| 340 B | <0.1% | `<<napi::threadsafe_function::ThreadsafeFunction<napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_plugin_context::BindingPluginContext, rolldown_binding::options::plugin::types::binding_hot_update_args::BindingHotUpdateArgs)>, napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<alloc::vec::Vec<alloc::string::String>>>, core::option::Option<alloc::vec::Vec<alloc::string::String>>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_plugin_context::BindingPluginContext, rolldown_binding::options::plugin::types::binding_hot_update_args::BindingHotUpdateArgs)>, napi::status::Status, false, true>>::call_async_catch::{closure#0}::{closure#0}::{closure#0} as core::ops::function::FnOnce<(core::result::Result<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<alloc::vec::Vec<alloc::string::String>>>, core::option::Option<alloc::vec::Vec<alloc::string::String>>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::error::Error>, napi::env::Env)>>::call_once::{shim:vtable#0}` |
| 292 B | <0.1% | `<<rolldown_plugin_vite_dynamic_import_vars::ViteDynamicImportVarsPlugin as core::convert::From<rolldown_binding::options::plugin::config::binding_vite_dynamic_import_vars_plugin_config::BindingViteDynamicImportVarsPluginConfig>>::from::{closure#0}::{closure#0} as core::ops::function::FnOnce<(alloc::string::String, alloc::string::String)>>::call_once::{shim:vtable#0}` |
| 276 B | <0.1% | `<rolldown_binding::utils::normalize_binding_options::normalize_on_log_option::{closure#0}::{closure#0} as core::ops::function::FnOnce<(rolldown_common::inner_bundler_options::types::log_level::LogLevel, rolldown_common::inner_bundler_options::types::on_log::Log)>>::call_once::{shim:vtable#0}` |
| 272 B | <0.1% | `<<napi::threadsafe_function::ThreadsafeFunction<napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_plugin_context::BindingPluginContext, alloc::string::String, core::option::Option<alloc::string::String>)>, napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_resolve_id_output::BindingHookResolveIdOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_resolve_id_output::BindingHookResolveIdOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_plugin_context::BindingPluginContext, alloc::string::String, core::option::Option<alloc::string::String>)>, napi::status::Status, false, true>>::call_async_catch::{closure#0}::{closure#0}::{closure#0} as core::ops::function::FnOnce<(core::result::Result<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_resolve_id_output::BindingHookResolveIdOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_resolve_id_output::BindingHookResolveIdOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::error::Error>, napi::env::Env)>>::call_once::{shim:vtable#0}` |
| 256 B | <0.1% | `<<rolldown_plugin_vite_resolve::vite_resolve_plugin::ViteResolveOptions as core::convert::From<rolldown_binding::options::plugin::config::binding_vite_resolve_plugin_config::BindingViteResolvePluginConfig>>::from::{closure#3}::{closure#0} as core::ops::function::FnOnce<(alloc::string::String,)>>::call_once::{shim:vtable#0}` |
| 256 B | <0.1% | `<<rolldown_plugin_vite_resolve::vite_resolve_plugin::ViteResolveOptions as core::convert::From<rolldown_binding::options::plugin::config::binding_vite_resolve_plugin_config::BindingViteResolvePluginConfig>>::from::{closure#4}::{closure#0} as core::ops::function::FnOnce<(alloc::string::String,)>>::call_once::{shim:vtable#0}` |

### Vtables by trait object

_recovered from the function pointers each anonymous vtable carries; bytes are the vtables themselves, not the methods they point at_

|     Size | Share | Trait object | Vtables |
|---------:|------:|---|--------:|
| 14.4 KiB |  0.1% | `dyn rolldown_plugin::plugin::Plugin` |     461 |
|  5.8 KiB | <0.1% | `dyn core::any::Any` |      16 |
|  2.3 KiB | <0.1% | `dyn core::fmt::Debug` |      50 |
|  2.2 KiB | <0.1% | `dyn core::ops::function::FnOnce` |      62 |
|  2.1 KiB | <0.1% | `dyn core::error::Error` |      24 |
|  1.9 KiB | <0.1% | `dyn core::fmt::Display` |      49 |
|  1.2 KiB | <0.1% | `dyn (trait not named by any slot)` |      39 |
|    768 B | <0.1% | `dyn core::convert::From` |      19 |
|    288 B | <0.1% | `dyn rolldown_error::build_diagnostic::events::AsAnyMut` |       4 |
|    144 B | <0.1% | `dyn core::fmt::Write` |       3 |
|    144 B | <0.1% | `dyn oxc_resolver::file_system::FileSystem` |       2 |
|    112 B | <0.1% | `dyn rolldown_sourcemap::source::Source` |       2 |
|     80 B | <0.1% | `dyn core::convert::TryFrom` |       2 |
|     32 B | <0.1% | `dyn tracing_core::field::Value` |       1 |
|     32 B | <0.1% | `dyn typedmap::typedkey::Key` |       1 |

## Reference graph

_who calls, addresses, and points at whom, read from the assembly; conservative, since an indirect call it cannot name may retain more_

### Called from one place

_nothing else reaches these — each exists for a single call site, where merging or inlining it would land_

|     Size | Share | Function | Only caller | Defined at |
|---------:|------:|---|---|---|
| 44.8 KiB |  0.3% | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}::{closure#0}` | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}` | `crates/rolldown/src/module_loader/module_loader.rs:379` |
| 35.7 KiB |  0.2% | `<rolldown::stages::link_stage::LinkStage>::link` | `<rolldown::bundle::bundle::Bundle>::bundle_up::{closure#0}` | `crates/rolldown/src/stages/link_stage/mod.rs:235` |
| 27.2 KiB |  0.2% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` | `<rolldown::stages::generate_stage::GenerateStage>::instantiate_chunks::{closure#0}::{closure#0}::{closure#2}::{closure#0}` | `crates/rolldown/src/ecmascript/ecma_generator.rs:48` |
| 25.9 KiB |  0.2% | `<rolldown::hmr::hmr_stage::HmrStage<rolldown_fs::os::OsFileSystem>>::compute_hmr_update_for_file_changes::{closure#0}` | `<rolldown::bundler::bundler::Bundler>::compute_hmr_update_for_file_changes::{closure#0}::{closure#0}` | `crates/rolldown/src/hmr/hmr_stage.rs:102` |
| 24.9 KiB |  0.2% | `<rolldown::stages::generate_stage::GenerateStage>::optimize_dynamic_entry_bits` | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunks::{closure#0}::{closure#0}` | `crates/rolldown/src/stages/generate_stage/dynamic_already_loaded.rs:55` |
| 20.7 KiB |  0.1% | `rolldown::utils::chunk::deconflict_chunk_symbols::deconflict_chunk_symbols` | `rayon::iter::plumbing::bridge_producer_consumer::helper::<rayon::iter::enumerate::EnumerateProducer<rayon::slice::IterMutProducer<rolldown_common::chunk::Chunk>>, rayon::iter::map::MapConsumer<rayon::iter::for_each::ForEachConsumer<<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}::{closure#4}::{closure#0}>, <oxc_index::IndexVec<rolldown_common::types::chunk_idx::ChunkIdx, rolldown_common::chunk::Chunk> as rolldown_utils::index_vec_ext::none_wasm::IndexVecExt<rolldown_common::types::chunk_idx::ChunkIdx, rolldown_common::chunk::Chunk>>::par_iter_mut_enumerated::{closure#0}>>` | `crates/rolldown/src/utils/chunk/deconflict_chunk_symbols.rs:19` |
| 20.7 KiB |  0.1% | `rolldown_plugin_bundle_analyzer::render_markdown::render_markdown` | `<rolldown_plugin_bundle_analyzer::BundleAnalyzerPlugin as rolldown_plugin::plugin::Plugin>::generate_bundle::{closure#0}` | `crates/rolldown_plugin_bundle_analyzer/src/render_markdown.rs:8` |
| 20.7 KiB |  0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}` | `<rolldown::bundle::bundle::Bundle>::bundle_up::{closure#0}` | `crates/rolldown/src/stages/generate_stage/mod.rs:145` |
| 17.4 KiB |  0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunks::{closure#0}::{closure#0}` | `<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}` | `crates/rolldown/src/stages/generate_stage/code_splitting.rs:53` |
| 17.3 KiB |  0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunk_name_and_preliminary_filenames::{closure#0}::{closure#0}` | `<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}` | `crates/rolldown/src/stages/generate_stage/mod.rs:255` |
| 16.1 KiB |  0.1% | `<rolldown_plugin_bundle_analyzer::BundleAnalyzerPlugin>::build_analyze_data` | `<rolldown_plugin_bundle_analyzer::BundleAnalyzerPlugin as rolldown_plugin::plugin::Plugin>::generate_bundle::{closure#0}` | `crates/rolldown_plugin_bundle_analyzer/src/lib.rs:132` |
| 13.3 KiB |  0.1% | `rolldown::utils::chunk::finalize_chunks::finalize_assets::{closure#0}::{closure#0}` | `<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}` | `crates/rolldown/src/utils/chunk/finalize_chunks.rs:37` |
| 12.7 KiB |  0.1% | `<rolldown::module_loader::module_task::ModuleTask<rolldown_fs::os::OsFileSystem>>::load_source::{closure#0}::{closure#0}` | `<rolldown::module_loader::module_task::ModuleTask<rolldown_fs::os::OsFileSystem>>::run::{closure#0}::{closure#0}` | `crates/rolldown/src/module_loader/module_task.rs:246` |
| 12.4 KiB |  0.1% | `rolldown::ecmascript::ecma_module_view_factory::create_ecma_view::{closure#0}` | `<rolldown::module_loader::module_task::ModuleTask<rolldown_fs::os::OsFileSystem>>::run::{closure#0}::{closure#0}` | `crates/rolldown/src/ecmascript/ecma_module_view_factory.rs:30` |
| 12.3 KiB |  0.1% | `<rolldown::stages::scan_stage::ScanStage<rolldown_fs::os::OsFileSystem>>::scan::{closure#0}::{closure#0}` | `<rolldown::bundle::bundle::Bundle>::scan_modules::{closure#0}::{closure#0}` | `crates/rolldown/src/stages/scan_stage.rs:158` |
| 12.1 KiB |  0.1% | `<rolldown::stages::generate_stage::GenerateStage>::optimize_facade_entry_chunks` | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunks::{closure#0}::{closure#0}` | `crates/rolldown/src/stages/generate_stage/chunk_optimizer.rs:987` |
| 11.9 KiB |  0.1% | `<rolldown_binding::options::binding_output_options::BindingOutputOptions as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` | `<rolldown_binding::types::binding_bundler_options::BindingBundlerOptions as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` | `crates/rolldown_binding/src/options/binding_output_options/mod.rs:38` |
| 11.8 KiB |  0.1% | `rolldown::utils::chunk::render_chunk_exports::render_chunk_exports` | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` | `crates/rolldown/src/utils/chunk/render_chunk_exports.rs:185` |
| 11.1 KiB |  0.1% | `<rolldown::stages::generate_stage::GenerateStage>::instantiate_chunks::{closure#0}::{closure#0}` | `<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}` | `crates/rolldown/src/stages/generate_stage/render_chunk_to_assets.rs:149` |
| 11.1 KiB |  0.1% | `<rolldown::utils::pre_process_ecma_ast::PreProcessEcmaAst>::build` | `rolldown::ecmascript::ecma_module_view_factory::create_ecma_view::{closure#0}` | `crates/rolldown/src/utils/pre_process_ecma_ast.rs:37` |

## Derives

_every impl of the trait, derived or hand-written alike; the total is attribution across many types, not a saving — the ↳ rows are the largest single impls, the ones worth acting on_

|      Size | Share | Derive | Impls |
|----------:|------:|---|------:|
| 253.6 KiB |  1.6% | Debug |  1983 |
|  10.4 KiB |  0.1% | ↳ `<&regress::insn::Insn as core::fmt::Debug>::fmt` |       |
|   7.4 KiB | <0.1% | ↳ `<oxc_resolver::error::ResolveError as core::fmt::Debug>::fmt` |       |
|   2.6 KiB | <0.1% | ↳ `<&[u8; 16] as core::fmt::Debug>::fmt` |       |
| 117.2 KiB |  0.7% | PartialOrd |   139 |
|  47.8 KiB |  0.3% | ↳ `core::slice::sort::unstable::quicksort::quicksort` |       |
|  12.9 KiB |  0.1% | ↳ `core::slice::sort::stable::quicksort::quicksort` |       |
|  10.5 KiB |  0.1% | ↳ `core::slice::sort::shared::smallsort::small_sort_general` |       |
|  66.0 KiB |  0.4% | Clone |    90 |
|  12.2 KiB |  0.1% | ↳ `<rolldown_common::module::normal_module::NormalModule as core::clone::Clone>::clone` |       |
|   8.9 KiB |  0.1% | ↳ `<oxc_resolver::error::ResolveError as core::clone::Clone>::clone` |       |
|   3.0 KiB | <0.1% | ↳ `<oxc_resolver::tsconfig::TsConfig as core::clone::Clone>::clone` |       |
|  28.5 KiB |  0.2% | Deserialize |    34 |
|  12.9 KiB |  0.1% | ↳ `<&mut serde_json::de::Deserializer<serde_json::read::SliceRead> as serde_core::de::Deserializer>::deserialize_struct` |       |
|  12.1 KiB |  0.1% | ↳ `<serde_json::value::Value as serde_core::de::Deserialize>::deserialize` |       |
|   1.1 KiB | <0.1% | ↳ `<serde_json::de::ParserNumber>::visit` |       |
|  20.3 KiB |  0.1% | Default |    52 |
|   7.1 KiB | <0.1% | ↳ `rayon::iter::plumbing::bridge_producer_consumer::helper` |       |
|   2.5 KiB | <0.1% | ↳ `rayon_core::join::join_context::{closure#0}` |       |
|   1.8 KiB | <0.1% | ↳ `<oxc_resolver::options::ResolveOptions as core::default::Default>::default` |       |
|  11.8 KiB |  0.1% | Serialize |     8 |
|  10.5 KiB |  0.1% | ↳ `<serde_json::value::Value as serde_core::ser::Serialize>::serialize` |       |
|   1.3 KiB | <0.1% | ↳ `<alloc::vec::Vec<alloc::string::String> as serde_core::ser::Serialize>::serialize` |       |
|   8.1 KiB | <0.1% | PartialEq |    24 |
|   3.3 KiB | <0.1% | ↳ `<std::path::Components as core::cmp::PartialEq>::eq` |       |
|   1.0 KiB | <0.1% | ↳ `<std::path::Component as core::cmp::PartialEq>::eq` |       |
|    1016 B | <0.1% | ↳ `<regex_syntax::hir::Hir as core::cmp::PartialEq>::eq` |       |
|   7.1 KiB | <0.1% | Hash |    11 |
|   5.1 KiB | <0.1% | ↳ `<std::path::Path as core::hash::Hash>::hash` |       |
|   1.3 KiB | <0.1% | ↳ `<std::path::PathBuf as core::hash::Hash>::hash` |       |
|     692 B | <0.1% | ↳ `<&rolldown_common::types::entry_point::EntryPoint as core::hash::Hash>::hash` |       |
|   1.2 KiB | <0.1% | Ord |     4 |
|     800 B | <0.1% | ↳ `<num_bigint::bigint::BigInt as core::cmp::Ord>::cmp` |       |
|     328 B | <0.1% | ↳ `<tracing_subscriber::filter::directive::StaticDirective as core::cmp::Ord>::cmp` |       |
|     148 B | <0.1% | ↳ `<compact_str::CompactString as core::cmp::Ord>::cmp` |       |

## Largest types

_in-memory layout size, not a share of the binary; a large type drives the moves, copies, and drop glue, and an enum is as large as its largest variant, so boxing that one shrinks every value_

|     Size | Type | Layout |
|---------:|---|---|
| 17.2 KiB | `Data` | 56 B padding |
| 14.2 KiB | `State` | 55 B padding |
|  9.7 KiB | `Pow10SignificandTable` | 32 B padding |
|  9.1 KiB | `{async_block_env#1}` | variants 3 9.1 KiB, 4 2.2 KiB, 0 800 B and 2 more; boxing 3 saves ~6.9 KiB per value |
|  8.1 KiB | `Zip<core::array::iter::IntoIter<u8, 8>, core::array::iter::IntoIter<[u32; 256], 8>>` |  |
|  8.0 KiB | `IntoIter<[u32; 256], 8>` |  |
|  8.0 KiB | `PolymorphicIter<[core::mem::maybe_uninit::MaybeUninit<[u32; 256]>; 8]>` |  |
|  7.9 KiB | `mi_theap_s` |  |
|  7.8 KiB | `Align64<[u8; 7936]>` |  |
|  7.5 KiB | `{async_fn_env#0}<rolldown_fs::os::OsFileSystem>` | variants 3 7.5 KiB, 4 7.5 KiB, 0 800 B and 2 more; boxing 3 saves ~40 B per value |
|  6.9 KiB | `{async_fn_env#0}<rolldown_binding::watcher::NapiWatcherEventHandler>` | variants 9 6.8 KiB, 3 6.6 KiB, 10 852 B and 10 more; boxing 9 saves ~204 B per value |
|  6.5 KiB | `{async_fn_env#0}` | variants 3 6.4 KiB, 0 96 B, 1 96 B and 1 more; boxing 3 saves ~6.3 KiB per value |
|  6.4 KiB | `mi_heap_s` |  |
|  6.2 KiB | `Instrumented<rolldown_dev::bundling_task::{impl#2}::rebuild::{async_fn#0}::{async_block_env#0}>` |  |
|  6.1 KiB | `Instrumented<rolldown_watcher::watch_task::{impl#0}::build::{async_fn#0}::{async_block_env#0}>` |  |
|  6.1 KiB | `{async_block_env#0}` | variants 4 6.1 KiB, 5 6.1 KiB, 7 220 B and 5 more |
|  6.0 KiB | `{async_fn_env#0}<core::option::Option<rolldown_dev_common::types::bundle_output::BundleOutput>, rolldown_watcher::watch_task::{impl#0}::build::{async_fn#0}::{async_block#0}::{closure_env#0}>` | variants 3 6.0 KiB, 0 49 B, 1 49 B and 1 more; boxing 3 saves ~6.0 KiB per value |
|  5.9 KiB | `{async_fn_env#0}<rolldown_dev_common::types::bundle_output::BundleOutput, rolldown::bundler::impl_bundler_incremental_build::{impl#0}::incremental_bundle::{async_fn#0}::{closure_env#0}>` | variants 3 5.9 KiB, 0 41 B, 1 41 B and 1 more; boxing 3 saves ~5.9 KiB per value |
|  5.9 KiB | `Instrumented<rolldown::bundle::bundle::{impl#0}::write::{async_fn#0}::{async_block_env#0}<rolldown_fs::os::OsFileSystem>>` |  |
|  5.9 KiB | `Instrumented<rolldown::bundle::bundle::{impl#0}::generate::{async_fn#0}::{async_block_env#0}<rolldown_fs::os::OsFileSystem>>` |  |

## Inlined code

- 6.1 MiB (38.4%) in 1589598 inlined instances, charged to their callers.

### Largest inlined functions

_bytes across every site a function was inlined into, counting only instructions not attributed to a deeper inline_

|      Size | Share | Function | Sites | Defined at |
|----------:|------:|---|------:|---|
| 276.4 KiB |  1.7% | `alloc::alloc::dealloc_nonnull` | 32217 | `library/alloc/src/alloc.rs:127` |
| 151.8 KiB |  0.9% | `core::ptr::copy_nonoverlapping::<u8>` | 15477 | `library/core/src/ptr/mod.rs:530` |
| 103.0 KiB |  0.6% | `rustc_hash::hash_bytes` |   608 | `rustc-hash-2.1.3/src/lib.rs:263` |
|  96.1 KiB |  0.6% | `<mimalloc_safe::MiMalloc as core::alloc::global::GlobalAlloc>::dealloc` | 11206 | `mimalloc-safe-0.1.64/src/lib.rs:60` |
|  83.9 KiB |  0.5% | `<oxc_allocator::arena::Arena>::alloc_layout::{closure#0}` |  3729 | `oxc_allocator-0.146.0/src/arena/alloc_impl.rs:33` |
|  79.9 KiB |  0.5% | `<u8 as core::slice::cmp::SlicePartialEq<u8>>::equal_same_length` |  3560 | `library/core/src/slice/cmp.rs:151` |
|  61.0 KiB |  0.4% | `alloc::alloc::alloc` |  7263 | `library/alloc/src/alloc.rs:95` |
|  55.8 KiB |  0.3% | `core::intrinsics::likely` |  4354 | `library/core/src/intrinsics/mod.rs:435` |
|  43.9 KiB |  0.3% | `alloc::boxed::box_new_uninit` |  2476 | `library/alloc/src/boxed.rs:247` |
|  40.2 KiB |  0.2% | `core::ptr::drop_glue::<alloc::string::String>` | 12782 | `library/core/src/ptr/mod.rs:825` |
|  40.1 KiB |  0.2% | `core::core_arch::arm_shared::neon::generated::vreinterpret_u64_u8` |  7723 | `library/core/src/../../stdarch/crates/core_arch/src/arm_shared/neon/generated.rs:46152` |
|  39.4 KiB |  0.2% | `<hashbrown::control::bitmask::BitMask as core::iter::traits::collect::IntoIterator>::into_iter` |  4543 | `hashbrown-0.17.1/src/control/bitmask.rs:86` |
|  36.9 KiB |  0.2% | `core::sync::atomic::atomic_sub::<usize, usize>` |  7676 | `library/core/src/sync/atomic.rs:3950` |
|  36.7 KiB |  0.2% | `<usize>::unchecked_mul` |  6551 | `library/core/src/num/uint_macros.rs:1347` |
|  36.3 KiB |  0.2% | `<mimalloc_safe::MiMalloc as core::alloc::global::GlobalAlloc>::alloc` |  3028 | `mimalloc-safe-0.1.64/src/lib.rs:50` |
|  35.4 KiB |  0.2% | `core::str::validations::next_code_point::<core::slice::iter::Iter<u8>>` |   505 | `library/core/src/str/validations.rs:35` |
|  33.9 KiB |  0.2% | `<usize>::wrapping_sub` |  7447 | `library/core/src/num/uint_macros.rs:2620` |
|  32.3 KiB |  0.2% | `<core::ops::range::Range<usize> as core::iter::range::RangeIteratorImpl>::spec_next` |  1321 | `library/core/src/iter/range.rs:899` |
|  28.6 KiB |  0.2% | `<u8 as core::slice::cmp::SliceOrd>::compare` |   966 | `library/core/src/slice/cmp.rs:325` |
|  28.4 KiB |  0.2% | `<*mut u8>::wrapping_offset` |  7139 | `library/core/src/ptr/mut_ptr.rs:465` |

### Functions holding the most inlined code

_how much of each body is other functions' instances — splitting the function or un-inlining the callees works here_

|     Size | Share | Of its body | Function |
|---------:|------:|------------:|---|
| 22.7 KiB |  0.1% |         51% | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}::{closure#0}` |
| 20.0 KiB |  0.1% |         64% | `core::ptr::drop_glue::<hashbrown::scopeguard::ScopeGuard<&mut hashbrown::raw::RawTableInner, <hashbrown::raw::RawTableInner>::rehash_in_place::{closure#0}>>` |
| 16.5 KiB |  0.1% |         61% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` |
| 14.6 KiB |  0.1% |         56% | `<rolldown::hmr::hmr_stage::HmrStage<rolldown_fs::os::OsFileSystem>>::compute_hmr_update_for_file_changes::{closure#0}` |
| 14.3 KiB |  0.1% |         63% | `napi_sys::functions::napi1::load` |
| 13.9 KiB |  0.1% |         48% | `<rolldown::module_finalizers::ScopeHoistingFinalizer as oxc_ast_visit::generated::visit_js_mut::VisitJsMut>::visit_program` |
| 13.4 KiB |  0.1% |         65% | `<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}` |
| 13.2 KiB |  0.1% |         36% | `rolldown_binding::utils::normalize_binding_options::normalize_binding_options` |
| 12.9 KiB |  0.1% |         62% | `rolldown_plugin_bundle_analyzer::render_markdown::render_markdown` |
| 12.9 KiB |  0.1% |         44% | `rolldown::utils::prepare_build_context::prepare_build_context` |
| 11.5 KiB |  0.1% |         66% | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunks::{closure#0}::{closure#0}` |
| 10.9 KiB |  0.1% |         53% | `rolldown::utils::chunk::deconflict_chunk_symbols::deconflict_chunk_symbols` |
| 10.3 KiB |  0.1% |         98% | `<&regress::insn::Insn as core::fmt::Debug>::fmt` |
| 10.0 KiB |  0.1% |         77% | `<&mut serde_json::de::Deserializer<serde_json::read::SliceRead> as serde_core::de::Deserializer>::deserialize_struct::<<oxc_resolver::tsconfig::CompilerOptions as serde_core::de::Deserialize>::deserialize::__Visitor>` |
|  9.8 KiB |  0.1% |         57% | `<rolldown_plugin::plugin_driver::PluginDriver>::resolve_id::{closure#0}::{closure#0}` |
|  9.8 KiB |  0.1% |         57% | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunk_name_and_preliminary_filenames::{closure#0}::{closure#0}` |
|  9.3 KiB |  0.1% |         75% | `<rolldown_watcher::watch_coordinator::WatchCoordinator<rolldown_binding::watcher::NapiWatcherEventHandler>>::run::{closure#0}` |
|  9.1 KiB |  0.1% |         99% | `std::backtrace_rs::symbolize::gimli::resolve` |
|  8.9 KiB |  0.1% |         70% | `<rolldown::module_loader::module_task::ModuleTask<rolldown_fs::os::OsFileSystem>>::load_source::{closure#0}::{closure#0}` |
|  8.9 KiB |  0.1% |         69% | `regress::unicodetables::unicode_property_value_script_from_str` |

### By the crate the inlined code came from

_the footprint of code that vanished into its callers, credited to the defining crate — the by-crate table above counts only surviving symbols_

|      Size | Share | Crate | Instances |
|----------:|------:|---|----------:|
|   3.0 MiB | 18.8% | `core` |    513888 |
| 731.5 KiB |  4.5% | `alloc` |    185269 |
| 321.6 KiB |  2.0% | `oxc_ast_visit` |     21924 |
| 145.3 KiB |  0.9% | `rolldown` |     10402 |
| 132.8 KiB |  0.8% | `mimalloc_safe` |     14263 |
| 127.4 KiB |  0.8% | `hashbrown` |     22478 |
| 120.8 KiB |  0.7% | `napi` |      3240 |
| 113.2 KiB |  0.7% | `rustc_hash` |      8472 |
| 109.8 KiB |  0.7% | `oxc_allocator` |     37194 |
|  94.3 KiB |  0.6% | `rolldown_binding` |      1229 |
|  92.8 KiB |  0.6% | `std` |     13837 |
|  79.4 KiB |  0.5% | `oxc_ast` |      3485 |
|  66.0 KiB |  0.4% | `serde_json` |      4090 |
|  59.3 KiB |  0.4% | `regress` |      5370 |
|  56.9 KiB |  0.3% | `oxc_index` |      8597 |
|  42.1 KiB |  0.3% | `oxc_transformer` |      9439 |
|  40.9 KiB |  0.3% | `oxc_traverse` |      2389 |
|  37.2 KiB |  0.2% | `tokio` |      3288 |
|  35.0 KiB |  0.2% | `rolldown_plugin` |       272 |
|  32.6 KiB |  0.2% | `napi_sys` |      1250 |

### Workspace lines that pulled in the most inlined code

|      Size | Share | Line | Inlined | Source |
|----------:|------:|---|--------:|---|
| 132.8 KiB |  0.8% | `crates/rolldown_binding/src/lib.rs:36` |   14263 | `static ALLOC: mimalloc_safe::MiMalloc = mimalloc_safe::MiMalloc;` |
|  20.0 KiB |  0.1% | `crates/rolldown_binding/src/types/binding_magic_string.rs:443` |     567 | `#[napi]` |
|  18.7 KiB |  0.1% | `crates/rolldown_binding/src/types/binding_normalized_options.rs:21` |     497 | `#[napi]` |
|   9.0 KiB |  0.1% | `crates/rolldown_binding/src/types/js_callback.rs:188` |      80 | `match self.call_async_catch(args).await? {` |
|   8.1 KiB | <0.1% | `crates/rolldown_binding/src/types/js_callback.rs:121` |      94 | `match self.call_async_catch(args).await? {` |
|   6.9 KiB | <0.1% | `crates/rolldown_binding/src/types/binding_output_chunk.rs:17` |     203 | `#[napi]` |
|   6.1 KiB | <0.1% | `crates/rolldown_binding/src/options/plugin/binding_plugin_context.rs:24` |     190 | `#[napi]` |
|   5.8 KiB | <0.1% | `crates/rolldown_binding/src/binding_dev_engine.rs:31` |     195 | `#[napi]` |
|   4.6 KiB | <0.1% | `crates/rolldown_binding/src/options/binding_output_options/mod.rs:38` |     213 | `#[napi_derive::napi(object, object_to_js = false)]` |
|   4.4 KiB | <0.1% | `crates/rolldown_binding/src/options/plugin/binding_plugin_options.rs:39` |     251 | `#[napi_derive::napi(object, object_to_js = false)]` |
|   4.3 KiB | <0.1% | `crates/rolldown/src/stages/link_stage/cross_module_optimization.rs:362` |     276 | `self.visit_path.pop();` |
|   4.2 KiB | <0.1% | `crates/rolldown_binding/src/utils/normalize_binding_options.rs:750` |     244 | `};` |
|   4.1 KiB | <0.1% | `crates/rolldown_binding/src/types/binding_rendered_chunk.rs:13` |     124 | `#[napi_derive::napi]` |
|   3.9 KiB | <0.1% | `crates/rolldown/src/esm_init_obligations.rs:492` |     150 | `targets.sort_by_key(\|target\| consumer_local_target_order(ctx, importee_idx, *target));` |
|   3.9 KiB | <0.1% | `crates/rolldown_binding/src/types/binding_module_info.rs:8` |     165 | `#[napi]` |
|   3.8 KiB | <0.1% | `crates/rolldown_plugin/src/plugin_context/native_plugin_context.rs:130` |       2 | `.await` |
|   3.7 KiB | <0.1% | `crates/rolldown/src/chunk_graph.rs:154` |     594 | `.sort_unstable_by_key(\|idx\| link_output.module_table[*idx].id().as_str());` |
|   3.7 KiB | <0.1% | `crates/rolldown_binding/src/types/binding_magic_string.rs:267` |     114 | `#[napi]` |
|   3.6 KiB | <0.1% | `crates/rolldown/src/module_loader/runtime_module_task.rs:63` |       1 | `if let Err(errs) = self.run_inner().await {` |
|   3.6 KiB | <0.1% | `crates/rolldown/src/module_loader/module_task.rs:81` |       1 | `if let Err(errs) = self.run_inner().await {` |

## Assembly

- 2.6 MiB (16.5%) of code in 5467 functions with assembly, 690835 instructions, 4.0 B each; from `/Users/boshen/github/rolldown/rolldown/target/bsize/release/deps/rolldown_binding.s`.
- 302 more functions in the assembly never reached the binary.
- `~` sizes below are instruction counts converted at that rate.

### Identical function bodies

_the same instructions under different names; a linker folding identical code keeps one, and so does instantiating one — 24.5 KiB recoverable from 153 groups (365 functions)_

| Recoverable | Share | Functions | Copies |    Each |
|------------:|------:|---|-------:|--------:|
|     1.5 KiB | <0.1% | `napi::js_values::deferred::napi_resolve_deferred::<napi::js_values::unknown::Unknown, napi::tokio_runtime::execute_future_impl<(), <rolldown_binding::options::plugin::binding_callable_builtin_plugin::BindingCallableBuiltinPlugin>::watch_change::{closure#0}, <napi::tokio_runtime::AsyncBlockBuilder<(), <rolldown_binding::options::plugin::binding_callable_builtin_plugin::BindingCallableBuiltinPlugin>::watch_change::{closure#0}, <rolldown_binding::options::plugin::binding_callable_builtin_plugin::BindingCallableBuiltinPlugin>::watch_change::{closure#1}>>::build::{closure#0}, napi::error::Error>::{closure#0}::{closure#0}>` ≡ `napi::js_values::deferred::napi_resolve_deferred::<napi::js_values::unknown::Unknown, napi::tokio_runtime::execute_future_impl<(), core::pin::Pin<alloc::boxed::Box<dyn core::future::future::Future<Output = core::result::Result<(), napi::error::Error>> + core::marker::Send>>, <napi::env::Env>::spawn_future<(), core::pin::Pin<alloc::boxed::Box<dyn core::future::future::Future<Output = core::result::Result<(), napi::error::Error>> + core::marker::Send>>>::{closure#0}, napi::error::Error>::{closure#0}::{closure#0}>` |      2 | 1.5 KiB |
|     1.2 KiB | <0.1% | `<hashbrown::raw::RawTable<usize>>::reserve_rehash::<indexmap::inner::get_hash<alloc::string::String, rolldown_binding::types::binding_rendered_chunk::BindingRenderedChunk>::{closure#0}>` ≡ `<hashbrown::raw::RawTable<usize>>::reserve_rehash::<indexmap::inner::get_hash<rolldown_common::types::symbol_ref::SymbolRef, alloc::vec::Vec<oxc_str::compact_str::CompactStr>>::{closure#0}>` |      2 | 1.2 KiB |
|       880 B | <0.1% | `core::ptr::drop_glue::<hashbrown::raw::RawTable<(rolldown_common::ecmascript::module_idx::ModuleIdx, alloc::vec::Vec<(rolldown_common::types::import_record::ImportRecordIdx, rolldown_common::ecmascript::module_idx::ModuleIdx)>)>>` ≡ `core::ptr::drop_glue::<std::collections::hash::map::HashMap<oxc_syntax::symbol::SymbolId, alloc::vec::Vec<oxc_syntax::scope::ScopeId>, rustc_hash::FxBuildHasher>>` ≡ `core::ptr::drop_glue::<std::collections::hash::map::HashMap<oxc_transformer::common::helper_loader::Helper, alloc::string::String, rustc_hash::FxBuildHasher>>` ≡ 2 more |      5 |   220 B |
|       680 B | <0.1% | `core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<alloc::string::String>>` ≡ `core::ptr::drop_glue::<alloc::vec::into_iter::IntoIter<std::path::PathBuf>>` ≡ `core::ptr::drop_glue::<core::iter::adapters::enumerate::Enumerate<alloc::vec::into_iter::IntoIter<alloc::string::String>>>` ≡ 3 more |      6 |   136 B |
|       560 B | <0.1% | `core::ptr::drop_glue::<alloc::vec::Vec<alloc::string::String>>` ≡ `core::ptr::drop_glue::<alloc::vec::Vec<alloc::vec::Vec<rolldown_common::types::ins_chunk_idx::InsChunkIdx>>>` ≡ `core::ptr::drop_glue::<alloc::vec::Vec<oxc_resolver::tsconfig::ProjectReference>>` ≡ 3 more |      6 |   112 B |
|       456 B | <0.1% | `core::ptr::drop_glue::<indexmap::map::IndexMap<(rolldown_common::ecmascript::module_idx::ModuleIdx, rolldown_common::types::import_record::ImportRecordIdx), alloc::string::String, core::hash::BuildHasherDefault<rustc_hash::FxHasher>>>` ≡ `core::ptr::drop_glue::<indexmap::map::IndexMap<alloc::string::String, rolldown_common::types::watch::WatcherChangeKind, core::hash::BuildHasherDefault<rustc_hash::FxHasher>>>` ≡ `core::ptr::drop_glue::<indexmap::map::IndexMap<rolldown_common::types::chunk_idx::ChunkIdx, alloc::vec::Vec<rolldown_common::chunk::types::cross_chunk_import_item::CrossChunkImportItem>, core::hash::BuildHasherDefault<rustc_hash::FxHasher>>>` ≡ 1 more |      4 |   152 B |
|       440 B | <0.1% | `core::ptr::drop_glue::<hashbrown::raw::RawTable<(alloc::string::String, u32)>>` ≡ `core::ptr::drop_glue::<std::collections::hash::map::HashMap<alloc::string::String, u32, rustc_hash::FxBuildHasher>>` ≡ `core::ptr::drop_glue::<std::collections::hash::map::HashMap<rolldown_utils::bitset::BitSet, rolldown_common::types::chunk_idx::ChunkIdx, rustc_hash::FxBuildHasher>>` |      3 |   220 B |
|       408 B | <0.1% | `core::ptr::drop_glue::<core::iter::adapters::map::Map<std::collections::hash::map::IntoIter<oxc_transformer::common::helper_loader::Helper, alloc::string::String>, <rolldown_binding::options::binding_transform_options::BindingEnhancedTransformResult>::from_enhanced_transform_result::{closure#0}>>` ≡ `core::ptr::drop_glue::<hashbrown::raw::RawIntoIter<(oxc_transformer::common::helper_loader::Helper, alloc::string::String)>>` ≡ `core::ptr::drop_glue::<std::collections::hash::map::IntoIter<rolldown_common::ecmascript::module_idx::ModuleIdx, alloc::vec::Vec<(rolldown_common::ecmascript::module_idx::ModuleIdx, rolldown_common::types::stmt_info::StmtInfoIdx, oxc_syntax::node::NodeId, rolldown_common::types::import_record::ImportRecordIdx)>>>` |      3 |   204 B |
|       408 B | <0.1% | `core::ptr::drop_glue::<indexmap::Bucket<alloc::string::String, alloc::string::String>>` ≡ `core::ptr::drop_glue::<napi::bindgen_runtime::js_values::function::FnArgs<(alloc::string::String, alloc::string::String)>>` ≡ `core::ptr::drop_glue::<oxc_resolver::error::JSONError>` ≡ 4 more |      7 |    68 B |
|       348 B | <0.1% | `core::ptr::drop_glue::<tokio::loom::std::mutex::Mutex<tokio::sync::broadcast::Slot<()>>>` ≡ `core::ptr::drop_glue::<tokio::loom::std::mutex::Mutex<tokio::sync::broadcast::Tail>>` ≡ `core::ptr::drop_glue::<tokio::sync::notify::Notify>` ≡ 1 more |      4 |   116 B |
|       344 B | <0.1% | `core::ptr::drop_glue::<alloc::vec::Vec<rolldown::hmr::hmr_stage::ModuleRenderInput>>` ≡ `core::ptr::drop_glue::<rayon::iter::collect::special_extend<rayon::iter::map::Map<rayon::vec::IntoIter<rolldown::hmr::hmr_stage::ModuleRenderInput>, <rolldown::hmr::hmr_stage::HmrStage<rolldown_fs::os::OsFileSystem>>::compute_hmr_update_for_file_changes::{closure#0}::{closure#7}>, (rolldown_common::ecmascript::module_idx::ModuleIdx, alloc::string::String)>::{closure#0}>` ≡ `core::ptr::drop_glue::<rayon::vec::IntoIter<rolldown::hmr::hmr_stage::ModuleRenderInput>>` |      3 |   172 B |
|       336 B | <0.1% | `<alloc::borrow::Cow<str> as alloc::string::ToString>::to_string` ≡ `<std::path::PathBuf as core::clone::Clone>::clone` ≡ `<std::path::PathBuf as core::convert::From<&str>>::from` ≡ 1 more |      4 |   112 B |
|       336 B | <0.1% | `core::ptr::drop_glue::<alloc::vec::Vec<regress::types::BracketContents>>` ≡ `core::ptr::drop_glue::<alloc::vec::Vec<rolldown_common::chunk::types::module_group::ModuleGroup>>` ≡ `core::ptr::drop_glue::<alloc::vec::Vec<rolldown_devtools_action::types::PluginItem>>` ≡ 1 more |      4 |   112 B |
|       332 B | <0.1% | `<hashbrown::raw::RawTable<(rolldown::stages::generate_stage::manual_code_splitting::ModuleGroupId, rolldown::stages::generate_stage::manual_code_splitting::ModuleGroupIdx)>>::reserve_rehash::<hashbrown::map::make_hasher<rolldown::stages::generate_stage::manual_code_splitting::ModuleGroupId, rolldown::stages::generate_stage::manual_code_splitting::ModuleGroupIdx, rustc_hash::FxBuildHasher>::{closure#0}>::{closure#0}` ≡ `<hashbrown::raw::RawTable<(rolldown::stages::generate_stage::manual_code_splitting::ModuleGroupId, usize)>>::reserve_rehash::<hashbrown::map::make_hasher<rolldown::stages::generate_stage::manual_code_splitting::ModuleGroupId, usize, rustc_hash::FxBuildHasher>::{closure#0}>::{closure#0}` |      2 |   332 B |
|       332 B | <0.1% | `<rolldown_common::types::hybrid_index_vec::HybridIndexVec<rolldown_common::ecmascript::module_idx::ModuleIdx, core::option::Option<rolldown_common::module::Module>>>::get_mut` ≡ `<rolldown_common::types::hybrid_index_vec::HybridIndexVec<rolldown_common::ecmascript::module_idx::ModuleIdx, core::option::Option<rolldown_ecmascript::ecma_ast::EcmaAst>>>::get_mut` |      2 |   332 B |
|       328 B | <0.1% | `core::ptr::drop_glue::<<rolldown_plugin_vite_reporter::ViteReporterPlugin as core::convert::From<rolldown_binding::options::plugin::config::binding_vite_reporter_plugin_config::BindingViteReporterPluginConfig>>::from::{closure#0}::{closure#0}::{closure#0}>` ≡ `core::ptr::drop_glue::<rolldown_binding::utils::normalize_binding_options::normalize_code_splitting::{closure#0}::{closure#0}::{closure#0}::{closure#1}::{closure#1}::{closure#0}>` |      2 |   328 B |
|       308 B | <0.1% | `<napi::bindgen_runtime::callback_info::CallbackInfo<0>>::unwrap_raw::<rolldown_binding::types::binding_magic_string::BindingMagicString>` ≡ `napi::bindgen_runtime::class_accessor::class_accessor_unwrap_this::<rolldown_binding::types::binding_magic_string::BindingMagicString>` |      2 |   308 B |
|       308 B | <0.1% | `<napi::bindgen_runtime::callback_info::CallbackInfo<0>>::unwrap_raw::<rolldown_binding::types::binding_magic_string::BindingSourceMap>` ≡ `napi::bindgen_runtime::class_accessor::class_accessor_unwrap_this::<rolldown_binding::types::binding_magic_string::BindingSourceMap>` |      2 |   308 B |
|       304 B | <0.1% | `<hashbrown::raw::RawTable<(alloc::string::String, rolldown_dev::types::client_session::ClientSession)>>::reserve_rehash::<hashbrown::map::make_hasher<alloc::string::String, rolldown_dev::types::client_session::ClientSession, rustc_hash::FxBuildHasher>::{closure#0}>::{closure#0}` ≡ `<hashbrown::raw::RawTable<(alloc::string::String, rolldown_dev::types::pending_payload::PendingPayload)>>::reserve_rehash::<hashbrown::map::make_hasher<alloc::string::String, rolldown_dev::types::pending_payload::PendingPayload, rustc_hash::FxBuildHasher>::{closure#0}>::{closure#0}` |      2 |   304 B |
|       296 B | <0.1% | `core::ptr::drop_glue::<std::collections::hash::map::HashMap<oxc_syntax::symbol::SymbolId, rolldown_common::types::constant_value::ConstExportMeta, rustc_hash::FxBuildHasher>>` ≡ `core::ptr::drop_glue::<std::collections::hash::map::HashMap<rolldown_common::types::symbol_ref::SymbolRef, rolldown_common::types::constant_value::ConstExportMeta, rustc_hash::FxBuildHasher>>` |      2 |   296 B |

### Panic call sites

_each is a compare, a branch, and a cold block that loads the location and calls; the location is 24 B more of read-only data_

- ~128.1 KiB (0.8%) in the blocks of 14095 sites: 336 bounds checks, 749 unwraps, 3115 allocation failures, 9895 other; 926 distinct locations and messages loaded by them.

|    ~Size | Share | Function | Sites | Instructions |
|---------:|------:|---|------:|-------------:|
| ~2.4 KiB | <0.1% | `rolldown_binding::types::binding_normalized_options::__napi_impl_helper_BindingNormalizedOptions_16::__napi_register__BindingNormalizedOptions_impl_285::_::__CTOR_PRIVATE_REF::__ctor_private` |    84 |          606 |
| ~2.3 KiB | <0.1% | `rolldown_binding::types::binding_magic_string::__napi_impl_helper_BindingMagicString_14::__napi_register__BindingMagicString_impl_240::_::__CTOR_PRIVATE_REF::__ctor_private` |    82 |          592 |
| ~1.3 KiB | <0.1% | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}::{closure#0}` |   173 |          338 |
| ~1.0 KiB | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunk_name_and_preliminary_filenames::{closure#0}::{closure#0}` |    81 |          267 |
|  ~1008 B | <0.1% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` |    99 |          252 |
|   ~900 B | <0.1% | `rolldown::utils::chunk::finalize_chunks::finalize_assets::{closure#0}::{closure#0}` |    64 |          225 |
|   ~868 B | <0.1% | `rolldown_binding::types::binding_output_chunk::__napi_impl_helper_BindingOutputChunk_18::__napi_register__BindingOutputChunk_impl_312::_::__CTOR_PRIVATE_REF::__ctor_private` |    35 |          217 |
|   ~852 B | <0.1% | `<rolldown::hmr::hmr_stage::HmrStage<rolldown_fs::os::OsFileSystem>>::compute_hmr_update_for_file_changes::{closure#0}` |    90 |          213 |
|   ~832 B | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunks::{closure#0}::{closure#0}` |    58 |          208 |
|   ~768 B | <0.1% | `<rolldown::stages::scan_stage::ScanStage<rolldown_fs::os::OsFileSystem>>::scan::{closure#0}::{closure#0}` |    78 |          192 |
|   ~756 B | <0.1% | `rolldown_binding::binding_dev_engine::__napi_impl_helper_BindingDevEngine_1::__napi_register__BindingDevEngine_impl_23::_::__CTOR_PRIVATE_REF::__ctor_private` |    31 |          189 |
|   ~704 B | <0.1% | `<rolldown_watcher::watch_coordinator::WatchCoordinator<rolldown_binding::watcher::NapiWatcherEventHandler>>::run::{closure#0}` |    59 |          176 |
|   ~668 B | <0.1% | `rolldown_utils::futures::block_on::<rolldown_utils::futures::block_on_spawn_all<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<alloc::string::String>, <rolldown_plugin_vite_dynamic_import_vars::ViteDynamicImportVarsPlugin as rolldown_plugin::plugin::Plugin>::transform::{closure#0}::{closure#0}>, core::option::Option<alloc::string::String>>::{closure#0}>::{closure#0}` |    50 |          167 |
|   ~660 B | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::instantiate_chunks::{closure#0}::{closure#0}` |    68 |          165 |
|   ~588 B | <0.1% | `rolldown_binding::types::binding_rendered_chunk::__napi_impl_helper_BindingRenderedChunk_19::__napi_register__BindingRenderedChunk_impl_330::_::__CTOR_PRIVATE_REF::__ctor_private` |    25 |          147 |
|   ~576 B | <0.1% | `rolldown::module_loader::resolve_utils::resolve_dependencies::<rolldown_fs::os::OsFileSystem>::{closure#0}` |    60 |          144 |
|   ~560 B | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}` |    70 |          140 |
|   ~532 B | <0.1% | `rolldown_binding::options::plugin::binding_plugin_context::__napi_impl_helper_BindingPluginContext_7::__napi_register__BindingPluginContext_impl_125::_::__CTOR_PRIVATE_REF::__ctor_private` |    23 |          133 |
|   ~532 B | <0.1% | `rolldown_binding::types::binding_magic_string::__napi_impl_helper_BindingSourceMap_12::__napi_register__BindingSourceMap_impl_190::_::__CTOR_PRIVATE_REF::__ctor_private` |    23 |          133 |
|   ~520 B | <0.1% | `<rolldown::bundle::bundle::Bundle>::scan_modules::{closure#0}::{closure#0}` |    31 |          130 |

### Formatting call sites

_calls into core::fmt and alloc::fmt; the block before each builds the Arguments_

- ~43.8 KiB (0.3%) in the blocks of 1036 sites.

|    ~Size | Share | Function | Sites | Instructions |
|---------:|------:|---|------:|-------------:|
| ~1.1 KiB | <0.1% | `<rolldown_plugin_vite_reporter::ViteReporterPlugin as rolldown_plugin::plugin::Plugin>::write_bundle::{closure#0}` |    18 |          274 |
|   ~844 B | <0.1% | `<rolldown_plugin_vite_resolve::vite_resolve_plugin::ViteResolvePlugin as rolldown_plugin::plugin::Plugin>::resolve_id::{closure#0}` |    17 |          211 |
|   ~672 B | <0.1% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` |    11 |          168 |
|   ~516 B | <0.1% | `<owo_colors::dyn_styles::Style>::fmt_prefix` |    22 |          129 |
|   ~500 B | <0.1% | `<oxc_resolver::error::ResolveError as core::fmt::Debug>::fmt` |     9 |          125 |
|   ~428 B | <0.1% | `<napi::status::Status as core::fmt::Debug>::fmt` |    22 |          107 |
|   ~428 B | <0.1% | `<&[u8; 16] as core::fmt::Debug>::fmt` |    18 |          107 |
|   ~404 B | <0.1% | `<&[u8; 15] as core::fmt::Debug>::fmt` |    17 |          101 |
|   ~400 B | <0.1% | `<rolldown_binding::options::binding_output_options::binding_manual_code_splitting_options::BindingMatchGroup as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` |     6 |          100 |
|   ~380 B | <0.1% | `<&[u8; 14] as core::fmt::Debug>::fmt` |    16 |           95 |
|   ~376 B | <0.1% | `<core::iter::adapters::filter_map::FilterMap<core::slice::iter::Iter<rolldown_common::types::import_record::ImportRecord<rolldown_common::types::import_record::ImportRecordStateResolved>>, <rolldown::bundle::bundle::Bundle>::trace_action_module_graph_ready::{closure#0}::{closure#0}> as core::iter::traits::iterator::Iterator>::next` |     6 |           94 |
|   ~356 B | <0.1% | `<&[u8; 13] as core::fmt::Debug>::fmt` |    15 |           89 |
|   ~332 B | <0.1% | `<&[u8; 12] as core::fmt::Debug>::fmt` |    14 |           83 |
|   ~308 B | <0.1% | `<&[u8; 11] as core::fmt::Debug>::fmt` |    13 |           77 |
|   ~304 B | <0.1% | `<rolldown::bundle::bundle::Bundle>::bundle_write::{closure#0}::{closure#0}` |     6 |           76 |
|   ~304 B | <0.1% | `<rolldown_binding::options::binding_input_options::binding_treeshake::BindingTreeshake as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` |     5 |           76 |
|   ~304 B | <0.1% | `<rolldown::bundle::bundle::Bundle>::scan_modules::{closure#0}::{closure#0}` |     4 |           76 |
|   ~284 B | <0.1% | `<&[u8; 10] as core::fmt::Debug>::fmt` |    12 |           71 |
|   ~260 B | <0.1% | `<&[u8; 9] as core::fmt::Debug>::fmt` |    11 |           65 |
|   ~252 B | <0.1% | `napi::bindgen_runtime::js_values::object::from_raw_required_field::<alloc::vec::Vec<core::option::Option<napi::bindgen_runtime::js_values::either::Either<rolldown_binding::options::plugin::binding_plugin_options::BindingPluginOptions, rolldown_binding::options::plugin::binding_builtin_plugin::BindingBuiltinPlugin>>>>` |     4 |           63 |

### Values copied through memory

_runs of 8 or more loads and stores back to back, and calls to memcpy for anything larger; boxing the value or passing it by reference removes them_

- ~146.3 KiB (0.9%) in 2921 runs, 37449 instructions, plus 1911 memcpy-family calls.

|    ~Size | Share | Function | Instructions | Runs | Calls |
|---------:|------:|---|-------------:|-----:|------:|
| ~4.1 KiB | <0.1% | `rolldown_binding::utils::normalize_binding_options::normalize_binding_options` |         1046 |   80 |     8 |
| ~3.2 KiB | <0.1% | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}::{closure#0}` |          807 |   67 |    37 |
| ~3.1 KiB | <0.1% | `rolldown_binding::types::binding_normalized_options::__napi_impl_helper_BindingNormalizedOptions_16::__napi_register__BindingNormalizedOptions_impl_285::_::__CTOR_PRIVATE_REF::__ctor_private` |          797 |   49 |     0 |
| ~2.9 KiB | <0.1% | `rolldown_binding::types::binding_magic_string::__napi_impl_helper_BindingMagicString_14::__napi_register__BindingMagicString_impl_240::_::__CTOR_PRIVATE_REF::__ctor_private` |          755 |   52 |     0 |
| ~2.3 KiB | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}` |          578 |   34 |    23 |
| ~2.1 KiB | <0.1% | `rolldown::ecmascript::ecma_module_view_factory::create_ecma_view::{closure#0}` |          544 |   34 |    17 |
| ~2.0 KiB | <0.1% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` |          522 |   36 |    14 |
| ~1.7 KiB | <0.1% | `<rolldown_binding::options::plugin::js_plugin::JsPlugin as rolldown_plugin::plugin::Plugin>::load::{closure#0}` |          446 |   19 |     6 |
| ~1.7 KiB | <0.1% | `<rolldown::hmr::hmr_stage::HmrStage<rolldown_fs::os::OsFileSystem>>::compute_hmr_update_for_file_changes::{closure#0}` |          436 |   31 |    20 |
| ~1.4 KiB | <0.1% | `rolldown::module_loader::resolve_utils::resolve_dependencies::<rolldown_fs::os::OsFileSystem>::{closure#0}` |          348 |   28 |    14 |
| ~1.3 KiB | <0.1% | `core::slice::sort::stable::quicksort::quicksort::<rolldown_common::types::sourcemap_chain_element::SourcemapChainElement, <[rolldown_common::types::sourcemap_chain_element::SourcemapChainElement]>::sort_by<<rolldown::stages::scan_stage::ScanStage<rolldown_fs::os::OsFileSystem>>::process_sourcemap_handler::{closure#1}>::{closure#0}>` |          337 |   18 |     3 |
| ~1.2 KiB | <0.1% | `napi::threadsafe_function::call_js_cb::<napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_load_context::BindingLoadPluginContext, alloc::string::String)>, napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_load_output::BindingHookLoadOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_load_output::BindingHookLoadOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_load_context::BindingLoadPluginContext, alloc::string::String)>, napi::status::Status, <napi::threadsafe_function::ThreadsafeFunction<napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_load_context::BindingLoadPluginContext, alloc::string::String)>, napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_load_output::BindingHookLoadOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_load_output::BindingHookLoadOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_load_context::BindingLoadPluginContext, alloc::string::String)>, napi::status::Status, false, true> as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value::{closure#0}, false>` |          301 |   14 |     4 |
| ~1.2 KiB | <0.1% | `rolldown_binding::options::plugin::types::binding_filter_expression::normalized_tokens` |          295 |   26 |     0 |
| ~1.1 KiB | <0.1% | `rolldown::utils::render_chunks::render_chunks::{closure#0}::{closure#0}` |          294 |   24 |     6 |
| ~1.1 KiB | <0.1% | `<rolldown_binding::options::plugin::js_plugin::JsPlugin as rolldown_plugin::plugin::Plugin>::transform::{closure#0}` |          283 |   20 |     6 |
| ~1.1 KiB | <0.1% | `<rolldown_watcher::watch_coordinator::WatchCoordinator<rolldown_binding::watcher::NapiWatcherEventHandler>>::run::{closure#0}` |          274 |   22 |     3 |
| ~1.1 KiB | <0.1% | `napi::threadsafe_function::call_js_cb::<napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_transform_context::BindingTransformPluginContext, rolldown_binding::options::plugin::types::binding_shared_string::BindingSharedString, alloc::string::String, rolldown_binding::options::plugin::types::binding_plugin_transform_extra_args::BindingTransformHookExtraArgs)>, napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_transform_output::BindingHookTransformOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_transform_output::BindingHookTransformOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_transform_context::BindingTransformPluginContext, rolldown_binding::options::plugin::types::binding_shared_string::BindingSharedString, alloc::string::String, rolldown_binding::options::plugin::types::binding_plugin_transform_extra_args::BindingTransformHookExtraArgs)>, napi::status::Status, <napi::threadsafe_function::ThreadsafeFunction<napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_transform_context::BindingTransformPluginContext, rolldown_binding::options::plugin::types::binding_shared_string::BindingSharedString, alloc::string::String, rolldown_binding::options::plugin::types::binding_plugin_transform_extra_args::BindingTransformHookExtraArgs)>, napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::either::Either<napi::bindgen_runtime::js_values::promise::Promise<core::option::Option<rolldown_binding::options::plugin::types::binding_hook_transform_output::BindingHookTransformOutput>>, core::option::Option<rolldown_binding::options::plugin::types::binding_hook_transform_output::BindingHookTransformOutput>>, rolldown_binding::types::js_callback::InvalidReturnValue>, napi::bindgen_runtime::js_values::function::FnArgs<(rolldown_binding::options::plugin::binding_transform_context::BindingTransformPluginContext, rolldown_binding::options::plugin::types::binding_shared_string::BindingSharedString, alloc::string::String, rolldown_binding::options::plugin::types::binding_plugin_transform_extra_args::BindingTransformHookExtraArgs)>, napi::status::Status, false, true> as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value::{closure#0}, false>` |          271 |   13 |     4 |
| ~1.0 KiB | <0.1% | `<rolldown_binding::options::binding_input_options::BindingInputOptions as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` |          261 |   22 |     7 |
|  ~1012 B | <0.1% | `rolldown_binding::utils::collapse_sourcemaps::collapse_sourcemaps` |          253 |   15 |     0 |
|   ~988 B | <0.1% | `<rolldown_binding::options::plugin::binding_plugin_options::BindingPluginOptions as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` |          247 |   22 |     0 |

### Workspace lines compiled to the most instructions

_the line an instruction came from, after inlining, every instantiation summed_

|     ~Size | Share | Line | Instructions | Source |
|----------:|------:|---|-------------:|---|
| ~16.8 KiB |  0.1% | `crates/rolldown_binding/src/types/binding_magic_string.rs:443` |         4293 | `#[napi]` |
| ~10.9 KiB |  0.1% | `crates/rolldown_binding/src/types/binding_normalized_options.rs:21` |         2796 | `#[napi]` |
|  ~5.2 KiB | <0.1% | `crates/rolldown_binding/src/types/binding_output_chunk.rs:17` |         1335 | `#[napi]` |
|  ~5.2 KiB | <0.1% | `crates/rolldown_binding/src/binding_dev_engine.rs:31` |         1330 | `#[napi]` |
|  ~5.1 KiB | <0.1% | `crates/rolldown_binding/src/utils/normalize_binding_options.rs:750` |         1295 | `};` |
|  ~4.3 KiB | <0.1% | `crates/rolldown_binding/src/options/plugin/binding_plugin_context.rs:24` |         1099 | `#[napi]` |
|  ~3.7 KiB | <0.1% | `crates/rolldown_binding/src/options/binding_output_options/mod.rs:38` |          948 | `#[napi_derive::napi(object, object_to_js = false)]` |
|  ~3.4 KiB | <0.1% | `crates/rolldown_binding/src/types/binding_module_info.rs:8` |          865 | `#[napi]` |
|  ~2.8 KiB | <0.1% | `crates/rolldown_binding/src/types/js_callback.rs:187` |          721 | `async fn await_call(&self, args: Args) -> Result<Ret, napi::Error> {` |
|  ~2.8 KiB | <0.1% | `crates/rolldown_binding/src/options/plugin/binding_plugin_options.rs:39` |          707 | `#[napi_derive::napi(object, object_to_js = false)]` |
|  ~2.7 KiB | <0.1% | `crates/rolldown_binding/src/binding_bundler.rs:31` |          687 | `#[napi]` |
|  ~2.7 KiB | <0.1% | `crates/rolldown_binding/src/types/binding_rendered_chunk.rs:13` |          682 | `#[napi_derive::napi]` |
|  ~2.6 KiB | <0.1% | `crates/rolldown_binding/src/types/js_callback.rs:188` |          675 | `match self.call_async_catch(args).await? {` |
|  ~2.6 KiB | <0.1% | `crates/rolldown_binding/src/transform.rs:171` |          665 | `#[napi(object, object_from_js = false)]` |
|  ~2.5 KiB | <0.1% | `crates/rolldown_binding/src/options/plugin/binding_callable_builtin_plugin.rs:38` |          652 | `#[napi]` |
|  ~2.4 KiB | <0.1% | `crates/rolldown_binding/src/types/binding_magic_string.rs:267` |          612 | `#[napi]` |
|  ~2.3 KiB | <0.1% | `crates/rolldown_binding/src/types/binding_output_asset.rs:19` |          594 | `#[napi]` |
|  ~2.3 KiB | <0.1% | `crates/rolldown_binding/src/options/binding_input_options/mod.rs:38` |          586 | `#[napi_derive::napi(object, object_to_js = false)]` |
|  ~2.0 KiB | <0.1% | `crates/rolldown/src/ecmascript/ecma_module_view_factory.rs:163` |          520 | `}` |
|  ~2.0 KiB | <0.1% | `crates/rolldown_binding/src/utils/normalize_binding_options.rs:812` |          503 | `}` |

## Constant data

_every constant the assembly spells out, sized from its directives and read by shape; only what a linked function reaches counts, and under lto="fat" that is the whole program_

- 151.2 KiB (0.9%) in 4896 constants a linked function reaches (5403 defined), against 1.9 MiB of read-only data sections.

### By kind

|     Size | Share | Kind | Constants |
|---------:|------:|---|----------:|
| 40.3 KiB |  0.2% | vtables |       930 |
| 27.9 KiB |  0.2% | type, variant, and field names |      1802 |
| 20.8 KiB |  0.1% | panic locations (24 B records) |       888 |
| 17.6 KiB |  0.1% | source paths (what panic locations point at) |       228 |
| 17.1 KiB |  0.1% | messages and other text |       393 |
| 16.2 KiB |  0.1% | byte tables |       474 |
|  5.1 KiB | <0.1% | other pointer data |        66 |
|  1.8 KiB | <0.1% | lookup tables for `match` |        32 |
|  1.6 KiB | <0.1% | function pointer tables |        34 |
|  1.4 KiB | <0.1% | jump tables for `match` |        32 |
|    896 B | <0.1% | string slices and format pieces |        11 |
|    576 B | <0.1% | tables of slices and records |         6 |
|  5.7 KiB | <0.1% | of the text: loaded only on the way to a panic (messages and their pieces) |       126 |

### Panic locations

_a 24 B record per panic site — an unwrap, an index, an expect — plus the source path once: 888 records in all, 38.4 KiB_

|  Size | Share | Workspace file | Records | Most on lines |
|------:|------:|---|--------:|---|
| 709 B | <0.1% | `crates/rolldown/src/bundle/bundle.rs` |      28 | 43, 45, 47, 49, 53 |
| 729 B | <0.1% | `crates/rolldown_plugin/src/plugin_driver/output_hooks.rs` |      28 | 17, 22, 38, 46, 58 |
| 704 B | <0.1% | `crates/rolldown_plugin/src/plugin_driver/build_hooks.rs` |      27 | 45, 50, 80, 89, 110 |
| 641 B | <0.1% | `crates/rolldown_binding/src/options/plugin/parallel_js_plugin.rs` |      24 | 49, 53, 55, 68, 91 |
| 603 B | <0.1% | `crates/rolldown/src/module_loader/module_loader.rs` |      23 | 1163, 354, 366, 379, 384 |
| 608 B | <0.1% | `crates/rolldown_binding/src/options/plugin/js_plugin.rs` |      23 | 89, 109, 168, 202, 240 |
| 589 B | <0.1% | `crates/rolldown_plugin/src/plugin.rs` |      23 | 44, 56, 71, 83, 95 |
| 447 B | <0.1% | `crates/rolldown_binding/src/utils/normalize_binding_options.rs` |      16 | 77, 98, 115, 136, 156 |
| 432 B | <0.1% | `crates/rolldown_plugin_vite_reporter/src/lib.rs` |      16 | 245, 51, 58, 70, 82 |
| 377 B | <0.1% | `crates/rolldown/src/utils/load_source.rs` |      14 | 180, 191, 22, 66, 67 |
| 394 B | <0.1% | `crates/rolldown_binding/src/types/binding_magic_string.rs` |      14 | 168, 267, 323, 335, 443 |
| 385 B | <0.1% | `crates/rolldown_watcher/src/watch_coordinator.rs` |      14 | 50, 73, 75, 109, 153 |
| 326 B | <0.1% | `crates/rolldown_dev/src/dev_engine.rs` |      12 | 145, 155, 156, 157, 184 |
| 330 B | <0.1% | `crates/rolldown_watcher/src/watch_task.rs` |      12 | 71, 72, 106, 138, 190 |
| 301 B | <0.1% | `crates/rolldown/src/hmr/hmr_stage.rs` |      11 | 102, 346, 511, 610, 615 |
| 314 B | <0.1% | `crates/rolldown_binding/src/binding_dev_engine.rs` |      11 | 31, 177, 189, 204, 220 |
| 310 B | <0.1% | `crates/rolldown_dev/src/bundle_coordinator.rs` |      11 | 83, 162, 205, 268, 329 |
| 327 B | <0.1% | `crates/rolldown_plugin_vite_resolve/src/vite_resolve_plugin.rs` |      11 | 202, 212, 226, 283, 284 |
| 265 B | <0.1% | `crates/rolldown/src/ecmascript/ecma_generator.rs` |       9 | 48, 59, 106, 124, 212 |
| 267 B | <0.1% | `crates/rolldown/src/utils/chunk/finalize_chunks.rs` |       9 | 37, 46, 141, 210, 223 |

|  Size | Share | Function loading the most records | Records |
|------:|------:|---|--------:|
| 528 B | <0.1% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` |      22 |
| 504 B | <0.1% | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}::{closure#0}` |      21 |
| 504 B | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunk_name_and_preliminary_filenames::{closure#0}::{closure#0}` |      21 |
| 432 B | <0.1% | `<rolldown_watcher::watch_coordinator::WatchCoordinator<rolldown_binding::watcher::NapiWatcherEventHandler>>::run::{closure#0}` |      18 |
| 384 B | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate_chunks::{closure#0}::{closure#0}` |      16 |
| 360 B | <0.1% | `<rolldown::module_loader::module_task::ModuleTask<rolldown_fs::os::OsFileSystem>>::load_source::{closure#0}::{closure#0}` |      15 |
| 360 B | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::instantiate_chunks::{closure#0}::{closure#0}` |      15 |
| 360 B | <0.1% | `<rolldown::stages::scan_stage::ScanStage<rolldown_fs::os::OsFileSystem>>::scan::{closure#0}::{closure#0}` |      15 |
| 360 B | <0.1% | `rolldown::utils::chunk::finalize_chunks::finalize_assets::{closure#0}::{closure#0}` |      15 |
| 360 B | <0.1% | `rolldown_utils::futures::block_on::<rolldown_utils::futures::block_on_spawn_all<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<alloc::string::String>, <rolldown_plugin_vite_dynamic_import_vars::ViteDynamicImportVarsPlugin as rolldown_plugin::plugin::Plugin>::transform::{closure#0}::{closure#0}>, core::option::Option<alloc::string::String>>::{closure#0}>::{closure#0}` |      15 |
| 336 B | <0.1% | `<async_scoped::spawner::use_tokio::Tokio as async_scoped::spawner::Blocker>::block_on::<(), <async_scoped::scoped::Scope<_, _> as pin_project::__private::PinnedDrop>::drop::__drop_inner<core::option::Option<alloc::string::String>, async_scoped::spawner::use_tokio::Tokio>::{closure#0}>::{closure#0}` |      14 |
| 336 B | <0.1% | `<async_scoped::spawner::use_tokio::Tokio as async_scoped::spawner::Blocker>::block_on::<alloc::vec::Vec<core::result::Result<core::option::Option<alloc::string::String>, tokio::runtime::task::error::JoinError>>, <async_scoped::scoped::Scope<core::option::Option<alloc::string::String>, async_scoped::spawner::use_tokio::Tokio>>::collect::{closure#0}>::{closure#0}` |      14 |
| 336 B | <0.1% | `<rolldown_plugin_vite_reporter::ViteReporterPlugin as rolldown_plugin::plugin::Plugin>::write_bundle::{closure#0}` |      14 |
| 288 B | <0.1% | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::process_barrel_import_record` |      12 |
| 288 B | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}` |      12 |
| 264 B | <0.1% | `rolldown::module_loader::resolve_utils::resolve_dependencies::<rolldown_fs::os::OsFileSystem>::{closure#0}` |      11 |
| 264 B | <0.1% | `rolldown::utils::process_code_and_sourcemap::prepare_sourcemap::{closure#0}` |      11 |
| 240 B | <0.1% | `<futures_util::future::future::shared::Shared<core::pin::Pin<alloc::boxed::Box<dyn core::future::future::Future<Output = ()> + core::marker::Send>>> as core::future::future::Future>::poll` |      10 |
| 240 B | <0.1% | `<rolldown_plugin_vite_dynamic_import_vars::ViteDynamicImportVarsPlugin as rolldown_plugin::plugin::Plugin>::transform::{closure#0}` |      10 |
| 216 B | <0.1% | `<rolldown::bundle::bundle::Bundle>::scan_modules::{closure#0}::{closure#0}` |       9 |

### Largest strings

|  Size | Share | String | Kind | Loaded by |
|------:|------:|---|---|---|
| 223 B | <0.1% | `"The id starts with \"virtual:\", which by convention denotes a vir…"` | message | `rolldown::module_loader::resolve_utils::resolve_dependencies::<rolldown_fs::os::OsFileSystem>::{closure#0}` |
| 196 B | <0.1% | `` "This MagicString was already passed to `sendMagicString()`, whic…" `` | message | `<rolldown_binding::types::binding_magic_string::BindingMagicString>::trim_start` and 33 more |
| 193 B | <0.1% | `"Cannot start a runtime from within a runtime. This happens becau…"` | message | `rolldown_utils::futures::block_on::<rolldown_utils::futures::block_on_spawn_all<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<alloc::string::String>, <rolldown_plugin_vite_dynamic_import_vars::ViteDynamicImportVarsPlugin as rolldown_plugin::plugin::Plugin>::transform::{closure#0}::{closure#0}>, core::option::Option<alloc::string::String>>::{closure#0}>::{closure#0}` and 2 more |
| 185 B | <0.1% | `` "Encountered a module with type `asset`, but no plugin handled it…" `` | message | `<rolldown::module_loader::module_task::ModuleTask<rolldown_fs::os::OsFileSystem>>::load_source::{closure#0}::{closure#0}` |
| 182 B | <0.1% | `` "Encountered a module with type `copy`, but no plugin handled it.…" `` | message | `<rolldown::module_loader::module_task::ModuleTask<rolldown_fs::os::OsFileSystem>>::load_source::{closure#0}::{closure#0}` |
| 172 B | <0.1% | `` "The `PluginContext.load` only work at `resolveId/load/transform/…" `` | message | `<rolldown_plugin::plugin_context::native_plugin_context::NativePluginContextImpl>::load::{closure#0}` |
| 159 B | <0.1% | `` "Maybe you expected `resolve.alias` to call other plugins resolve…" `` | message | `rolldown::module_loader::resolve_utils::resolve_dependencies::<rolldown_fs::os::OsFileSystem>::{closure#0}` |
| 152 B | <0.1% | `` "Memory has been freed by `freeExternalMemory()`. Cannot access p…" `` | message | `<rolldown_binding::types::binding_output_chunk::BindingOutputChunk>::get_exports` and 19 more |
| 142 B | <0.1% | `"Bundling CSS is no longer supported (experimental support has be…"` | message | `<rolldown::module_loader::module_task::ModuleTask<rolldown_fs::os::OsFileSystem>>::run::{closure#0}::{closure#0}` |
| 141 B | <0.1% | `"TransformPluginContext: failed to send MagicString to sourcemap …"` | message | `<rolldown_binding::options::plugin::binding_transform_context::BindingTransformPluginContext>::send_magic_string` |
| 135 B | <0.1% | `` "Encountered a module with type `asset` in read_file_by_module_ty…" `` | message | `<rolldown::module_loader::module_task::ModuleTask<rolldown_fs::os::OsFileSystem>>::load_source::{closure#0}::{closure#0}` |
| 127 B | <0.1% | `"EventListener was not inserted into the linked list, make sure y…"` | message | `<event_listener::InnerListener<(), alloc::sync::Arc<event_listener::Inner<()>>>>::wait_with_parker` and 1 more |
| 113 B | <0.1% | `"devtools writer thread disconnected before acknowledging flush; …"` | message | `<rolldown_binding::binding_bundler::BindingBundler>::close::{closure#0}` |
| 113 B | <0.1% | `". See https://vite.dev/guide/troubleshooting.html#module-externa…"` | message | `<rolldown_plugin_vite_resolve::vite_resolve_plugin::ViteResolvePlugin as rolldown_plugin::plugin::Plugin>::resolve_id::{closure#0}` |
| 112 B | <0.1% | `"rolldown_binding::options::binding_output_options::binding_manua…"` | name | `napi::bindgen_runtime::js_values::class::new_instance::<rolldown_binding::options::binding_output_options::binding_manual_code_splitting_options::BindingChunkingContext>` |
| 111 B | <0.1% | `"The \"main\" field here was ignored. Main fields must be configure…"` | message | `rolldown::module_loader::resolve_utils::resolve_dependencies::<rolldown_fs::os::OsFileSystem>::{closure#0}` |
| 111 B | <0.1% | `"devtools writer did not acknowledge session flush within 30s; no…"` | message | `<rolldown_binding::binding_bundler::BindingBundler>::close::{closure#0}` |
| 109 B | <0.1% | `"Object.defineProperties(exports, { __esModule: { value: true }, …"` | message | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` |
| 106 B | <0.1% | `"ModuleLoader: failed to send external module build errors - main…"` | message | `<rolldown::module_loader::external_module_task::ExternalModuleTask<rolldown_fs::os::OsFileSystem>>::run::{closure#0}::{closure#0}` |
| 106 B | <0.1% | `"ModuleLoader channel closed while sending external module comple…"` | message | `<rolldown::module_loader::external_module_task::ExternalModuleTask<rolldown_fs::os::OsFileSystem>>::run::{closure#0}::{closure#0}` |

### Lookup and jump tables

_by the function whose match built them_

|  Size | Share | Function | Lookup | Jump |
|------:|------:|---|-------:|-----:|
| 464 B | <0.1% | `<rolldown_binding::options::binding_transform_options::BindingEnhancedTransformResult>::from_enhanced_transform_result` |      2 |    0 |
| 288 B | <0.1% | `<rolldown_binding::options::plugin::types::binding_builtin_plugin_name::BindingBuiltinPluginName as core::fmt::Debug>::fmt` |      2 |    0 |
| 192 B | <0.1% | `<rolldown_binding::options::plugin::types::binding_filter_expression::FilterTokenKind as core::fmt::Debug>::fmt` |      2 |    0 |
| 144 B | <0.1% | `<rolldown_plugin_isolated_declaration::type_import_visitor::TypeImportVisitor as oxc_ast_visit::generated::visit_js_mut::VisitJsMut>::visit_statement` |      0 |    1 |
| 144 B | <0.1% | `<rolldown_plugin_vite_dynamic_import_vars::ast_visit::DynamicImportVarsVisit as oxc_ast_visit::generated::visit_js::VisitJs>::visit_statement` |      0 |    1 |
| 144 B | <0.1% | `oxc_traverse::generated::walk::walk_statement::<(), rolldown::hmr::hmr_ast_finalizer::HmrAstFinalizer>` |      0 |    1 |
| 112 B | <0.1% | `<rolldown_common::types::module_def_format::ModuleDefFormat as core::fmt::Debug>::fmt` |      2 |    0 |
| 102 B | <0.1% | `<rolldown_plugin_isolated_declaration::type_import_visitor::TypeImportVisitor as oxc_ast_visit::generated::visit_js_mut::VisitJsMut>::visit_expression` |      0 |    1 |
| 102 B | <0.1% | `oxc_traverse::generated::walk::walk_expression::<(), rolldown::hmr::hmr_ast_finalizer::HmrAstFinalizer>` |      0 |    1 |
|  96 B | <0.1% | `<&core::num::error::IntErrorKind as core::fmt::Debug>::fmt` |      2 |    0 |
|  86 B | <0.1% | `<&regress::insn::Insn as core::fmt::Debug>::fmt` |      0 |    1 |
|  80 B | <0.1% | `rolldown_binding::types::binding_watcher_event::__napi_impl_helper_BindingWatcherEvent_21::bundle_event_kind_c_callback` |      2 |    0 |
|  76 B | <0.1% | `oxc_traverse::generated::walk::walk_ts_type::<(), rolldown::hmr::hmr_ast_finalizer::HmrAstFinalizer>` |      0 |    1 |
|  64 B | <0.1% | `rolldown_binding::types::binding_normalized_options::__napi_impl_helper_BindingNormalizedOptions_16::exports_c_callback` |      2 |    0 |
|  64 B | <0.1% | `rolldown_binding::types::binding_normalized_options::__napi_impl_helper_BindingNormalizedOptions_16::format_c_callback` |      2 |    0 |
|  64 B | <0.1% | `rolldown_binding::types::binding_watcher_event::__napi_impl_helper_BindingWatcherEvent_21::event_kind_c_callback` |      2 |    0 |
|  56 B | <0.1% | `<rolldown::module_loader::module_task::ModuleTask<rolldown_fs::os::OsFileSystem>>::load_source::{closure#0}::{closure#0}` |      0 |    2 |
|  56 B | <0.1% | `rolldown_plugin::utils::resolve_id_with_plugins::resolve_id::<rolldown_fs::os::OsFileSystem>` |      1 |    0 |
|  52 B | <0.1% | `<rolldown_watcher::watch_coordinator::WatchCoordinator<rolldown_binding::watcher::NapiWatcherEventHandler>>::run::{closure#0}` |      0 |    2 |
|  48 B | <0.1% | `<&rolldown_common::types::side_effects::HookSideEffects as core::fmt::Debug>::fmt` |      2 |    0 |

### Functions carrying the most constant data

_ranked by what only that function reaches, directly or through a table's pointers — what rewriting it alone frees; the total counts shared constants too_

| Exclusive | Share | Function | Constants |   Total |
|----------:|------:|---|----------:|--------:|
|   7.7 KiB | <0.1% | `<alloc::sync::Arc<dyn rolldown_plugin::pluginable::Pluginable> as core::convert::TryFrom<rolldown_binding::options::plugin::binding_builtin_plugin::BindingBuiltinPlugin>>::try_from` |        32 | 7.7 KiB |
|   1.4 KiB | <0.1% | `<rolldown::ecmascript::ecma_generator::EcmaGenerator as rolldown::types::generator::Generator>::instantiate_chunk::{closure#0}` |        70 | 2.1 KiB |
|   1.4 KiB | <0.1% | `<rolldown_plugin_vite_module_preload_polyfill::ViteModulePreloadPolyfillPlugin as rolldown_plugin::plugin::Plugin>::load::{closure#0}` |         4 | 1.4 KiB |
|   1.2 KiB | <0.1% | `<rolldown::module_loader::module_task::ModuleTask<rolldown_fs::os::OsFileSystem>>::load_source::{closure#0}::{closure#0}` |        36 | 1.7 KiB |
|   1.1 KiB | <0.1% | `<&regress::insn::Insn as core::fmt::Debug>::fmt` |        85 | 1.5 KiB |
|   1.0 KiB | <0.1% | `<rolldown_binding::options::plugin::binding_plugin_options::BindingPluginOptions as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` |        88 | 1.1 KiB |
|     979 B | <0.1% | `<rolldown_plugin_vite_resolve::vite_resolve_plugin::ViteResolvePlugin as rolldown_plugin::plugin::Plugin>::resolve_id::{closure#0}` |        31 | 1.3 KiB |
|     978 B | <0.1% | `<rolldown::stages::generate_stage::GenerateStage>::generate::{closure#0}::{closure#0}` |        63 | 1.9 KiB |
|     941 B | <0.1% | `<rolldown_binding::options::binding_transform_options::BindingEnhancedTransformResult>::from_enhanced_transform_result` |        33 |   981 B |
|     909 B | <0.1% | `<rolldown::module_loader::module_loader::ModuleLoader<rolldown_fs::os::OsFileSystem>>::fetch_modules::{closure#0}::{closure#0}` |        79 | 3.2 KiB |
|     905 B | <0.1% | `<rolldown_dev::bundle_coordinator::BundleCoordinator>::run::{closure#0}` |        27 | 1.2 KiB |
|     879 B | <0.1% | `<rolldown_binding::generated::binding_checks_options::BindingChecksOptions as napi::bindgen_runtime::js_values::FromNapiValue>::from_napi_value` |        49 |   906 B |
|     749 B | <0.1% | `<string_wizard::magic_string::MagicString>::replace_with::<alloc::string::String>` |        14 |   824 B |
|     730 B | <0.1% | `<rolldown_dev::bundle_coordinator::BundleCoordinator>::schedule_build_if_stale::{closure#0}` |        17 |   945 B |
|     726 B | <0.1% | `rolldown::module_loader::resolve_utils::resolve_dependencies::<rolldown_fs::os::OsFileSystem>::{closure#0}` |        39 | 1.8 KiB |
|     720 B | <0.1% | `<rolldown_plugin_vite_dynamic_import_vars::ViteDynamicImportVarsPlugin as rolldown_plugin::plugin::Plugin>::load::{closure#0}` |         4 |   795 B |
|     719 B | <0.1% | `<oxc_resolver::error::ResolveError as core::fmt::Debug>::fmt` |        46 |   938 B |
|     717 B | <0.1% | `rolldown_utils::futures::block_on::<rolldown_utils::futures::block_on_spawn_all<core::iter::adapters::map::Map<alloc::vec::into_iter::IntoIter<alloc::string::String>, <rolldown_plugin_vite_dynamic_import_vars::ViteDynamicImportVarsPlugin as rolldown_plugin::plugin::Plugin>::transform::{closure#0}::{closure#0}>, core::option::Option<alloc::string::String>>::{closure#0}>::{closure#0}` |        42 | 2.2 KiB |
|     661 B | <0.1% | `<rolldown_plugin_vite_reporter::ViteReporterPlugin as rolldown_plugin::plugin::Plugin>::write_bundle::{closure#0}` |        39 | 1.7 KiB |
|     617 B | <0.1% | `<rolldown_dev::bundling_task::BundlingTask>::run::{closure#0}` |        18 |   690 B |

## Dynamic relocations

_every pointer kept in data is a slot the loader fills at start; the records are compressed, so the slot's own 8 B is the cost — offsets instead of pointers remove it_

- 10.2 KiB (0.1%) in relocation records for 38212 pointer slots (298.5 KiB of slots).
